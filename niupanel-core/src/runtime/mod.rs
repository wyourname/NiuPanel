use crate::script::interpreter::node::NodeEnvironment;
use crate::script::interpreter::python::PythonEnvironment;
use crate::script::sdk_env;
use crate::sys::tools::ToolService;
use niupanel_common::error::{AppError, Result};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

pub struct RuntimeManager;

impl RuntimeManager {
    pub async fn list_local_node_versions() -> Result<Vec<(String, bool)>> {
        NodeEnvironment::list_local_node_versions().await
    }

    pub async fn list_available_node_versions() -> Result<String> {
        NodeEnvironment::list_available_node_versions().await
    }

    pub async fn list_available_python_versions(
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<String> {
        PythonEnvironment::list_available_python_versions(mirrors).await
    }

    pub fn open_node_environment(
        version: Option<&str>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<NodeEnvironment> {
        NodeEnvironment::open(
            PathBuf::new(),
            version.unwrap_or_default().to_string(),
            mirrors,
        )
    }

    pub async fn create_node_environment(
        version: String,
        sender: UnboundedSender<String>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<()> {
        NodeEnvironment::create(PathBuf::new(), version, sender, mirrors).await
    }

    pub async fn uninstall_node_version(version: &str) -> Result<()> {
        NodeEnvironment::uninstall_node_version(version).await
    }

    pub async fn set_node_default(version: &str) -> Result<()> {
        NodeEnvironment::ensure_fnm_installed().await?;
        let envs = ToolService::build_fnm_env(None)?;
        let status = ToolService::run_status(
            &NodeEnvironment::get_fnm_bin(),
            &["default", version],
            None,
            Some(&envs),
            false,
        )
        .await?;

        if !status.success() {
            return Err(AppError::Generic(format!(
                "Failed to set Node.js {} as default",
                version
            )));
        }
        Ok(())
    }

    pub async fn open_python_environment(
        venv_path: PathBuf,
        python_version_request: Option<&str>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<PythonEnvironment> {
        PythonEnvironment::new(venv_path, python_version_request, mirrors).await
    }

    pub async fn create_python_environment(
        venv_path: PathBuf,
        python_version_request: Option<&str>,
        sender: UnboundedSender<String>,
        mirrors: Option<HashMap<String, String>>,
    ) -> Result<()> {
        PythonEnvironment::create(venv_path, python_version_request, sender, mirrors).await
    }

    pub async fn install_python_requirements(
        env: &PythonEnvironment,
        requirements: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        env.install_requirements(requirements, false, sender).await
    }

    pub async fn sync_python_requirements(
        env: &PythonEnvironment,
        requirements: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        env.sync_requirements(requirements, sender).await
    }

    pub async fn install_node_packages(
        env: &NodeEnvironment,
        packages: &[String],
        fnm_using_arg: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        env.install_packages_with_version(packages, fnm_using_arg, sender)
            .await
    }

    pub async fn build_node_command(
        env: &NodeEnvironment,
        script_path: &Path,
        fnm_using_arg: &str,
        args: &[String],
        env_vars: &HashMap<String, String>,
    ) -> Result<std::process::Command> {
        let mut cmd = env.command_with_version(script_path, fnm_using_arg).await?;
        cmd.args(args);

        if let Some(parent) = script_path.parent() {
            cmd.current_dir(parent);
        }

        let mut final_env = env.runtime_envs()?;
        final_env.extend(env_vars.clone());
        sdk_env::inject_python_sdk_path(&mut final_env);
        sdk_env::inject_node_sdk_path(&mut final_env);
        sdk_env::inject_node_sdk_preload(&mut final_env);
        if let Some(version) = fnm_using_arg
            .strip_prefix("--using=")
            .and_then(NodeEnvironment::normalize_version)
            .or_else(|| env.version.clone())
        {
            NodeEnvironment::inject_shared_dependency_env(&mut final_env, &version);
        }
        cmd.envs(&final_env);
        Ok(cmd)
    }
}
