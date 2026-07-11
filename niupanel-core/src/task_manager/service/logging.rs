use super::{TaskManagerService, job_log_key, task_run_log_key};
use crate::event_bus::{SystemEvent, TaskEvent};
use crate::sys::status_manager::TaskStatusEvent;
use crate::task_log::{LogEvent, LogKind};
use niupanel_common::config::Config;
use niupanel_common::error::AppError;
use niupanel_common::{error as error_msg, info, warn};
use niupanel_entity::task_status::TaskStatus;
use tokio::sync::broadcast;

impl TaskManagerService {
    pub(crate) fn spawn_status_listener(&self) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut rx = service.status_manager.subscribe();
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let terminal_message = terminal_status_message(event.status);
                        if let Some(message) = terminal_message {
                            service
                                .log_system_event(event.task_id, message.to_string())
                                .await;
                        }

                        if event.status == TaskStatus::Finished {
                            let service_clone = service.clone();
                            let task_id = event.task_id;
                            tokio::spawn(async move {
                                if let Err(e) =
                                    service_clone.check_and_trigger_next_tasks(task_id).await
                                {
                                    warn!("触发后续任务失败 (Task {}): {}", task_id, e);
                                }
                            });
                        }

                        let output = if terminal_message.is_some() {
                            service
                                .get_latest_log_content(event.task_id, false, None)
                                .await
                                .ok()
                                .map(|(s, _)| s)
                        } else {
                            None
                        };

                        let system_event = SystemEvent::Task(TaskEvent::StatusChanged {
                            task_id: event.task_id,
                            job_id: None,
                            run_id: event.run_id,
                            status: event.status.clone(),
                            is_system: false,
                            output,
                            cpu_usage: event.cpu_usage,
                            memory_usage: event.memory_usage,
                        });
                        service.event_bus.publish(system_event);

                        if terminal_message.is_some() {
                            service.cleanup_task_logger(event.task_id, event.run_id);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("状态监听器处理落后，跳过了 {} 个事件", skipped);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("状态监听器通道关闭");
                        break;
                    }
                }
            }
        });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.event_bus.subscribe()
    }

    pub fn subscribe_logs(
        &self,
        id: i32,
        is_system: bool,
    ) -> Result<broadcast::Receiver<LogEvent>, AppError> {
        if is_system {
            return self.subscribe_job_logs(id);
        }

        if let Some(run_id) = self.active_task_runs.get(&id).map(|entry| *entry.value()) {
            self.subscribe_task_run_logs(id, run_id)
        } else {
            Err(AppError::Generic("进程未运行或无实时日志可用".to_string()))
        }
    }

    pub fn subscribe_task_run_logs(
        &self,
        task_id: i32,
        run_id: i32,
    ) -> Result<broadcast::Receiver<LogEvent>, AppError> {
        let key = task_run_log_key(run_id);
        let Some(session) = self.log_sessions.get(&key) else {
            return Err(AppError::Generic(
                "运行记录未运行或无实时日志可用".to_string(),
            ));
        };

        if session.task_id() != Some(task_id) {
            return Err(AppError::NotFound("该运行记录与指定任务不匹配".to_string()));
        }

        Ok(session.subscribe())
    }

    pub fn subscribe_job_logs(
        &self,
        job_id: i32,
    ) -> Result<broadcast::Receiver<LogEvent>, AppError> {
        let key = job_log_key(job_id);
        self.log_sessions
            .get(&key)
            .map(|session| session.subscribe())
            .ok_or_else(|| AppError::Generic("系统任务未运行或无实时日志可用".to_string()))
    }

    pub async fn get_latest_log_path(&self, id: i32, is_system: bool) -> Option<String> {
        let log_dir = if is_system {
            std::path::Path::new(&Config::global().jobs_dir).join(id.to_string())
        } else {
            std::path::Path::new(&Config::global().logs_dir).join(id.to_string())
        };

        if !log_dir.exists() {
            return None;
        }

        if let Ok(mut entries) = tokio::fs::read_dir(&log_dir).await {
            let mut files = Vec::new();
            while let Ok(Some(entry)) = entries.next_entry().await {
                files.push(entry.path());
            }
            files.sort();
            files.last().map(|p| p.to_string_lossy().to_string())
        } else {
            None
        }
    }

    pub async fn get_latest_log_content(
        &self,
        id: i32,
        is_system: bool,
        limit_bytes: Option<u64>,
    ) -> Result<(String, u64), AppError> {
        const DEFAULT_MAX_LOG_READ_BYTES: u64 = 5 * 1024 * 1024;

        if let Some(path_str) = self.get_latest_log_path(id, is_system).await {
            let effective_limit = limit_bytes
                .unwrap_or(DEFAULT_MAX_LOG_READ_BYTES)
                .min(DEFAULT_MAX_LOG_READ_BYTES);

            let mut file = tokio::fs::File::open(&path_str)
                .await
                .map_err(AppError::Io)?;
            let metadata = file.metadata().await.map_err(AppError::Io)?;
            let total_len = metadata.len();

            if total_len > effective_limit {
                use tokio::io::{AsyncReadExt, AsyncSeekExt};
                let seek_pos = total_len - effective_limit;
                file.seek(std::io::SeekFrom::Start(seek_pos))
                    .await
                    .map_err(AppError::Io)?;

                let mut buf = Vec::new();
                file.read_to_end(&mut buf).await.map_err(AppError::Io)?;
                return Ok((String::from_utf8_lossy(&buf).to_string(), total_len));
            }

            let content = tokio::fs::read_to_string(path_str)
                .await
                .map_err(AppError::Io)?;
            Ok((content, total_len))
        } else {
            Ok(("".to_string(), 0))
        }
    }

    pub async fn log_system_event(&self, task_id: i32, message: String) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let formatted_msg = format!("{}\n时间:{}\n", message, timestamp);

        if self
            .write_task_log_raw(task_id, None, formatted_msg.clone())
            .await
            .is_some()
        {
            return;
        }

        let mut offset = 0;
        if let Some(path_str) = self.get_latest_log_path(task_id, false).await {
            use tokio::io::AsyncWriteExt;
            match tokio::fs::OpenOptions::new()
                .create(false)
                .append(true)
                .open(path_str)
                .await
            {
                Ok(mut file) => {
                    if let Err(e) = file.write_all(formatted_msg.as_bytes()).await {
                        error_msg!("Failed to write system event log (write): {}", e);
                    }
                    if let Err(e) = file.flush().await {
                        error_msg!("Failed to write system event log (flush): {}", e);
                    }
                    if let Ok(meta) = file.metadata().await {
                        offset = meta.len();
                    }
                }
                Err(e) => {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        error_msg!("Failed to open log file for system event: {}", e);
                    }
                }
            }
        }

        let _ = offset;
    }

    pub(super) async fn write_task_log_raw(
        &self,
        task_id: i32,
        run_id: Option<i32>,
        content: String,
    ) -> Option<u64> {
        let resolved_run_id = run_id.or_else(|| {
            self.active_task_runs
                .get(&task_id)
                .map(|entry| *entry.value())
        });

        let Some(run_id) = resolved_run_id else {
            return None;
        };

        let session = self
            .log_sessions
            .get(&task_run_log_key(run_id))
            .map(|entry| entry.value().clone());

        let Some(session) = session else {
            return None;
        };

        match session.write_raw(LogKind::Control, content).await {
            Ok(event) => Some(event.offset),
            Err(e) => {
                error_msg!("Failed to write task log through session: {}", e);
                None
            }
        }
    }

    fn cleanup_task_logger(&self, task_id: i32, run_id: Option<i32>) {
        let run_id = run_id.or_else(|| {
            self.active_task_runs
                .get(&task_id)
                .map(|entry| *entry.value())
        });
        let Some(run_id) = run_id else {
            return;
        };

        self.log_sessions.remove(&task_run_log_key(run_id));

        if self
            .active_task_runs
            .get(&task_id)
            .is_some_and(|current| *current.value() == run_id)
        {
            self.active_task_runs.remove(&task_id);
        }
    }

    pub(crate) async fn update_task_status(
        &self,
        task_id: i32,
        status: TaskStatus,
        run_id: Option<i32>,
    ) -> Result<(), AppError> {
        self.status_manager.emit(TaskStatusEvent {
            task_id,
            status,
            pid: None,
            run_id,
            cpu_usage: None,
            memory_usage: None,
        });
        Ok(())
    }
}

fn terminal_status_message(status: TaskStatus) -> Option<&'static str> {
    match status {
        TaskStatus::Finished => Some("###### 任务结束 ######"),
        TaskStatus::Failed => Some("###### 任务失败 ######"),
        TaskStatus::Stopped => Some("###### 任务停止 ######"),
        TaskStatus::Cancelled => Some("###### 任务取消 ######"),
        _ => None,
    }
}
