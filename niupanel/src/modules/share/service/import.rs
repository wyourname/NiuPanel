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
use niupanel_common::variable::{normalize_variable_key, validate_variable_value};
use niupanel_common::{error, info};
use niupanel_core::task::cleanup_orphan_script_variables;
use niupanel_entity::{tasks, variables};
use reqwest::header::{HeaderMap, HeaderValue};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir;
use tokio::fs::{self, create_dir_all, remove_file};
use walkdir::WalkDir;

const MAX_COMPRESSED_PACKAGE_SIZE: u64 = 100 * 1024 * 1024;

fn setup_import_paths(staging_id: &str) -> Result<(PathBuf, PathBuf, PathBuf)> {
    validate_staging_id(staging_id)?;
    let import_dir = Path::new(&Config::global().share_import_dir);
    Ok((
        import_dir.join(format!("{}.npack", staging_id)),
        import_dir.join(format!("{}.status", staging_id)),
        import_dir.join(format!("{}.meta", staging_id)),
    ))
}

fn validate_staging_id(staging_id: &str) -> Result<()> {
    if staging_id.is_empty()
        || staging_id.len() > 64
        || !staging_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(AppError::ValidationError("无效的分享包暂存 ID".to_string()));
    }
    Ok(())
}

fn validate_share_code(code: &str) -> Result<()> {
    if code.is_empty()
        || code.len() > 128
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(AppError::ValidationError("无效的分享包代码".to_string()));
    }
    Ok(())
}

pub(crate) async fn build_safe_download_client(raw_url: &str) -> Result<(reqwest::Client, String)> {
    if raw_url.len() > 4096 {
        return Err(AppError::ValidationError(
            "分享地址不能超过 4096 个字符".to_string(),
        ));
    }
    let url = reqwest::Url::parse(raw_url)
        .map_err(|_| AppError::ValidationError("分享地址格式无效".to_string()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::ValidationError(
            "分享地址只允许 http 或 https 协议".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::ValidationError(
            "分享地址不能包含内嵌凭据".to_string(),
        ));
    }

    let host = url
        .host_str()
        .ok_or_else(|| AppError::ValidationError("分享地址缺少主机名".to_string()))?;
    let normalized_host = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized_host == "localhost" || normalized_host.ends_with(".localhost") {
        return Err(AppError::ValidationError(
            "分享地址不能指向本机或内网地址".to_string(),
        ));
    }

    let port = url
        .port_or_known_default()
        .ok_or_else(|| AppError::ValidationError("分享地址端口无效".to_string()))?;
    let mut addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|err| AppError::ExternalApi(format!("分享地址 DNS 解析失败: {}", err)))?
        .collect::<Vec<SocketAddr>>();
    addrs.sort_unstable();
    addrs.dedup();
    if addrs.is_empty() || addrs.iter().any(|addr| !is_public_ip(addr.ip())) {
        return Err(AppError::ValidationError(
            "分享地址不能解析到本机、内网或保留地址".to_string(),
        ));
    }

    // Pin the validated DNS result and reject redirects. This closes both DNS
    // rebinding and redirect-to-private-address SSRF paths.
    let client = reqwest::Client::builder()
        .user_agent("NiuPanel/1.0.0")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, &addrs)
        .build()
        .map_err(AppError::Reqwest)?;

    Ok((client, url.to_string()))
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || ip.is_unspecified()
        || octets[0] == 0
        || octets[0] >= 240
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 198 && matches!(octets[1], 18 | 19)))
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] & 0xffc0) == 0xfec0
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

pub async fn stage_import(
    _client: &reqwest::Client,
    url: &str,
    password: Option<String>,
) -> Result<String> {
    let staging_id = nanoid::nanoid!(12);
    let (safe_client, safe_url) = build_safe_download_client(url).await?;
    let (dest_path, status_path, meta_path) = setup_import_paths(&staging_id)?;

    if let Some(parent) = dest_path.parent() {
        create_dir_all(parent).await.map_err(AppError::Io)?;
    }

    update_status_file(&status_path, ImportState::Pending, None, 0).await;

    info!("提交导入任务 (ID: {}), URL: {}", staging_id, safe_url);
    let sid = staging_id.clone();
    tokio::spawn(async move {
        let _ = perform_download(
            safe_client,
            safe_url,
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
    _client: &reqwest::Client,
    staging_id: &str,
    url: &str,
    password: Option<String>,
) -> Result<()> {
    let (safe_client, safe_url) = build_safe_download_client(url).await?;
    let (dest_path, status_path, meta_path) = setup_import_paths(staging_id)?;
    update_status_file(&status_path, ImportState::Pending, None, 0).await;

    info!("重试导入任务 (ID: {}), URL: {}", staging_id, safe_url);
    let sid = staging_id.to_string();
    tokio::spawn(async move {
        let _ = perform_download(
            safe_client,
            safe_url,
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
            max_size: Some(MAX_COMPRESSED_PACKAGE_SIZE),
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
    let meta_json = serde_json::to_string(&meta).map_err(AppError::Json)?;
    fs::write(&meta_path, meta_json)
        .await
        .map_err(AppError::Io)?;
    update_status_file(&status_path, ImportState::Ready, None, 100).await;
    info!("下载完成 (ID: {})", staging_id);
    Ok(())
}

pub async fn get_import_status(staging_id: &str) -> Result<ImportStatusResponse> {
    let (_, path, _) = setup_import_paths(staging_id)?;
    let content = fs::read_to_string(&path).await.map_err(AppError::Io)?;
    serde_json::from_str(&content).map_err(|_| AppError::Generic("无效的状态文件".to_string()))
}

pub async fn get_staged_package(staging_id: &str) -> Result<NiuPackage> {
    let (path, _, _) = setup_import_paths(staging_id)?;
    read_package(&path).await
}

pub async fn finalize_import(
    db: &DatabaseConnection,
    staging_id: &str,
    selected_tasks: Option<Vec<String>>,
    user: &AuthenticatedUser,
    _ip: Option<String>,
    update_existing: bool,
) -> Result<ImportSummary> {
    let (pack_path, status_path, meta_path) = setup_import_paths(staging_id)?;
    let package = read_package(&pack_path).await?;

    let import_source = fs::read_to_string(&meta_path)
        .await
        .ok()
        .and_then(|c| serde_json::from_str::<StagedImportMeta>(&c).ok())
        .map(|m| m.source_url)
        .unwrap_or_else(|| staging_id.to_string());

    let share_code = import_source
        .split('/')
        .next_back()
        .filter(|code| validate_share_code(code).is_ok())
        .map(str::to_string);
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
        .or_else(|| task_data.files.first().map(|f| f.path.clone()))
        .map(|path| validate_import_file_path(&path))
        .transpose()?;
    let mut seen_paths = HashSet::new();

    for file in &task_data.files {
        let relative_path = validate_import_file_path(&file.path)?;
        if !seen_paths.insert(relative_path.clone()) {
            return Err(AppError::ValidationError(format!(
                "分享包包含重复文件路径: {}",
                file.path
            )));
        }
        let target = temp_dir.join(&relative_path);

        if let Some(p) = target.parent() {
            create_dir_all(p).await?;
        }
        fs::write(&target, &file.content).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Strip setuid/setgid/sticky bits from untrusted package metadata.
            fs::set_permissions(&target, std::fs::Permissions::from_mode(file.mode & 0o777))
                .await
                .map_err(AppError::Io)?;
        }

        if main_file.as_ref() == Some(&relative_path) {
            if let Ok(rel) = get_relative_path(&final_dir.join(&relative_path)) {
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

fn validate_import_file_path(path: &str) -> Result<PathBuf> {
    if path.is_empty() || path.len() > 1024 || path.contains('\\') || path.contains('\0') {
        return Err(AppError::ValidationError(format!(
            "分享包包含无效文件路径: {}",
            path
        )));
    }

    let candidate = Path::new(path);
    if candidate.is_absolute()
        || !candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(AppError::ValidationError(format!(
            "分享包文件路径必须是普通相对路径: {}",
            path
        )));
    }
    Ok(candidate.to_path_buf())
}

#[derive(Debug)]
struct ImportedFileRollback {
    destination: PathBuf,
    backup: Option<PathBuf>,
}

async fn ensure_safe_import_directory(base: &Path, directory: &Path) -> Result<()> {
    let relative = directory.strip_prefix(base).map_err(|_| {
        AppError::ValidationError(format!("导入目标目录越出任务目录: {}", directory.display()))
    })?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        let Component::Normal(segment) = component else {
            return Err(AppError::ValidationError(
                "导入目标目录包含非法路径组件".into(),
            ));
        };
        current.push(segment);
        match fs::symlink_metadata(&current).await {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(AppError::ValidationError(format!(
                    "导入目标目录不是安全的普通目录: {}",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).await?;
            }
            Err(err) => return Err(AppError::Io(err)),
        }
    }
    Ok(())
}

async fn rollback_import_files(files: &[ImportedFileRollback]) -> Result<()> {
    for file in files.iter().rev() {
        match fs::symlink_metadata(&file.destination).await {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                return Err(AppError::Internal(format!(
                    "回滚时目标文件被替换为目录: {}",
                    file.destination.display()
                )));
            }
            Ok(_) => fs::remove_file(&file.destination).await?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(AppError::Io(err)),
        }

        if let Some(backup) = &file.backup {
            if let Some(parent) = file.destination.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::rename(backup, &file.destination).await?;
        }
    }
    Ok(())
}

async fn abort_staged_import_install<T>(
    files: &[ImportedFileRollback],
    rollback_dir: &Path,
    install_error: AppError,
) -> Result<T> {
    match rollback_import_files(files).await {
        Ok(()) => {
            let _ = fs::remove_dir_all(rollback_dir).await;
            Err(install_error)
        }
        Err(rollback_error) => Err(AppError::Internal(format!(
            "{}；文件回滚失败: {}，备份保留在 {}",
            install_error,
            rollback_error,
            rollback_dir.display()
        ))),
    }
}

async fn install_staged_import_files(
    staging_dir: &Path,
    final_dir: &Path,
) -> Result<(Vec<ImportedFileRollback>, PathBuf)> {
    match fs::symlink_metadata(final_dir).await {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(AppError::ValidationError(format!(
                "导入任务目录不是安全的普通目录: {}",
                final_dir.display()
            )));
        }
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            create_dir_all(final_dir).await?;
        }
        Err(err) => return Err(AppError::Io(err)),
    }

    let mut staged_files = Vec::new();
    for entry in WalkDir::new(staging_dir) {
        let entry = entry.map_err(|err| {
            AppError::Io(
                err.into_io_error()
                    .unwrap_or_else(|| std::io::Error::other("遍历导入暂存目录失败")),
            )
        })?;
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(staging_dir)
                .map_err(|_| AppError::Internal("暂存文件无法转换为相对路径".to_string()))?;
            if !relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
            {
                return Err(AppError::ValidationError("暂存文件包含非法路径组件".into()));
            }
            staged_files.push((entry.path().to_path_buf(), relative.to_path_buf()));
        } else if !entry.file_type().is_dir() {
            return Err(AppError::ValidationError(format!(
                "暂存目录包含不支持的文件类型: {}",
                entry.path().display()
            )));
        }
    }

    let rollback_dir = final_dir.join(format!(".import-rollback-{}", nanoid::nanoid!(12)));
    fs::create_dir(&rollback_dir).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(err) =
            fs::set_permissions(&rollback_dir, std::fs::Permissions::from_mode(0o700)).await
        {
            let _ = fs::remove_dir_all(&rollback_dir).await;
            return Err(AppError::Io(err));
        }
    }

    let mut installed_files = Vec::new();
    for (source, relative) in staged_files {
        let destination = final_dir.join(&relative);
        let parent = destination
            .parent()
            .ok_or_else(|| AppError::Internal("导入文件缺少父目录".to_string()))?;
        if let Err(err) = ensure_safe_import_directory(final_dir, parent).await {
            return abort_staged_import_install(&installed_files, &rollback_dir, err).await;
        }

        let backup_result: Result<Option<PathBuf>> = async {
            match fs::symlink_metadata(&destination).await {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                    Err(AppError::ValidationError(format!(
                        "导入目标不是安全的普通文件: {}",
                        destination.display()
                    )))
                }
                Ok(_) => {
                    let backup = rollback_dir.join(&relative);
                    if let Some(parent) = backup.parent() {
                        fs::create_dir_all(parent).await?;
                    }
                    fs::rename(&destination, &backup).await?;
                    Ok(Some(backup))
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(err) => Err(AppError::Io(err)),
            }
        }
        .await;
        let backup = match backup_result {
            Ok(backup) => backup,
            Err(err) => {
                return abort_staged_import_install(&installed_files, &rollback_dir, err).await;
            }
        };

        installed_files.push(ImportedFileRollback {
            destination: destination.clone(),
            backup,
        });
        if fs::rename(&source, &destination).await.is_err()
            && let Err(copy_err) = fs::copy(&source, &destination).await
        {
            return abort_staged_import_install(
                &installed_files,
                &rollback_dir,
                AppError::Io(copy_err),
            )
            .await;
        }
    }

    Ok((installed_files, rollback_dir))
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
    // Validate variable data before moving any files into their final location.
    let normalized_variables = task_data
        .meta
        .variables
        .as_ref()
        .map(|variables| {
            variables
                .iter()
                .map(|variable| {
                    validate_variable_value(&variable.value)?;
                    Ok((
                        normalize_variable_key(&variable.key)?,
                        variable.value.clone(),
                    ))
                })
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?;

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

    let (installed_files, rollback_dir) =
        install_staged_import_files(temp_dir_handle.path(), final_dir).await?;

    let database_result: Result<()> = async {
        let txn = db.begin().await?;
        let task_id = if is_update {
            let existing = existing
                .ok_or_else(|| AppError::Internal("更新导入任务时未找到原任务记录".to_string()))?;
            let mut active: tasks::ActiveModel = existing.into();
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
                requirements: Set(task_data.meta.requirements),
                notify: Set(task_data.meta.notify),
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

        if let Some(vars) = normalized_variables {
            let existing_relations = niupanel_entity::variables_tasks::Entity::find()
                .filter(niupanel_entity::variables_tasks::Column::TaskId.eq(task_id))
                .all(&txn)
                .await?;
            let mut previous_variable_ids = existing_relations
                .into_iter()
                .map(|relation| relation.variable_id)
                .collect::<Vec<_>>();
            let legacy_variables = variables::Entity::find()
                .filter(variables::Column::Scope.eq("Script"))
                .filter(variables::Column::ScopeId.eq(task_id))
                .all(&txn)
                .await?;
            previous_variable_ids.extend(legacy_variables.into_iter().map(|variable| variable.id));
            previous_variable_ids.sort_unstable();
            previous_variable_ids.dedup();

            niupanel_entity::variables_tasks::Entity::delete_many()
                .filter(niupanel_entity::variables_tasks::Column::TaskId.eq(task_id))
                .exec(&txn)
                .await?;
            cleanup_orphan_script_variables(&txn, &previous_variable_ids).await?;

            for (index, (key, value)) in vars.into_iter().enumerate() {
                let created = variables::ActiveModel {
                    key: Set(key),
                    value: Set(value),
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

        txn.commit().await?;
        Ok(())
    }
    .await;

    if let Err(database_err) = database_result {
        error!("数据库操作失败，正在回滚导入文件: {}", database_err);
        return match rollback_import_files(&installed_files).await {
            Ok(()) => {
                let _ = fs::remove_dir_all(&rollback_dir).await;
                Err(database_err)
            }
            Err(rollback_err) => Err(AppError::Internal(format!(
                "{}；文件回滚失败: {}，备份保留在 {}",
                database_err,
                rollback_err,
                rollback_dir.display()
            ))),
        };
    }

    if let Err(err) = fs::remove_dir_all(&rollback_dir).await {
        error!(
            "导入已成功，但清理文件回滚目录 {} 失败: {}",
            rollback_dir.display(),
            err
        );
    }
    Ok(true)
}

pub async fn delete_imported_tasks(db: &DatabaseConnection, task_id: i32) -> Result<u64> {
    let task = tasks::Entity::find_by_id(task_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("任务不存在".to_string()))?;

    let name = task.name.clone();
    let import_base = resolve_path("import")?;
    let cleanup_path = imported_cleanup_path(&task.path, &import_base)
        .ok()
        .flatten();
    niupanel_core::task::delete_task_records(db, vec![task_id], false, false).await?;
    if let Some(path) = cleanup_path {
        info!(
            "删除已导入任务的物理文件: {} (路径: {})",
            name,
            path.display()
        );
        remove_imported_path(&path).await.map_err(|err| {
            AppError::Generic(format!("任务记录已删除，但物理文件清理失败: {err}"))
        })?;
    }
    Ok(1)
}

pub async fn delete_imported_by_source(db: &DatabaseConnection, source: &str) -> Result<u64> {
    info!("清理来源为 {} 的所有已导入任务...", source);
    let tasks = tasks::Entity::find()
        .filter(tasks::Column::ImportSource.eq(source))
        .all(db)
        .await?;
    let import_base = resolve_path("import")?;
    let task_ids = tasks.iter().map(|task| task.id).collect::<Vec<_>>();
    let cleanup_paths = tasks
        .iter()
        .filter_map(|task| {
            imported_cleanup_path(&task.path, &import_base)
                .ok()
                .flatten()
        })
        .collect::<HashSet<_>>();
    niupanel_core::task::delete_task_records(db, task_ids.clone(), false, false).await?;
    for path in cleanup_paths {
        remove_imported_path(&path).await.map_err(|err| {
            AppError::Generic(format!(
                "来源任务记录已删除，但物理文件清理失败 '{}': {err}",
                path.display()
            ))
        })?;
    }
    info!(
        "已成功清理来源 {} 的任务，共 {} 条记录",
        source,
        task_ids.len()
    );
    Ok(task_ids.len() as u64)
}

pub async fn delete_imported_by_share_code(db: &DatabaseConnection, code: &str) -> Result<u64> {
    validate_share_code(code)?;
    info!("清理 ShareCode 为 {} 的所有任务及目录...", code);
    let dir = resolve_path("import")?.join(format!("src_{}", code));
    let task_ids = tasks::Entity::find()
        .filter(tasks::Column::ShareCode.eq(code))
        .all(db)
        .await?
        .into_iter()
        .map(|task| task.id)
        .collect::<Vec<_>>();
    niupanel_core::task::delete_task_records(db, task_ids.clone(), false, false).await?;
    info!("移除物理目录: {}", dir.display());
    remove_imported_path(&dir).await.map_err(|err| {
        AppError::Generic(format!(
            "分享任务记录已删除，但物理目录清理失败 '{}': {err}",
            dir.display()
        ))
    })?;
    info!(
        "已清理 ShareCode {} 的任务，共 {} 条记录",
        code,
        task_ids.len()
    );
    Ok(task_ids.len() as u64)
}

async fn remove_imported_path(path: &Path) -> Result<()> {
    let result = match fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
            fs::remove_file(path).await
        }
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path).await,
        Ok(_) => {
            return Err(AppError::ValidationError(format!(
                "不支持的导入文件类型: {}",
                path.display()
            )));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(AppError::Io(err)),
    };
    result.map_err(AppError::Io)
}

fn imported_cleanup_path(relative_path: &str, import_base: &Path) -> Result<Option<PathBuf>> {
    let relative_path = validate_import_file_path(relative_path)?;
    let candidate = Config::global().scripts_dir.join(relative_path);
    let Some(parent) = candidate.parent() else {
        return Ok(None);
    };
    let canonical_parent = match parent.canonicalize() {
        Ok(parent) => parent,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(AppError::Io(err)),
    };
    if !canonical_parent.starts_with(import_base) {
        return Ok(None);
    }
    let Some(file_name) = candidate.file_name() else {
        return Ok(None);
    };
    Ok(Some(canonical_parent.join(file_name)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_id_accepts_only_bounded_ascii_identifiers() {
        assert!(validate_staging_id("Abc_123-xyz").is_ok());
        assert!(validate_staging_id("").is_err());
        assert!(validate_staging_id("../escape").is_err());
        assert!(validate_staging_id("含中文").is_err());
        assert!(validate_staging_id(&"a".repeat(65)).is_err());
    }

    #[test]
    fn share_code_rejects_directory_traversal() {
        assert!(validate_share_code("Abc_123-xyz").is_ok());
        assert!(validate_share_code("../escape").is_err());
        assert!(validate_share_code("nested/code").is_err());
        assert!(validate_share_code(&"a".repeat(129)).is_err());
    }

    #[test]
    fn import_file_paths_cannot_escape_or_use_platform_separators() {
        assert_eq!(
            validate_import_file_path("nested/task.py").unwrap(),
            PathBuf::from("nested/task.py")
        );
        for invalid in [
            "",
            "../task.py",
            "nested/../task.py",
            "/etc/passwd",
            r"nested\task.py",
            "task\0.py",
        ] {
            assert!(
                validate_import_file_path(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn staged_import_can_restore_overwritten_files() {
        let root = tempdir().unwrap();
        let staging = root.path().join("staging");
        let final_dir = root.path().join("final");
        fs::create_dir_all(&staging).await.unwrap();
        fs::create_dir_all(&final_dir).await.unwrap();
        fs::write(staging.join("task.sh"), b"new").await.unwrap();
        fs::write(final_dir.join("task.sh"), b"old").await.unwrap();

        let (installed, rollback_dir) = install_staged_import_files(&staging, &final_dir)
            .await
            .unwrap();
        assert_eq!(fs::read(final_dir.join("task.sh")).await.unwrap(), b"new");

        rollback_import_files(&installed).await.unwrap();
        assert_eq!(fs::read(final_dir.join("task.sh")).await.unwrap(), b"old");
        fs::remove_dir_all(rollback_dir).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn staged_import_rejects_symlinked_destination_directory() {
        use std::os::unix::fs::symlink;

        let root = tempdir().unwrap();
        let staging = root.path().join("staging");
        let target = root.path().join("target");
        let final_dir = root.path().join("final");
        fs::create_dir_all(&staging).await.unwrap();
        fs::create_dir_all(&target).await.unwrap();
        fs::write(staging.join("task.sh"), b"new").await.unwrap();
        symlink(&target, &final_dir).unwrap();

        assert!(
            install_staged_import_files(&staging, &final_dir)
                .await
                .is_err()
        );
        assert!(!target.join("task.sh").exists());
    }

    #[test]
    fn ssrf_filter_rejects_non_public_address_ranges() {
        for address in [
            "0.0.0.0",
            "10.0.0.1",
            "100.64.0.1",
            "127.0.0.1",
            "169.254.1.1",
            "192.0.2.1",
            "198.18.0.1",
            "224.0.0.1",
            "240.0.0.1",
            "::",
            "::1",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "::ffff:127.0.0.1",
        ] {
            let ip = address.parse::<IpAddr>().unwrap();
            assert!(!is_public_ip(ip), "{address} should be rejected");
        }
    }

    #[test]
    fn ssrf_filter_allows_public_ipv4_and_ipv6() {
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            let ip = address.parse::<IpAddr>().unwrap();
            assert!(is_public_ip(ip), "{address} should be allowed");
        }
    }
}
