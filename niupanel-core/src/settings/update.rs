use crate::settings::{SYSTEM_GITHUB_PROXY, SYSTEM_UPDATE_CHANNEL, SettingsManager};
use niupanel_common::download::{DownloadEvent, DownloadOptions, download_with_resume};
use niupanel_common::error::{AppError, Result};
use niupanel_common::models::update::{ReleaseInfo, UpdateState, UpdateStatus, WebReleaseInfo};
use niupanel_common::version::{
    CORE_RELEASE_MANIFEST_FILE, CoreActivationReason, CoreReleaseDescriptor, CoreReleaseManifest,
    PendingCoreActivation, read_core_runtime_state, write_core_runtime_state,
};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use sea_orm::DatabaseConnection;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Component;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tokio_util::sync::CancellationToken;

const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/wyourname/NiuPanel/releases/latest";
const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/wyourname/NiuPanel/releases";
const UPDATE_USER_AGENT: &str = "NiuPanel-Updater/1.0";
const RELEASE_META_TIMEOUT_SECS: u64 = 30;
const RELEASE_DOWNLOAD_TIMEOUT_SECS: u64 = 60 * 30;
const UPDATE_DOWNLOAD_RETRIES: u8 = 5;

#[derive(serde::Deserialize, Debug, Clone)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Clone)]
pub struct UpdateService {
    db: DatabaseConnection,
    http_client: reqwest::Client,
    pub app_version: String,
    pub update_status: Arc<RwLock<UpdateStatus>>,
    pub update_cancellation_token: Arc<Mutex<Option<CancellationToken>>>,
}

pub struct DownloadedWebRelease {
    pub info: WebReleaseInfo,
    pub package_path: PathBuf,
    pub temp_dir: tempfile::TempDir,
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
        let current_version = self.app_version.clone();
        let channel = self.update_channel().await;
        let release = match self.fetch_target_release(&channel).await {
            Ok(release) => release,
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
                });
            }
        };

        let asset_size = self
            .find_matching_asset(&release)
            .map(|asset| asset.size)
            .unwrap_or(0);

        Ok(ReleaseInfo {
            tag_name: release.tag_name.clone(),
            html_url: release.html_url,
            body: release.body,
            current_version: current_version.clone(),
            channel,
            prerelease: release.prerelease,
            update_available: compare_versions(&release.tag_name, &current_version),
            size: asset_size,
        })
    }

    pub async fn check_web_update(&self, current_version: &str) -> Result<WebReleaseInfo> {
        let channel = self.update_channel().await;
        let release = self.fetch_target_release(&channel).await?;
        let (asset, version) = find_web_release_asset(&release)?;
        Ok(WebReleaseInfo {
            version: version.clone(),
            current_version: current_version.to_string(),
            release_tag: release.tag_name,
            html_url: release.html_url,
            body: release.body,
            channel,
            prerelease: release.prerelease,
            update_available: compare_versions(&version, current_version),
            size: asset.size,
        })
    }

    pub async fn download_web_update(&self, current_version: &str) -> Result<DownloadedWebRelease> {
        let channel = self.update_channel().await;
        let release = self.fetch_target_release(&channel).await?;
        let (asset, version) = find_web_release_asset(&release)?;
        let info = WebReleaseInfo {
            version: version.clone(),
            current_version: current_version.to_string(),
            release_tag: release.tag_name,
            html_url: release.html_url,
            body: release.body,
            channel,
            prerelease: release.prerelease,
            update_available: compare_versions(&version, current_version),
            size: asset.size,
        };
        let temp_dir = tempfile::Builder::new()
            .prefix("niupanel_web_update_")
            .tempdir()
            .map_err(AppError::Io)?;
        let package_path = temp_dir.path().join(&asset.name);
        download_component_asset(self, &asset, &package_path).await?;
        verify_asset_digest(&asset, &package_path)?;
        Ok(DownloadedWebRelease {
            info,
            package_path,
            temp_dir,
        })
    }

    pub async fn execute_update(&self) -> Result<()> {
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

    pub async fn execute_local_update(&self, bytes: Vec<u8>) -> Result<()> {
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
            state: UpdateState::Installing,
            progress: 100,
            message: "准备安装本地更新包...".to_string(),
            error: None,
        };
        drop(status_guard);

        let self_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = perform_local_update_task(self_clone.clone(), token, bytes).await {
                match e {
                    AppError::Cancelled => {
                        niupanel_common::info!("Local update task was cancelled.");
                    }
                    _ => {
                        tracing::error!("Local update failed: {}", e);
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

    async fn fetch_target_release(&self, channel: &str) -> Result<GithubRelease> {
        if channel == "preview" {
            self.fetch_latest_preview_release().await
        } else {
            self.fetch_latest_stable_release().await
        }
    }

    async fn fetch_latest_stable_release(&self) -> Result<GithubRelease> {
        let proxy_url: String = SettingsManager::get(&self.db, SYSTEM_GITHUB_PROXY)
            .await
            .unwrap_or_default();
        let target_url = build_target_url(&proxy_url, GITHUB_LATEST_RELEASE_API);

        let response = self
            .http_client
            .get(target_url)
            .header("User-Agent", UPDATE_USER_AGENT)
            .timeout(std::time::Duration::from_secs(RELEASE_META_TIMEOUT_SECS))
            .send()
            .await
            .map_err(AppError::Reqwest)?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApi(format!(
                "Failed to get latest release info: HTTP {}",
                response.status()
            )));
        }

        response
            .json::<GithubRelease>()
            .await
            .map_err(AppError::Reqwest)
    }

    async fn fetch_latest_preview_release(&self) -> Result<GithubRelease> {
        let proxy_url: String = SettingsManager::get(&self.db, SYSTEM_GITHUB_PROXY)
            .await
            .unwrap_or_default();
        let target_url = build_target_url(&proxy_url, GITHUB_RELEASES_API);

        let response = self
            .http_client
            .get(target_url)
            .header("User-Agent", UPDATE_USER_AGENT)
            .timeout(std::time::Duration::from_secs(RELEASE_META_TIMEOUT_SECS))
            .send()
            .await
            .map_err(AppError::Reqwest)?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApi(format!(
                "Failed to get preview release info: HTTP {}",
                response.status()
            )));
        }

        let releases = response
            .json::<Vec<GithubRelease>>()
            .await
            .map_err(AppError::Reqwest)?;

        releases
            .into_iter()
            .filter(|release| !release.draft && release.prerelease)
            .max_by(|left, right| {
                parse_version(&left.tag_name).cmp(&parse_version(&right.tag_name))
            })
            .ok_or_else(|| AppError::NotFound("未找到可用的预览版本".to_string()))
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

    fn find_matching_asset<'a>(
        &self,
        release: &'a GithubRelease,
    ) -> Result<&'a GithubReleaseAsset> {
        let arch = std::env::consts::ARCH;
        let os = std::env::consts::OS;

        release
            .assets
            .iter()
            .find(|asset| asset_matches_current_target(asset, os, arch))
            .ok_or(AppError::ExternalApi(format!(
                "No matching asset found for {}/{}",
                os, arch
            )))
    }
}

fn find_web_release_asset(release: &GithubRelease) -> Result<(GithubReleaseAsset, String)> {
    release
        .assets
        .iter()
        .filter_map(|asset| {
            asset
                .name
                .strip_prefix("niupanel_web_")
                .and_then(|name| name.strip_suffix(".tar.gz"))
                .filter(|version| !version.is_empty())
                .map(|version| (asset.clone(), version.to_string()))
        })
        .max_by(|left, right| parse_version(&left.1).cmp(&parse_version(&right.1)))
        .ok_or_else(|| AppError::NotFound("该发布版本没有独立 Web UI 包".to_string()))
}

fn verify_asset_digest(asset: &GithubReleaseAsset, package_path: &Path) -> Result<()> {
    let Some(expected) = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
    else {
        return Ok(());
    };
    let actual = file_sha256(package_path)?;
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "下载文件 {} 的 SHA-256 与发布信息不一致",
            asset.name
        )))
    }
}

async fn download_component_asset(
    service: &UpdateService,
    asset: &GithubReleaseAsset,
    package_path: &Path,
) -> Result<()> {
    let proxy_url: String = SettingsManager::get(&service.db, SYSTEM_GITHUB_PROXY)
        .await
        .unwrap_or_default();
    let download_url = build_target_url(&proxy_url, &asset.browser_download_url);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(UPDATE_USER_AGENT));
    download_with_resume(
        &service.http_client,
        &download_url,
        package_path,
        Some(asset.size),
        DownloadOptions {
            headers,
            timeout: Some(std::time::Duration::from_secs(
                RELEASE_DOWNLOAD_TIMEOUT_SECS,
            )),
            retries: UPDATE_DOWNLOAD_RETRIES,
            retry_delay: std::time::Duration::from_secs(2),
        },
        || false,
        |_| async {},
    )
    .await
}

fn compare_versions(remote: &str, local: &str) -> bool {
    parse_version(remote) > parse_version(local)
}

#[derive(Debug, Eq, PartialEq)]
struct VersionKey {
    numbers: Vec<u32>,
    prerelease: Option<Vec<PrereleasePart>>,
}

#[derive(Debug, Eq, PartialEq)]
enum PrereleasePart {
    Number(u32),
    Text(String),
}

impl Ord for VersionKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let max_len = self.numbers.len().max(other.numbers.len());
        for index in 0..max_len {
            let left = self.numbers.get(index).copied().unwrap_or(0);
            let right = other.numbers.get(index).copied().unwrap_or(0);
            match left.cmp(&right) {
                std::cmp::Ordering::Equal => {}
                ordering => return ordering,
            }
        }

        match (&self.prerelease, &other.prerelease) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(left), Some(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for VersionKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrereleasePart {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.cmp(right),
            (Self::Number(_), Self::Text(_)) => std::cmp::Ordering::Less,
            (Self::Text(_), Self::Number(_)) => std::cmp::Ordering::Greater,
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for PrereleasePart {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_version(version: &str) -> VersionKey {
    let cleaned = version
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit());
    let without_build = cleaned.split_once('+').map(|(v, _)| v).unwrap_or(cleaned);
    let (base, prerelease) = without_build
        .split_once('-')
        .map(|(base, pre)| (base, Some(pre)))
        .unwrap_or((without_build, None));

    let numbers = base
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(|char| char.is_ascii_digit())
                .collect::<String>()
        })
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect::<Vec<_>>();

    let prerelease = prerelease.map(|pre| {
        pre.split(['.', '-'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                part.parse::<u32>()
                    .map(PrereleasePart::Number)
                    .unwrap_or_else(|_| PrereleasePart::Text(part.to_ascii_lowercase()))
            })
            .collect::<Vec<_>>()
    });

    VersionKey {
        numbers,
        prerelease,
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
    let release = service.fetch_target_release(&channel).await?;
    if token.is_cancelled() {
        return Err(AppError::Cancelled);
    }

    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Downloading;
        guard.message = "正在下载更新包...".to_string();
        guard.progress = 0;
    }

    let asset = service.find_matching_asset(&release)?.clone();
    let temp_dir = tempfile::Builder::new()
        .prefix("niupanel_update_download_")
        .tempdir()
        .map_err(AppError::Io)?;
    let package_path = temp_dir.path().join(&asset.name);

    download_release_asset(&service, &token, &asset, &package_path).await?;
    install_update_archive(service, temp_dir, package_path).await
}

async fn perform_local_update_task(
    service: UpdateService,
    token: CancellationToken,
    bytes: Vec<u8>,
) -> Result<()> {
    if token.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let temp_dir = tempfile::Builder::new()
        .prefix("niupanel_local_update_")
        .tempdir()
        .map_err(AppError::Io)?;
    let package_path = temp_dir.path().join("local_update.tar.gz");
    tokio::fs::write(&package_path, bytes)
        .await
        .map_err(AppError::Io)?;
    install_update_archive(service, temp_dir, package_path).await
}

async fn install_update_archive(
    service: UpdateService,
    temp_dir: tempfile::TempDir,
    package_path: PathBuf,
) -> Result<()> {
    if std::env::var("NIUPANEL_LAUNCHED").as_deref() != Ok("1") {
        return Err(AppError::ValidationError(
            "Core 更新和回退需要由 niupanel-launcher 启动当前服务".to_string(),
        ));
    }
    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Installing;
        guard.message = "正在校验并安装 Core 候选版本...".to_string();
        guard.progress = 100;
    }

    let descriptor = tokio::task::spawn_blocking(move || -> Result<CoreReleaseDescriptor> {
        let unpack_dir = temp_dir.path().join("unpacked");
        fs::create_dir_all(&unpack_dir).map_err(AppError::Io)?;
        unpack_core_archive(&package_path, &unpack_dir)?;
        stage_core_release(&unpack_dir)
    })
    .await
    .map_err(|error| AppError::Internal(format!("Update task join error: {error}")))??;

    let system_root = absolute_system_root()?;
    let mut runtime = read_core_runtime_state(&system_root).map_err(AppError::Internal)?;
    if runtime.pending.is_some() {
        return Err(AppError::ValidationError(
            "已有等待激活的 Core 版本".to_string(),
        ));
    }
    if runtime.active.version == descriptor.version {
        return Err(AppError::ValidationError(format!(
            "Core {} 已经是当前版本；版本目录不可原地覆盖",
            descriptor.version
        )));
    }
    runtime.pending = Some(PendingCoreActivation {
        transaction_id: nanoid::nanoid!(16),
        candidate: descriptor,
        reason: CoreActivationReason::Update,
        requested_at: chrono::Utc::now().to_rfc3339(),
        restore_before_start: None,
    });
    write_core_runtime_state(&system_root, &runtime).map_err(AppError::Internal)?;

    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Restarting;
        guard.message = "候选版本已安装，正在交给 launcher 激活...".to_string();
    }
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    std::process::exit(0);
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

fn unpack_core_archive(package_path: &Path, destination: &Path) -> Result<()> {
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
            return Err(AppError::ValidationError(
                "Core 更新包只能包含普通文件和目录".to_string(),
            ));
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

fn stage_core_release(unpack_dir: &Path) -> Result<CoreReleaseDescriptor> {
    let package_root = find_core_package_root(unpack_dir)?;
    let manifest: CoreReleaseManifest = serde_json::from_slice(
        &fs::read(package_root.join(CORE_RELEASE_MANIFEST_FILE)).map_err(AppError::Io)?,
    )
    .map_err(AppError::Json)?;
    if manifest.component != "core" {
        return Err(AppError::ValidationError(
            "Core manifest component 必须为 core".to_string(),
        ));
    }
    if manifest.launcher_protocol != niupanel_common::version::RELEASE_PROTOCOL_VERSION {
        return Err(AppError::ValidationError(format!(
            "Core launcher 协议 {} 与当前协议 {} 不兼容",
            manifest.launcher_protocol,
            niupanel_common::version::RELEASE_PROTOCOL_VERSION
        )));
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

    let release_root = absolute_system_root()?
        .join("releases/core")
        .join(&manifest.version);
    fs::create_dir_all(&release_root).map_err(AppError::Io)?;
    let installed_binary = release_root.join("niupanel");
    if installed_binary.exists() {
        if !file_sha256(&installed_binary)?.eq_ignore_ascii_case(&manifest.binary_sha256) {
            return Err(AppError::ValidationError(format!(
                "Core {} 已存在但内容不同；版本号必须保持不可变",
                manifest.version
            )));
        }
    } else {
        fs::copy(&binary, &installed_binary).map_err(AppError::Io)?;
        #[cfg(unix)]
        fs::set_permissions(
            &installed_binary,
            fs::metadata(&binary).map_err(AppError::Io)?.permissions(),
        )
        .map_err(AppError::Io)?;
    }
    fs::write(
        release_root.join(CORE_RELEASE_MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).map_err(AppError::Json)?,
    )
    .map_err(AppError::Io)?;

    Ok(CoreReleaseDescriptor {
        version: manifest.version,
        path: installed_binary.to_string_lossy().into_owned(),
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

fn build_target_url(proxy_url: &str, original_url: &str) -> String {
    if proxy_url.is_empty() || !original_url.starts_with("http") {
        return original_url.to_string();
    }

    if original_url.starts_with(proxy_url) {
        return original_url.to_string();
    }

    format!("{}{}", proxy_url, original_url)
}

fn asset_matches_current_target(asset: &GithubReleaseAsset, os: &str, arch: &str) -> bool {
    let name = asset.name.to_lowercase();
    if !name.contains(os) {
        return false;
    }

    match arch {
        "aarch64" => name.contains("aarch64"),
        "x86_64" => name.contains("amd64") || name.contains("x86_64"),
        _ => name.contains(arch),
    }
}

async fn download_release_asset(
    service: &UpdateService,
    token: &CancellationToken,
    asset: &GithubReleaseAsset,
    package_path: &Path,
) -> Result<()> {
    let proxy_url: String = SettingsManager::get(&service.db, SYSTEM_GITHUB_PROXY)
        .await
        .unwrap_or_default();
    let download_url = build_target_url(&proxy_url, &asset.browser_download_url);
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(UPDATE_USER_AGENT));

    download_with_resume(
        &service.http_client,
        &download_url,
        package_path,
        Some(asset.size),
        DownloadOptions {
            headers,
            timeout: Some(std::time::Duration::from_secs(
                RELEASE_DOWNLOAD_TIMEOUT_SECS,
            )),
            retries: UPDATE_DOWNLOAD_RETRIES,
            retry_delay: std::time::Duration::from_secs(2),
        },
        || token.is_cancelled(),
        |event| async move {
            if let Ok(mut guard) = service.update_status.write() {
                match event {
                    DownloadEvent::Attempt {
                        attempt,
                        resumed_from,
                    } => {
                        guard.message = if attempt == 1 {
                            format!("正在下载更新包: {}", asset.name)
                        } else if resumed_from > 0 {
                            format!(
                                "下载中断，正在继续下载 {}/{}...",
                                attempt, UPDATE_DOWNLOAD_RETRIES
                            )
                        } else {
                            format!(
                                "下载中断，正在重试 {}/{}...",
                                attempt, UPDATE_DOWNLOAD_RETRIES
                            )
                        };
                    }
                    DownloadEvent::Progress(progress) => {
                        if progress.percent > guard.progress {
                            guard.progress = progress.percent.min(99);
                        }
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

    #[test]
    fn selects_highest_web_asset_from_release() {
        let release = GithubRelease {
            tag_name: "v0.8.1".into(),
            html_url: "https://example.com/release".into(),
            body: String::new(),
            draft: false,
            prerelease: false,
            assets: vec![
                GithubReleaseAsset {
                    name: "niupanel_linux_x86_64.tar.gz".into(),
                    browser_download_url: "https://example.com/core".into(),
                    size: 1,
                    digest: None,
                },
                GithubReleaseAsset {
                    name: "niupanel_web_2.0.0.tar.gz".into(),
                    browser_download_url: "https://example.com/web-2.0.0".into(),
                    size: 2,
                    digest: None,
                },
                GithubReleaseAsset {
                    name: "niupanel_web_2.1.0.tar.gz".into(),
                    browser_download_url: "https://example.com/web-2.1.0".into(),
                    size: 3,
                    digest: None,
                },
            ],
        };
        let (asset, version) = find_web_release_asset(&release).unwrap();
        assert_eq!(version, "2.1.0");
        assert_eq!(asset.size, 3);
    }
}
