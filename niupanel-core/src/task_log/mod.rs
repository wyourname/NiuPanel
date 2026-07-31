use niupanel_common::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogKind {
    Stdout,
    Stderr,
    Control,
    Dependency,
    System,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogEvent {
    pub task_id: Option<i32>,
    pub run_id: Option<i32>,
    pub job_id: Option<i32>,
    pub seq: u64,
    pub offset: u64,
    pub kind: LogKind,
    pub content: String,
}

enum LogCommand {
    Write {
        kind: LogKind,
        content: String,
        ack: oneshot::Sender<std::result::Result<LogEvent, String>>,
    },
}

#[derive(Clone)]
pub struct LogSession {
    task_id: Option<i32>,
    run_id: Option<i32>,
    job_id: Option<i32>,
    path: PathBuf,
    tx: mpsc::UnboundedSender<LogCommand>,
    broadcaster: broadcast::Sender<LogEvent>,
}

impl LogSession {
    pub async fn create_task_run(logs_dir: &Path, task_id: i32, run_id: i32) -> Result<Self> {
        let log_dir = logs_dir.join(task_id.to_string());
        let path = unique_log_path(&log_dir, Some(run_id), "run").await?;
        Self::create(path, Some(task_id), Some(run_id), None).await
    }

    pub async fn create_system_job(jobs_dir: &Path, job_id: i32) -> Result<Self> {
        let log_dir = jobs_dir.join(job_id.to_string());
        let path = unique_log_path(&log_dir, Some(job_id), "job").await?;
        Self::create(path, None, None, Some(job_id)).await
    }

    #[cfg(test)]
    async fn create_for_test(path: PathBuf) -> Result<Self> {
        Self::create(path, Some(1), Some(1), None).await
    }

    async fn create(
        path: PathBuf,
        task_id: Option<i32>,
        run_id: Option<i32>,
        job_id: Option<i32>,
    ) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(AppError::Io)?;
        }

        let file = tokio::fs::OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)
            .await
            .map_err(AppError::Io)?;
        let offset = file.metadata().await.map(|m| m.len()).unwrap_or(0);
        let (tx, rx) = mpsc::unbounded_channel();
        let (broadcaster, _) = broadcast::channel(1000);

        spawn_writer(
            rx,
            file,
            offset,
            task_id,
            run_id,
            job_id,
            broadcaster.clone(),
        );

        Ok(Self {
            task_id,
            run_id,
            job_id,
            path,
            tx,
            broadcaster,
        })
    }

    pub fn task_id(&self) -> Option<i32> {
        self.task_id
    }

    pub fn run_id(&self) -> Option<i32> {
        self.run_id
    }

    pub fn job_id(&self) -> Option<i32> {
        self.job_id
    }

    pub fn path_string(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEvent> {
        self.broadcaster.subscribe()
    }

    pub async fn write_line(&self, kind: LogKind, line: impl Into<String>) -> Result<LogEvent> {
        let mut content = line.into();
        content.push('\n');
        self.write_raw(kind, content).await
    }

    pub async fn write_raw(&self, kind: LogKind, content: impl Into<String>) -> Result<LogEvent> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(LogCommand::Write {
                kind,
                content: content.into(),
                ack: ack_tx,
            })
            .map_err(|_| AppError::Generic("日志会话已关闭".to_string()))?;

        match ack_rx.await {
            Ok(Ok(event)) => Ok(event),
            Ok(Err(message)) => Err(AppError::Generic(message)),
            Err(_) => Err(AppError::Generic("日志会话未返回写入结果".to_string())),
        }
    }
}

async fn unique_log_path(log_dir: &Path, id: Option<i32>, label: &str) -> Result<PathBuf> {
    tokio::fs::create_dir_all(log_dir)
        .await
        .map_err(AppError::Io)?;

    let timestamp = chrono::Local::now()
        .format("%Y-%m-%d_%H-%M-%S%.3f")
        .to_string();
    let id_part = id
        .map(|value| format!("_{}-{}", label, value))
        .unwrap_or_default();

    for attempt in 0..100 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{}", attempt)
        };
        let path = log_dir.join(format!("{}{}{}.log", timestamp, id_part, suffix));
        if !path.exists() {
            return Ok(path);
        }
    }

    Err(AppError::Generic(
        "无法生成唯一日志文件名，请稍后重试".to_string(),
    ))
}

fn spawn_writer(
    mut rx: mpsc::UnboundedReceiver<LogCommand>,
    file: tokio::fs::File,
    initial_offset: u64,
    task_id: Option<i32>,
    run_id: Option<i32>,
    job_id: Option<i32>,
    broadcaster: broadcast::Sender<LogEvent>,
) {
    tokio::spawn(async move {
        let mut writer = BufWriter::with_capacity(16 * 1024, file);
        let mut offset = initial_offset;
        let mut seq = 0;

        while let Some(command) = rx.recv().await {
            match command {
                LogCommand::Write { kind, content, ack } => {
                    let result = async {
                        writer
                            .write_all(content.as_bytes())
                            .await
                            .map_err(|e| e.to_string())?;
                        writer.flush().await.map_err(|e| e.to_string())?;

                        seq += 1;
                        offset += content.len() as u64;
                        let event = LogEvent {
                            task_id,
                            run_id,
                            job_id,
                            seq,
                            offset,
                            kind,
                            content,
                        };
                        let _ = broadcaster.send(event.clone());
                        Ok(event)
                    }
                    .await;

                    let _ = ack.send(result);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn write_line_persists_and_broadcasts_same_content() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("niupanel-log-session-{unique}"));
        let path = dir.join("task.log");
        let session = LogSession::create_for_test(path.clone())
            .await
            .expect("create log session");
        let mut rx = session.subscribe();

        let written = session
            .write_line(LogKind::Control, "hello")
            .await
            .expect("write log line");
        let broadcast = rx.recv().await.expect("receive broadcast");
        let file_content = tokio::fs::read_to_string(&path)
            .await
            .expect("read log file");

        assert_eq!(written.offset, "hello\n".len() as u64);
        assert_eq!(broadcast.offset, written.offset);
        assert_eq!(broadcast.seq, 1);
        assert_eq!(broadcast.kind, LogKind::Control);
        assert_eq!(broadcast.content, "hello\n");
        assert_eq!(file_content, broadcast.content);

        let _ = tokio::fs::remove_dir_all(dir).await;
    }
}
