use crate::settings::{SYSTEM_GITHUB_PROXY, SYSTEM_UPDATE_CHANNEL, SettingsManager};
use niupanel_common::download::{DownloadEvent, DownloadOptions, download_with_resume};
use niupanel_common::error::{AppError, Result};
use niupanel_common::models::update::{ReleaseInfo, UpdateState, UpdateStatus};
use niupanel_common::panel_runtime::{
    PanelReleaseDescriptor, PendingPanelActivation, directory_digest, enqueue_panel_activation,
    list_panel_releases, read_panel_runtime_state, verify_panel_release,
};
use niupanel_common::version::{
    API_CONTRACT_VERSION, CORE_RELEASE_MANIFEST_FILE, CoreReleaseDescriptor, CoreReleaseManifest,
    MINIMUM_UPDATE_CORE_VERSION, PanelActivationReason, UPDATE_CHANNEL_INDEX_SCHEMA_VERSION,
    UpdateAsset, UpdateChannelIndex, UpdateCoreAsset, UpdateCoreComponent, UpdateWebComponent,
    WEB_RELEASE_MANIFEST_FILE, WebReleaseManifest,
};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use sea_orm::DatabaseConnection;
use semver::Version;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tokio_util::sync::CancellationToken;

const UPDATE_CHANNEL_INDEX_BASE_URL: &str =
    "https://raw.githubusercontent.com/wyourname/NiuPanel/main/release/channels";
const UPDATE_USER_AGENT: &str = "NiuPanel-Updater/1.0";
const RELEASE_META_TIMEOUT_SECS: u64 = 30;
const RELEASE_DOWNLOAD_TIMEOUT_SECS: u64 = 60 * 30;
const UPDATE_DOWNLOAD_RETRIES: u8 = 5;
const RELEASE_MANIFEST_MAX_SIZE: u64 = 1024 * 1024;

mod contract;
use contract::*;

#[derive(Clone)]
pub struct UpdateService {
    db: DatabaseConnection,
    http_client: reqwest::Client,
    pub app_version: String,
    pub update_status: Arc<RwLock<UpdateStatus>>,
    pub update_cancellation_token: Arc<Mutex<Option<CancellationToken>>>,
}

impl UpdateService {
    pub fn new(
        db: DatabaseConnection,
        http_client: reqwest::Client,
        app_version: String,
        update_status: Arc<RwLock<UpdateStatus>>,
        update_cancellation_token: Arc<Mutex<Option<CancellationToken>>>,
    ) -> Self {
        Self {
            db,
            http_client,
            app_version,
            update_status,
            update_cancellation_token,
        }
    }

    pub async fn check_update(&self) -> Result<ReleaseInfo> {
        let current_version = active_panel_version(&self.app_version);
        let channel = self.update_channel().await;
        let index = match self.fetch_update_index(&channel).await {
            Ok(index) => index,
            Err(e) => {
                tracing::error!("Failed to check update: {}", e);
                return Ok(ReleaseInfo {
                    tag_name: "unknown".to_string(),
                    html_url: "".to_string(),
                    body: e.to_string(),
                    current_version,
                    channel,
                    prerelease: false,
                    update_available: false,
                    size: 0,
                    launcher_managed: launcher_managed(),
                });
            }
        };

        let update_available = compare_versions(&index.release.version, &current_version);
        let core_asset = core_asset_for_current_target(&index)?;
        let runtime = match absolute_system_root() {
            Ok(root) => read_panel_runtime_state(root).await.ok(),
            Err(_) => None,
        };
        let asset_size = if update_available {
            match runtime {
                Some(runtime) => {
                    u64::from(runtime.active.core.version != index.release.core.version)
                        * core_asset.size
                        + u64::from(runtime.active.web_version != index.release.web.version)
                            * index.release.web.asset.size
                }
                None => core_asset.size + index.release.web.asset.size,
            }
        } else {
            0
        };

        Ok(ReleaseInfo {
            tag_name: index.release.tag.clone(),
            html_url: index.release.release_url,
            body: index.release.notes,
            current_version: current_version.clone(),
            channel,
            prerelease: index.channel == "preview",
            update_available,
            size: asset_size,
            launcher_managed: launcher_managed(),
        })
    }

    pub async fn execute_update(&self) -> Result<()> {
        if !launcher_managed() {
            return Err(AppError::ValidationError(
                "当前为直接启动模式，Panel 更新需要由 niupanel-launcher 启动服务；Docker 部署请更新镜像"
                    .to_string(),
            ));
        }
        let mut status_guard = self
            .update_status
            .write()
            .map_err(|_| AppError::Internal("Lock poisoned".to_string()))?;

        match status_guard.state {
            UpdateState::Downloading | UpdateState::Installing | UpdateState::Restarting => {
                return Err(AppError::ValidationError(
                    "Update already in progress".to_string(),
                ));
            }
            _ => {}
        }

        let token = CancellationToken::new();
        {
            let mut token_guard = self
                .update_cancellation_token
                .lock()
                .map_err(|_| AppError::Internal("Lock poisoned".to_string()))?;
            *token_guard = Some(token.clone());
        }

        *status_guard = UpdateStatus {
            state: UpdateState::Downloading,
            progress: 0,
            message: "正在初始化更新...".to_string(),
            error: None,
        };
        drop(status_guard);

        let self_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = perform_update_task(self_clone.clone(), token).await {
                match e {
                    AppError::Cancelled => {
                        niupanel_common::info!("Update task was cancelled.");
                    }
                    _ => {
                        tracing::error!("Update failed: {}", e);
                        if let Ok(mut guard) = self_clone.update_status.write() {
                            if guard.state != UpdateState::Idle {
                                guard.state = UpdateState::Error;
                                guard.error = Some(e.to_string());
                                guard.message = format!("更新失败: {}", e);
                            }
                        }
                    }
                }
            }
            if let Ok(mut token_guard) = self_clone.update_cancellation_token.lock() {
                *token_guard = None;
            }
        });

        Ok(())
    }

    pub async fn cancel_update(&self) -> Result<()> {
        let mut token_guard = self
            .update_cancellation_token
            .lock()
            .map_err(|_| AppError::Internal("Lock poisoned".to_string()))?;

        if let Some(token) = token_guard.take() {
            let status = self
                .update_status
                .read()
                .map_err(|_| AppError::Internal("Lock poisoned".to_string()))?;
            if matches!(
                status.state,
                UpdateState::Installing | UpdateState::Restarting
            ) {
                *token_guard = Some(token);
                return Err(AppError::Forbidden(
                    "Cannot cancel update during installation/restart phase.".to_string(),
                ));
            }
            drop(status);

            token.cancel();

            let mut status_guard = self
                .update_status
                .write()
                .map_err(|_| AppError::Internal("Lock poisoned".to_string()))?;
            status_guard.state = UpdateState::Idle;
            status_guard.message = "用户已取消更新。".to_string();
            status_guard.progress = 0;
            status_guard.error = None;
        } else {
            return Err(AppError::NotFound("No update in progress".to_string()));
        }
        Ok(())
    }

    pub fn get_status(&self) -> Result<UpdateStatus> {
        self.update_status
            .read()
            .map(|status| status.clone())
            .map_err(|_| AppError::Internal("Update status lock poisoned".to_string()))
    }

    async fn fetch_update_index(&self, channel: &str) -> Result<UpdateChannelIndex> {
        let proxy_url: String = SettingsManager::get(&self.db, SYSTEM_GITHUB_PROXY)
            .await
            .unwrap_or_default();
        let target_url = build_target_url(
            &proxy_url,
            &format!("{UPDATE_CHANNEL_INDEX_BASE_URL}/{channel}.json"),
        );
        let response = self
            .http_client
            .get(target_url)
            .header(USER_AGENT, UPDATE_USER_AGENT)
            .timeout(std::time::Duration::from_secs(RELEASE_META_TIMEOUT_SECS))
            .send()
            .await
            .map_err(AppError::Reqwest)?;
        if !response.status().is_success() {
            return Err(AppError::ExternalApi(format!(
                "Failed to download {channel} update index: HTTP {}",
                response.status()
            )));
        }
        let bytes = response.bytes().await.map_err(AppError::Reqwest)?;
        if bytes.len() as u64 > RELEASE_MANIFEST_MAX_SIZE {
            return Err(AppError::ValidationError(format!(
                "{channel} 更新索引大小不合法"
            )));
        }
        let index = serde_json::from_slice::<UpdateChannelIndex>(&bytes).map_err(AppError::Json)?;
        validate_update_channel_index(&index, channel)?;
        Ok(index)
    }

    async fn update_channel(&self) -> String {
        let channel = SettingsManager::get(&self.db, SYSTEM_UPDATE_CHANNEL)
            .await
            .unwrap_or_else(|_| "stable".to_string())
            .trim()
            .to_string()
            .to_lowercase();

        if channel == "preview" {
            channel
        } else {
            "stable".to_string()
        }
    }
}

fn active_panel_version(fallback: &str) -> String {
    std::env::var("NIUPANEL_ACTIVE_PANEL_VERSION")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

fn launcher_managed() -> bool {
    std::env::var("NIUPANEL_LAUNCHED").as_deref() == Ok("1")
}

fn verify_downloaded_asset(
    name: &str,
    indexed_size: u64,
    expected_sha256: &str,
    expected_size: u64,
    package_path: &Path,
) -> Result<()> {
    let actual_size = fs::metadata(package_path).map_err(AppError::Io)?.len();
    if actual_size != expected_size || actual_size != indexed_size {
        return Err(AppError::ValidationError(format!(
            "下载文件 {} 大小与更新索引不一致",
            name
        )));
    }
    let actual = file_sha256(package_path)?;
    if !actual.eq_ignore_ascii_case(expected_sha256) {
        return Err(AppError::ValidationError(format!(
            "下载文件 {} 的 SHA-256 与更新索引不一致",
            name
        )));
    }
    Ok(())
}

async fn download_indexed_asset(
    service: &UpdateService,
    token: &CancellationToken,
    name: &str,
    url: &str,
    size: u64,
    package_path: &Path,
    report_progress: bool,
) -> Result<()> {
    let proxy_url: String = SettingsManager::get(&service.db, SYSTEM_GITHUB_PROXY)
        .await
        .unwrap_or_default();
    let download_url = build_target_url(&proxy_url, url);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(UPDATE_USER_AGENT));
    download_with_resume(
        &service.http_client,
        &download_url,
        package_path,
        Some(size),
        DownloadOptions {
            headers,
            timeout: Some(std::time::Duration::from_secs(
                RELEASE_DOWNLOAD_TIMEOUT_SECS,
            )),
            retries: UPDATE_DOWNLOAD_RETRIES,
            retry_delay: std::time::Duration::from_secs(2),
            max_size: None,
        },
        || token.is_cancelled(),
        |event| async move {
            if !report_progress {
                return;
            }
            if let Ok(mut guard) = service.update_status.write() {
                match event {
                    DownloadEvent::Attempt {
                        attempt,
                        resumed_from,
                    } => {
                        guard.message = if attempt == 1 {
                            format!("正在下载更新包: {name}")
                        } else if resumed_from > 0 {
                            format!("下载中断，正在继续下载 {attempt}/{UPDATE_DOWNLOAD_RETRIES}...")
                        } else {
                            format!("下载中断，正在重试 {attempt}/{UPDATE_DOWNLOAD_RETRIES}...")
                        };
                    }
                    DownloadEvent::Progress(progress) => {
                        guard.progress = guard.progress.max(progress.percent.min(99));
                    }
                    DownloadEvent::Completed { .. } => {
                        guard.progress = 100;
                        guard.message = "更新包下载完成，准备安装...".to_string();
                    }
                }
            }
        },
    )
    .await
}

fn compare_versions(remote: &str, local: &str) -> bool {
    match (
        Version::parse(remote.trim().trim_start_matches('v')),
        Version::parse(local.trim().trim_start_matches('v')),
    ) {
        (Ok(remote), Ok(local)) => remote > local,
        _ => false,
    }
}

async fn perform_update_task(service: UpdateService, token: CancellationToken) -> Result<()> {
    if token.is_cancelled() {
        return Err(AppError::Cancelled);
    }

    if let Ok(mut guard) = service.update_status.write() {
        guard.message = "正在获取发布信息...".to_string();
    }

    let channel = service.update_channel().await;
    let index = service.fetch_update_index(&channel).await?;
    if token.is_cancelled() {
        return Err(AppError::Cancelled);
    }

    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Downloading;
        guard.message = "正在下载更新包...".to_string();
        guard.progress = 0;
    }

    let system_root = absolute_system_root()?;
    let runtime = read_panel_runtime_state(&system_root)
        .await
        .map_err(AppError::Internal)?;
    verify_panel_release(&runtime.active).map_err(AppError::ValidationError)?;
    if runtime.active.version == index.release.version {
        return Err(AppError::ValidationError(format!(
            "Panel {} 已经是当前版本",
            index.release.version
        )));
    }
    let core_asset = core_asset_for_current_target(&index)?.clone();
    if !compare_versions(&index.release.version, &runtime.active.version) {
        return Err(AppError::ValidationError(format!(
            "Panel {} 不高于当前版本 {}；历史版本只能通过回退流程激活",
            index.release.version, runtime.active.version
        )));
    }
    if let Some(installed) = list_panel_releases(&system_root)
        .await
        .map_err(AppError::Internal)?
        .into_iter()
        .find(|release| release.version == index.release.version)
    {
        validate_installed_panel_release(&installed, &index, &core_asset)?;
        return queue_panel_activation(&service, &runtime.active.version, installed).await;
    }
    let orphan = system_root
        .join("releases/panel")
        .join(&index.release.version);
    if orphan.exists() {
        fs::remove_dir_all(&orphan).map_err(AppError::Io)?;
    }
    let web_asset = index.release.web.asset.clone();
    let temp_dir = tempfile::Builder::new()
        .prefix("niupanel_panel_update_")
        .tempdir()
        .map_err(AppError::Io)?;
    let core_package = if runtime.active.core.version == index.release.core.version {
        None
    } else {
        Some(download_release_asset(&service, &token, &core_asset, temp_dir.path()).await?)
    };
    let web_package = if runtime.active.web_version == index.release.web.version {
        None
    } else {
        Some(download_release_asset(&service, &token, &web_asset, temp_dir.path()).await?)
    };
    install_panel_release(
        service,
        index,
        runtime.active,
        temp_dir,
        core_package,
        web_package,
    )
    .await
}

async fn download_release_asset(
    service: &UpdateService,
    token: &CancellationToken,
    asset: &impl IndexedAsset,
    directory: &Path,
) -> Result<PathBuf> {
    let package_path = directory.join(asset.name());
    download_indexed_asset(
        service,
        token,
        asset.name(),
        asset.url(),
        asset.size(),
        &package_path,
        true,
    )
    .await?;
    verify_downloaded_asset(
        asset.name(),
        asset.size(),
        asset.sha256(),
        asset.size(),
        &package_path,
    )?;
    Ok(package_path)
}

trait IndexedAsset {
    fn name(&self) -> &str;
    fn url(&self) -> &str;
    fn sha256(&self) -> &str;
    fn size(&self) -> u64;
}

impl IndexedAsset for UpdateAsset {
    fn name(&self) -> &str {
        &self.name
    }
    fn url(&self) -> &str {
        &self.url
    }
    fn sha256(&self) -> &str {
        &self.sha256
    }
    fn size(&self) -> u64 {
        self.size
    }
}

impl IndexedAsset for UpdateCoreAsset {
    fn name(&self) -> &str {
        &self.name
    }
    fn url(&self) -> &str {
        &self.url
    }
    fn sha256(&self) -> &str {
        &self.sha256
    }
    fn size(&self) -> u64 {
        self.size
    }
}

async fn install_panel_release(
    service: UpdateService,
    index: UpdateChannelIndex,
    active: PanelReleaseDescriptor,
    temp_dir: tempfile::TempDir,
    core_package: Option<PathBuf>,
    web_package: Option<PathBuf>,
) -> Result<()> {
    if std::env::var("NIUPANEL_LAUNCHED").as_deref() != Ok("1") {
        return Err(AppError::ValidationError(
            "Panel 更新和回退需要由 niupanel-launcher 启动当前服务".to_string(),
        ));
    }
    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Installing;
        guard.message = "正在校验并安装 Panel 候选版本...".to_string();
        guard.progress = 100;
    }

    let expected_active_version = active.version.clone();
    let descriptor = tokio::task::spawn_blocking(move || -> Result<PanelReleaseDescriptor> {
        stage_panel_release(
            &index,
            &active,
            temp_dir.path(),
            core_package.as_deref(),
            web_package.as_deref(),
        )
    })
    .await
    .map_err(|error| AppError::Internal(format!("Update task join error: {error}")))??;

    queue_panel_activation(&service, &expected_active_version, descriptor).await
}

fn validate_installed_panel_release(
    release: &PanelReleaseDescriptor,
    index: &UpdateChannelIndex,
    asset: &UpdateCoreAsset,
) -> Result<()> {
    let expected = &index.release;
    if release.version != expected.version
        || release.core.version != expected.core.version
        || release.core.launcher_protocol != expected.core.launcher_protocol
        || release.core.api_contract != expected.core.api_contract
        || release.core.schema_epoch != expected.core.schema_epoch
        || release.core.schema_revision != expected.core.schema_revision
        || release.core.target != asset.target
        || release.web_version != expected.web.version
    {
        return Err(AppError::ValidationError(format!(
            "已安装的 Panel {} 与当前通道发布描述不一致",
            release.version
        )));
    }
    verify_panel_release(release).map_err(AppError::ValidationError)
}

async fn queue_panel_activation(
    service: &UpdateService,
    expected_active_version: &str,
    descriptor: PanelReleaseDescriptor,
) -> Result<()> {
    let system_root = absolute_system_root()?;
    let pending = PendingPanelActivation {
        transaction_id: nanoid::nanoid!(16),
        candidate: descriptor,
        reason: PanelActivationReason::Update,
        requested_at: chrono::Utc::now().to_rfc3339(),
        activation_snapshot_path: None,
        restore_before_start: None,
    };
    enqueue_panel_activation(&system_root, expected_active_version, pending)
        .await
        .map_err(AppError::ValidationError)?;

    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Restarting;
        guard.message = "候选版本已安装，正在交给 launcher 激活...".to_string();
    }
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    std::process::exit(0);
}

fn stage_panel_release(
    index: &UpdateChannelIndex,
    active: &PanelReleaseDescriptor,
    download_root: &Path,
    core_package: Option<&Path>,
    web_package: Option<&Path>,
) -> Result<PanelReleaseDescriptor> {
    let system_root = absolute_system_root()?;
    let releases_root = system_root.join("releases/panel");
    let final_root = releases_root.join(&index.release.version);
    if final_root.exists() {
        return Err(AppError::ValidationError(format!(
            "Panel {} 已存在；发布版本是不可变的",
            index.release.version
        )));
    }
    fs::create_dir_all(&releases_root).map_err(AppError::Io)?;
    let staging_root = system_root.join("staging/panel").join(format!(
        "{}-{}",
        index.release.version,
        nanoid::nanoid!(10)
    ));
    fs::create_dir_all(&staging_root).map_err(AppError::Io)?;

    let result = (|| {
        let core_root = staging_root.join("core");
        let web_root = staging_root.join("web");
        let final_core_binary = final_root.join("core/niupanel");
        let final_web_root = final_root.join("web");

        let core = if active.core.version == index.release.core.version {
            let source = Path::new(&active.core.path)
                .parent()
                .ok_or_else(|| AppError::ValidationError("当前 Core 路径没有父目录".to_string()))?;
            copy_release_tree(source, &core_root)?;
            if !core_root.join("niupanel").is_file() {
                return Err(AppError::ValidationError(
                    "当前 Core 发布目录缺少 niupanel".to_string(),
                ));
            }
            CoreReleaseDescriptor {
                path: final_core_binary.to_string_lossy().into_owned(),
                ..active.core.clone()
            }
        } else {
            let package = core_package.ok_or_else(|| {
                AppError::ValidationError("更新 Core 时缺少 Core 资产".to_string())
            })?;
            let unpack_root = download_root.join("core-unpacked");
            fs::create_dir_all(&unpack_root).map_err(AppError::Io)?;
            unpack_release_archive(package, &unpack_root, "Core")?;
            let package_root = find_core_package_root(&unpack_root)?;
            let asset = core_asset_for_current_target(index)?;
            stage_core_component(
                &package_root,
                &core_root,
                &final_core_binary,
                &index.release.core,
                asset,
            )?
        };
        let core_digest = directory_digest(&core_root).map_err(AppError::ValidationError)?;
        if active.core.version == index.release.core.version
            && !core_digest.eq_ignore_ascii_case(&active.core_digest)
        {
            return Err(AppError::ValidationError(
                "当前 Core 文件与 runtime.db 中的完整性记录不一致".to_string(),
            ));
        }

        if active.web_version == index.release.web.version {
            copy_release_tree(Path::new(&active.web_path), &web_root)?;
        } else {
            let package = web_package
                .ok_or_else(|| AppError::ValidationError("更新 Web 时缺少 Web 资产".to_string()))?;
            let unpack_root = download_root.join("web-unpacked");
            fs::create_dir_all(&unpack_root).map_err(AppError::Io)?;
            unpack_release_archive(package, &unpack_root, "Web")?;
            let package_root = find_web_package_root(&unpack_root)?;
            stage_web_component(&package_root, &web_root, &index.release.web)?;
        }
        if !web_root.join("index.html").is_file() {
            return Err(AppError::ValidationError(
                "Panel Web 目录缺少 index.html".to_string(),
            ));
        }
        let web_digest = directory_digest(&web_root).map_err(AppError::ValidationError)?;
        if active.web_version == index.release.web.version
            && !web_digest.eq_ignore_ascii_case(&active.web_digest)
        {
            return Err(AppError::ValidationError(
                "当前 Web 文件与 runtime.db 中的完整性记录不一致".to_string(),
            ));
        }
        fs::rename(&staging_root, &final_root).map_err(AppError::Io)?;
        Ok(PanelReleaseDescriptor {
            version: index.release.version.clone(),
            core,
            core_digest,
            web_version: index.release.web.version.clone(),
            web_path: final_web_root.to_string_lossy().into_owned(),
            web_digest,
            installed_at: chrono::Utc::now().to_rfc3339(),
        })
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_root);
    }
    result
}

fn absolute_system_root() -> Result<PathBuf> {
    let path = &niupanel_common::config::Config::global().system_dir;
    if path.is_absolute() {
        Ok(path.clone())
    } else {
        Ok(std::env::current_dir().map_err(AppError::Io)?.join(path))
    }
}

fn validate_archive_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::ValidationError(format!(
            "Core 更新包包含不安全路径: {}",
            path.display()
        )));
    }
    Ok(())
}

fn unpack_release_archive(package_path: &Path, destination: &Path, component: &str) -> Result<()> {
    let file = fs::File::open(package_path).map_err(AppError::Io)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive
        .entries()
        .map_err(|error| AppError::ValidationError(error.to_string()))?
    {
        let mut entry = entry.map_err(|error| AppError::ValidationError(error.to_string()))?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(AppError::ValidationError(format!(
                "{component} 更新包只能包含普通文件和目录"
            )));
        }
        let path = entry
            .path()
            .map_err(|error| AppError::ValidationError(error.to_string()))?;
        validate_archive_path(&path)?;
        entry
            .unpack_in(destination)
            .map_err(|error| AppError::ValidationError(error.to_string()))?;
    }
    Ok(())
}

fn find_core_package_root(unpack_dir: &Path) -> Result<PathBuf> {
    let manifests = walkdir::WalkDir::new(unpack_dir)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name() == CORE_RELEASE_MANIFEST_FILE)
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    if manifests.len() != 1 {
        return Err(AppError::ValidationError(
            "Core 更新包必须包含且只能包含一个 core-release.json".to_string(),
        ));
    }
    Ok(manifests[0]
        .parent()
        .expect("Core manifest path has a parent")
        .to_path_buf())
}

fn find_web_package_root(unpack_dir: &Path) -> Result<PathBuf> {
    let manifests = walkdir::WalkDir::new(unpack_dir)
        .min_depth(1)
        .max_depth(4)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name() == WEB_RELEASE_MANIFEST_FILE)
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    if manifests.len() != 1 {
        return Err(AppError::ValidationError(
            "Web 更新包必须包含且只能包含一个 release-manifest.json".to_string(),
        ));
    }
    Ok(manifests[0]
        .parent()
        .expect("Web manifest path has a parent")
        .to_path_buf())
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(AppError::Io)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(AppError::Io)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn copy_release_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(AppError::Io)?;
    for entry in fs::read_dir(source).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        let file_type = entry.file_type().map_err(AppError::Io)?;
        if matches!(
            entry.file_name().to_str(),
            Some(CORE_RELEASE_MANIFEST_FILE | WEB_RELEASE_MANIFEST_FILE)
        ) {
            continue;
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_release_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(AppError::Io)?;
            #[cfg(unix)]
            fs::set_permissions(
                &target,
                fs::metadata(entry.path())
                    .map_err(AppError::Io)?
                    .permissions(),
            )
            .map_err(AppError::Io)?;
        } else {
            return Err(AppError::ValidationError(
                "发布目录只能包含普通文件和目录".to_string(),
            ));
        }
    }
    Ok(())
}

fn stage_release_tools(package_root: &Path, release_root: &Path) -> Result<()> {
    let source = package_root.join("tools");
    if !source.is_dir() {
        return Ok(());
    }
    if source.join("fnm").exists() {
        return Err(AppError::ValidationError(
            "Core 更新包不能再包含 fnm，请使用 pnpm 运行时工具".to_string(),
        ));
    }
    if !source.join("pnpm").is_file() {
        return Err(AppError::ValidationError(
            "Core 更新包 tools 缺少 pnpm".to_string(),
        ));
    }

    let destination = release_root.join("tools");
    if destination.is_dir() {
        return Ok(());
    }

    let staging = release_root.join(format!(".tools-staging-{}", nanoid::nanoid!(10)));
    let result = copy_release_tree(&source, &staging).and_then(|()| {
        fs::rename(&staging, &destination).map_err(AppError::Io)?;
        Ok(())
    });
    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn stage_core_component(
    package_root: &Path,
    destination: &Path,
    final_binary: &Path,
    expected: &UpdateCoreComponent,
    expected_asset: &UpdateCoreAsset,
) -> Result<CoreReleaseDescriptor> {
    let manifest: CoreReleaseManifest = serde_json::from_slice(
        &fs::read(package_root.join(CORE_RELEASE_MANIFEST_FILE)).map_err(AppError::Io)?,
    )
    .map_err(AppError::Json)?;
    if manifest.component != "core" {
        return Err(AppError::ValidationError(
            "Core manifest component 必须为 core".to_string(),
        ));
    }
    if manifest.version != expected.version
        || manifest.launcher_protocol != expected.launcher_protocol
        || manifest.api_contract != expected.api_contract
        || manifest.schema_epoch != expected.schema_epoch
        || manifest.schema_revision != expected.schema_revision
        || manifest.target != expected_asset.target
    {
        return Err(AppError::ValidationError(
            "Core manifest 与通道索引不一致".to_string(),
        ));
    }
    let binary = package_root.join("niupanel");
    if !binary.is_file() {
        return Err(AppError::ValidationError(
            "Core 更新包缺少 niupanel 可执行文件".to_string(),
        ));
    }
    let hash = file_sha256(&binary)?;
    if !hash.eq_ignore_ascii_case(&manifest.binary_sha256) {
        return Err(AppError::ValidationError(
            "Core 二进制 SHA-256 与 manifest 不一致".to_string(),
        ));
    }
    let output = std::process::Command::new(&binary).arg("--help").output();
    match output {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            return Err(AppError::ValidationError(format!(
                "Core 二进制验证失败，退出码 {:?}",
                output.status.code()
            )));
        }
        Err(error) => {
            return Err(AppError::ValidationError(format!(
                "Core 二进制无法执行: {error}"
            )));
        }
    }

    fs::create_dir_all(destination).map_err(AppError::Io)?;
    let staged_binary = destination.join("niupanel");
    fs::copy(&binary, &staged_binary).map_err(AppError::Io)?;
    #[cfg(unix)]
    fs::set_permissions(
        &staged_binary,
        fs::metadata(&binary).map_err(AppError::Io)?.permissions(),
    )
    .map_err(AppError::Io)?;
    stage_release_tools(package_root, destination)?;

    Ok(CoreReleaseDescriptor {
        version: manifest.version,
        path: final_binary.to_string_lossy().into_owned(),
        launcher_protocol: manifest.launcher_protocol,
        api_contract: manifest.api_contract,
        schema_epoch: manifest.schema_epoch,
        schema_revision: manifest.schema_revision,
        target: manifest.target,
        installed_at: manifest
            .built_at
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    })
}

fn stage_web_component(
    package_root: &Path,
    destination: &Path,
    expected: &UpdateWebComponent,
) -> Result<()> {
    let manifest: WebReleaseManifest = serde_json::from_slice(
        &fs::read(package_root.join(WEB_RELEASE_MANIFEST_FILE)).map_err(AppError::Io)?,
    )
    .map_err(AppError::Json)?;
    if manifest.component != "web"
        || manifest.version != expected.version
        || manifest.api_contract != expected.api_contract
        || manifest.core.min != expected.core.min
        || manifest.core.max != expected.core.max
    {
        return Err(AppError::ValidationError(
            "Web manifest 与通道索引不一致".to_string(),
        ));
    }
    if manifest.files.is_empty() || !package_root.join("index.html").is_file() {
        return Err(AppError::ValidationError(
            "Web manifest 缺少文件摘要或更新包缺少 index.html".to_string(),
        ));
    }
    let mut actual_files = BTreeSet::new();
    for entry in walkdir::WalkDir::new(package_root) {
        let entry = entry.map_err(|error| AppError::Io(error.into()))?;
        if !entry.file_type().is_file() || entry.file_name() == WEB_RELEASE_MANIFEST_FILE {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(package_root)
            .map_err(|error| AppError::Internal(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        actual_files.insert(relative);
    }
    let declared_files = manifest.files.keys().cloned().collect::<BTreeSet<_>>();
    if actual_files != declared_files {
        return Err(AppError::ValidationError(
            "Web 更新包的实际文件与 manifest 不一致".to_string(),
        ));
    }
    for (relative, expected_hash) in &manifest.files {
        validate_archive_path(Path::new(relative))?;
        if !file_sha256(&package_root.join(relative))?.eq_ignore_ascii_case(expected_hash) {
            return Err(AppError::ValidationError(format!(
                "Web 文件摘要不一致: {relative}"
            )));
        }
    }
    copy_release_tree(package_root, destination)
}

fn build_target_url(proxy_url: &str, original_url: &str) -> String {
    if proxy_url.is_empty() || !original_url.starts_with("http") {
        return original_url.to_string();
    }

    if original_url.starts_with(proxy_url) {
        return original_url.to_string();
    }

    format!("{}{}", proxy_url, original_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_basic_versions() {
        assert!(compare_versions("v1.2.4", "1.2.3"));
        assert!(!compare_versions("v1.2.3", "1.2.3"));
        assert!(!compare_versions("v1.2.2", "1.2.3"));
    }

    #[test]
    fn compares_prerelease_versions() {
        assert!(compare_versions("v1.2.4-beta.1", "1.2.3"));
        assert!(compare_versions("v1.2.3-beta.2", "v1.2.3-beta.1"));
        assert!(compare_versions("v1.2.3", "v1.2.3-beta.1"));
        assert!(!compare_versions("v1.2.3-beta.1", "v1.2.3"));
    }

    fn update_index_fixture() -> UpdateChannelIndex {
        let mut core_assets = std::collections::BTreeMap::new();
        for (architecture, target) in [
            ("x86_64", "x86_64-unknown-linux-musl"),
            ("aarch64", "aarch64-unknown-linux-musl"),
            ("armv7", "armv7-unknown-linux-musleabihf"),
        ] {
            core_assets.insert(
                architecture.to_string(),
                UpdateCoreAsset {
                    name: format!("niupanel_linux_{architecture}.tar.gz"),
                    url: format!(
                        "https://example.com/releases/download/Core-v0.8.1/niupanel_linux_{architecture}.tar.gz"
                    ),
                    target: target.to_string(),
                    sha256: "a".repeat(64),
                    size: 1,
                },
            );
        }
        UpdateChannelIndex {
            schema_version: UPDATE_CHANNEL_INDEX_SCHEMA_VERSION,
            channel: "preview".into(),
            updated_at: "2026-08-01T00:00:00Z".into(),
            release: niupanel_common::version::PanelUpdateRelease {
                version: "0.8.1".into(),
                tag: "v0.8.1".into(),
                release_url: "https://example.com/releases/tag/v0.8.1".into(),
                notes: String::new(),
                launcher_protocol: niupanel_common::version::RELEASE_PROTOCOL_VERSION,
                core: niupanel_common::version::UpdateCoreComponent {
                    version: "0.8.1".into(),
                    tag: "Core-v0.8.1".into(),
                    release_url: "https://example.com/releases/tag/Core-v0.8.1".into(),
                    notes: String::new(),
                    launcher_protocol: niupanel_common::version::RELEASE_PROTOCOL_VERSION,
                    api_contract: API_CONTRACT_VERSION,
                    schema_epoch: 1,
                    schema_revision: 30,
                    assets: core_assets,
                },
                web: niupanel_common::version::UpdateWebComponent {
                    version: "2.0.0".into(),
                    tag: "web-v2.0.0".into(),
                    release_url: "https://example.com/releases/tag/web-v2.0.0".into(),
                    notes: String::new(),
                    api_contract: API_CONTRACT_VERSION,
                    core: niupanel_common::version::ComponentCompatibility {
                        min: "0.8.1".into(),
                        max: None,
                    },
                    asset: UpdateAsset {
                        name: "niupanel_web_2.0.0.tar.gz".into(),
                        url: "https://example.com/releases/download/web-v2.0.0/niupanel_web_2.0.0.tar.gz".into(),
                        sha256: "b".repeat(64),
                        size: 2,
                    },
                },
            },
        }
    }

    #[test]
    fn update_index_accepts_matching_components() {
        let index = update_index_fixture();

        validate_update_channel_index(&index, "preview").unwrap();
        assert_eq!(core_asset_for_current_target(&index).unwrap().size, 1);
    }

    #[test]
    fn update_index_rejects_legacy_core_versions() {
        let mut index = update_index_fixture();
        index.release.core.version = "0.7.9".into();
        index.release.core.tag = "Core-v0.7.9".into();

        let error = validate_update_channel_index(&index, "preview").unwrap_err();

        assert!(error.to_string().contains("最低支持更新基线"));
    }

    #[test]
    fn copies_bundled_runtime_tools_recursively() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        fs::create_dir_all(source.path().join("dist")).unwrap();
        fs::write(source.path().join("pnpm"), b"pnpm").unwrap();
        fs::write(source.path().join("dist/pnpm.mjs"), b"runtime").unwrap();

        let target = destination.path().join("tools");
        copy_release_tree(source.path(), &target).unwrap();

        assert_eq!(fs::read(target.join("pnpm")).unwrap(), b"pnpm");
        assert_eq!(fs::read(target.join("dist/pnpm.mjs")).unwrap(), b"runtime");
    }

    #[test]
    fn rejects_core_packages_that_still_bundle_fnm() {
        let package = tempfile::tempdir().unwrap();
        let release = tempfile::tempdir().unwrap();
        fs::create_dir_all(package.path().join("tools")).unwrap();
        fs::write(package.path().join("tools/fnm"), b"legacy").unwrap();
        fs::write(package.path().join("tools/pnpm"), b"pnpm").unwrap();

        let error = stage_release_tools(package.path(), release.path()).unwrap_err();

        assert!(error.to_string().contains("fnm"));
        assert!(!release.path().join("tools").exists());
    }
}
