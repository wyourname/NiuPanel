use super::{TaskManagerService, job_log_key};
use crate::event_bus::{SystemEvent, TaskEvent};
use crate::task_log::{LogKind, LogSession};
use futures::FutureExt;
use niupanel_common::config::Config;
use niupanel_common::error::AppError;
use niupanel_entity::system_jobs;
use niupanel_entity::task_status::TaskStatus;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use tokio::sync::mpsc;

impl TaskManagerService {
    pub async fn submit_system_task<F, Fut>(
        &self,
        name: String,
        task_fn: F,
    ) -> Result<i32, AppError>
    where
        F: FnOnce(mpsc::UnboundedSender<String>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<(), AppError>> + Send + 'static,
    {
        let permit = self
            .running_permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                AppError::ConcurrencyLimitExceeded("系统任务并发限制已达，请稍候".to_string())
            })?;

        let new_job = system_jobs::ActiveModel {
            name: Set(name.clone()),
            status: Set(TaskStatus::Pending),
            created_at: Set(chrono::Utc::now().into()),
            ..Default::default()
        };
        let job_record = new_job.insert(&self.db).await?;
        let job_id = job_record.id;

        let log_session =
            match LogSession::create_system_job(&Config::global().jobs_dir, job_id).await {
                Ok(session) => session,
                Err(err) => {
                    let failed_job = system_jobs::ActiveModel {
                        id: Set(job_id),
                        status: Set(TaskStatus::Failed),
                        ended_at: Set(Some(chrono::Utc::now().into())),
                        ..Default::default()
                    };
                    if let Err(update_err) = failed_job.update(&self.db).await {
                        tracing::error!(
                            "failed to mark system job {} as failed after log setup error: {}",
                            job_id,
                            update_err
                        );
                    }
                    return Err(err);
                }
            };
        self.log_sessions
            .insert(job_log_key(job_id), log_session.clone());

        let mut job_active: system_jobs::ActiveModel = job_record.into();
        job_active.log_path = Set(Some(log_session.path_string()));
        job_active.status = Set(TaskStatus::Running);
        job_active.update(&self.db).await?;

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let log_session_for_task = log_session.clone();
        let log_writer = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(err) = log_session_for_task.write_line(LogKind::System, msg).await {
                    tracing::error!("failed to write system job {} log: {}", job_id, err);
                }
            }
        });

        let event = SystemEvent::Task(TaskEvent::StatusChanged {
            task_id: 0,
            job_id: Some(job_id),
            run_id: None,
            status: TaskStatus::Running,
            is_system: true,
            output: None,
            cpu_usage: None,
            memory_usage: None,
        });
        self.event_bus.publish(event);

        let service = self.clone();
        let handle = tokio::spawn(async move {
            let _permit = permit;
            let result_tx = tx.clone();
            let task_result = AssertUnwindSafe(task_fn(tx)).catch_unwind().await;
            let error_message = match task_result {
                Ok(Ok(())) => None,
                Ok(Err(err)) => Some(err.to_string()),
                Err(panic) => {
                    let panic_message = panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "unknown panic".to_string());
                    Some(format!("system task panicked: {panic_message}"))
                }
            };
            let final_status = if error_message.is_some() {
                TaskStatus::Failed
            } else {
                TaskStatus::Finished
            };
            if let Some(message) = &error_message {
                tracing::error!("system job {} failed: {}", job_id, message);
                let _ = result_tx.send(format!("[System] Job failed: {message}"));
            }
            drop(result_tx);

            let job_update = system_jobs::ActiveModel {
                id: Set(job_id),
                status: Set(final_status),
                ended_at: Set(Some(chrono::Utc::now().into())),
                ..Default::default()
            };
            if let Err(err) = job_update.update(&service.db).await {
                tracing::error!(
                    "failed to persist system job {} final status: {}",
                    job_id,
                    err
                );
            }

            let event = SystemEvent::Task(TaskEvent::StatusChanged {
                task_id: 0,
                job_id: Some(job_id),
                run_id: None,
                status: final_status,
                is_system: true,
                output: error_message,
                cpu_usage: None,
                memory_usage: None,
            });
            service.event_bus.publish(event);

            if let Err(err) = log_writer.await {
                tracing::error!("system job {} log writer failed: {}", job_id, err);
            }
            service.log_sessions.remove(&job_log_key(job_id));
            service.system_job_handles.remove(&job_id);
        });

        self.system_job_handles
            .insert(job_id, handle.abort_handle());

        Ok(job_id)
    }

    pub async fn cancel_system_job(&self, job_id: i32) -> Result<(), AppError> {
        if let Some((_, handle)) = self.system_job_handles.remove(&job_id) {
            handle.abort();

            let job_update = system_jobs::ActiveModel {
                id: Set(job_id),
                status: Set(TaskStatus::Cancelled),
                ended_at: Set(Some(chrono::Utc::now().into())),
                ..Default::default()
            };
            job_update.update(&self.db).await?;

            if let Some(session) = self
                .log_sessions
                .get(&job_log_key(job_id))
                .map(|entry| entry.value().clone())
            {
                let _ = session
                    .write_line(LogKind::System, "[System] Job cancelled by user.")
                    .await;
            }

            let event = SystemEvent::Task(TaskEvent::StatusChanged {
                task_id: 0,
                job_id: Some(job_id),
                run_id: None,
                status: TaskStatus::Cancelled,
                is_system: true,
                output: None,
                cpu_usage: None,
                memory_usage: None,
            });
            self.event_bus.publish(event);

            self.log_sessions.remove(&job_log_key(job_id));
            return Ok(());
        }

        Err(AppError::NotFound(
            "System job not found or not running".to_string(),
        ))
    }

    pub async fn update_system_job_metadata(
        &self,
        job_id: i32,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        let mut job: system_jobs::ActiveModel = system_jobs::Entity::find_by_id(job_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("System job not found".to_string()))?
            .into();
        job.metadata = Set(Some(metadata));
        job.update(&self.db).await?;
        Ok(())
    }
}
