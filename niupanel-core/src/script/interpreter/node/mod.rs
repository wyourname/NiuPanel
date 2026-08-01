mod pnpm;

use crate::sys::tools::ToolService;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::{debug, info};
use pnpm::PnpmManager;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

const DEFAULT_NODE_DIST_MIRROR: &str = "https://mirrors.ustc.edu.cn/node/";
const DEFAULT_VERSION_FILE: &str = "default-version";

pub struct NodeEnvironment {
    pub mirrors: HashMap<String, String>,
    pub version: Option<String>,
}

#[derive(Debug, Default)]
pub struct NodeVersionCatalog {
    pub recommended_lts: Option<String>,
    pub versions: String,
}

#[derive(Deserialize)]
struct NodeRelease {
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    lts: serde_json::Value,
    version: String,
}

impl NodeEnvironment {
    pub fn get_pnpm_bin() -> PathBuf {
        PnpmManager::resolve_bin().unwrap_or_else(|| PathBuf::from("pnpm"))
    }

    pub async fn ensure_pnpm_installed(
        mirrors: Option<&HashMap<String, String>>,
    ) -> Result<PathBuf> {
        let empty = HashMap::new();
        let pnpm = PnpmManager::ensure_installed(mirrors.unwrap_or(&empty)).await?;
        info!("Using pnpm from {}", pnpm.display());
        Ok(pnpm)
    }

    /// 打开已有的 Node 环境。该操作不下载运行时。
    pub fn open(
        _env_dir: PathBuf,
        version: String,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        Ok(Self {
            mirrors: mirrors.unwrap_or_default(),
            version: Self::normalize_version(&version),
        })
    }

    /// 创建或打开一个由 `pnpm runtime` 管理的 Node 版本目录。
    pub async fn new(
        _env_dir: PathBuf,
        version: String,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        let mirrors = mirrors.unwrap_or_default();
        let version = Self::normalize_version(&version).ok_or_else(|| {
            AppError::ValidationError("Node version must be specified".to_string())
        })?;

        Self::ensure_pnpm_installed(Some(&mirrors)).await?;
        Self::ensure_runtime_project(&version, &mirrors).await?;

        Ok(Self {
            mirrors,
            version: Some(version),
        })
    }

    pub async fn create(
        env_dir: PathBuf,
        version: String,
        sender: UnboundedSender<String>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<()> {
        info!("Downloading Node runtime version...");
        let _ = sender.send(format!("Downloading Node {} with pnpm runtime...", version));
        Self::new(env_dir, version, mirrors).await?;
        let _ = sender.send("Node version installed successfully.".to_string());
        Ok(())
    }

    pub fn normalize_version(version: &str) -> Option<String> {
        let trimmed = version.trim();
        if trimmed.is_empty() || trimmed == "system" || trimmed == "default" {
            return None;
        }

        let value = trimmed
            .strip_prefix("venv_")
            .unwrap_or(trimmed)
            .split_whitespace()
            .next()?
            .trim_start_matches('v');
        if value.is_empty()
            || !value.as_bytes().first().is_some_and(u8::is_ascii_digit)
            || value.contains("..")
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
        {
            return None;
        }
        Some(value.to_string())
    }

    pub fn shared_root_for_version(version: &str) -> PathBuf {
        absolute_config_path(
            Config::global()
                .runtimes_dir
                .join("node")
                .join("shared")
                .join(version),
        )
    }

    pub fn shared_node_modules_for_version(version: &str) -> PathBuf {
        Self::shared_root_for_version(version).join("node_modules")
    }

    pub fn shared_bin_for_version(version: &str) -> PathBuf {
        Self::shared_node_modules_for_version(version).join(".bin")
    }

    pub fn node_bin_for_version(version: &str) -> PathBuf {
        Self::shared_bin_for_version(version).join("node")
    }

    pub fn package_install_path(pkg: &str) -> PathBuf {
        let normalized = pkg.trim();

        if let Some(scoped) = normalized.strip_prefix('@')
            && let Some((scope, rest)) = scoped.split_once('/')
        {
            let package_name = rest.rsplit_once('@').map(|(name, _)| name).unwrap_or(rest);
            return PathBuf::from(format!("@{}", scope)).join(package_name);
        }

        let package_name = normalized
            .split_once('@')
            .map(|(name, _)| name)
            .unwrap_or(normalized);
        PathBuf::from(package_name)
    }

    pub async fn list_local_node_versions() -> Result<Vec<(String, bool)>> {
        let default = Self::read_default_version().await;
        let shared_root = Self::shared_root();
        let mut versions = Vec::new();
        let mut entries = match tokio::fs::read_dir(&shared_root).await {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(versions),
            Err(error) => return Err(AppError::Io(error)),
        };

        while let Some(entry) = entries.next_entry().await.map_err(AppError::Io)? {
            if !entry.file_type().await.map_err(AppError::Io)?.is_dir() {
                continue;
            }
            let version = entry.file_name().to_string_lossy().to_string();
            if Self::normalize_version(&version).as_deref() != Some(version.as_str())
                || !Self::node_bin_for_version(&version).is_file()
            {
                continue;
            }
            versions.push((
                version.clone(),
                default.as_deref() == Some(version.as_str()),
            ));
        }
        versions.sort_by(|left, right| compare_node_versions(&right.0, &left.0));
        if !versions.iter().any(|(_, is_default)| *is_default) && !versions.is_empty() {
            versions[0].1 = true;
        }
        Ok(versions)
    }

    pub async fn resolve_default_version() -> Result<String> {
        if let Some(version) = Self::read_default_version().await
            && Self::node_bin_for_version(&version).is_file()
        {
            return Ok(version);
        }

        let installed = Self::list_local_node_versions().await?;
        if let Some((version, _)) = installed.first() {
            return Ok(version.clone());
        }
        Err(AppError::ValidationError(
            "No Node.js runtime is installed. Install a version before running this script."
                .to_string(),
        ))
    }

    pub async fn set_default_version(version: &str) -> Result<()> {
        let version = Self::normalize_version(version).ok_or_else(|| {
            AppError::ValidationError("Node version must be specified".to_string())
        })?;
        if !Self::node_bin_for_version(&version).is_file() {
            return Err(AppError::ValidationError(format!(
                "Node.js {version} is not installed"
            )));
        }
        let marker = Self::default_version_path();
        if let Some(parent) = marker.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(AppError::Io)?;
        }
        tokio::fs::write(marker, format!("{version}\n"))
            .await
            .map_err(AppError::Io)
    }

    pub async fn uninstall_node_version(version: &str) -> Result<()> {
        let version = Self::normalize_version(version).ok_or_else(|| {
            AppError::ValidationError("Node version must be specified".to_string())
        })?;
        let root = Self::shared_root_for_version(&version);
        let metadata = match tokio::fs::symlink_metadata(&root).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(AppError::Io(error)),
        };
        if metadata.file_type().is_symlink() {
            tokio::fs::remove_file(&root).await.map_err(AppError::Io)?;
        } else {
            tokio::fs::remove_dir_all(&root)
                .await
                .map_err(AppError::Io)?;
        }

        if Self::read_default_version().await.as_deref() == Some(version.as_str()) {
            match tokio::fs::remove_file(Self::default_version_path()).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(AppError::Io(error)),
            }
        }
        Ok(())
    }

    pub async fn command(&self, script_path: &Path) -> Result<std::process::Command> {
        let version = self.resolved_version().await?;
        self.command_with_version(script_path, &version).await
    }

    pub async fn command_with_version(
        &self,
        script_path: &Path,
        version: &str,
    ) -> Result<std::process::Command> {
        let version = Self::normalize_version(version).ok_or_else(|| {
            AppError::ValidationError("Node version must be specified".to_string())
        })?;
        let node = Self::node_bin_for_version(&version);
        if !node.is_file() {
            return Err(AppError::Environment(format!(
                "Node.js {version} is not installed"
            )));
        }

        let mut envs = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut envs, &version);
        let mut cmd = std::process::Command::new(node);
        cmd.arg(script_path).envs(envs);

        let existing_node_options = std::env::var("NODE_OPTIONS").unwrap_or_default();
        if !existing_node_options.contains("--max-old-space-size") {
            let separator = if existing_node_options.is_empty() {
                ""
            } else {
                " "
            };
            cmd.env(
                "NODE_OPTIONS",
                format!("{existing_node_options}{separator}--max-old-space-size=2048"),
            );
        }
        Ok(cmd)
    }

    pub async fn install_packages(
        &self,
        packages: &[String],
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        let version = self.resolved_version().await?;
        self.install_packages_with_version(packages, &version, sender)
            .await
    }

    pub async fn install_packages_with_version(
        &self,
        packages: &[String],
        version: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let version = Self::normalize_version(version).ok_or_else(|| {
            AppError::ValidationError(
                "Node version must be specified for package install".to_string(),
            )
        })?;
        Self::ensure_runtime_project(&version, &self.mirrors).await?;
        let shared_root = Self::shared_root_for_version(&version);
        let package_list = packages.join(" ");
        let _ = sender.send(format!(
            "Installing packages '{}' with pnpm...",
            package_list
        ));
        let mut args = vec![
            "add".to_string(),
            "--save-prod".to_string(),
            "--".to_string(),
        ];
        args.extend_from_slice(packages);
        let mut envs = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut envs, &version);

        ToolService::run_streaming(
            &Self::get_pnpm_bin(),
            &args,
            Some(&shared_root),
            Some(&envs),
            &format!("pnpm add {} ({version})", package_list),
            sender,
        )
        .await
    }

    pub async fn uninstall_package(
        &self,
        package_name: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        let version = self.resolved_version().await?;
        Self::ensure_runtime_project(&version, &self.mirrors).await?;
        let shared_root = Self::shared_root_for_version(&version);
        let mut envs = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut envs, &version);
        let args = vec![
            "remove".to_string(),
            "--".to_string(),
            package_name.to_string(),
        ];
        let _ = sender.send(format!(
            "Uninstalling package {} with pnpm...",
            package_name
        ));
        ToolService::run_streaming(
            &Self::get_pnpm_bin(),
            &args,
            Some(&shared_root),
            Some(&envs),
            "pnpm remove",
            sender,
        )
        .await
    }

    pub async fn list_packages(&self) -> Result<String> {
        let version = self.resolved_version().await?;
        Self::ensure_runtime_project(&version, &self.mirrors).await?;
        let shared_root = Self::shared_root_for_version(&version);
        let mut envs = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut envs, &version);
        let output = ToolService::capture_output(
            &Self::get_pnpm_bin(),
            &["list", "--depth=0", "--json"],
            Some(&shared_root),
            Some(&envs),
        )
        .await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(serde_json::json!({ "dependencies": {} }).to_string());
        }

        let value: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|error| AppError::Environment(format!("Invalid pnpm list output: {error}")))?;
        let mut project = value
            .as_array()
            .and_then(|projects| projects.first())
            .cloned()
            .unwrap_or(value);
        if let Some(object) = project.as_object_mut() {
            object
                .entry("dependencies")
                .or_insert_with(|| serde_json::json!({}));
        }
        serde_json::to_string(&project).map_err(|error| AppError::Generic(error.to_string()))
    }

    pub async fn list_available_node_versions(
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<String> {
        Ok(Self::available_node_version_catalog(mirrors)
            .await?
            .versions)
    }

    pub async fn available_node_version_catalog(
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<NodeVersionCatalog> {
        let mirrors = mirrors.unwrap_or_default();
        let base = node_dist_mirror(&mirrors)?;
        let url = format!("{}/index.json", base.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|error| AppError::Generic(error.to_string()))?;
        let releases = client
            .get(url)
            .send()
            .await
            .map_err(|error| {
                AppError::Environment(format!("Node.js version query failed: {error}"))
            })?
            .error_for_status()
            .map_err(|error| {
                AppError::Environment(format!("Node.js version query failed: {error}"))
            })?
            .json::<Vec<NodeRelease>>()
            .await
            .map_err(|error| {
                AppError::Environment(format!("Invalid Node.js version index: {error}"))
            })?;
        Ok(node_version_catalog_from_releases(
            releases,
            node_release_file_for_arch(std::env::consts::ARCH),
        ))
    }

    pub async fn version(&self) -> Result<String> {
        let version = self.resolved_version().await?;
        let output = ToolService::capture_output(
            &Self::node_bin_for_version(&version),
            &["--version"],
            None,
            Some(&self.runtime_envs()?),
        )
        .await?;
        if !output.status.success() {
            return Err(AppError::Environment(format!(
                "Node.js {version} failed to run"
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn get_default_node_bin_dir() -> Option<PathBuf> {
        let version = Self::resolve_default_version().await.ok()?;
        let bin = Self::shared_bin_for_version(&version);
        bin.is_dir().then_some(bin)
    }

    pub fn runtime_envs(&self) -> Result<HashMap<String, String>> {
        let mut envs = PnpmManager::build_env(Some(&self.mirrors));
        envs.insert("NO_COLOR".to_string(), "1".to_string());
        Ok(envs)
    }

    pub fn inject_shared_dependency_env(env: &mut HashMap<String, String>, version: &str) {
        let node_modules = Self::shared_node_modules_for_version(version);
        let bin_dir = Self::shared_bin_for_version(version);
        Self::inject_dependency_paths(env, &node_modules, &bin_dir);
    }

    /// Adds a Node.js dependency root and its executable directory to a child-process environment.
    ///
    /// Keeping this path merge independent from the configured runtime layout lets all script
    /// executors reuse the same resolution behavior and makes it testable without global config.
    pub fn inject_dependency_paths(
        env: &mut HashMap<String, String>,
        node_modules: &Path,
        bin_dir: &Path,
    ) {
        prepend_path_like(env, "NODE_PATH", node_modules.to_path_buf());
        prepend_path_like(env, "PATH", bin_dir.to_path_buf());
    }

    async fn resolved_version(&self) -> Result<String> {
        match self.version.clone() {
            Some(version) => Ok(version),
            None => Self::resolve_default_version().await,
        }
    }

    async fn ensure_runtime_project(
        version: &str,
        mirrors: &HashMap<String, String>,
    ) -> Result<()> {
        let shared_root = Self::shared_root_for_version(version);
        tokio::fs::create_dir_all(&shared_root)
            .await
            .map_err(AppError::Io)?;
        ensure_shared_package_json(&shared_root).await?;
        ensure_pnpm_workspace(&shared_root, mirrors).await?;

        let node = Self::node_bin_for_version(version);
        if node_matches_version(&node, version).await {
            debug!("Node {} already installed via pnpm runtime.", version);
            return Ok(());
        }

        debug!("Installing Node {} via pnpm runtime...", version);
        let pnpm = Self::ensure_pnpm_installed(Some(mirrors)).await?;
        let envs = PnpmManager::build_env(Some(mirrors));
        ToolService::run_checked(
            &pnpm,
            &["runtime", "set", "node", version],
            Some(&shared_root),
            Some(&envs),
            "pnpm runtime set node",
        )
        .await?;

        if !node_matches_version(&node, version).await {
            return Err(AppError::Environment(format!(
                "pnpm completed but Node.js {version} is unavailable"
            )));
        }
        Ok(())
    }

    fn shared_root() -> PathBuf {
        absolute_config_path(Config::global().runtimes_dir.join("node").join("shared"))
    }

    fn default_version_path() -> PathBuf {
        absolute_config_path(
            Config::global()
                .runtimes_dir
                .join("node")
                .join(DEFAULT_VERSION_FILE),
        )
    }

    async fn read_default_version() -> Option<String> {
        let content = tokio::fs::read_to_string(Self::default_version_path())
            .await
            .ok()?;
        Self::normalize_version(&content)
    }
}

async fn ensure_shared_package_json(shared_root: &Path) -> Result<()> {
    let package_json = shared_root.join("package.json");
    if package_json.exists() {
        return Ok(());
    }

    let content = serde_json::to_vec_pretty(&serde_json::json!({
        "private": true,
        "name": "niupanel-node-shared-environment"
    }))
    .map_err(|error| AppError::Generic(error.to_string()))?;
    tokio::fs::write(package_json, content)
        .await
        .map_err(AppError::Io)
}

async fn ensure_pnpm_workspace(
    shared_root: &Path,
    mirrors: &HashMap<String, String>,
) -> Result<()> {
    let mirror = node_dist_mirror(mirrors)?;
    let quoted =
        serde_json::to_string(&mirror).map_err(|error| AppError::Generic(error.to_string()))?;
    let content = format!("nodeDownloadMirrors:\n  release: {quoted}\n");
    tokio::fs::write(shared_root.join("pnpm-workspace.yaml"), content)
        .await
        .map_err(AppError::Io)
}

fn node_dist_mirror(mirrors: &HashMap<String, String>) -> Result<String> {
    let value = mirrors
        .get("PNPM_NODE_DIST_MIRROR")
        .or_else(|| mirrors.get("FNM_NODE_DIST_MIRROR"))
        .map(String::as_str)
        .unwrap_or(DEFAULT_NODE_DIST_MIRROR)
        .trim();
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| AppError::ValidationError("无效的 Node.js 发行版镜像 URL".to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::ValidationError(
            "Node.js 发行版镜像仅支持 HTTP 或 HTTPS".to_string(),
        ));
    }
    Ok(value.trim_end_matches('/').to_string())
}

async fn node_matches_version(node: &Path, expected: &str) -> bool {
    let Ok(output) = tokio::process::Command::new(node)
        .arg("--version")
        .output()
        .await
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let actual = String::from_utf8_lossy(&output.stdout);
    actual.trim().trim_start_matches('v') == expected
}

fn compare_node_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| {
        value
            .split(['.', '-', '+'])
            .take(3)
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    parse(left).cmp(&parse(right)).then_with(|| left.cmp(right))
}

fn node_release_file_for_arch(arch: &str) -> Option<&'static str> {
    match arch {
        "x86_64" => Some("linux-x64"),
        "aarch64" => Some("linux-arm64"),
        "arm" => Some("linux-armv7l"),
        _ => None,
    }
}

fn node_version_catalog_from_releases(
    releases: Vec<NodeRelease>,
    required_file: Option<&str>,
) -> NodeVersionCatalog {
    let mut versions = Vec::new();
    let mut recommended_lts = None;

    for release in releases {
        if required_file.is_some_and(|file| !release.files.iter().any(|item| item == file)) {
            continue;
        }
        let Some(version) = NodeEnvironment::normalize_version(&release.version) else {
            continue;
        };
        if recommended_lts.is_none() && release.lts.as_str().is_some() {
            recommended_lts = Some(version.clone());
        }
        versions.push(format!("v{version}"));
    }

    NodeVersionCatalog {
        recommended_lts,
        versions: versions.join("\n"),
    }
}

fn prepend_path_like(env: &mut HashMap<String, String>, key: &str, path: PathBuf) {
    let existing = env
        .get(key)
        .cloned()
        .map(OsString::from)
        .or_else(|| std::env::var_os(key));
    let mut entries = vec![path];
    if let Some(existing) = existing {
        for entry in std::env::split_paths(&existing) {
            if !entries.contains(&entry) {
                entries.push(entry);
            }
        }
    }
    if let Ok(joined) = std::env::join_paths(entries) {
        env.insert(key.to_string(), joined.to_string_lossy().to_string());
    }
}

fn absolute_config_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(&path))
        .unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::{NodeEnvironment, NodeRelease, node_version_catalog_from_releases};

    #[test]
    fn node_version_normalization_rejects_paths() {
        assert_eq!(
            NodeEnvironment::normalize_version("v22.11.0"),
            Some("22.11.0".to_string())
        );
        assert_eq!(NodeEnvironment::normalize_version("../../tmp"), None);
        assert_eq!(NodeEnvironment::normalize_version("22/../../tmp"), None);
    }

    #[test]
    fn package_paths_handle_scoped_and_versioned_packages() {
        assert_eq!(
            NodeEnvironment::package_install_path("@scope/pkg@1.2.3"),
            std::path::PathBuf::from("@scope/pkg")
        );
        assert_eq!(
            NodeEnvironment::package_install_path("lodash@4.17.21"),
            std::path::PathBuf::from("lodash")
        );
    }

    #[test]
    fn recommended_lts_respects_the_runtime_architecture() {
        let releases = || {
            vec![
                NodeRelease {
                    files: vec!["linux-x64".to_string(), "linux-arm64".to_string()],
                    lts: serde_json::json!("Krypton"),
                    version: "v24.18.0".to_string(),
                },
                NodeRelease {
                    files: vec![
                        "linux-x64".to_string(),
                        "linux-arm64".to_string(),
                        "linux-armv7l".to_string(),
                    ],
                    lts: serde_json::json!("Jod"),
                    version: "v22.23.1".to_string(),
                },
            ]
        };

        let x64 = node_version_catalog_from_releases(releases(), Some("linux-x64"));
        assert_eq!(x64.recommended_lts.as_deref(), Some("24.18.0"));

        let armv7 = node_version_catalog_from_releases(releases(), Some("linux-armv7l"));
        assert_eq!(armv7.recommended_lts.as_deref(), Some("22.23.1"));
        assert!(!armv7.versions.contains("v24.18.0"));
    }
}
