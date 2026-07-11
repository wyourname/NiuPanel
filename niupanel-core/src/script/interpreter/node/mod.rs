use crate::sys::tools::ToolService;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::{debug, info};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

// removed complex caching since we use pure native filesystem routing now

pub struct NodeEnvironment {
    pub mirrors: HashMap<String, String>,
    pub version: Option<String>,
}

impl NodeEnvironment {
    /// 获取 fnm 二进制文件的完整路径
    pub fn get_fnm_bin() -> PathBuf {
        ToolService::ensure_fnm().unwrap_or_else(|_| PathBuf::from("fnm"))
    }

    /// 确保系统上存在 fnm
    pub async fn ensure_fnm_installed() -> Result<()> {
        let fnm_bin = ToolService::ensure_fnm()?;
        info!("Using fnm from {}", fnm_bin.display());
        Ok(())
    }

    /// 打开一个已有的 Node 环境（只读操作，不执行 fnm install）
    /// 适用于：列出包、查询状态等不需要修改环境的场景
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

    pub async fn new(
        _env_dir: PathBuf,
        version: String,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        Self::ensure_fnm_installed().await?;

        let mirrors = mirrors.unwrap_or_default();
        let version = Self::normalize_version(&version).ok_or_else(|| {
            AppError::ValidationError("Node version must be specified".to_string())
        })?;

        // 1. Install Node version if not already installed via fnm
        //    First check if already installed to avoid redundant downloads
        let using_arg = format!("--using={}", version);
        let fnm_env = ToolService::build_fnm_env(Some(&mirrors))?;
        let version_check = ToolService::run_status(
            &Self::get_fnm_bin(),
            &["exec", &using_arg, "node", "--version"],
            None,
            Some(&fnm_env),
            true,
        )
        .await;

        if !matches!(version_check, Ok(s) if s.success()) {
            debug!("Node {} not installed, downloading via fnm...", version);
            ToolService::run_checked(
                &Self::get_fnm_bin(),
                &["install", &version],
                None,
                Some(&fnm_env),
                "fnm install",
            )
            .await?;
        } else {
            debug!(
                "Node {} already installed via fnm, skipping download.",
                version
            );
        }

        // Do not create env_dir sandboxes anymore.

        Ok(Self {
            mirrors,
            version: Some(version),
        })
    }

    pub async fn create(
        _env_dir: PathBuf,
        version: String,
        sender: UnboundedSender<String>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<()> {
        info!("Downloading Node runtime version...");
        let _ = sender.send(format!("Downloading Node {}...", version));

        let _env = Self::new(_env_dir.clone(), version.clone(), mirrors).await?;

        let _ = sender.send("Node version installed successfully.".to_string());

        Ok(())
    }

    pub fn normalize_version(version: &str) -> Option<String> {
        let trimmed = version.trim();
        if trimmed.is_empty() || trimmed == "system" || trimmed == "default" {
            return None;
        }

        trimmed
            .strip_prefix("venv_")
            .unwrap_or(trimmed)
            .split_whitespace()
            .next()
            .map(|value| value.trim_start_matches('v').to_string())
            .filter(|value| !value.is_empty())
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

    pub fn package_install_path(pkg: &str) -> PathBuf {
        let normalized = pkg.trim();

        if let Some(scoped) = normalized.strip_prefix('@') {
            if let Some((scope, rest)) = scoped.split_once('/') {
                let package_name = rest.rsplit_once('@').map(|(name, _)| name).unwrap_or(rest);
                return PathBuf::from(format!("@{}", scope)).join(package_name);
            }
        }

        let package_name = normalized
            .split_once('@')
            .map(|(name, _)| name)
            .unwrap_or(normalized);
        PathBuf::from(package_name)
    }

    /// 解析 fnm list 获取所有已安装版本及 active 信息
    pub async fn list_local_node_versions() -> Result<Vec<(String, bool)>> {
        Self::ensure_fnm_installed().await?;
        let envs = ToolService::build_fnm_env(None)?;
        let output =
            ToolService::capture_output(&Self::get_fnm_bin(), &["list"], None, Some(&envs)).await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions = Vec::new();
        // fnm list 输出示例:
        //   * v20.11.0
        //   * v20.11.1 default   ← 仅此行包含 "default" 表示全局活跃版本
        //   * v22.1.0
        //   * system
        // 注：'*' 表示"已安装"，不是"活跃"，所有已安装版本都有 *
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.contains("system") {
                continue;
            }
            // 只有包含 "default" 关键字的行才是当前全局活跃版本
            let is_default = line.contains("default");
            let parts: Vec<&str> = line.split_whitespace().collect();
            // 找到以 'v' 开头的版本号（跳过 '*' 前缀）
            if let Some(version_str) = parts.iter().find(|&&p| p.starts_with('v')) {
                let v = version_str
                    .strip_prefix('v')
                    .unwrap_or(version_str)
                    .to_string();
                versions.push((v, is_default));
            }
        }
        Ok(versions)
    }

    pub async fn uninstall_node_version(version: &str) -> Result<()> {
        Self::ensure_fnm_installed().await?;
        let envs = ToolService::build_fnm_env(None)?;
        let status = ToolService::run_status(
            &Self::get_fnm_bin(),
            &["uninstall", version],
            None,
            Some(&envs),
            false,
        )
        .await?;
        if !status.success() {
            return Err(AppError::Generic(format!(
                "Failed to uninstall Node.js version {}",
                version
            )));
        }
        Ok(())
    }

    /// 执行在特定环境下的 fnm + node 命令
    pub fn command_fnm_exec(&self) -> std::process::Command {
        let mut cmd = std::process::Command::new(Self::get_fnm_bin());
        if let Ok(envs) = ToolService::build_fnm_env(None) {
            cmd.envs(envs);
        }
        cmd.arg("exec").arg(self.fnm_using_arg());
        cmd
    }

    /// 使用当前环境版本构建 node 执行命令（保留兼容接口）
    pub async fn command(&self, script_path: &Path) -> Result<std::process::Command> {
        self.command_with_version(script_path, &self.fnm_using_arg())
            .await
    }

    /// 使用指定版本构建 node 执行命令
    /// `fnm_using_arg` 例如: "--using=20.11.1" 或 "--using=default"
    pub async fn command_with_version(
        &self,
        script_path: &Path,
        fnm_using_arg: &str,
    ) -> Result<std::process::Command> {
        let mut envs = self.runtime_envs()?;
        let version = Self::version_from_using_arg(fnm_using_arg).or_else(|| self.version.clone());
        if let Some(version) = version.as_deref() {
            Self::inject_shared_dependency_env(&mut envs, version);
        }

        let mut cmd = ToolService::build_fnm_exec_command(fnm_using_arg, script_path, Some(&envs))?;

        // Set default memory limit if not already set in NODE_OPTIONS
        let existing_node_options = std::env::var("NODE_OPTIONS").unwrap_or_default();
        if !existing_node_options.contains("--max-old-space-size") {
            let mut options = existing_node_options;
            if !options.is_empty() {
                options.push(' ');
            }
            // Default to 2GB if not specified, which is usually enough for most scripts
            // but prevents the default ~512MB limit in some environments
            options.push_str("--max-old-space-size=2048");
            cmd.env("NODE_OPTIONS", options);
        }

        Ok(cmd)
    }

    /// 使用当前环境版本安装包（保留兼容接口）
    pub async fn install_packages(
        &self,
        packages: &[String],
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        self.install_packages_with_version(packages, &self.fnm_using_arg(), sender)
            .await
    }

    /// 使用指定版本安装包
    pub async fn install_packages_with_version(
        &self,
        packages: &[String],
        fnm_using_arg: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let package_list = packages.join(" ");
        info!(
            "Installing packages '{}' ({})...",
            package_list, fnm_using_arg
        );
        let _ = sender.send(format!("Installing packages '{}'...", package_list));

        let version = Self::version_from_using_arg(fnm_using_arg)
            .or_else(|| self.version.clone())
            .ok_or_else(|| {
                AppError::ValidationError(
                    "Node version must be specified for package install".to_string(),
                )
            })?;
        let shared_root = Self::shared_root_for_version(&version);
        tokio::fs::create_dir_all(&shared_root)
            .await
            .map_err(AppError::Io)?;

        ensure_shared_package_json(&shared_root).await?;

        let desc = format!("npm install {} ({})", package_list, version);

        let args = vec![
            "exec".to_string(),
            fnm_using_arg.to_string(),
            "npm".to_string(),
            "install".to_string(),
            "--prefix".to_string(),
            shared_root.to_string_lossy().to_string(),
        ];

        let mut final_args = args;
        final_args.extend_from_slice(packages);

        let mut env_override = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut env_override, &version);

        ToolService::run_streaming(
            &Self::get_fnm_bin(),
            &final_args,
            Some(&shared_root),
            Some(&env_override),
            &desc,
            sender,
        )
        .await?;

        Ok(())
    }

    pub async fn uninstall_package(
        &self,
        package_name: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        info!(
            "Uninstalling package {} from Node shared environment...",
            package_name
        );
        let _ = sender.send(format!("Uninstalling package {}...", package_name));

        let version = self.version.clone().ok_or_else(|| {
            AppError::ValidationError(
                "Node version must be specified for package uninstall".to_string(),
            )
        })?;
        let shared_root = Self::shared_root_for_version(&version);
        tokio::fs::create_dir_all(&shared_root)
            .await
            .map_err(AppError::Io)?;
        ensure_shared_package_json(&shared_root).await?;

        let args = vec![
            "exec".to_string(),
            self.fnm_using_arg(),
            "npm".to_string(),
            "uninstall".to_string(),
            "--prefix".to_string(),
            shared_root.to_string_lossy().to_string(),
            package_name.to_string(),
        ];

        let mut env_override = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut env_override, &version);

        ToolService::run_streaming(
            &Self::get_fnm_bin(),
            &args,
            Some(&shared_root),
            Some(&env_override),
            "npm uninstall",
            sender,
        )
        .await?;

        Ok(())
    }

    pub async fn list_packages(&self) -> Result<String> {
        let version = self.version.clone().ok_or_else(|| {
            AppError::ValidationError(
                "Node version must be specified for package listing".to_string(),
            )
        })?;
        let shared_root = Self::shared_root_for_version(&version);
        tokio::fs::create_dir_all(&shared_root)
            .await
            .map_err(AppError::Io)?;
        ensure_shared_package_json(&shared_root).await?;
        let mut envs = self.runtime_envs()?;
        Self::inject_shared_dependency_env(&mut envs, &version);

        // npm list may exit with non-zero when there are peer dep issues,
        // but still outputs valid JSON. Capture stdout even on failure.
        let output = ToolService::capture_output(
            &Self::get_fnm_bin(),
            &[
                "exec",
                &self.fnm_using_arg(),
                "npm",
                "list",
                "--prefix",
                &shared_root.to_string_lossy(),
                "--depth=0",
                "--json",
            ],
            Some(&shared_root),
            Some(&envs),
        )
        .await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if stdout.trim().is_empty() {
            Ok(serde_json::json!({ "dependencies": {} }).to_string())
        } else {
            Ok(stdout)
        }
    }

    pub async fn list_available_node_versions() -> Result<String> {
        Self::ensure_fnm_installed().await?;
        let envs = ToolService::build_fnm_env(None)?;
        ToolService::run_stdout(
            &Self::get_fnm_bin(),
            &["ls-remote"],
            None,
            Some(&envs),
            "fnm ls-remote",
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn version(&self) -> Result<String> {
        let envs = ToolService::build_fnm_env(None)?;
        let using_arg = self.fnm_using_arg();
        ToolService::run_stdout(
            &Self::get_fnm_bin(),
            &["exec", &using_arg, "node", "--version"],
            None,
            Some(&envs),
            "node --version",
        )
        .await
    }

    /// 获取当前默认 Node.js 的 bin 目录路径
    /// 优化方案：优先从 fnm 的目录结构推断，避免进程调用开销
    pub async fn get_default_node_bin_dir() -> Option<PathBuf> {
        // 1. 尝试从 FNM_DIR 推断 (Docker 或手动设置)
        let fnm_dir = if let Ok(dir) = std::env::var("FNM_DIR") {
            Some(PathBuf::from(dir))
        } else {
            home::home_dir().map(|h| h.join(".local/share/fnm"))
        };

        if let Some(base) = fnm_dir {
            // fnm 通常维护 aliases/default 软链接
            let default_bin = base.join("aliases/default/bin");
            if default_bin.exists() {
                return Some(default_bin);
            }
        }

        // 2. Fallback: 如果推断失败，再尝试运行一次命令（仅作为保底）
        let envs = ToolService::build_fnm_env(None).ok()?;
        let output = ToolService::capture_output(
            &Self::get_fnm_bin(),
            &[
                "exec",
                "--using=default",
                "node",
                "-e",
                "console.log(require('path').dirname(process.execPath))",
            ],
            None,
            Some(&envs),
        )
        .await
        .ok()?;

        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let path = PathBuf::from(path_str);
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    pub fn fnm_using_arg(&self) -> String {
        self.version
            .as_deref()
            .map(|version| format!("--using={version}"))
            .unwrap_or_else(|| "--using=default".to_string())
    }

    pub fn runtime_envs(&self) -> Result<HashMap<String, String>> {
        let mut envs = self.mirrors.clone();
        envs.insert("NO_COLOR".to_string(), "1".to_string());
        ToolService::build_fnm_env(Some(&envs))
    }

    pub fn inject_shared_dependency_env(env: &mut HashMap<String, String>, version: &str) {
        let node_modules = Self::shared_node_modules_for_version(version);
        let bin_dir = Self::shared_bin_for_version(version);
        prepend_path_like(env, "NODE_PATH", node_modules);
        prepend_path_like(env, "PATH", bin_dir);
    }

    fn version_from_using_arg(fnm_using_arg: &str) -> Option<String> {
        fnm_using_arg
            .strip_prefix("--using=")
            .and_then(Self::normalize_version)
    }
}

async fn ensure_shared_package_json(shared_root: &Path) -> Result<()> {
    let package_json = shared_root.join("package.json");
    if package_json.exists() {
        return Ok(());
    }

    let content = serde_json::json!({
        "private": true,
        "name": "niupanel-node-shared-environment"
    })
    .to_string();
    tokio::fs::write(package_json, content)
        .await
        .map_err(AppError::Io)
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
