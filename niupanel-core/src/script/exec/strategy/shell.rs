use crate::script::exec::executor::Executor;
use crate::script::sdk_env;
use niupanel_common::error::Result;
use std::path::Path;
use std::process::Command;
use tokio::sync::mpsc::UnboundedSender;

pub struct ShellStrategy;

impl ShellStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl Executor for ShellStrategy {
    async fn prepare(&mut self) -> Result<()> {
        Ok(())
    }

    async fn install_dependencies(
        &self,
        _requirements: Option<&str>,
        _script_dir: &Path,
        _sender: UnboundedSender<String>,
    ) -> Result<()> {
        Ok(())
    }

    async fn build_command(
        &self,
        script_path: &Path,
        args: &[String],
        env_vars: &std::collections::HashMap<String, String>,
    ) -> Command {
        let mut cmd = if script_path.extension().map_or(false, |ext| ext == "sh") {
            let mut c = Command::new("bash");
            c.arg(script_path);
            c
        } else {
            // Try to run directly. The user must ensure it has shebang and +x,
            // OR we rely on `tokio::process::Command` failing if not.
            Command::new(script_path)
        };

        cmd.args(args);

        // Update PATH and SDK envs
        let mut final_env = env_vars.clone();

        sdk_env::inject_python_sdk_path(&mut final_env);
        sdk_env::inject_node_sdk_path(&mut final_env);
        sdk_env::inject_node_sdk_preload(&mut final_env);
        sdk_env::inject_node_runtime_path(&mut final_env).await;

        cmd.envs(&final_env);
        cmd
    }

    fn name(&self) -> &str {
        "Shell"
    }
}
