use niupanel_common::debug;
use niupanel_common::error::{AppError, Result};
use std::collections::VecDeque;
use std::process::Output;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::UnboundedSender;

#[allow(async_fn_in_trait)]
pub trait CommandExt {
    async fn execute_checked(&mut self, desc: &str) -> Result<Output>;
    async fn execute_and_get_stdout(&mut self, desc: &str) -> Result<String>;
    async fn execute_with_streaming(
        &mut self,
        input: Option<&str>,
        desc: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()>;
}

impl CommandExt for Command {
    async fn execute_checked(&mut self, desc: &str) -> Result<Output> {
        // debug!("Executing command: {}", desc);

        let output = self.output().await.map_err(|e| AppError::ProcessStart {
            command: desc.to_string(),
            source: e,
        })?;

        if output.status.success() {
            Ok(output)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(AppError::ProcessFailed {
                command: desc.to_string(),
                exit_code: output.status.code(),
                stderr,
            })
        }
    }

    async fn execute_and_get_stdout(&mut self, desc: &str) -> Result<String> {
        let output = self.execute_checked(desc).await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if !stdout.trim().is_empty() {
            debug!("Command '{}' stdout:\n{}", desc, stdout.trim());
        }
        Ok(stdout)
    }

    async fn execute_with_streaming(
        &mut self,
        input: Option<&str>,
        desc: &str,
        sender: UnboundedSender<String>,
    ) -> Result<()> {
        debug!("Executing command with streaming: {}", desc);

        self.kill_on_drop(true);
        self.stdin(Stdio::piped());
        self.stdout(Stdio::piped());
        self.stderr(Stdio::piped());

        let mut child = self.spawn().map_err(|e| AppError::ProcessStart {
            command: desc.to_string(),
            source: e,
        })?;

        // Handle Input
        if let Some(input_str) = input {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input_str.as_bytes()).await.map_err(|e| {
                    AppError::ProcessStart {
                        command: format!("{} (write stdin)", desc),
                        source: e,
                    }
                })?;
            }
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Generic("Failed to capture stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Generic("Failed to capture stderr".to_string()))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let sender_clone = sender.clone();
        let sender_clone2 = sender.clone();

        // Buffer to capture the last 50 lines of stderr for error reporting
        let stderr_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(50)));
        let stderr_buffer_clone = stderr_buffer.clone();

        let stdout_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                debug!("stdout: {}", line);
                let _ = sender_clone.send(format!(" {}", line));
            }
        });

        let stderr_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                debug!("[STDERR] {}", line);
                // Send to log stream
                let _ = sender_clone2.send(format!("{}", line));

                // Save to buffer
                if let Ok(mut buf) = stderr_buffer_clone.lock() {
                    if buf.len() >= 50 {
                        buf.pop_front();
                    }
                    buf.push_back(line);
                }
            }
        });

        // Wait for process to finish
        let status = child.wait().await.map_err(|e| AppError::ProcessStart {
            command: desc.to_string(),
            source: e,
        })?;

        // Ensure we drained pipes
        let _ = tokio::join!(stdout_task, stderr_task);

        if status.success() {
            debug!("Command '{}' completed successfully.", desc);
            Ok(())
        } else {
            // Retrieve captured stderr from buffer
            let captured_stderr = if let Ok(buf) = stderr_buffer.lock() {
                let lines: Vec<String> = buf.iter().cloned().collect();
                lines.join("\n")
            } else {
                "(failed to lock stderr buffer)".to_string()
            };

            let final_stderr = if captured_stderr.is_empty() {
                "(no stderr output)".to_string()
            } else {
                captured_stderr
            };

            let err_msg = format!(
                "Command '{}' failed with exit code: {:?}",
                desc,
                status.code()
            );
            let _ = sender.send(format!(
                "[ERROR] {}\nLast stderr:\n{}",
                err_msg, final_stderr
            ));

            Err(AppError::ProcessFailed {
                command: desc.to_string(),
                exit_code: status.code(),
                stderr: final_stderr,
            })
        }
    }
}

/// Helper function to execute a command (previously with PTY, now standard process with piped output).
pub async fn execute_with_pty(
    program: &str,
    args: &[String],
    current_dir: Option<&std::path::Path>,
    envs: Option<&std::collections::HashMap<String, String>>,
    input: Option<&str>,
    desc: &str,
    sender: UnboundedSender<String>,
) -> Result<()> {
    debug!("Executing command (native): {}", desc);

    let mut cmd = Command::new(program);
    cmd.args(args);

    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }

    if let Some(env_vars) = envs {
        cmd.envs(env_vars);
    } else {
        // Important: if envs is NOT provided, inherit?
        // Existing pty implementation used pty_cmd.env(k, v) only if envs was Some.
        // It did not explicitly clear envs. So Command::new inherits by default.
    }

    // Force non-interactive mode for tools that might detect TTY and change behavior
    cmd.env("CI", "true");
    cmd.env("DEBIAN_FRONTEND", "noninteractive");

    cmd.execute_with_streaming(input, desc, sender).await
}
