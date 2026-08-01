use crate::runtime::RuntimeManager;
use crate::script::exec::cache::DependencyCache;
use crate::script::exec::dependency_lock::DependencyInstallLock;
use crate::script::exec::executor::Executor;
use crate::script::interpreter::python::PythonEnvironment;
use crate::script::sdk_env;
use crate::sys::tools::ToolService;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

pub struct PythonStrategy {
    env: Option<PythonEnvironment>,
    venv_path: PathBuf,
    python_version: Option<String>,
    mirrors: Option<HashMap<String, String>>,
}

impl PythonStrategy {
    pub fn new(python_version: Option<&str>, mirrors: Option<HashMap<String, String>>) -> Self {
        let venv_base_dir = Config::global().runtimes_dir.join("python");

        let (venv_name, pure_version) = if let Some(ver) = python_version {
            if ver.is_empty() || ver == "venv_default" || ver == "default" {
                (String::new(), None)
            } else if ver.starts_with("venv_") {
                (
                    ver.to_string(),
                    Some(ver.trim_start_matches("venv_").to_string()),
                )
            } else {
                (format!("venv_{}", ver), Some(ver.to_string()))
            }
        } else {
            (String::new(), None)
        };

        let venv_path = if venv_name.is_empty() {
            PathBuf::new()
        } else {
            venv_base_dir.join(venv_name)
        };

        Self {
            env: None,
            venv_path,
            python_version: pure_version,
            mirrors,
        }
    }
}

impl Executor for PythonStrategy {
    async fn prepare(&mut self) -> Result<()> {
        ToolService::ensure_uv()?;

        if self
            .python_version
            .as_deref()
            .unwrap_or_default()
            .is_empty()
            || self.venv_path.as_os_str().is_empty()
        {
            return Err(AppError::ValidationError(
                "Python version must be specified. Set a task version or system default Python version."
                    .to_string(),
            ));
        }

        if let Some(parent) = self.venv_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(AppError::Io)?;
        }

        // Initialize and store the environment
        let env = RuntimeManager::open_python_environment(
            self.venv_path.clone(),
            self.python_version.as_deref(),
            self.mirrors.clone(),
        )
        .await?;
        self.env = Some(env);
        Ok(())
    }

    async fn install_dependencies(
        &self,
        requirements: Option<&str>,
        _script_dir: &Path,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        let env = self
            .env
            .as_ref()
            .ok_or_else(|| AppError::Generic("Environment not initialized".to_string()))?;

        let Some(content) = requirements.filter(|content| !content.trim().is_empty()) else {
            return Ok(());
        };

        let fingerprint = DependencyCache::python_fingerprint(
            &env.venv_path,
            self.python_version.as_deref(),
            content,
            self.mirrors.as_ref(),
        )?;
        let fingerprint_path =
            DependencyCache::python_fingerprint_marker_path(&env.venv_path, &fingerprint);

        if DependencyCache::is_match(&fingerprint_path, &fingerprint).await {
            let _ = sender
                .send("[Python] Dependency declaration unchanged, skipping sync.".to_string());
            return Ok(());
        }

        let _install_lock =
            DependencyInstallLock::acquire(format!("python:{}", env.venv_path.display())).await;

        if DependencyCache::is_match(&fingerprint_path, &fingerprint).await {
            let _ = sender.send(
                "[Python] Dependency declaration already synced by another task.".to_string(),
            );
            return Ok(());
        }

        let _ = sender.send("[Python] Dependency declaration changed, syncing...".to_string());
        RuntimeManager::sync_python_requirements(env, content, sender.clone()).await?;
        DependencyCache::write(&fingerprint_path, &fingerprint).await?;

        Ok(())
    }

    async fn build_command(
        &self,
        script_path: &Path,
        args: &[String],
        env_vars: &std::collections::HashMap<String, String>,
    ) -> std::process::Command {
        let env = self
            .env
            .as_ref()
            .expect("Python environment not initialized. Call prepare() first.");

        let mut c = std::process::Command::new(&env.python_executable);
        c.arg(script_path);
        c.args(args);

        // Inject VIRTUAL_ENV and update PATH
        let mut final_env = env_vars.clone();

        // 1. Set VIRTUAL_ENV (Standard Python practice)
        final_env.insert(
            "VIRTUAL_ENV".to_string(),
            env.venv_path.to_string_lossy().to_string(),
        );

        // 2. Inject SDK import paths
        sdk_env::inject_python_sdk_path(&mut final_env);
        sdk_env::inject_node_sdk_path(&mut final_env);
        sdk_env::inject_node_sdk_preload(&mut final_env);

        // 4. Update PATH
        #[cfg(not(windows))]
        let bin_dir = env.venv_path.join("bin");
        #[cfg(windows)]
        let bin_dir = env.venv_path.join("Scripts");

        let mut paths = if let Some(path_var) = final_env
            .get("PATH")
            .cloned()
            .map(std::ffi::OsString::from)
            .or_else(|| std::env::var_os("PATH"))
        {
            std::env::split_paths(&path_var).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Prepend Python venv bin to PATH (highest priority)
        if !paths.contains(&bin_dir) {
            paths.insert(0, bin_dir.clone());
        }

        if let Ok(new_path) = std::env::join_paths(paths) {
            final_env.insert("PATH".to_string(), new_path.to_string_lossy().to_string());
        }

        sdk_env::inject_default_node_runtime_environment(&mut final_env).await;

        c.envs(&final_env);
        c
    }

    fn name(&self) -> &str {
        "Python"
    }
}
