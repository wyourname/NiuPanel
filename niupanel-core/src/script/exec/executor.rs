use crate::script::exec::strategy::node::NodeStrategy;
use crate::script::exec::strategy::python::PythonStrategy;
use crate::script::exec::strategy::shell::ShellStrategy;
use niupanel_common::error::Result;
use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

#[allow(async_fn_in_trait)]
pub trait Executor: Send + Sync {
    fn prepare(&mut self) -> impl std::future::Future<Output = Result<()>> + Send;
    async fn install_dependencies(
        &self,
        requirements: Option<&str>,
        script_dir: &Path,
        sender: UnboundedSender<String>,
    ) -> Result<()>;
    async fn build_command(
        &self,
        script_path: &Path,
        args: &[String],
        env_vars: &std::collections::HashMap<String, String>,
    ) -> std::process::Command;
    fn name(&self) -> &str;
}

pub enum ExecutorInstance {
    Python(PythonStrategy),
    Node(NodeStrategy),
    Shell(ShellStrategy),
}

impl Executor for ExecutorInstance {
    async fn prepare(&mut self) -> Result<()> {
        match self {
            Self::Python(s) => s.prepare().await,
            Self::Node(s) => s.prepare().await,
            Self::Shell(s) => s.prepare().await,
        }
    }

    async fn install_dependencies(
        &self,
        requirements: Option<&str>,
        script_dir: &Path,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        match self {
            Self::Python(s) => {
                s.install_dependencies(requirements, script_dir, sender)
                    .await
            }
            Self::Node(s) => {
                s.install_dependencies(requirements, script_dir, sender)
                    .await
            }
            Self::Shell(s) => {
                s.install_dependencies(requirements, script_dir, sender)
                    .await
            }
        }
    }

    async fn build_command(
        &self,
        script_path: &Path,
        args: &[String],
        env_vars: &std::collections::HashMap<String, String>,
    ) -> std::process::Command {
        match self {
            Self::Python(s) => s.build_command(script_path, args, env_vars).await,
            Self::Node(s) => s.build_command(script_path, args, env_vars).await,
            Self::Shell(s) => s.build_command(script_path, args, env_vars).await,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Python(s) => s.name(),
            Self::Node(s) => s.name(),
            Self::Shell(s) => s.name(),
        }
    }
}
