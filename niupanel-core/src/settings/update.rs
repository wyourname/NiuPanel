use crate::settings::{SYSTEM_GITHUB_PROXY, SYSTEM_UPDATE_CHANNEL, SettingsManager};
use niupanel_common::download::{DownloadEvent, DownloadOptions, download_with_resume};
use niupanel_common::error::{AppError, Result};
use niupanel_common::models::update::{ReleaseInfo, UpdateState, UpdateStatus, WebReleaseInfo};
use niupanel_common::version::{
    API_CONTRACT_VERSION, CORE_RELEASE_MANIFEST_FILE, CoreActivationReason, CoreReleaseDescriptor,
    CoreReleaseManifest, MINIMUM_UPDATE_CORE_VERSION, PendingCoreActivation,
    UPDATE_CHANNEL_INDEX_SCHEMA_VERSION, UpdateAsset, UpdateChannelIndex, UpdateCoreAsset,
    read_core_runtime_state, write_core_runtime_state,
};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use sea_orm::DatabaseConnection;
use semver::Version;
use sha2::{Digest, Sha256};
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

        let asset_size = core_asset_for_current_target(&index)
            .map(|asset| asset.size)
            .unwrap_or(0);

        Ok(ReleaseInfo {
            tag_name: index.core.tag.clone(),
            html_url: index.core.release_url,
            body: index.core.notes,
            current_version: current_version.clone(),
            channel,
            prerelease: index.channel == "preview",
            update_available: compare_versions(&index.core.version, &current_version),
            size: asset_size,
            launcher_managed: launcher_managed(),
        })
    }

    pub async fn check_web_update(&self, current_version: &str) -> Result<WebReleaseInfo> {
        let channel = self.update_channel().await;
        let index = self.fetch_update_index(&channel).await?;
        let version = index.web.version.clone();
        Ok(WebReleaseInfo {
            version: version.clone(),
            current_version: current_version.to_string(),
            release_tag: index.web.tag,
            html_url: index.web.release_url,
            body: index.web.notes,
            channel,
            prerelease: index.channel == "preview",
            update_available: compare_versions(&version, current_version),
            size: index.web.asset.size,
        })
    }

    pub async fn download_web_update(&self, current_version: &str) -> Result<DownloadedWebRelease> {
        let channel = self.update_channel().await;
        let index = self.fetch_update_index(&channel).await?;
        let asset = index.web.asset.clone();
        let version = index.web.version.clone();
        let info = WebReleaseInfo {
            version: version.clone(),
            current_version: current_version.to_string(),
            release_tag: index.web.tag,
            html_url: index.web.release_url,
            body: index.web.notes,
            channel,
            prerelease: index.channel == "preview",
            update_available: compare_versions(&version, current_version),
            size: asset.size,
        };
        let temp_dir = tempfile::Builder::new()
            .prefix("niupanel_web_update_")
            .tempdir()
            .map_err(AppError::Io)?;
        let package_path = temp_dir.path().join(&asset.name);
        download_component_asset(self, &asset, &package_path).await?;
        verify_downloaded_asset(
            &asset.name,
            asset.size,
            &asset.sha256,
            asset.size,
            &package_path,
        )?;
        Ok(DownloadedWebRelease {
            info,
            package_path,
            temp_dir,
        })
    }

    pub async fn execute_update(&self) -> Result<()> {
        if !launcher_managed() {
            return Err(AppError::ValidationError(
                "当前为直接启动模式，Core 更新需要由 niupanel-launcher 启动服务；Docker 部署请更新镜像"
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

    pub async fn execute_local_update(&self, package_path: tempfile::TempPath) -> Result<()> {
        if !launcher_managed() {
            return Err(AppError::ValidationError(
                "当前为直接启动模式，Core 更新需要由 niupanel-launcher 启动服务；Docker 部署请更新镜像"
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
            state: UpdateState::Installing,
            progress: 100,
            message: "准备安装本地更新包...".to_string(),
            error: None,
        };
        drop(status_guard);

        let self_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = perform_local_update_task(self_clone.clone(), token, package_path).await
            {
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

fn parse_plain_version(value: &str, label: &str) -> Result<Version> {
    let version = Version::parse(value)
        .map_err(|error| AppError::ValidationError(format!("{label} 不是有效版本号: {error}")))?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err(AppError::ValidationError(format!(
            "{label} 必须使用纯数字版本号，不能包含预发布或构建后缀"
        )));
    }
    Ok(version)
}

fn validate_asset_descriptor(name: &str, url: &str, sha256: &str, size: u64) -> Result<()> {
    if name.is_empty()
        || Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name)
        || !url.starts_with("https://")
        || size == 0
        || sha256.len() != 64
        || !sha256.chars().all(|value| value.is_ascii_hexdigit())
    {
        return Err(AppError::ValidationError(format!(
            "更新索引中的资产 {} 描述不合法",
            name
        )));
    }
    Ok(())
}

fn validate_update_channel_index(index: &UpdateChannelIndex, expected_channel: &str) -> Result<()> {
    if index.schema_version != UPDATE_CHANNEL_INDEX_SCHEMA_VERSION {
        return Err(AppError::ValidationError(format!(
            "不支持的更新索引 schema {}",
            index.schema_version
        )));
    }
    if index.channel != expected_channel || !matches!(index.channel.as_str(), "preview" | "stable")
    {
        return Err(AppError::ValidationError(format!(
            "更新索引通道 {} 与请求通道 {} 不一致",
            index.channel, expected_channel
        )));
    }
    let minimum_supported = parse_plain_version(MINIMUM_UPDATE_CORE_VERSION, "最低支持 Core 版本")?;
    let core_version = parse_plain_version(&index.core.version, "Core 版本")?;
    if core_version < minimum_supported {
        return Err(AppError::ValidationError(format!(
            "Core {} 低于最低支持更新基线 {}",
            index.core.version, MINIMUM_UPDATE_CORE_VERSION
        )));
    }
    if index.core.tag != format!("core-v{}", index.core.version) {
        return Err(AppError::ValidationError(
            "Core Tag 与版本不一致".to_string(),
        ));
    }
    if index.web.tag != format!("web-v{}", index.web.version) {
        return Err(AppError::ValidationError(
            "Web Tag 与版本不一致".to_string(),
        ));
    }
    if !index.core.release_url.starts_with("https://")
        || !index.web.release_url.starts_with("https://")
    {
        return Err(AppError::ValidationError(
            "更新索引中的 Release 地址不合法".to_string(),
        ));
    }
    if index.core.api_contract != API_CONTRACT_VERSION
        || index.web.api_contract != API_CONTRACT_VERSION
    {
        return Err(AppError::ValidationError(format!(
            "更新索引 API contract 不兼容：Core={}, Web={}, 当前={}",
            index.core.api_contract, index.web.api_contract, API_CONTRACT_VERSION
        )));
    }
    if index.core.api_contract != index.web.api_contract {
        return Err(AppError::ValidationError(
            "更新索引 Core 与 Web 的 API contract 不一致".to_string(),
        ));
    }
    if index.core.launcher_protocol != niupanel_common::version::RELEASE_PROTOCOL_VERSION {
        return Err(AppError::ValidationError(format!(
            "更新索引 Core launcher 协议 {} 与当前协议不兼容",
            index.core.launcher_protocol
        )));
    }

    for (architecture, target) in [
        ("x86_64", "x86_64-unknown-linux-musl"),
        ("aarch64", "aarch64-unknown-linux-musl"),
        ("armv7", "armv7-unknown-linux-musleabihf"),
    ] {
        let asset = index.core.assets.get(architecture).ok_or_else(|| {
            AppError::ValidationError(format!("更新索引缺少 {architecture} Core 资产"))
        })?;
        validate_asset_descriptor(&asset.name, &asset.url, &asset.sha256, asset.size)?;
        if asset.target != target {
            return Err(AppError::ValidationError(format!(
                "Core 资产 {} 的 target {} 与架构 {} 不一致",
                asset.name, asset.target, architecture
            )));
        }
    }
    validate_asset_descriptor(
        &index.web.asset.name,
        &index.web.asset.url,
        &index.web.asset.sha256,
        index.web.asset.size,
    )?;
    let minimum = parse_plain_version(&index.web.core.min, "Web 最低 Core 版本")?;
    if core_version < minimum {
        return Err(AppError::ValidationError(format!(
            "Core {} 低于 Web 最低要求 {}",
            index.core.version, index.web.core.min
        )));
    }
    if let Some(maximum) = index.web.core.max.as_deref() {
        let maximum = parse_plain_version(maximum, "Web 最高 Core 版本")?;
        if core_version > maximum {
            return Err(AppError::ValidationError(format!(
                "Core {} 高于 Web 最高支持版本 {}",
                index.core.version, maximum
            )));
        }
    }
    Ok(())
}

fn core_asset_for_current_target(index: &UpdateChannelIndex) -> Result<&UpdateCoreAsset> {
    let architecture = match std::env::consts::ARCH {
        "arm" => "armv7",
        value => value,
    };
    index.core.assets.get(architecture).ok_or_else(|| {
        AppError::ExternalApi(format!(
            "Core {} 不支持当前架构 {}",
            index.core.version, architecture
        ))
    })
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

async fn download_component_asset(
    service: &UpdateService,
    asset: &UpdateAsset,
    package_path: &Path,
) -> Result<()> {
    let token = CancellationToken::new();
    download_indexed_asset(
        service,
        &token,
        &asset.name,
        &asset.url,
        asset.size,
        package_path,
        false,
    )
    .await
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
    let index = service.fetch_update_index(&channel).await?;
    if token.is_cancelled() {
        return Err(AppError::Cancelled);
    }

    if let Ok(mut guard) = service.update_status.write() {
        guard.state = UpdateState::Downloading;
        guard.message = "正在下载更新包...".to_string();
        guard.progress = 0;
    }

    let asset = core_asset_for_current_target(&index)?.clone();
    let temp_dir = tempfile::Builder::new()
        .prefix("niupanel_update_download_")
        .tempdir()
        .map_err(AppError::Io)?;
    let package_path = temp_dir.path().join(&asset.name);

    download_indexed_asset(
        &service,
        &token,
        &asset.name,
        &asset.url,
        asset.size,
        &package_path,
        true,
    )
    .await?;
    verify_downloaded_asset(
        &asset.name,
        asset.size,
        &asset.sha256,
        asset.size,
        &package_path,
    )?;
    install_update_archive(service, temp_dir, package_path).await
}

async fn perform_local_update_task(
    service: UpdateService,
    token: CancellationToken,
    package_path: tempfile::TempPath,
) -> Result<()> {
    if token.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let temp_dir = tempfile::Builder::new()
        .prefix("niupanel_local_update_")
        .tempdir()
        .map_err(AppError::Io)?;
    let result = install_update_archive(service, temp_dir, package_path.to_path_buf()).await;
    drop(package_path);
    result
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

fn copy_release_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(AppError::Io)?;
    for entry in fs::read_dir(source).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        let file_type = entry.file_type().map_err(AppError::Io)?;
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
                "Core 更新包 tools 只能包含普通文件和目录".to_string(),
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
    stage_release_tools(&package_root, &release_root)?;
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
                    url: format!("https://example.com/{architecture}.tar.gz"),
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
            core: niupanel_common::version::UpdateCoreComponent {
                version: "0.8.1".into(),
                tag: "core-v0.8.1".into(),
                release_url: "https://example.com/core".into(),
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
                release_url: "https://example.com/web".into(),
                notes: String::new(),
                api_contract: API_CONTRACT_VERSION,
                core: niupanel_common::version::ComponentCompatibility {
                    min: "0.8.1".into(),
                    max: None,
                },
                asset: UpdateAsset {
                    name: "niupanel_web_2.0.0.tar.gz".into(),
                    url: "https://example.com/web.tar.gz".into(),
                    sha256: "b".repeat(64),
                    size: 2,
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
        index.core.version = "0.7.9".into();
        index.core.tag = "core-v0.7.9".into();

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
