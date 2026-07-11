use crate::sys::process::ProcessManager;
use crate::task_log::{LogKind, LogSession};
use niupanel_common::auth::sdk_token;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::{debug, error as error_msg, info, warn};
use serde_json::json;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::{OwnedSemaphorePermit, mpsc};
use tokio::task::JoinHandle;
pub mod cache;
pub mod dependency_lock;
pub mod executor;
pub mod factory;
pub mod strategy;
use crate::script::exec::executor::{Executor, ExecutorInstance};
use crate::script::exec::factory::ExecutorFactory;

pub enum ScriptType {
    Python,
    NodeJs,
    Shell,
}

impl ScriptType {
    pub fn try_from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "python" => Ok(ScriptType::Python),
            "nodejs" | "node" => Ok(ScriptType::NodeJs),
            "shell" | "sh" | "bash" | "local" => Ok(ScriptType::Shell),
            _ => Err(AppError::Environment(format!(
                "Unsupported script environment type: {}",
                s
            ))),
        }
    }
}

pub struct ScriptConfig {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub env_type: String,
    pub env_version: Option<String>,
    pub requirements: Option<String>,
    pub env_vars: HashMap<String, String>,
}

pub struct ExecutionContext {
    pub task_id: i32,
    pub task_run_id: Option<i32>,
    pub process_manager: ProcessManager,
    pub permit: OwnedSemaphorePermit,
    pub(crate) log_session: LogSession,
}

pub struct ResourceLimits {
    pub cpu_limit: Option<i32>,
    pub timeout_sec: Option<i32>,
    pub memory: Option<i32>,
}

const DEFAULT_INTERNAL_SDK_TOKEN_TTL_SECS: i64 = 24 * 60 * 60;
const MAX_INTERNAL_SDK_TOKEN_TTL_SECS: i64 = 7 * 24 * 60 * 60;

pub struct ScriptExecutor {
    context: ExecutionContext,
    resources: ResourceLimits,
    mirrors: Option<HashMap<String, String>>,
}

impl ScriptExecutor {
    pub fn new(
        context: ExecutionContext,
        resources: ResourceLimits,
        mirrors: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            context,
            resources,
            mirrors,
        }
    }

    pub async fn execute(self, config: ScriptConfig) -> Result<u32> {
        let mut runtime = ExecutorFactory::create(
            &config.env_type,
            config.env_version.as_deref(),
            self.mirrors.clone(),
        )?;

        // 1. Prepare Environment & Install Dependencies
        self.prepare_environment(&mut runtime, &config).await?;

        // 2. Build Command
        let final_env_vars = self.build_execution_env(&config.env_vars);
        let inner_std_cmd = runtime
            .build_command(&config.path, &config.args, &final_env_vars)
            .await;
        let desc = self.execution_description(runtime.name());

        self.write_log_line(format!("### Starting Execution: {} ###", desc))
            .await;
        self.write_sdk_status_log(&final_env_vars, runtime.name())
            .await;

        let work_dir = Self::working_dir(&config.path);
        let mut tokio_cmd = self.build_tokio_command(&inner_std_cmd, &work_dir);

        // Apply Resource Limits & Process Group (Unix Only)
        crate::sys::sandbox::apply_limits(&mut tokio_cmd, self.resources.memory);

        debug!("启动命令: {}", desc);
        let mut child = tokio_cmd.spawn().map_err(|e| AppError::ProcessStart {
            command: desc.clone(),
            source: e,
        })?;

        let pid = child.id().ok_or(AppError::ProcessHasNoPid)?;
        let cpu_cgroup = self.apply_post_spawn_limits(pid).await;

        // Output Handling
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        let output_drain = spawn_output_handler(stdout, stderr, self.context.log_session.clone());

        self.context
            .process_manager
            .insert(
                self.context.task_id,
                child,
                self.context.permit,
                self.context.task_run_id,
                cpu_cgroup,
                Some(output_drain),
                self.resources.timeout_sec,
                Some(self.context.log_session.clone()),
            )
            .await?;

        Ok(pid)
    }

    fn build_execution_env(&self, env_vars: &HashMap<String, String>) -> HashMap<String, String> {
        let mut final_env_vars = env_vars.clone();
        final_env_vars.insert("TERM".to_string(), "xterm-256color".to_string());
        final_env_vars.insert("FORCE_COLOR".to_string(), "1".to_string());
        final_env_vars.insert("PYTHONUNBUFFERED".to_string(), "1".to_string());
        final_env_vars.insert("NIU_TASK_ID".to_string(), self.context.task_id.to_string());
        final_env_vars.insert(
            "NIUPANEL_TASK_ID".to_string(),
            self.context.task_id.to_string(),
        );
        final_env_vars
            .entry("NIUPANEL_SDK_AUTO_GLOBALS".to_string())
            .or_insert_with(|| "1".to_string());

        if let Some(run_id) = self.context.task_run_id {
            final_env_vars.insert("NIU_TASK_RUN_ID".to_string(), run_id.to_string());
            final_env_vars.insert("NIUPANEL_TASK_RUN_ID".to_string(), run_id.to_string());
        }

        self.inject_internal_sdk_context(&mut final_env_vars);

        final_env_vars
    }

    fn inject_internal_sdk_context(&self, env: &mut HashMap<String, String>) {
        let config = Config::global();
        let ttl_secs = self.internal_sdk_token_ttl_secs();
        let token = sdk_token::issue_internal_sdk_token(
            &config.session_key,
            self.context.task_id,
            self.context.task_run_id,
            ttl_secs,
        );
        let expires_at = current_unix_timestamp().saturating_add(ttl_secs);
        let base_url = Self::local_open_api_base_url(&config.server_addr);
        let context_path =
            Self::sdk_context_path(config, self.context.task_id, self.context.task_run_id);
        let context = json!({
            "version": 1,
            "base_url": base_url,
            "token": token,
            "task_id": self.context.task_id,
            "run_id": self.context.task_run_id,
            "expires_at": expires_at,
        });

        if let Err(e) = write_private_file(&context_path, context.to_string().as_bytes()) {
            warn!(
                "Task {} SDK context injection skipped: {}",
                self.context.task_id, e
            );
            return;
        }

        env.insert(
            "NIUPANEL_SDK_CONTEXT".to_string(),
            context_path.to_string_lossy().to_string(),
        );
    }

    fn internal_sdk_token_ttl_secs(&self) -> i64 {
        self.resources
            .timeout_sec
            .map(|timeout| i64::from(timeout).saturating_add(3600))
            .unwrap_or(DEFAULT_INTERNAL_SDK_TOKEN_TTL_SECS)
            .clamp(3600, MAX_INTERNAL_SDK_TOKEN_TTL_SECS)
    }

    fn sdk_context_path(config: &Config, task_id: i32, run_id: Option<i32>) -> PathBuf {
        let run_part = run_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        let context_path = config
            .runtimes_dir
            .join("sdk-context")
            .join(task_id.to_string())
            .join(format!("run-{run_part}.json"));

        if context_path.is_absolute() {
            context_path
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(&context_path))
                .unwrap_or(context_path)
        }
    }

    fn local_open_api_base_url(server_addr: &str) -> String {
        let port = server_addr
            .parse::<std::net::SocketAddr>()
            .map(|addr| addr.port())
            .ok()
            .or_else(|| {
                server_addr
                    .rsplit_once(':')
                    .and_then(|(_, port)| port.parse::<u16>().ok())
            })
            .unwrap_or(7788);
        format!("http://127.0.0.1:{port}/open/api")
    }

    fn execution_description(&self, runtime_name: &str) -> String {
        format!(
            "{}|Task-{}|Run-{:?}",
            runtime_name, self.context.task_id, self.context.task_run_id
        )
    }

    fn working_dir(script_path: &Path) -> PathBuf {
        if script_path.is_dir() {
            script_path.to_path_buf()
        } else {
            script_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        }
    }

    fn build_tokio_command(&self, inner_std_cmd: &StdCommand, work_dir: &Path) -> TokioCommand {
        let program = Self::resolve_program_path(inner_std_cmd.get_program());
        let mut tokio_cmd = TokioCommand::new(program);
        tokio_cmd.args(inner_std_cmd.get_args());
        tokio_cmd.current_dir(work_dir);

        for (key, value) in inner_std_cmd.get_envs() {
            if let Some(val) = value {
                tokio_cmd.env(key, val);
            } else {
                tokio_cmd.env_remove(key);
            }
        }

        tokio_cmd.env("CI", "true");
        tokio_cmd.env("PYTHONUNBUFFERED", "1");
        tokio_cmd.stdout(Stdio::piped());
        tokio_cmd.stderr(Stdio::piped());
        tokio_cmd.stdin(Stdio::null());
        tokio_cmd
    }

    fn resolve_program_path(program: &std::ffi::OsStr) -> OsString {
        let path = Path::new(program);
        if path.is_absolute() || !Self::program_has_path_component(path) {
            return program.to_os_string();
        }

        std::env::current_dir()
            .map(|cwd| cwd.join(path).into_os_string())
            .unwrap_or_else(|_| program.to_os_string())
    }

    fn program_has_path_component(path: &Path) -> bool {
        let mut normal_components = 0usize;
        for component in path.components() {
            match component {
                Component::Normal(_) => normal_components += 1,
                _ => return true,
            }
        }
        normal_components > 1
    }

    async fn apply_post_spawn_limits(&self, pid: u32) -> Option<PathBuf> {
        match crate::sys::sandbox::apply_post_spawn_limits(
            self.context.task_id,
            self.context.task_run_id,
            pid,
            self.resources.cpu_limit,
        )
        .await
        {
            Ok(path) => path,
            Err(e) => {
                warn!(
                    "Task {} CPU limit setup skipped: {}",
                    self.context.task_id, e
                );
                self.write_log_line(format!(
                    "[WARN] CPU limit not applied, continuing without it: {}",
                    e
                ))
                .await;
                None
            }
        }
    }

    async fn write_log_line(&self, message: String) {
        if let Err(e) = self
            .context
            .log_session
            .write_line(LogKind::Control, message)
            .await
        {
            error_msg!("Failed to write task log: {}", e);
        }
    }

    async fn write_sdk_status_log(&self, env: &HashMap<String, String>, runtime_name: &str) {
        let auth_status = if env.contains_key("NIUPANEL_SDK_CONTEXT") {
            "内部任务身份已启用，无需配置 API Key"
        } else {
            "内部任务身份未启用，将回退到外部 API Key"
        };
        let global_status = if env
            .get("NIUPANEL_SDK_AUTO_GLOBALS")
            .is_some_and(|value| matches!(value.as_str(), "0" | "false" | "False" | "FALSE"))
        {
            "自动全局变量已关闭"
        } else if matches!(runtime_name, "Python" | "Node.js") {
            "脚本内可直接使用 niu"
        } else {
            "Shell 内可通过 Python/Node SDK 路径使用 niu"
        };

        self.write_log_line(format!(
            "[NiuPanel SDK] {}；{}。",
            auth_status, global_status
        ))
        .await;
    }

    async fn prepare_environment(
        &self,
        runtime: &mut ExecutorInstance,
        config: &ScriptConfig,
    ) -> Result<()> {
        let runtime_name = runtime.name().to_string();
        self.write_log_line(format!("Preparing {} environment...", runtime_name))
            .await;

        runtime.prepare().await?;

        let script_dir = config.path.parent().unwrap_or_else(|| Path::new("."));
        let (tx, rx) = mpsc::unbounded_channel::<String>();
        let install_log_prefix = format!("[{} Install]", runtime_name);

        let install_log_task =
            stream_log_task(rx, self.context.log_session.clone(), install_log_prefix);

        let install_result = runtime
            .install_dependencies(config.requirements.as_deref(), script_dir, tx)
            .await;
        install_log_task.await.ok();
        install_result?;

        Ok(())
    }
}

// --- Helper Functions ---

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn write_private_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?
    };

    #[cfg(not(unix))]
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    file.write_all(contents)?;
    file.flush()
}

fn spawn_output_handler<R1, R2>(stdout: R1, stderr: R2, log_session: LogSession) -> JoinHandle<()>
where
    R1: tokio::io::AsyncRead + Unpin + Send + 'static,
    R2: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    // Use smaller buffer for real-time responsiveness, but large enough for efficiency
    const BUF_SIZE: usize = 1024; // 1KB chunks
    const FLUSH_INTERVAL_MS: u64 = 100;

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);
    let (tx, mut rx) = mpsc::channel::<(LogKind, String)>(100);

    let writer_task = tokio::spawn(async move {
        let flush_interval = tokio::time::Duration::from_millis(FLUSH_INTERVAL_MS);

        loop {
            match tokio::time::timeout(flush_interval, rx.recv()).await {
                Ok(Some((kind, chunk))) => {
                    if let Err(e) = log_session.write_raw(kind, chunk).await {
                        error_msg!("写入log失败: {}", e);
                    }
                }
                Ok(None) => break,
                Err(_) => {}
            }
        }
    });

    // Stdout Reader (Byte/Chunk based)
    let tx_out = tx.clone();
    tokio::spawn(async move {
        stream_reader_chunks(&mut stdout_reader, tx_out, LogKind::Stdout, BUF_SIZE).await;
    });

    // Stderr Reader (Byte/Chunk based)
    let tx_err = tx.clone();
    tokio::spawn(async move {
        stream_reader_chunks(&mut stderr_reader, tx_err, LogKind::Stderr, BUF_SIZE).await;
    });

    writer_task
}

async fn stream_reader_chunks<R>(
    reader: &mut BufReader<R>,
    sender: mpsc::Sender<(LogKind, String)>,
    kind: LogKind,
    buf_size: usize,
) where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = vec![0u8; buf_size];
    let mut pending = Vec::new();

    loop {
        match reader.read(&mut buf).await {
            Ok(0) => {
                if !pending.is_empty() {
                    let chunk = String::from_utf8_lossy(&pending).to_string();
                    let _ = sender.send((kind, chunk)).await;
                }
                break;
            }
            Ok(n) => {
                pending.extend_from_slice(&buf[..n]);

                match std::str::from_utf8(&pending) {
                    Ok(valid) => {
                        if sender.send((kind, valid.to_string())).await.is_err() {
                            break;
                        }
                        pending.clear();
                    }
                    Err(err) => {
                        let valid_up_to = err.valid_up_to();
                        if valid_up_to > 0 {
                            let valid =
                                String::from_utf8_lossy(&pending[..valid_up_to]).to_string();
                            if sender.send((kind, valid)).await.is_err() {
                                break;
                            }
                            pending.drain(..valid_up_to);
                        }

                        if err.error_len().is_some() {
                            let invalid = String::from_utf8_lossy(&pending).to_string();
                            if sender.send((kind, invalid)).await.is_err() {
                                break;
                            }
                            pending.clear();
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn keeps_path_lookup_program_unchanged() {
        let resolved = ScriptExecutor::resolve_program_path(OsStr::new("python"));

        assert_eq!(resolved, OsString::from("python"));
    }

    #[test]
    fn resolves_relative_program_with_path_segments_from_process_cwd() {
        let program = Path::new("data")
            .join("runtimes")
            .join("python")
            .join("venv_3.11")
            .join("bin")
            .join("python");
        let resolved = PathBuf::from(ScriptExecutor::resolve_program_path(program.as_os_str()));

        assert_eq!(resolved, std::env::current_dir().unwrap().join(program));
    }
}

fn stream_log_task(
    mut rx: mpsc::UnboundedReceiver<String>,
    log_session: LogSession,
    prefix: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        const FLUSH_INTERVAL_MS: u64 = 100;
        let flush_interval = tokio::time::Duration::from_millis(FLUSH_INTERVAL_MS);

        loop {
            match tokio::time::timeout(flush_interval, rx.recv()).await {
                Ok(Some(line)) => {
                    let msg = if prefix.is_empty() {
                        line.clone()
                    } else {
                        format!("{} {}", prefix, line)
                    };

                    info!("{}", msg);

                    if let Err(e) = log_session.write_line(LogKind::Dependency, msg).await {
                        error_msg!("Failed to write to log file: {}", e);
                    }
                }
                Ok(None) => break,
                Err(_) => {}
            }
        }
    })
}
