use crate::event_bus::{EventBus, SystemEvent, SystemNotification, TaskEvent};
use chrono_tz::Tz;
use niupanel_common::{error, info, warn};
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{settings, task_runs, tasks};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::str::FromStr;

pub struct DbSyncHandler {
    event_bus: EventBus,
    db: DatabaseConnection,
}

impl DbSyncHandler {
    pub fn new(event_bus: EventBus, db: DatabaseConnection) -> Self {
        Self { event_bus, db }
    }

    pub async fn run(self) {
        let mut rx = self.event_bus.subscribe();

        // Initial fetch of timezone
        let mut current_tz = self.get_initial_timezone().await;
        info!("数据库同步时区: {:?}", current_tz);

        loop {
            match rx.recv().await {
                Ok(event) => match event {
                    SystemEvent::Task(TaskEvent::StatusChanged {
                        task_id,
                        status,
                        is_system,
                        job_id: _,
                        run_id,
                        output: _,
                        ..
                    }) => {
                        if !is_system {
                            self.handle_task_status(task_id, status, run_id, current_tz)
                                .await;
                        }
                    }
                    SystemEvent::System(SystemNotification::SettingChanged { key, value }) => {
                        if key == "system.timezone" {
                            if let Ok(tz) = Tz::from_str(&value) {
                                info!("DbSyncHandler updated timezone to: {:?}", tz);
                                current_tz = tz;
                            } else if value == "UTC" {
                                current_tz = chrono_tz::UTC;
                            }
                        }
                    }
                    _ => {}
                },
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("DbSyncHandler channel closed, exiting.");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(
                        "DbSyncHandler lagged behind by {} events. Database status might be out of sync.",
                        skipped
                    );
                }
            }
        }
    }

    async fn handle_task_status(
        &self,
        task_id: i32,
        status: TaskStatus,
        run_id: Option<i32>,
        tz: Tz,
    ) {
        // 1. Update Task Status (Always)
        let task_active = tasks::ActiveModel {
            id: Set(task_id),
            status: Set(status),
            ..Default::default()
        };
        // Use update but we only set ID and status. SeaORM updates fields that are set.
        if let Err(e) = task_active.update(&self.db).await {
            error!("Failed to update task status db: {}", e);
        }

        let is_finished = matches!(
            status,
            TaskStatus::Finished | TaskStatus::Failed | TaskStatus::Stopped | TaskStatus::Cancelled
        );

        if is_finished {
            // Update Run Record
            // Note: Log path is now set at start (in TaskManagerService), so we don't need to scan for it.
            // We just update the status and end time.

            if let Some(rid) = run_id {
                let mut run_active = task_runs::ActiveModel {
                    id: Set(rid),
                    ..Default::default()
                };
                run_active.status = Set(status);
                run_active.ended_at = Set(Some(chrono::Utc::now().into()));
                let _ = run_active.update(&self.db).await;
            } else {
                // Fallback: We assume the latest "Running" run is the one finishing.
                if let Ok(Some(run)) = task_runs::Entity::find()
                    .filter(task_runs::Column::TaskId.eq(task_id))
                    .filter(task_runs::Column::Status.eq(TaskStatus::Running))
                    .order_by_desc(task_runs::Column::Id)
                    .one(&self.db)
                    .await
                {
                    let mut run_active: task_runs::ActiveModel = run.into();
                    run_active.status = Set(status);
                    run_active.ended_at = Set(Some(chrono::Utc::now().into()));
                    let _ = run_active.update(&self.db).await;
                }
            }
            if let Ok(Some(task)) = tasks::Entity::find_by_id(task_id).one(&self.db).await {
                let next_run = self.calculate_next_run(&task.cron_schedule, tz).await;

                let task_active = tasks::ActiveModel {
                    id: Set(task.id),
                    last_finished_at: Set(Some(chrono::Utc::now().into())),
                    next_run_at: Set(next_run),
                    ..Default::default()
                };

                let _ = task_active.update(&self.db).await;
            }
        }
    }

    async fn calculate_next_run(
        &self,
        cron_schedule: &Option<String>,
        tz: Tz,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        if let Some(cron_str) = cron_schedule {
            if !cron_str.trim().is_empty() {
                let normalized = self.normalize_cron(cron_str);
                // Use cached tz
                if let Ok(schedule) = normalized.parse::<cron::Schedule>() {
                    return schedule
                        .upcoming(tz)
                        .next()
                        .map(|t| t.with_timezone(&chrono::Utc));
                }
            }
        }
        None
    }

    fn normalize_cron(&self, cron_expression: &str) -> String {
        let parts: Vec<&str> = cron_expression.split_whitespace().collect();
        if parts.len() == 5 {
            format!("0 {}", cron_expression)
        } else {
            cron_expression.to_string()
        }
    }

    async fn get_initial_timezone(&self) -> Tz {
        let default_tz = chrono_tz::Asia::Shanghai;
        if let Ok(Some(val)) = settings::Entity::find_by_id("system.timezone")
            .one(&self.db)
            .await
        {
            if let Ok(tz) = Tz::from_str(&val.value) {
                return tz;
            } else if val.value == "UTC" {
                return chrono_tz::UTC;
            }
        }
        default_tz
    }
}
