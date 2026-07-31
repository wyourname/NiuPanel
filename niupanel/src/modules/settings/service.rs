use super::models::{BackupManifest, BackupOptions, LogCleanupReport};
use chrono::{Local, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use niupanel_common::config::Config;
use niupanel_common::constants::defaults;
use niupanel_common::error::{AppError, Result};
use niupanel_common::{error, info, warn};
use niupanel_core::settings::{SYSTEM_LOG_RETENTION, SYSTEM_SESSION_KEY, SettingsManager};
use niupanel_entity::{
    audit_logs, environments, settings, system_jobs, task_runs, task_status::TaskStatus, tasks,
    variables, variables_tasks,
};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use tar::Builder;
use tokio::fs;
use walkdir::WalkDir;

const MAX_BACKUP_ARCHIVE_SIZE: u64 = 512 * 1024 * 1024;
const MAX_BACKUP_UNPACKED_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_BACKUP_ENTRIES: usize = 100_000;
const MAX_SUPPORTED_BACKUP_FORMAT: u32 = 2;

// --- Progress Tracking ---

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MaintenanceStatus {
    pub status: String, // "pending", "processing", "completed", "error"
    pub progress: u8,
    pub message: String,
    pub filename: Option<String>,
}

static MAINTENANCE_STATUS: OnceLock<Arc<RwLock<MaintenanceStatus>>> = OnceLock::new();
static MAINTENANCE_LOCK: OnceLock<Arc<tokio::sync::Mutex<()>>> = OnceLock::new();

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
    get_status_ptr()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn update_status(status: &str, progress: u8, message: &str, filename: Option<String>) {
    let mut s = get_status_ptr()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    s.status = status.to_string();
    s.progress = progress;
    s.message = message.to_string();
    s.filename = filename;
}

fn acquire_maintenance_lock() -> Result<tokio::sync::OwnedMutexGuard<()>> {
    MAINTENANCE_LOCK
        .get_or_init(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
        .try_lock_owned()
        .map_err(|_| AppError::ConcurrencyLimitExceeded("已有维护任务正在执行".to_string()))
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

async fn write_backup_json<T>(path: PathBuf, value: &T) -> Result<()>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(value).map_err(AppError::Json)?;
    fs::write(path, bytes).await.map_err(AppError::Io)
}

fn should_skip_backup_script(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some("node_modules" | "venv" | "__pycache__" | ".git")
        )
    })
}

mod log_cleanup;

pub use log_cleanup::{cleanup_logs, preview_log_cleanup, start_log_cleanup_scheduler};
#[cfg(test)]
use log_cleanup::{
    load_log_cleanup_candidates, scan_expired_log_files, validate_log_retention_days,
};
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
            if validate_backup_filename(name).is_ok()
                && entry.file_type().await.map_err(AppError::Io)?.is_file()
            {
                files.push(name.to_string());
            }
        }
    }
    files.sort_by(|a, b| b.cmp(a)); // Newest first
    Ok(files)
}

pub async fn delete_backup(filename: &str) -> Result<()> {
    let path = resolve_backup_path(filename).await?;
    fs::remove_file(path).await.map_err(AppError::Io)?;
    info!("备份文件已删除: {}", filename);
    Ok(())
}

pub async fn resolve_backup_path(filename: &str) -> Result<PathBuf> {
    validate_backup_filename(filename)?;
    let backup_root = fs::canonicalize(&Config::global().backup_dir)
        .await
        .map_err(AppError::Io)?;
    let candidate = backup_root.join(filename);
    let resolved = fs::canonicalize(&candidate)
        .await
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => AppError::NotFound("备份文件不存在".to_string()),
            _ => AppError::Io(err),
        })?;
    if !resolved.starts_with(&backup_root) {
        return Err(AppError::Forbidden("备份文件路径越界".to_string()));
    }
    let metadata = fs::symlink_metadata(&candidate)
        .await
        .map_err(AppError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AppError::ValidationError(
            "备份目标必须是普通文件".to_string(),
        ));
    }
    Ok(resolved)
}

fn validate_backup_filename(filename: &str) -> Result<()> {
    let path = Path::new(filename);
    if filename.is_empty()
        || filename.len() > 255
        || !filename.starts_with("niupanel_backup_")
        || !filename.ends_with(".tar.gz")
        || !filename
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(AppError::ValidationError("无效的备份文件名".to_string()));
    }
    Ok(())
}

pub async fn start_backup_task(db: DatabaseConnection, options: BackupOptions) -> Result<()> {
    let maintenance_guard = acquire_maintenance_lock()?;
    update_status("processing", 0, "正在初始化备份任务...", None);
    tokio::spawn(async move {
        let _maintenance_guard = maintenance_guard;
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
    let filename = format!(
        "niupanel_backup_{}_{}.tar.gz",
        timestamp,
        nanoid::nanoid!(8)
    );
    let backup_path = backup_dir.join(&filename);
    let partial_path = backup_dir.join(format!(".{}.partial", filename));

    let temp_dir_handle = tempfile::tempdir().map_err(AppError::Io)?;
    let temp_dir = temp_dir_handle.path();

    // 1. 任务数据 (跟随任务导出脚本变量)
    if options.tasks {
        let tasks_data = tasks::Entity::find().all(db).await?;
        write_backup_json(temp_dir.join("tasks.json"), &tasks_data).await?;

        let task_vars = variables::Entity::find()
            .filter(variables::Column::Scope.eq("Script"))
            .order_by_asc(variables::Column::ScopeId)
            .order_by_asc(variables::Column::Id)
            .all(db)
            .await?;
        write_backup_json(temp_dir.join("task_variables.json"), &task_vars).await?;

        let task_variable_bindings = variables_tasks::Entity::find()
            .order_by_asc(variables_tasks::Column::TaskId)
            .order_by_asc(variables_tasks::Column::SortOrder)
            .all(db)
            .await?;
        write_backup_json(
            temp_dir.join("task_variable_bindings.json"),
            &task_variable_bindings,
        )
        .await?;
    }

    // 2. 全局变量 (非 Script 作用域)
    if options.variables {
        let global_vars = variables::Entity::find()
            .filter(variables::Column::Scope.ne("Script"))
            .order_by_asc(variables::Column::Key)
            .order_by_asc(variables::Column::Id)
            .all(db)
            .await?;
        write_backup_json(temp_dir.join("global_variables.json"), &global_vars).await?;
    }

    // 3. 设置
    if options.settings {
        let sets = settings::Entity::find()
            .filter(settings::Column::Key.ne(SYSTEM_SESSION_KEY))
            .all(db)
            .await?;
        write_backup_json(temp_dir.join("settings.json"), &sets).await?;
    }

    // 4. 环境配置 (元数据)
    if options.environments {
        let envs = environments::Entity::find().all(db).await?;
        write_backup_json(temp_dir.join("environments.json"), &envs).await?;
    }

    // 5. TG 配置
    if options.telegram {
        let tg_cmds = niupanel_entity::tg_commands::Entity::find().all(db).await?;
        let tg_flows = niupanel_entity::tg_workflows::Entity::find()
            .all(db)
            .await?;
        write_backup_json(temp_dir.join("tg_commands.json"), &tg_cmds).await?;
        write_backup_json(temp_dir.join("tg_workflows.json"), &tg_flows).await?;
    }

    update_status("processing", 40, "正在压缩备份文件...", None);

    let manifest = BackupManifest {
        format_version: 2,
        version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: Utc::now().timestamp(),
        options: options.clone(),
        files: vec![],
    };
    write_backup_json(temp_dir.join("manifest.json"), &manifest).await?;

    let dest_path = partial_path.clone();
    let temp_dir_path = temp_dir.to_path_buf();
    let scripts_path_str = Config::global().scripts_dir.clone();

    let archive_result = tokio::task::spawn_blocking(move || -> Result<()> {
        let mut file_options = std::fs::OpenOptions::new();
        file_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            file_options.mode(0o600);
        }
        let file = file_options.open(&dest_path).map_err(AppError::Io)?;
        let mut tar = Builder::new(GzEncoder::new(file, Compression::default()));

        tar.append_dir_all("data_json", &temp_dir_path)
            .map_err(AppError::Io)?;

        if options.tasks {
            let p = Path::new(&scripts_path_str);
            if p.exists() {
                for entry in WalkDir::new(p) {
                    let entry = entry.map_err(|err| {
                        AppError::Io(std::io::Error::other(format!("遍历脚本目录失败: {err}")))
                    })?;
                    if entry.file_type().is_symlink() {
                        continue;
                    }
                    let path = entry.path();
                    let rel_path = path
                        .strip_prefix(p)
                        .map_err(|_| AppError::ValidationError("备份脚本路径越界".to_string()))?;
                    if should_skip_backup_script(rel_path) {
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

        let encoder = tar.into_inner().map_err(AppError::Io)?;
        encoder.finish().map_err(AppError::Io)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::Generic(e.to_string()))?;
    if let Err(err) = archive_result {
        let _ = fs::remove_file(&partial_path).await;
        return Err(err);
    }
    if let Err(err) = fs::rename(&partial_path, &backup_path).await {
        let _ = fs::remove_file(&partial_path).await;
        return Err(AppError::Io(err));
    }

    update_status("completed", 100, "备份完成", Some(filename));
    info!("备份创建完成: {}", backup_path.display());
    Ok(backup_path)
}

pub async fn start_restore_task(
    db: DatabaseConnection,
    source_path: PathBuf,
    cleanup_source: bool,
) -> Result<()> {
    let maintenance_guard = acquire_maintenance_lock()?;
    update_status("processing", 0, "正在初始化恢复任务...", None);
    tokio::spawn(async move {
        let _maintenance_guard = maintenance_guard;
        if let Err(e) = perform_restore(&db, source_path.clone()).await {
            error!("后台恢复任务失败: {}", e);
            update_status("error", 0, &format!("恢复失败: {}", e), None);
        }
        if cleanup_source {
            if let Err(err) = fs::remove_file(&source_path).await {
                if err.kind() != std::io::ErrorKind::NotFound {
                    warn!("清理上传的恢复文件失败: {}", err);
                }
            }
        }
    });
    Ok(())
}

pub async fn perform_restore(db: &DatabaseConnection, source_path: PathBuf) -> Result<()> {
    update_status("processing", 10, "正在解压备份包...", None);

    let source_metadata = fs::symlink_metadata(&source_path)
        .await
        .map_err(AppError::Io)?;
    if source_metadata.file_type().is_symlink() || !source_metadata.is_file() {
        return Err(AppError::ValidationError(
            "恢复源必须是普通备份文件".to_string(),
        ));
    }
    if source_metadata.len() > MAX_BACKUP_ARCHIVE_SIZE {
        return Err(AppError::FileSizeLimitExceeded(
            "备份文件超过 512MB 限制".to_string(),
        ));
    }

    let temp_extract_handle = tempfile::tempdir().map_err(AppError::Io)?;
    let temp_extract = temp_extract_handle.path();

    let source_clone = source_path.clone();
    let extract_path = temp_extract.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<()> {
        extract_backup_archive(&source_clone, &extract_path)
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
    if !(1..=MAX_SUPPORTED_BACKUP_FORMAT).contains(&manifest.format_version) {
        return Err(AppError::ValidationError(format!(
            "不支持的备份格式版本: {}",
            manifest.format_version
        )));
    }

    // 1. 恢复设置
    if manifest.options.settings {
        update_status("processing", 30, "正在恢复系统设置...", None);
        let path = temp_extract.join("data_json/settings.json");
        if path.exists() {
            let txn = db.begin().await?;
            let sets: Vec<niupanel_entity::settings::Model> = read_backup_json(&path).await?;
            for s in sets
                .into_iter()
                .filter(|setting| setting.key != SYSTEM_SESSION_KEY)
            {
                let active = s.into_active_model();
                niupanel_entity::settings::Entity::insert(active)
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
                    .exec(&txn)
                    .await?;
            }
            txn.commit().await?;
        }
    }

    // 2. 恢复全局变量
    if manifest.options.variables {
        update_status("processing", 50, "正在同步全局环境变量...", None);
        let path = temp_extract.join("data_json/global_variables.json");
        if path.exists() {
            let txn = db.begin().await?;
            let vars: Vec<niupanel_entity::variables::Model> = read_backup_json(&path).await?;
            for v in vars {
                let active = v.into_active_model();
                niupanel_entity::variables::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::variables::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::variables::Column::Key,
                            niupanel_entity::variables::Column::Value,
                            niupanel_entity::variables::Column::Remarks,
                            niupanel_entity::variables::Column::Enabled,
                            niupanel_entity::variables::Column::Scope,
                            niupanel_entity::variables::Column::ScopeId,
                            niupanel_entity::variables::Column::SortOrder,
                            niupanel_entity::variables::Column::UpdatedAt,
                        ])
                        .to_owned(),
                    )
                    .exec(&txn)
                    .await?;
            }
            txn.commit().await?;
        }
    }

    // 3. 恢复任务及脚本 (包含脚本变量)
    if manifest.options.tasks {
        update_status("processing", 70, "正在同步任务及脚本变量...", None);
        let txn = db.begin().await?;
        let path = temp_extract.join("data_json/tasks.json");
        if path.exists() {
            let tasks: Vec<niupanel_entity::tasks::Model> = read_backup_json(&path).await?;
            for t in tasks {
                let active = t.into_active_model();
                niupanel_entity::tasks::Entity::insert(active)
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
                    .exec(&txn)
                    .await?;
            }
        }

        // 恢复脚本级变量
        let path_vars = temp_extract.join("data_json/task_variables.json");
        let mut restored_variable_ids = Vec::new();
        if path_vars.exists() {
            let vars: Vec<niupanel_entity::variables::Model> = read_backup_json(&path_vars).await?;
            for v in vars {
                restored_variable_ids.push(v.id);
                let active = v.into_active_model();
                niupanel_entity::variables::Entity::insert(active)
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::column(
                            niupanel_entity::variables::Column::Id,
                        )
                        .update_columns([
                            niupanel_entity::variables::Column::Key,
                            niupanel_entity::variables::Column::Value,
                            niupanel_entity::variables::Column::Remarks,
                            niupanel_entity::variables::Column::Enabled,
                            niupanel_entity::variables::Column::Scope,
                            niupanel_entity::variables::Column::ScopeId,
                            niupanel_entity::variables::Column::SortOrder,
                            niupanel_entity::variables::Column::UpdatedAt,
                        ])
                        .to_owned(),
                    )
                    .exec(&txn)
                    .await?;
            }
        }

        if !restored_variable_ids.is_empty() {
            let restored_variable_id_set = restored_variable_ids
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            variables_tasks::Entity::delete_many()
                .filter(variables_tasks::Column::VariableId.is_in(restored_variable_ids.clone()))
                .exec(&txn)
                .await?;

            let bindings_path = temp_extract.join("data_json/task_variable_bindings.json");
            let bindings = if bindings_path.exists() {
                read_backup_json::<Vec<variables_tasks::Model>>(&bindings_path).await?
            } else {
                // Backward compatibility for v1 backups, which only stored scope_id.
                variables::Entity::find()
                    .filter(variables::Column::Id.is_in(restored_variable_ids.clone()))
                    .filter(variables::Column::Scope.eq("Script"))
                    .all(&txn)
                    .await?
                    .into_iter()
                    .filter_map(|variable| {
                        variable.scope_id.map(|task_id| variables_tasks::Model {
                            variable_id: variable.id,
                            task_id,
                            sort_order: variable.sort_order,
                        })
                    })
                    .collect()
            };

            for binding in bindings
                .into_iter()
                .filter(|binding| restored_variable_id_set.contains(&binding.variable_id))
            {
                variables_tasks::Entity::insert(binding.into_active_model())
                    .on_conflict(
                        sea_orm::sea_query::OnConflict::columns([
                            variables_tasks::Column::VariableId,
                            variables_tasks::Column::TaskId,
                        ])
                        .update_column(variables_tasks::Column::SortOrder)
                        .to_owned(),
                    )
                    .exec(&txn)
                    .await?;
            }
        }
        txn.commit().await?;

        let src_scripts = temp_extract.join("scripts");
        if src_scripts.exists() {
            let dest_scripts = Path::new(&Config::global().scripts_dir);
            fs::create_dir_all(dest_scripts)
                .await
                .map_err(AppError::Io)?;
            let walk = WalkDir::new(&src_scripts);
            for entry in walk {
                let entry = entry.map_err(|err| {
                    AppError::Io(
                        err.into_io_error()
                            .unwrap_or_else(|| std::io::Error::other("遍历恢复脚本失败")),
                    )
                })?;
                if entry.file_type().is_file() {
                    let rel = entry
                        .path()
                        .strip_prefix(&src_scripts)
                        .map_err(|_| AppError::ValidationError("恢复脚本路径越界".to_string()))?;
                    let target = dest_scripts.join(rel);
                    if let Some(p) = target.parent() {
                        fs::create_dir_all(p).await.map_err(AppError::Io)?;
                    }
                    fs::copy(entry.path(), target).await.map_err(AppError::Io)?;
                }
            }
        }
    }

    // 4. 恢复环境配置
    if manifest.options.environments {
        update_status("processing", 85, "正在同步运行环境配置...", None);
        let path = temp_extract.join("data_json/environments.json");
        if path.exists() {
            let txn = db.begin().await?;
            let envs: Vec<niupanel_entity::environments::Model> = read_backup_json(&path).await?;
            for e in envs {
                let active = e.into_active_model();
                niupanel_entity::environments::Entity::insert(active)
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
                    .exec(&txn)
                    .await?;
            }
            txn.commit().await?;
        }
    }

    // 5. 恢复 TG 配置
    if manifest.options.telegram {
        update_status("processing", 95, "正在同步 TG 机器人配置...", None);
        let txn = db.begin().await?;
        let path = temp_extract.join("data_json/tg_commands.json");
        if path.exists() {
            let cmds: Vec<niupanel_entity::tg_commands::Model> = read_backup_json(&path).await?;
            for c in cmds {
                let active = c.into_active_model();
                niupanel_entity::tg_commands::Entity::insert(active)
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
                    .exec(&txn)
                    .await?;
            }
        }
        let path_wf = temp_extract.join("data_json/tg_workflows.json");
        if path_wf.exists() {
            let flows: Vec<niupanel_entity::tg_workflows::Model> =
                read_backup_json(&path_wf).await?;
            for f in flows {
                let active = f.into_active_model();
                niupanel_entity::tg_workflows::Entity::insert(active)
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
                    .exec(&txn)
                    .await?;
            }
        }
        txn.commit().await?;
    }

    update_status("completed", 100, "恢复完成，系统即将刷新", None);
    info!("系统恢复成功，更改已即时生效。");
    Ok(())
}

fn extract_backup_archive(source_path: &Path, extract_path: &Path) -> Result<()> {
    let file = std::fs::File::open(source_path).map_err(AppError::Io)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut entry_count = 0usize;
    let mut unpacked_size = 0u64;

    for entry in archive.entries().map_err(AppError::Io)? {
        let mut entry = entry.map_err(AppError::Io)?;
        entry_count = entry_count.saturating_add(1);
        if entry_count > MAX_BACKUP_ENTRIES {
            return Err(AppError::FileSizeLimitExceeded(format!(
                "备份条目数超过 {} 个限制",
                MAX_BACKUP_ENTRIES
            )));
        }

        let path = entry.path().map_err(AppError::Io)?.into_owned();
        validate_backup_archive_path(&path)?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(AppError::ValidationError(format!(
                "备份包包含不允许的链接或特殊文件: {}",
                path.display()
            )));
        }

        let entry_size = entry.header().size().map_err(AppError::Io)?;
        unpacked_size = unpacked_size.saturating_add(entry_size);
        if unpacked_size > MAX_BACKUP_UNPACKED_SIZE {
            return Err(AppError::FileSizeLimitExceeded(
                "备份解压后超过 2GB 限制".to_string(),
            ));
        }

        let mode = if entry_type.is_dir() {
            0o700
        } else if entry.header().mode().unwrap_or_default() & 0o111 != 0 {
            0o700
        } else {
            0o600
        };
        if !entry.unpack_in(extract_path).map_err(AppError::Io)? {
            return Err(AppError::ValidationError(format!(
                "备份条目路径越界: {}",
                path.display()
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                extract_path.join(&path),
                std::fs::Permissions::from_mode(mode),
            )
            .map_err(AppError::Io)?;
        }
    }
    Ok(())
}

fn validate_backup_archive_path(path: &Path) -> Result<()> {
    let mut components = path.components();
    let root = match components.next() {
        Some(Component::Normal(root)) => root,
        _ => {
            return Err(AppError::ValidationError("备份包包含无效路径".to_string()));
        }
    };
    if root != "data_json" && root != "scripts" {
        return Err(AppError::ValidationError(format!(
            "备份包包含未知根目录: {}",
            path.display()
        )));
    }
    if !components.all(|component| matches!(component, Component::Normal(_))) {
        return Err(AppError::ValidationError(format!(
            "备份包包含路径穿越: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::{Migrator, MigratorTrait};
    use sea_orm::{ConnectOptions, ConnectionTrait, Database};

    #[test]
    fn backup_filename_validation_rejects_traversal_and_unexpected_extensions() {
        assert!(validate_backup_filename("niupanel_backup_20260728_abcd1234.tar.gz").is_ok());
        for invalid in [
            "",
            "../backup.tar.gz",
            "nested/backup.tar.gz",
            "backup.zip",
            ".hidden.tar.gz",
            "backup tar.gz",
            "备份.tar.gz",
        ] {
            assert!(
                validate_backup_filename(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[test]
    fn backup_archive_paths_are_confined_to_known_roots() {
        assert!(validate_backup_archive_path(Path::new("data_json/settings.json")).is_ok());
        assert!(validate_backup_archive_path(Path::new("scripts/nested/task.sh")).is_ok());
        for invalid in [
            "../scripts/task.sh",
            "/etc/passwd",
            "unknown/file",
            "scripts/../data_json/settings.json",
        ] {
            assert!(
                validate_backup_archive_path(Path::new(invalid)).is_err(),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[test]
    fn backup_script_filter_matches_components_not_substrings() {
        assert!(should_skip_backup_script(Path::new(
            "project/node_modules/pkg/index.js"
        )));
        assert!(should_skip_backup_script(Path::new("repo/.git/config")));
        assert!(should_skip_backup_script(Path::new(
            "python/__pycache__/task.pyc"
        )));
        assert!(!should_skip_backup_script(Path::new(
            "my_node_modules/task.js"
        )));
        assert!(!should_skip_backup_script(Path::new("venv_setup/task.sh")));
    }

    #[test]
    fn log_retention_validation_uses_shared_bounds() {
        assert!(validate_log_retention_days(defaults::LOG_RETENTION_MIN_DAYS).is_ok());
        assert!(validate_log_retention_days(defaults::LOG_RETENTION_MAX_DAYS).is_ok());
        assert!(validate_log_retention_days(defaults::LOG_RETENTION_MIN_DAYS - 1).is_err());
        assert!(validate_log_retention_days(defaults::LOG_RETENTION_MAX_DAYS + 1).is_err());
    }

    #[test]
    fn log_file_scan_preserves_protected_and_current_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("logs");
        let task_dir = root.join("1");
        std::fs::create_dir_all(&task_dir).unwrap();

        let old_server = root.join("niupanel.2020-01-01.log");
        let current_server = root.join("niupanel.2020-01-02.log");
        let protected_task = task_dir.join("protected.log");
        let orphan_task = task_dir.join("orphan.log");
        for path in [&old_server, &current_server, &protected_task, &orphan_task] {
            std::fs::write(path, b"log data").unwrap();
        }

        let old_time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(10);
        let newer_time = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(20);
        for path in [&old_server, &protected_task, &orphan_task] {
            std::fs::File::open(path)
                .unwrap()
                .set_modified(old_time)
                .unwrap();
        }
        std::fs::File::open(&current_server)
            .unwrap()
            .set_modified(newer_time)
            .unwrap();

        let canonical_root = root.canonicalize().unwrap();
        let protected_paths = HashSet::from([protected_task.canonicalize().unwrap()]);
        let mut files = BTreeMap::new();
        let mut warnings = Vec::new();
        scan_expired_log_files(
            Some(&canonical_root),
            Utc::now(),
            true,
            &protected_paths,
            &HashSet::new(),
            &mut files,
            &mut warnings,
        );

        assert!(files.contains_key(&old_server.canonicalize().unwrap()));
        assert!(files.contains_key(&orphan_task.canonicalize().unwrap()));
        assert!(!files.contains_key(&current_server.canonicalize().unwrap()));
        assert!(!files.contains_key(&protected_task.canonicalize().unwrap()));
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn cleanup_candidates_only_include_expired_terminal_records() {
        let mut options = ConnectOptions::new("sqlite::memory:");
        options.max_connections(1).min_connections(1);
        let db = Database::connect(options).await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO users (username, password_hash, role) \
             VALUES ('cleanup-tester', 'hash', 'Admin')",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO tasks (name, path, env_type, user_id) \
             VALUES \
             ('cleanup-task', '', 'shell', 1), \
             ('inactive-task', '', 'shell', 1)",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO task_runs \
             (id, task_id, status, started_at, ended_at, log_path) VALUES \
             (1, 1, 'Finished', '2023-01-01T00:00:00Z', '2023-01-02T00:00:00Z', 'expired.log'), \
             (2, 1, 'Running', '2023-01-01T00:00:00Z', NULL, 'active.log'), \
             (3, 1, 'Finished', '2026-01-01T00:00:00Z', '2026-01-02T00:00:00Z', 'recent.log'), \
             (4, 2, 'Finished', '2023-01-01T00:00:00Z', '2023-01-02T00:00:00Z', 'inactive-latest.log')",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO system_jobs \
             (id, name, status, created_at, ended_at, log_path) VALUES \
             (1, 'expired-job', 'Failed', '2023-01-01T00:00:00Z', '2023-01-02T00:00:00Z', 'expired-job.log'), \
             (2, 'active-job', 'Running', '2023-01-01T00:00:00Z', NULL, 'active-job.log')",
        )
        .await
        .unwrap();
        db.execute_unprepared(
            "INSERT INTO audit_logs (action, resource, created_at) VALUES \
             ('old', 'settings', '2023-01-01T00:00:00Z'), \
             ('recent', 'settings', '2026-01-01T00:00:00Z')",
        )
        .await
        .unwrap();

        let threshold = chrono::DateTime::parse_from_rfc3339("2025-01-01T00:00:00Z").unwrap();
        let candidates = load_log_cleanup_candidates(&db, threshold).await.unwrap();

        assert_eq!(
            candidates
                .task_runs
                .iter()
                .map(|run| run.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            candidates
                .system_jobs
                .iter()
                .map(|job| job.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(candidates.audit_logs, 1);
        assert!(
            candidates
                .protected_paths
                .contains(&"active.log".to_string())
        );
        assert!(
            candidates
                .protected_paths
                .contains(&"recent.log".to_string())
        );
        assert!(
            candidates
                .protected_paths
                .contains(&"inactive-latest.log".to_string())
        );
        assert!(
            candidates
                .protected_paths
                .contains(&"active-job.log".to_string())
        );
    }
}
