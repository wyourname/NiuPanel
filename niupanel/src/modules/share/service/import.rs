use super::common::read_package;
use crate::modules::auth::service::AuthenticatedUser;
use crate::modules::share::models::{
    ImportFailure, ImportHistoryItem, ImportSourceGroup, ImportState, ImportStatusResponse,
    ImportSummary, NiuPackage, StagedImportMeta, TaskData, TaskMeta,
};
use niupanel_common::config::Config;
use niupanel_common::download::{DownloadEvent, DownloadOptions, download_with_resume};
use niupanel_common::error::{AppError, Result};
use niupanel_common::filesystem::{get_relative_path, resolve_path};
use niupanel_common::{error, info};
use niupanel_entity::{tasks, variables};
use reqwest::header::{HeaderMap, HeaderValue};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use tokio::fs::{self, create_dir_all, remove_file};
use walkdir::WalkDir;

async fn setup_import_paths(staging_id: &str) -> (PathBuf, PathBuf, PathBuf) {
    let import_dir = Path::new(&Config::global().share_import_dir);
    (
        import_dir.join(format!("{}.npack", staging_id)),
        import_dir.join(format!("{}.status", staging_id)),
        import_dir.join(format!("{}.meta", staging_id)),
    )
}

pub async fn stage_import(
    client: &reqwest::Client,
    url: &str,
    password: Option<String>,
) -> Result<String> {
    let staging_id = nanoid::nanoid!(12);
    let (dest_path, status_path, meta_path) = setup_import_paths(&staging_id).await;

    if let Some(parent) = dest_path.parent() {
        create_dir_all(parent).await.map_err(AppError::Io)?;
    }

    update_status_file(&status_path, ImportState::Pending, None, 0).await;

    info!("提交导入任务 (ID: {}), URL: {}", staging_id, url);
    let (client, url, sid) = (client.clone(), url.to_string(), staging_id.clone());
    tokio::spawn(async move {
        let _ = perform_download(
            client,
            url,
            dest_path,
            status_path,
            meta_path,
            password,
            sid,
        )
        .await;
    });

    Ok(staging_id)
}

pub async fn retry_import(
    client: &reqwest::Client,
    staging_id: &str,
    url: &str,
    password: Option<String>,
) -> Result<()> {
    let (dest_path, status_path, meta_path) = setup_import_paths(staging_id).await;
    update_status_file(&status_path, ImportState::Pending, None, 0).await;

    info!("重试导入任务 (ID: {}), URL: {}", staging_id, url);
    let (client, url, sid) = (client.clone(), url.to_string(), staging_id.to_string());
    tokio::spawn(async move {
        let _ = perform_download(
            client,
            url,
            dest_path,
            status_path,
            meta_path,
            password,
            sid,
        )
        .await;
    });
    Ok(())
}

async fn update_status_file(path: &Path, state: ImportState, msg: Option<String>, prog: u8) {
    let status = ImportStatusResponse {
        state,
        message: msg,
        progress: prog,
    };
    if let Ok(json) = serde_json::to_string(&status) {
        let _ = fs::write(path, json).await;
    }
}

async fn perform_download(
    client: reqwest::Client,
    url: String,
    dest_path: PathBuf,
    status_path: PathBuf,
    meta_path: PathBuf,
    password: Option<String>,
    staging_id: String,
) -> Result<()> {
    update_status_file(&status_path, ImportState::Downloading, None, 0).await;
    let mut headers = HeaderMap::new();
    if let Some(password) = password {
        let header_value = HeaderValue::from_str(&password)
            .map_err(|_| AppError::ValidationError("分享密码包含非法请求头字符".to_string()))?;
        headers.insert("X-Share-Password", header_value);
    }

    if let Err(err) = download_with_resume(
        &client,
        &url,
        &dest_path,
        None,
        DownloadOptions {
            headers,
            timeout: None,
            retries: 3,
            retry_delay: std::time::Duration::from_secs(2),
        },
        || false,
        |event| {
            let status_path = status_path.clone();
            let staging_id = staging_id.clone();
            async move {
                match event {
                    DownloadEvent::Attempt { resumed_from, .. } => {
                        if resumed_from > 0 {
                            info!(
                                "尝试断点续传 (ID: {}), 已下载: {} 字节",
                                staging_id, resumed_from
                            );
                        }
                    }
                    DownloadEvent::Progress(progress) => {
                        if progress.total_size > 0 {
                            update_status_file(
                                &status_path,
                                ImportState::Downloading,
                                None,
                                progress.percent,
                            )
                            .await;
                        }
                    }
                    DownloadEvent::Completed { .. } => {}
                }
            }
        },
    )
    .await
    {
        error!("下载请求失败 (ID: {}): {}", staging_id, err);
        update_status_file(&status_path, ImportState::Error, Some(err.to_string()), 0).await;
        return Err(err);
    }

    let meta = StagedImportMeta {
        source_url: url,
        staging_id: staging_id.clone(),
        created_at: chrono::Utc::now().timestamp(),
    };
    fs::write(&meta_path, serde_json::to_string(&meta).unwrap())
        .await
        .map_err(AppError::Io)?;
    update_status_file(&status_path, ImportState::Ready, None, 100).await;
    info!("下载完成 (ID: {})", staging_id);
    Ok(())
}

pub async fn get_import_status(staging_id: &str) -> Result<ImportStatusResponse> {
    let path = Path::new(&Config::global().share_import_dir).join(format!("{}.status", staging_id));
    let content = fs::read_to_string(&path).await.map_err(AppError::Io)?;
    serde_json::from_str(&content).map_err(|_| AppError::Generic("无效的状态文件".to_string()))
}

pub async fn get_staged_package(staging_id: &str) -> Result<NiuPackage> {
    let path = Path::new(&Config::global().share_import_dir).join(format!("{}.npack", staging_id));
    read_package(
        path.to_str()
            .ok_or(AppError::Serialization("路径无效".to_string()))?,
    )
    .await
}

pub async fn finalize_import(
    db: &DatabaseConnection,
    staging_id: &str,
    selected_tasks: Option<Vec<String>>,
    user: &AuthenticatedUser,
    _ip: Option<String>,
    update_existing: bool,
) -> Result<ImportSummary> {
    let (pack_path, status_path, meta_path) = setup_import_paths(staging_id).await;
    let package = read_package(pack_path.to_str().unwrap()).await?;

    let import_source = fs::read_to_string(&meta_path)
        .await
        .ok()
        .and_then(|c| serde_json::from_str::<StagedImportMeta>(&c).ok())
        .map(|m| m.source_url)
        .unwrap_or_else(|| staging_id.to_string());

    let share_code = import_source.split('/').last().map(|s| s.to_string());
    let dir_name = share_code
        .as_ref()
        .map(|c| format!("src_{}", c))
        .unwrap_or_else(|| {
            use sha2::{Digest, Sha256};
            format!(
                "src_{}",
                hex::encode(&Sha256::digest(import_source.as_bytes())[..8])
            )
        });

    let shared_import_dir = resolve_path("import")?.join(&dir_name);
    create_dir_all(&shared_import_dir)
        .await
        .map_err(AppError::Io)?;

    info!(
        "开始确认导入 (ID: {}), 来源: {}, 目标目录: {}",
        staging_id, import_source, dir_name
    );

    let mut summary = ImportSummary::default();
    let tasks_to_import = if let Some(sel) = selected_tasks {
        package
            .tasks
            .into_iter()
            .filter(|t| sel.contains(&t.meta.name))
            .collect()
    } else {
        package.tasks
    };

    for task_data in tasks_to_import {
        match import_single_task(
            db,
            task_data.clone(),
            user,
            &shared_import_dir,
            update_existing,
            &import_source,
            share_code.clone(),
        )
        .await
        {
            Ok(true) => summary.success_count += 1,
            Ok(false) => summary.skip_count += 1,
            Err(e) => {
                error!("任务导入失败 ({}): {}", task_data.meta.name, e);
                summary.failure_count += 1;
                summary.failures.push(ImportFailure {
                    task_name: task_data.meta.name,
                    error: e.to_string(),
                });
            }
        }
    }

    let _ = remove_file(pack_path).await;
    let _ = remove_file(meta_path).await;
    let _ = remove_file(status_path).await;

    info!(
        "确认导入完成: 成功 {}, 忽略 {}, 失败 {}",
        summary.success_count, summary.skip_count, summary.failure_count
    );
    Ok(summary)
}

async fn identify_task_matching(
    db: &DatabaseConnection,
    meta: &TaskMeta,
    source: &str,
    update_existing: bool,
) -> Result<(bool, String, Option<tasks::Model>)> {
    if update_existing {
        let mut query = tasks::Entity::find().filter(tasks::Column::ImportSource.eq(source));
        if let Some(rid) = &meta.remote_task_id {
            query = query.filter(tasks::Column::RemoteTaskId.eq(rid));
        } else {
            query = query.filter(tasks::Column::Name.eq(&meta.name));
        }
        if let Some(existing) = query.one(db).await? {
            return Ok((true, existing.name.clone(), Some(existing)));
        }
    }

    let mut name = meta.name.clone();
    let mut i = 1;
    while tasks::Entity::find()
        .filter(tasks::Column::Name.eq(&name))
        .one(db)
        .await?
        .is_some()
    {
        name = format!("{}_{}", meta.name, i);
        i += 1;
    }
    Ok((false, name, None))
}

async fn process_task_files(
    task_data: &TaskData,
    temp_dir: &Path,
    final_dir: &Path,
) -> Result<(String, String)> {
    let mut final_entry = String::new();
    let mut cmd = task_data.meta.command.clone().unwrap_or_default();
    let main_file = task_data
        .main_file
        .clone()
        .or_else(|| task_data.files.first().map(|f| f.path.clone()));

    for file in &task_data.files {
        let target = temp_dir.join(&file.path);
        if !target.starts_with(temp_dir) {
            continue;
        }

        if let Some(p) = target.parent() {
            create_dir_all(p).await?;
        }
        fs::write(&target, &file.content).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target, std::fs::Permissions::from_mode(file.mode)).await;
        }

        if main_file.as_ref() == Some(&file.path) {
            if let Ok(rel) = get_relative_path(&final_dir.join(&file.path)) {
                final_entry = rel.clone();
                cmd = cmd
                    .replace(&format!("./{}", file.path), &rel)
                    .replace(&file.path, &rel);
            }
        }
    }

    if final_entry.is_empty() {
        final_entry = get_relative_path(&final_dir.to_path_buf())?;
    }

    Ok((final_entry, cmd))
}

async fn import_single_task(
    db: &DatabaseConnection,
    task_data: TaskData,
    user: &AuthenticatedUser,
    final_dir: &Path,
    update_existing: bool,
    source: &str,
    share_code: Option<String>,
) -> Result<bool> {
    let (is_update, task_name, existing) =
        identify_task_matching(db, &task_data.meta, source, update_existing).await?;

    if is_update {
        info!("正在更新现有任务: {} (来源: {})", task_name, source);
    } else {
        info!("正在创建新任务: {} (来源: {})", task_name, source);
    }

    let temp_dir_handle = tempdir().map_err(AppError::Io)?;
    let (final_path, command) =
        process_task_files(&task_data, temp_dir_handle.path(), final_dir).await?;

    if !final_dir.exists() {
        create_dir_all(final_dir).await?;
    }
    let mut moved_files = Vec::new();
    for entry in WalkDir::new(temp_dir_handle.path())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            continue;
        }
        let rel = entry.path().strip_prefix(temp_dir_handle.path()).unwrap();
        let dest = final_dir.join(rel);
        if let Some(p) = dest.parent() {
            create_dir_all(p).await?;
        }
        if fs::rename(entry.path(), &dest).await.is_err() {
            fs::copy(entry.path(), &dest).await.map_err(AppError::Io)?;
        }
        moved_files.push(dest);
    }

    let txn = db.begin().await?;
    let task_id = if is_update {
        let mut active: tasks::ActiveModel = existing.unwrap().into();
        active.command = Set(Some(command));
        active.description = Set(task_data.meta.description);
        active.env_type = Set(task_data.meta.env_type);
        active.env_version = Set(task_data.meta.env_version);
        active.cron_schedule = Set(task_data.meta.cron_schedule);
        active.requirements = Set(task_data.meta.requirements);
        active.notify = Set(task_data.meta.notify);
        active.path = Set(final_path);
        active.import_source = Set(Some(source.to_string()));
        active.remote_task_id = Set(task_data.meta.remote_task_id);
        active.share_code = Set(share_code);
        active.update(&txn).await?.id
    } else {
        tasks::ActiveModel {
            name: Set(task_name),
            path: Set(final_path),
            command: Set(Some(command)),
            description: Set(task_data.meta.description),
            env_type: Set(task_data.meta.env_type),
            env_version: Set(task_data.meta.env_version),
            cron_schedule: Set(task_data.meta.cron_schedule),
            user_id: Set(user.id),
            enabled: Set(true),
            import_source: Set(Some(source.to_string())),
            remote_task_id: Set(task_data.meta.remote_task_id),
            share_code: Set(share_code),
            ..Default::default()
        }
        .insert(&txn)
        .await?
        .id
    };

    if let Some(vars) = task_data.meta.variables {
        niupanel_entity::variables_tasks::Entity::delete_many()
            .filter(niupanel_entity::variables_tasks::Column::TaskId.eq(task_id))
            .exec(&txn)
            .await?;
        for (index, var) in vars.into_iter().enumerate() {
            let created = variables::ActiveModel {
                key: Set(var.key),
                value: Set(var.value),
                scope: Set("Script".to_string()),
                scope_id: Set(Some(task_id)),
                enabled: Set(true),
                ..Default::default()
            }
            .insert(&txn)
            .await?;

            niupanel_entity::variables_tasks::ActiveModel {
                variable_id: Set(created.id),
                task_id: Set(task_id),
                sort_order: Set(index as i32 + 1),
            }
            .insert(&txn)
            .await?;
        }
    }

    match txn.commit().await {
        Ok(_) => Ok(true),
        Err(e) => {
            error!("数据库提交失败，正在回滚文件: {}", e);
            for path in moved_files {
                let _ = remove_file(path).await;
            }
            Err(AppError::Database(e))
        }
    }
}

pub async fn delete_imported_tasks(db: &DatabaseConnection, task_id: i32) -> Result<u64> {
    let task = tasks::Entity::find_by_id(task_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("任务不存在".to_string()))?;

    let name = task.name.clone();
    let res = tasks::Entity::delete_by_id(task_id).exec(db).await?;
    if let Ok(p) = resolve_path(&task.path) {
        if p.exists() && p.starts_with(resolve_path("import")?) {
            info!("删除已导入任务的物理文件: {} (路径: {})", name, p.display());
            let _ = if p.is_dir() {
                fs::remove_dir_all(p).await
            } else {
                fs::remove_file(p).await
            };
        }
    }
    Ok(res.rows_affected)
}

pub async fn delete_imported_by_source(db: &DatabaseConnection, source: &str) -> Result<u64> {
    info!("清理来源为 {} 的所有已导入任务...", source);
    let tasks = tasks::Entity::find()
        .filter(tasks::Column::ImportSource.eq(source))
        .all(db)
        .await?;
    let import_base = resolve_path("import")?;
    for task in tasks {
        if let Ok(p) = resolve_path(&task.path) {
            if p.exists() && p.starts_with(&import_base) {
                let _ = if p.is_dir() {
                    fs::remove_dir_all(p).await
                } else {
                    fs::remove_file(p).await
                };
            }
        }
    }
    let res = tasks::Entity::delete_many()
        .filter(tasks::Column::ImportSource.eq(source))
        .exec(db)
        .await?;
    info!(
        "已成功清理来源 {} 的任务，共 {} 条记录",
        source, res.rows_affected
    );
    Ok(res.rows_affected)
}

pub async fn delete_imported_by_share_code(db: &DatabaseConnection, code: &str) -> Result<u64> {
    info!("清理 ShareCode 为 {} 的所有任务及目录...", code);
    let dir = resolve_path("import")?.join(format!("src_{}", code));
    if dir.exists() {
        info!("移除物理目录: {}", dir.display());
        let _ = fs::remove_dir_all(dir).await;
    }
    let res = tasks::Entity::delete_many()
        .filter(tasks::Column::ShareCode.eq(code))
        .exec(db)
        .await?;
    info!(
        "已清理 ShareCode {} 的任务，共 {} 条记录",
        code, res.rows_affected
    );
    Ok(res.rows_affected)
}

pub async fn get_imported_sources_grouped(
    db: &DatabaseConnection,
) -> Result<Vec<ImportSourceGroup>> {
    let tasks = tasks::Entity::find()
        .filter(tasks::Column::ImportSource.is_not_null())
        .all(db)
        .await?;
    let mut groups: HashMap<String, ImportSourceGroup> = HashMap::new();

    for t in tasks {
        let source = t.import_source.clone().unwrap_or_default();
        let key = format!("{}_{}", source, t.share_code.as_deref().unwrap_or("legacy"));
        let group = groups.entry(key).or_insert(ImportSourceGroup {
            share_code: t.share_code.clone(),
            url: source,
            tasks: Vec::new(),
            task_count: 0,
            last_updated_at: 0,
        });
        group.last_updated_at = group.last_updated_at.max(t.updated_at.timestamp());
        group.tasks.push(ImportHistoryItem {
            id: t.id,
            url: group.url.clone(),
            share_code: t.share_code,
            task_name: t.name,
            note: t.description,
            created_at: t.created_at.timestamp(),
            updated_at: t.updated_at.timestamp(),
        });
        group.task_count += 1;
    }

    let mut res: Vec<_> = groups.into_values().collect();
    res.sort_by(|a, b| b.last_updated_at.cmp(&a.last_updated_at));
    Ok(res)
}
