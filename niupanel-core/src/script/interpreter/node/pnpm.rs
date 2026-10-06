use niupanel_common::config::Config;
use niupanel_common::download::{DownloadOptions, download_with_resume};
use niupanel_common::error::{AppError, Result};
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

pub const PNPM_VERSION: &str = "12.9.1";
const DEFAULT_REGISTRY: &str = "https://registry.npmmirror.com";
const MAX_ARCHIVE_SIZE: u64 = 80 * 1024 * 1024;

static INSTALL_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Clone, Copy)]
struct Artifact {
    package: &'static str,
    archive: &'static str,
    size: u64,
    sha512: &'static str,
}

const BASE_ARTIFACT: Artifact = Artifact {
    package: "@pnpm/exe",
    archive: "exe",
    size: 908_979,
    sha512: "4d9395bd8abc9f0ce6c3c211dcfd4629993ceed57bbc33c24f7a0994bec886060d65f2c6a2fe235f427c449c6e017c484f7bb183fba27289eb2cb06ff97e4ac8",
};

const LINUX_X64: Artifact = Artifact {
    package: "@pnpm/exe.linux-x64",
    archive: "exe.linux-x64",
    size: 23_369_948,
    sha512: "f8ce4def884ce1f73cec001c931c7f5085be7549d67f67a4a3e1dc849b3f3595317a12e17a4372308fc635a35796f77e91647ef7b5d7043e755d192436252dac",
};

const LINUX_STATIC_X64: Artifact = Artifact {
    package: "@pnpm/exe.linux-x64-musl",
    archive: "exe.linux-x64-musl",
    size: 23_540_622,
    sha512: "9900e106c1aa71392b2d0338ab98a099115cafa10097850cfc519ac18fb409619ca8fa5499852f0f2f4b1f2f34c71907216549f3cea0befb28d973a37dabf527",
};

const LINUX_ARM64: Artifact = Artifact {
    package: "@pnpm/exe.linux-arm64",
    archive: "exe.linux-arm64",
    size: 21_515_849,
    sha512: "731fae17ac6ac04b66ec1f0d57ed6f7b4c4183d81cbb3419e51fb45a0396bca1bf87defc5258d2e81816b358a9206b8681475e5619b73ca98196d5c871cccb5c",
};

const LINUX_STATIC_ARM64: Artifact = Artifact {
    package: "@pnpm/exe.linux-arm64-musl",
    archive: "exe.linux-arm64-musl",
    size: 21_673_973,
    sha512: "7592f6112d5c90358cf6ff84b71a0e92567458d76d6b2a288bd88b88352e6e8a6e757df35cf351da696bc414608db0e01023a77507a7d929b2c4be5e66206e2b",
};

pub struct PnpmManager;

impl PnpmManager {
    pub fn managed_root() -> PathBuf {
        absolute_config_path(
            Config::global()
                .runtimes_dir
                .join("node")
                .join("tools")
                .join("pnpm"),
        )
    }

    pub fn managed_bin() -> PathBuf {
        Self::managed_root().join(PNPM_VERSION).join("pnpm")
    }

    pub fn pnpm_home() -> PathBuf {
        absolute_config_path(Config::global().runtimes_dir.join("node").join("pnpm-home"))
    }

    pub fn resolve_bin() -> Option<PathBuf> {
        let managed = Self::managed_bin();
        if managed.is_file() {
            return Some(managed);
        }
        niupanel_common::tools::resolve_tool_path("pnpm")
    }

    pub async fn ensure_installed(mirrors: &HashMap<String, String>) -> Result<PathBuf> {
        if let Some(path) = Self::resolve_bin()
            && pnpm_is_supported(&path).await
        {
            return Ok(path);
        }

        let _guard = INSTALL_LOCK.lock().await;
        if let Some(path) = Self::resolve_bin()
            && pnpm_is_supported(&path).await
        {
            return Ok(path);
        }

        Self::install_managed(mirrors).await?;
        let path = Self::managed_bin();
        if !pnpm_is_supported(&path).await {
            return Err(AppError::Environment(format!(
                "自动安装的 pnpm {} 无法运行，请检查系统 libc 和 libstdc++ 运行库",
                PNPM_VERSION
            )));
        }
        Ok(path)
    }

    pub fn build_env(extra_envs: Option<&HashMap<String, String>>) -> HashMap<String, String> {
        let mut envs = extra_envs.cloned().unwrap_or_default();
        let pnpm_home = Self::pnpm_home();
        let registry = envs
            .get("npm_config_registry")
            .or_else(|| envs.get("NPM_CONFIG_REGISTRY"))
            .cloned()
            .unwrap_or_else(|| DEFAULT_REGISTRY.to_string());
        envs.insert("npm_config_registry".to_string(), registry);
        envs.insert(
            "PNPM_HOME".to_string(),
            pnpm_home.to_string_lossy().to_string(),
        );
        prepend_path(&mut envs, pnpm_home);
        if let Some(bin) = Self::resolve_bin()
            && let Some(parent) = bin.parent()
        {
            prepend_path(&mut envs, parent.to_path_buf());
        }
        envs
    }

    async fn install_managed(mirrors: &HashMap<String, String>) -> Result<()> {
        let platform = platform_artifact()?;
        let registry = registry_base(mirrors)?;
        let install_root = Self::managed_root();
        let target = install_root.join(PNPM_VERSION);
        if target.exists() {
            tokio::fs::remove_dir_all(&target)
                .await
                .map_err(AppError::Io)?;
        }

        tokio::fs::create_dir_all(&install_root)
            .await
            .map_err(AppError::Io)?;
        let staging = tempfile::tempdir_in(&install_root).map_err(AppError::Io)?;
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|error| AppError::Generic(error.to_string()))?;

        let artifacts = [BASE_ARTIFACT, platform].into_iter();
        for (index, artifact) in artifacts.enumerate() {
            let archive_path = staging.path().join(format!("artifact-{index}.tgz"));
            let url = artifact_url(&registry, artifact);
            download_with_resume(
                &client,
                &url,
                &archive_path,
                Some(artifact.size),
                DownloadOptions {
                    timeout: Some(Duration::from_secs(180)),
                    retries: 3,
                    max_size: Some(MAX_ARCHIVE_SIZE),
                    ..DownloadOptions::default()
                },
                || false,
                |_| async {},
            )
            .await?;
            verify_sha512(&archive_path, artifact.sha512).await?;
            unpack_tgz(&archive_path, staging.path()).await?;
        }

        let pnpm = staging.path().join("package").join("pnpm");
        set_executable(&pnpm).await?;
        if !pnpm_is_supported(&pnpm).await {
            return Err(AppError::Environment(
                "下载的 pnpm 独立版未通过运行检查".to_string(),
            ));
        }

        tokio::fs::rename(staging.path().join("package"), &target)
            .await
            .map_err(AppError::Io)?;
        Ok(())
    }
}

fn platform_artifact() -> Result<Artifact> {
    if std::env::consts::OS != "linux" {
        return Err(AppError::Environment(
            "NiuPanel 当前仅支持在 Linux 上自动安装 pnpm 独立版".to_string(),
        ));
    }

    let glibc = has_glibc();
    match (std::env::consts::ARCH, glibc) {
        ("x86_64", true) => Ok(LINUX_X64),
        ("x86_64", false) => Ok(LINUX_STATIC_X64),
        ("aarch64", true) => Ok(LINUX_ARM64),
        ("aarch64", false) => Ok(LINUX_STATIC_ARM64),
        (arch, _) => Err(AppError::Environment(format!(
            "pnpm 独立版暂不支持当前架构 {arch}；可通过 NIUPANEL_TOOL_PNPM_PATH 提供 pnpm 12"
        ))),
    }
}

pub(super) fn has_glibc() -> bool {
    std::process::Command::new("getconf")
        .arg("GNU_LIBC_VERSION")
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains("glibc")
        })
}

fn registry_base(mirrors: &HashMap<String, String>) -> Result<String> {
    let value = mirrors
        .get("npm_config_registry")
        .or_else(|| mirrors.get("NPM_CONFIG_REGISTRY"))
        .map(String::as_str)
        .unwrap_or(DEFAULT_REGISTRY)
        .trim()
        .trim_end_matches('/');
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| AppError::ValidationError("无效的 pnpm/npm Registry URL".to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::ValidationError(
            "pnpm/npm Registry 仅支持 HTTP 或 HTTPS".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn artifact_url(registry: &str, artifact: Artifact) -> String {
    format!(
        "{registry}/{}/-/{}-{PNPM_VERSION}.tgz",
        artifact.package, artifact.archive
    )
}

async fn pnpm_is_supported(path: &Path) -> bool {
    let output = tokio::process::Command::new(path)
        .arg("--version")
        .output()
        .await;
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    pnpm_version_is_supported(&String::from_utf8_lossy(&output.stdout))
}

fn pnpm_version_is_supported(version: &str) -> bool {
    fn parse(value: &str) -> Option<(u64, u64, u64)> {
        let mut parts = value.trim().split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.split('-').next()?.parse().ok()?;
        Some((major, minor, patch))
    }

    parse(version)
        .zip(parse(PNPM_VERSION))
        .is_some_and(|(actual, required)| actual >= required)
}

async fn verify_sha512(path: &Path, expected: &str) -> Result<()> {
    let mut file = tokio::fs::File::open(path).await.map_err(AppError::Io)?;
    let mut hasher = Sha512::new();
    let mut buffer = vec![0_u8; 128 * 1024];
    loop {
        let count = file.read(&mut buffer).await.map_err(AppError::Io)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual != expected {
        return Err(AppError::Environment(format!(
            "pnpm 下载文件校验失败：期望 {expected}，实际 {actual}"
        )));
    }
    Ok(())
}

async fn unpack_tgz(archive_path: &Path, destination: &Path) -> Result<()> {
    let archive_path = archive_path.to_path_buf();
    let destination = destination.to_path_buf();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::open(&archive_path).map_err(AppError::Io)?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries().map_err(AppError::Io)? {
            let mut entry = entry.map_err(AppError::Io)?;
            if !entry.unpack_in(&destination).map_err(AppError::Io)? {
                return Err(AppError::Environment(
                    "pnpm 压缩包包含不安全路径".to_string(),
                ));
            }
        }
        Ok(())
    })
    .await
    .map_err(|error| AppError::Generic(error.to_string()))?
}

#[cfg(unix)]
async fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = tokio::fs::metadata(path)
        .await
        .map_err(AppError::Io)?
        .permissions();
    permissions.set_mode(0o755);
    tokio::fs::set_permissions(path, permissions)
        .await
        .map_err(AppError::Io)
}

#[cfg(not(unix))]
async fn set_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn prepend_path(envs: &mut HashMap<String, String>, path: PathBuf) {
    let existing = envs
        .get("PATH")
        .cloned()
        .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
    let mut entries = vec![path];
    entries.extend(std::env::split_paths(&existing));
    if let Ok(joined) = std::env::join_paths(entries) {
        envs.insert("PATH".to_string(), joined.to_string_lossy().to_string());
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
    use super::pnpm_version_is_supported;

    #[test]
    fn requires_the_pinned_pnpm_version_or_newer() {
        assert!(pnpm_version_is_supported("12.9.1"));
        assert!(pnpm_version_is_supported("12.10.0"));
        assert!(!pnpm_version_is_supported("12.0.0"));
        assert!(!pnpm_version_is_supported("11.17.0"));
        assert!(!pnpm_version_is_supported("invalid"));
    }
}
