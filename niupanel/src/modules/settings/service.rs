use super::models::{BackupManifest, BackupOptions};
use chrono::{Local, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::{error, info, warn};
use niupanel_core::settings::{SYSTEM_LOG_RETENTION, SettingsManager};
use niupanel_entity::{environments, settings, task_runs, tasks, variables};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use tar::Builder;
use tokio::fs;
use walkdir::{DirEntry, WalkDir};

// --- Progress Tracking ---

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MaintenanceStatus {
    pub status: String, // "pending", "processing", "completed", "error"
    pub progress: u8,
    pub message: String,
    pub filename: Option<String>,
}

static MAINTENANCE_STATUS: OnceLock<Arc<RwLock<MaintenanceStatus>>> = OnceLock::new();

fn get_status_ptr() -> &'static Arc<RwLock<MaintenanceStatus>> {
    MAINTENANCE_STATUS.get_or_init(|| {
        Arc::new(RwLock::new(MaintenanceStatus {
            status: "pending".to_string(),
            progress: 0,
            message: "准备中".to_string(),
            filename: None,
        }))
    })
}

pub fn get_maintenance_status() -> MaintenanceStatus {
    get_status_ptr().read().unwrap().clone()
}

fn update_status(status: &str, progress: u8, message: &str, filename: Option<String>) {
    let mut s = get_status_ptr().write().unwrap();
    s.status = status.to_string();
    s.progress = progress;
    s.message = message.to_string();
    if filename.is_some() {
        s.filename = filename;
    }
}

async fn read_backup_json<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let bytes = fs::read(path).await.map_err(AppError::Io)?;
    serde_json::from_slice(&bytes).map_err(|e| {
        AppError::ValidationError(format!("备份文件 '{}' 解析失败: {}", path.display(), e))
    })
}

// --- Log Cleanup Logic ---

fn is_old_log_file(entry: &DirEntry, threshold: chrono::DateTime<Utc>) -> bool {
    let path = entry.path();
    if !path.is_file() {
        return false;
    }
    if path.extension().and_then(|s| s.to_str()) != Some("log") {
        return false;
    }

    if let Ok(metadata) = entry.metadata() {
        if let Ok(modified) = metadata.modified() {
            let modified_dt: chrono::DateTime<Utc> = modified.into();
            return modified_dt < threshold;
        }
    }
    false
}

pub async fn cleanup_logs(db: &DatabaseConnection, days: i64) -> Result<u64> {
    let threshold = Utc::now() - chrono::Duration::days(days);
    let logs_dir = Config::global().logs_dir.clone();

    let count = tokio::task::spawn_blocking(move || -> Result<u64> {
        let logs_path = Path::new(&logs_dir);
        if !logs_path.exists() {
            return Ok(0);
        }

        let deleted_count = WalkDir::new(logs_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| is_old_log_file(e, threshold))
            .map(|file| {
                let path = file.path();
                if let Err(e) = std::fs::remove_file(path) {
                    warn!("无法删除日志文件 {:?}: {}", path, e);
                    0
                } else {
                    1
                }
            })
            .sum();
        Ok(deleted_count)
    })
    .await
    .map_err(|e| AppError::Generic(format!("日志清理任务失败: {}", e)))??;

    if count > 0 {
        info!("已清理 {} 个过期日志文件 (早于 {} 天)。", count, days);
    }

    let dt: chrono::DateTime<chrono::FixedOffset> = threshold.into();
    let db_deleted = task_runs::Entity::delete_many()
        .filter(task_runs::Column::StartedAt.lt(dt))
        .exec(db)
        .await?;

    if db_deleted.rows_affected > 0 {
        info!(
            "已从数据库清理 {} 条过期的运行记录。",
            db_deleted.rows_affected
        );
    }

    Ok(count)
}

pub async fn start_log_cleanup_scheduler(db: DatabaseConnection) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            interval.tick().await;
            let days_str = SettingsManager::get(&db, SYSTEM_LOG_RETENTION)
                .await
                .unwrap_or_default();
            let days = days_str.parse::<i64>().unwrap_or(30);

            if let Err(e) = cleanup_logs(&db, days).await {
                error!("定时清理日志失败: {}", e);
            }
        }
    });
}

// --- Backup & Restore Logic ---

pub async fn list_backups() -> Result<Vec<String>> {
    let backup_dir = Path::new(&Config::global().backup_dir);
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(backup_dir).await.map_err(AppError::Io)?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(AppError::Io)? {
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".tar.gz") {
                files.push(name.to_string());
            }
        }
    }
    files.sort_by(|a, b| b.cmp(a)); // Newest first
    Ok(files)
}

pub async fn delete_backup(filename: &str) -> Result<()> {
    let path = Path::new(&Config::global().backup_dir).join(filename);
    if !path.exists() {
        return Err(AppError::NotFound("备份文件不存在".to_string()));
    }
    fs::remove_file(path).await.map_err(AppError::Io)?;
    info!("备份文件已删除: {}", filename);
    Ok(())
}

pub async fn start_backup_task(db: DatabaseConnection, options: BackupOptions) -> Result<()> {
    update_status("processing", 0, "正在初始化备份任务...", None);
    tokio::spawn(async move {
        if let Err(e) = create_system_backup(&db, options).await {
            error!("后台备份任务失败: {}", e);
            update_status("error", 0, &format!("错误: {}", e), None);
        }
    });
    Ok(())
}

pub async fn create_system_backup(
    db: &DatabaseConnection,
    options: BackupOptions,
) -> Result<PathBuf> {
    update_status("processing", 10, "正在导出数据...", None);

    let backup_dir = Path::new(&Config::global().backup_dir);
    if !backup_dir.exists() {
        fs::create_dir_all(backup_dir).await.map_err(AppError::Io)?;
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("niupanel_backup_{}.tar.gz", timestamp);
    let backup_path = backup_dir.join(&filename);

    let temp_dir_handle = tempfile::tempdir().map_err(AppError::Io)?;
    let temp_dir = temp_dir_handle.path();

    // 1. 任务数据 (跟随任务导出脚本变量)
    if options.tasks {
        let tasks_data = tasks::Entity::find().all(db).await?;
        fs::write(
            temp_dir.join("tasks.json"),
            serde_json::to_vec(&tasks_data).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;

        let task_vars = variables::Entity::find()
            .filter(variables::Column::Scope.eq("Script"))
            .order_by_asc(variables::Column::ScopeId)
            .order_by_asc(variables::Column::Id)
            .all(db)
            .await?;
        fs::write(
            temp_dir.join("task_variables.json"),
            serde_json::to_vec(&task_vars).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;
    }

    // 2. 全局变量 (非 Script 作用域)
    if options.variables {
        let global_vars = variables::Entity::find()
            .filter(variables::Column::Scope.ne("Script"))
            .order_by_asc(variables::Column::Key)
            .order_by_asc(variables::Column::Id)
            .all(db)
            .await?;
        fs::write(
            temp_dir.join("global_variables.json"),
            serde_json::to_vec(&global_vars).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;
    }

    // 3. 设置
    if options.settings {
        let sets = settings::Entity::find().all(db).await?;
        fs::write(
            temp_dir.join("settings.json"),
            serde_json::to_vec(&sets).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;
    }

    // 4. 环境配置 (元数据)
    if options.environments {
        let envs = environments::Entity::find().all(db).await?;
        fs::write(
            temp_dir.join("environments.json"),
            serde_json::to_vec(&envs).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;
    }

    // 5. TG 配置
    if options.telegram {
        let tg_cmds = niupanel_entity::tg_commands::Entity::find().all(db).await?;
        let tg_flows = niupanel_entity::tg_workflows::Entity::find()
            .all(db)
            .await?;
        fs::write(
            temp_dir.join("tg_commands.json"),
            serde_json::to_vec(&tg_cmds).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;
        fs::write(
            temp_dir.join("tg_workflows.json"),
            serde_json::to_vec(&tg_flows).unwrap(),
        )
        .await
        .map_err(AppError::Io)?;
    }

    update_status("processing", 40, "正在压缩备份文件...", None);

    let manifest = BackupManifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now().timestamp(),
        options: options.clone(),
        files: vec![],
    };
    fs::write(
        temp_dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .await
    .map_err(AppError::Io)?;

    let dest_path = backup_path.clone();
    let temp_dir_path = temp_dir.to_path_buf();
    let scripts_path_str = Config::global().scripts_dir.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::create(&dest_path).map_err(AppError::Io)?;
        let mut tar = Builder::new(GzEncoder::new(file, Compression::default()));

        tar.append_dir_all("data_json", &temp_dir_path)
            .map_err(AppError::Io)?;

        if options.tasks {
            let p = Path::new(&scripts_path_str);
            if p.exists() {
                for entry in WalkDir::new(p).into_iter().filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let rel_path = path.strip_prefix(p).unwrap();
                    let path_str = rel_path.to_string_lossy();
                    if path_str.contains("node_modules")
                        || path_str.contains("venv")
                        || path_str.contains("__pycache__")
                        || path_str.contains(".git")
                    {
                        continue;
                    }
                    if path.is_file() {
                        let tar_path = Path::new("scripts").join(rel_path);
                        tar.append_path_with_name(path, tar_path)
                            .map_err(AppError::Io)?;
                    }
                }
            }
        }

        tar.finish().map_err(AppError::Io)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::Generic(e.to_string()))??;

    update_status("completed", 100, "备份完成", Some(filename));
    info!("备份创建完成: {}", backup_path.display());
    Ok(backup_path)
}

pub async fn start_restore_task(db: DatabaseConnection, source_path: PathBuf) -> Result<()> {
    update_status("processing", 0, "正在初始化恢复任务...", None);
    tokio::spawn(async move {
        if let Err(e) = perform_restore(&db, source_path).await {
            error!("后台恢复任务失败: {}", e);
            update_status("error", 0, &format!("恢复失败: {}", e), None);
        }
    });
    Ok(())
}

pub async fn perform_restore(db: &DatabaseConnection, source_path: PathBuf) -> Result<()> {
    update_status("processing", 10, "正在解压备份包...", None);

    let temp_extract_handle = tempfile::tempdir().map_err(AppError::Io)?;
    let temp_extract = temp_extract_handle.path();

    let source_clone = source_path.clone();
    let extract_path = temp_extract.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::open(&source_clone).map_err(AppError::Io)?;
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        archive.unpack(&extract_path).map_err(AppError::Io)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::Generic(e.to_string()))??;

    let manifest_path = temp_extract.join("data_json/manifest.json");
    if !manifest_path.exists() {
        return Err(AppError::ValidationError(
            "备份包无效: 缺少 manifest.json".to_string(),
        ));
    }
    let manifest: BackupManifest = serde_json::from_slice(&fs::read(manifest_path).await?)
        .map_err(|e| AppError::Serialization(e.to_string()))?;

    // 1. 恢复设置
    if manifest.options.settings {
        update_status("processing", 30, "正在恢复系统设置...", None);
        let path = temp_extract.join("data_json/settings.json");
        if path.exists() {
            let sets: Vec<niupanel_entity::settings::Model> = read_backup_json(&path).await?;
            for s in sets {
                let active = s.into_active_model();
                let _ = niupanel_entity::settings::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::settings::Column::Key,
                        )
                        .update_columns([
                            niupanel_entity::settings::Column::Value,
                            niupanel_entity::settings::Column::Category,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }
    }

    // 2. 恢复全局变量
    if manifest.options.variables {
        update_status("processing", 50, "正在同步全局环境变量...", None);
        let path = temp_extract.join("data_json/global_variables.json");
        if path.exists() {
            let vars: Vec<niupanel_entity::variables::Model> = read_backup_json(&path).await?;
            for v in vars {
                let active = v.into_active_model();
                let _ = niupanel_entity::variables::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::variables::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::variables::Column::Key,
                            niupanel_entity::variables::Column::Value,
                            niupanel_entity::variables::Column::Scope,
                            niupanel_entity::variables::Column::ScopeId,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }
    }

    // 3. 恢复任务及脚本 (包含脚本变量)
    if manifest.options.tasks {
        update_status("processing", 70, "正在同步任务及脚本变量...", None);
        let path = temp_extract.join("data_json/tasks.json");
        if path.exists() {
            let tasks: Vec<niupanel_entity::tasks::Model> = read_backup_json(&path).await?;
            for t in tasks {
                let active = t.into_active_model();
                let _ = niupanel_entity::tasks::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(niupanel_entity::tasks::Column::Id)
                            .update_columns([
                                niupanel_entity::tasks::Column::Name,
                                niupanel_entity::tasks::Column::Path,
                                niupanel_entity::tasks::Column::Command,
                                niupanel_entity::tasks::Column::Description,
                                niupanel_entity::tasks::Column::CronSchedule,
                                niupanel_entity::tasks::Column::Enabled,
                                niupanel_entity::tasks::Column::Notify,
                                niupanel_entity::tasks::Column::Tags,
                                niupanel_entity::tasks::Column::EnvType,
                                niupanel_entity::tasks::Column::EnvVersion,
                                niupanel_entity::tasks::Column::Requirements,
                                niupanel_entity::tasks::Column::CpuLimit,
                                niupanel_entity::tasks::Column::TimeoutSec,
                                niupanel_entity::tasks::Column::MemoryLimit,
                                niupanel_entity::tasks::Column::IsPinned,
                                niupanel_entity::tasks::Column::TriggerNextTasks,
                                niupanel_entity::tasks::Column::ImportSource,
                                niupanel_entity::tasks::Column::RemoteTaskId,
                                niupanel_entity::tasks::Column::ShareCode,
                                niupanel_entity::tasks::Column::RandomConfig,
                            ])
                            .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }

        // 恢复脚本级变量
        let path_vars = temp_extract.join("data_json/task_variables.json");
        if path_vars.exists() {
            let vars: Vec<niupanel_entity::variables::Model> = read_backup_json(&path_vars).await?;
            for v in vars {
                let active = v.into_active_model();
                let _ = niupanel_entity::variables::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::variables::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::variables::Column::Key,
                            niupanel_entity::variables::Column::Value,
                            niupanel_entity::variables::Column::Scope,
                            niupanel_entity::variables::Column::ScopeId,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }

        let src_scripts = temp_extract.join("scripts");
        if src_scripts.exists() {
            let dest_scripts = Path::new(&Config::global().scripts_dir);
            let _ = fs::create_dir_all(dest_scripts).await;
            let walk = WalkDir::new(&src_scripts);
            for entry in walk.into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let rel = entry.path().strip_prefix(&src_scripts).unwrap();
                    let target = dest_scripts.join(rel);
                    if let Some(p) = target.parent() {
                        let _ = fs::create_dir_all(p).await;
                    }
                    let _ = fs::copy(entry.path(), target).await;
                }
            }
        }
    }

    // 4. 恢复环境配置
    if manifest.options.environments {
        update_status("processing", 85, "正在同步运行环境配置...", None);
        let path = temp_extract.join("data_json/environments.json");
        if path.exists() {
            let envs: Vec<niupanel_entity::environments::Model> = read_backup_json(&path).await?;
            for e in envs {
                let active = e.into_active_model();
                let _ = niupanel_entity::environments::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::environments::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::environments::Column::Name,
                            niupanel_entity::environments::Column::EnvType,
                            niupanel_entity::environments::Column::Version,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }
    }

    // 5. 恢复 TG 配置
    if manifest.options.telegram {
        update_status("processing", 95, "正在同步 TG 机器人配置...", None);
        let path = temp_extract.join("data_json/tg_commands.json");
        if path.exists() {
            let cmds: Vec<niupanel_entity::tg_commands::Model> = read_backup_json(&path).await?;
            for c in cmds {
                let active = c.into_active_model();
                let _ = niupanel_entity::tg_commands::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::tg_commands::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::tg_commands::Column::Name,
                            niupanel_entity::tg_commands::Column::Script,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }
        let path_wf = temp_extract.join("data_json/tg_workflows.json");
        if path_wf.exists() {
            let flows: Vec<niupanel_entity::tg_workflows::Model> =
                read_backup_json(&path_wf).await?;
            for f in flows {
                let active = f.into_active_model();
                let _ = niupanel_entity::tg_workflows::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::tg_workflows::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::tg_workflows::Column::EventType,
                            niupanel_entity::tg_workflows::Column::ActionType,
                            niupanel_entity::tg_workflows::Column::ConfigJson,
                        ])
                        .to_owned(),
                    )
                    .exec(db)
                    .await;
            }
        }
    }

    update_status("completed", 100, "恢复完成，系统即将刷新", None);
    info!("系统恢复成功，更改已即时生效。");
    Ok(())
}
