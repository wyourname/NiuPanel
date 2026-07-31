use crate::sys::command::{CommandExt, execute_with_pty};
use niupanel_common::error::{AppError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

pub struct ToolService;

impl ToolService {
    pub fn ensure_uv() -> Result<PathBuf> {
        niupanel_common::tools::require_tool_path("uv", "Python 环境管理")
    }

    pub fn ensure_git() -> Result<PathBuf> {
        niupanel_common::tools::require_tool_path("git", "Git 同步")
    }

    pub async fn run_checked(
        tool_path: &Path,
        args: &[&str],
        current_dir: Option<&Path>,
        envs: Option<&HashMap<String, String>>,
        desc: &str,
    ) -> Result<Output> {
        let mut cmd = Command::new(tool_path);
        cmd.args(args);
        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }
        if let Some(envs) = envs {
            cmd.envs(envs);
        }
        cmd.execute_checked(desc).await
    }

    pub async fn run_stdout(
        tool_path: &Path,
        args: &[&str],
        current_dir: Option<&Path>,
        envs: Option<&HashMap<String, String>>,
        desc: &str,
    ) -> Result<String> {
        let mut cmd = Command::new(tool_path);
        cmd.args(args);
        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }
        if let Some(envs) = envs {
            cmd.envs(envs);
        }
        cmd.execute_and_get_stdout(desc).await
    }

    pub async fn capture_output(
        tool_path: &Path,
        args: &[&str],
        current_dir: Option<&Path>,
        envs: Option<&HashMap<String, String>>,
    ) -> Result<Output> {
        let mut cmd = Command::new(tool_path);
        cmd.args(args);
        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }
        if let Some(envs) = envs {
            cmd.envs(envs);
        }
        cmd.output().await.map_err(AppError::Io)
    }

    pub async fn run_status(
        tool_path: &Path,
        args: &[&str],
        current_dir: Option<&Path>,
        envs: Option<&HashMap<String, String>>,
        quiet: bool,
    ) -> Result<std::process::ExitStatus> {
        let mut cmd = Command::new(tool_path);
        cmd.args(args);
        if let Some(dir) = current_dir {
            cmd.current_dir(dir);
        }
        if let Some(envs) = envs {
            cmd.envs(envs);
        }
        if quiet {
            cmd.stdout(Stdio::null());
            cmd.stderr(Stdio::null());
        }
        cmd.status().await.map_err(AppError::Io)
    }

    pub async fn run_streaming(
        tool_path: &Path,
        args: &[String],
        current_dir: Option<&Path>,
        envs: Option<&HashMap<String, String>>,
        desc: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        execute_with_pty(
            &tool_path.to_string_lossy(),
            args,
            current_dir,
            envs,
            None,
            desc,
            sender,
        )
        .await
    }
}
