use crate::script::sdk_env;
use crate::sys::tools::ToolService;
use niupanel_common::error::{AppError, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct PythonEnvironment {
    pub venv_path: PathBuf,
    pub python_executable: PathBuf,
    pub mirrors: HashMap<String, String>,
}

impl PythonEnvironment {
    pub fn get_uv_bin() -> Result<PathBuf> {
        ToolService::ensure_uv()
    }

    /// 获取 uv 相关的镜像环境变量
    fn get_uv_envs(custom_mirrors: Option<&HashMap<String, String>>) -> HashMap<String, String> {
        let mut envs = HashMap::new();

        if let Some(mirrors) = custom_mirrors {
            if let Some(m) = mirrors.get("UV_PYTHON_INSTALL_MIRROR") {
                if !m.is_empty() {
                    envs.insert("UV_PYTHON_INSTALL_MIRROR".to_string(), m.clone());
                }
            }
            if let Some(m) = mirrors.get("UV_INDEX_URL") {
                if !m.is_empty() {
                    envs.insert("UV_INDEX_URL".to_string(), m.clone());
                }
            }
        }

        // Defaults if not set
        if !envs.contains_key("UV_PYTHON_INSTALL_MIRROR") {
            envs.insert(
                "UV_PYTHON_INSTALL_MIRROR".to_string(),
                "https://gh-proxy.com/https://github.com/astral-sh/python-build-standalone/releases/download/".to_string(),
            );
        }
        if !envs.contains_key("UV_INDEX_URL") {
            envs.insert(
                "UV_INDEX_URL".to_string(),
                "https://pypi.tuna.tsinghua.edu.cn/simple".to_string(),
            );
        }

        envs
    }

    pub async fn new(
        venv_path: PathBuf,
        python_version_request: Option<&str>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<Self> {
        debug!(
            "Ensuring Python virtual environment at: {}",
            venv_path.display()
        );

        let env_vars = Self::get_uv_envs(mirrors.as_ref());

        // Check if venv exists
        if !venv_path.exists() {
            info!("Creating virtual environment...");
            let venv_path_str = venv_path.to_string_lossy().to_string();
            let mut args = vec!["venv", venv_path_str.as_str()];
            if let Some(version) = python_version_request {
                args.push("--python");
                args.push(version);
            }
            ToolService::run_checked(
                &Self::get_uv_bin()?,
                &args,
                None,
                Some(&env_vars),
                "uv venv",
            )
            .await?;
        }

        // Determine python executable path (standard venv structure)
        #[cfg(windows)]
        let python_executable = venv_path.join("Scripts").join("python.exe");
        #[cfg(not(windows))]
        let python_executable = venv_path.join("bin").join("python");

        if !python_executable.exists() {
            return Err(AppError::PythonEnv(format!(
                "Python executable not found at expected path: {}",
                python_executable.display()
            )));
        }

        debug!("Using Python environment at: {}", venv_path.display());

        Ok(Self {
            venv_path,
            python_executable,
            mirrors: env_vars,
        })
    }

    pub async fn create(
        venv_path: PathBuf,
        python_version_request: Option<&str>,
        sender: UnboundedSender<String>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<()> {
        if venv_path.exists() {
            let _ = sender.send("Environment directory already exists.".to_string());
            return Ok(());
        }

        info!("Creating virtual environment...");
        let _ = sender.send("Starting creation of virtual environment...".to_string());

        let mut args = vec!["venv".to_string(), venv_path.to_string_lossy().to_string()];
        if let Some(version) = python_version_request {
            args.push("--python".to_string());
            args.push(version.to_string());
        }

        let envs = Self::get_uv_envs(mirrors.as_ref());

        ToolService::run_streaming(
            &Self::get_uv_bin()?,
            &args,
            None,
            Some(&envs),
            "uv venv",
            sender,
        )
        .await?;

        Ok(())
    }

    pub async fn install_requirements(
        &self,
        requirements_content: &str,
        upgrade: bool,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        if requirements_content.trim().is_empty() {
            return Ok(());
        }

        let packages = Self::parse_requirement_entries(requirements_content);

        if packages.is_empty() {
            return Ok(());
        }

        // 快速过滤：只把尚未安装的包传给 uv
        let can_fast_filter = packages.iter().all(|pkg| Self::is_plain_package_name(pkg));
        let missing_packages: Vec<String> = if !upgrade && can_fast_filter {
            self.filter_missing_packages(&packages, &sender).await
        } else {
            if !can_fast_filter {
                let _ = sender.send(
                    "[Python] Requirement contains constraints or options, using package manager resolution."
                        .to_string(),
                );
            }
            packages
        };

        if missing_packages.is_empty() {
            return Ok(());
        }

        self.install_packages(&missing_packages, upgrade, sender)
            .await
    }

    pub async fn sync_requirements(
        &self,
        requirements_content: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        if requirements_content.trim().is_empty() {
            return Ok(());
        }

        let packages = Self::parse_requirement_entries(requirements_content);

        if packages.is_empty() {
            return Ok(());
        }

        self.install_packages(&packages, false, sender).await
    }

    fn parse_requirement_entries(requirements_content: &str) -> Vec<String> {
        requirements_content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(ToString::to_string)
            .collect()
    }

    fn is_plain_package_name(requirement: &str) -> bool {
        let trimmed = requirement.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('-')
            || trimmed.contains("://")
            || trimmed.starts_with('.')
            || trimmed.starts_with('/')
        {
            return false;
        }

        trimmed
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric())
            && trimmed
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    }

    /// 通过文件系统检查，过滤出尚未安装的包
    /// 检查 venv/lib/python*/site-packages/<pkg_name> 是否存在
    async fn filter_missing_packages(
        &self,
        packages: &[String],
        sender: &UnboundedSender<String>,
    ) -> Vec<String> {
        let lib_dir = self.venv_path.join("lib");

        // 同步扫描找到 site-packages（lib/ 目录只有一个 python* 子目录，极快）
        let site_packages_dir = std::fs::read_dir(&lib_dir).ok().and_then(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("python"))
                .map(|e| e.path().join("site-packages"))
                .find(|p| p.exists())
        });

        let Some(site_packages) = site_packages_dir else {
            // 找不到 site-packages，直接全量安装
            return packages.to_vec();
        };

        let mut missing = Vec::new();
        for pkg in packages {
            // 提取纯包名：去掉版本限制（requests>=2.0, flask==2.1.0, etc.）
            let pkg_base = pkg
                .split(|c| c == '>' || c == '<' || c == '=' || c == '!' || c == '[')
                .next()
                .unwrap_or(pkg)
                .trim();
            // 将连字符和下划线标准化（Python 包名大小写/连字符不敏感）
            let normalized = pkg_base.replace('-', "_").to_lowercase();

            let pkg_dir = site_packages.join(&normalized);
            // 也检查 dist-info 形式（包目录本身可能不存在，但 .dist-info 会存在）
            let dist_info_pattern = format!("{}-", normalized);
            let installed_via_dist = std::fs::read_dir(&site_packages)
                .ok()
                .map(|entries| {
                    entries.filter_map(|e| e.ok()).any(|e| {
                        let name = e.file_name().to_string_lossy().to_lowercase();
                        name.starts_with(&dist_info_pattern) && name.ends_with(".dist-info")
                    })
                })
                .unwrap_or(false);

            if pkg_dir.exists() || installed_via_dist {
                debug!(
                    "Python Package '{}' already installed (found in site-packages), skipping.",
                    pkg
                );
                let _ = sender.send(format!(
                    "[Python] Package '{}' already installed, skipping.",
                    pkg
                ));
            } else {
                missing.push(pkg.clone());
            }
        }
        missing
    }

    pub async fn install_packages(
        &self,
        packages: &[String],
        upgrade: bool,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let action = if upgrade { "Upgrading" } else { "Installing" };
        let pkg_str = packages.join(" ");
        debug!("{} packages: {} ...", action, pkg_str);
        let _ = sender.send(format!("{} packages: {} ...", action, pkg_str));

        let mut args = vec!["pip".to_string(), "install".to_string()];

        if upgrade {
            args.push("--upgrade".to_string());
        }

        args.extend_from_slice(packages);

        let mut envs = self.mirrors.clone();
        envs.insert("PYTHONUNBUFFERED".to_string(), "1".to_string());
        envs.insert(
            "VIRTUAL_ENV".to_string(),
            self.venv_path.to_string_lossy().to_string(),
        );
        envs.insert("NO_COLOR".to_string(), "1".to_string());
        envs.insert("TERM".to_string(), "dumb".to_string());

        let desc = format!("uv pip install {}", pkg_str);

        ToolService::run_streaming(
            &Self::get_uv_bin()?,
            &args,
            None,
            Some(&envs),
            &desc,
            sender,
        )
        .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn uninstall_package(
        &self,
        package_name: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        info!("Uninstalling package {}...", package_name);
        let _ = sender.send(format!("Uninstalling package {}...", package_name));

        let args = vec![
            "pip".to_string(),
            "uninstall".to_string(),
            package_name.to_string(),
        ];

        let mut envs = self.mirrors.clone();
        envs.insert(
            "VIRTUAL_ENV".to_string(),
            self.venv_path.to_string_lossy().to_string(),
        );
        envs.insert("NO_COLOR".to_string(), "1".to_string());
        envs.insert("TERM".to_string(), "dumb".to_string());

        ToolService::run_streaming(
            &Self::get_uv_bin()?,
            &args,
            None,
            Some(&envs),
            "uv pip uninstall",
            sender,
        )
        .await?;

        Ok(())
    }

    pub async fn list_packages(&self) -> Result<String> {
        let mut envs = self.mirrors.clone();
        envs.insert(
            "VIRTUAL_ENV".to_string(),
            self.venv_path.to_string_lossy().to_string(),
        );
        ToolService::run_stdout(
            &Self::get_uv_bin()?,
            &["pip", "list", "--format", "json"],
            None,
            Some(&envs),
            "uv pip list --format json",
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn list_available_python_versions(
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<String> {
        let envs = Self::get_uv_envs(mirrors.as_ref());
        ToolService::run_stdout(
            &Self::get_uv_bin()?,
            &["python", "list"],
            None,
            Some(&envs),
            "uv python list",
        )
        .await
    }

    pub async fn command(&self, script_path: &std::path::Path) -> Result<std::process::Command> {
        let mut cmd = std::process::Command::new(&self.python_executable);
        cmd.arg(script_path);

        let mut sdk_envs = HashMap::new();
        sdk_env::inject_python_sdk_path(&mut sdk_envs);
        cmd.envs(&sdk_envs);

        Ok(cmd)
    }
}
