use super::TaskManagerService;
use niupanel_common::error::AppError;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{task_runs, tasks};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

impl TaskManagerService {
    pub(crate) async fn cleanup_zombie_tasks(&self) -> Result<(), AppError> {
        let zombies = task_runs::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(task_runs::Column::Status.eq(TaskStatus::Running))
                    .add(task_runs::Column::Status.eq(TaskStatus::Paused)),
            )
            .all(&self.db)
            .await?;

        for run in zombies {
            let mut active: task_runs::ActiveModel = run.into();
            active.status = Set(TaskStatus::Failed);
            active.ended_at = Set(Some(chrono::Utc::now().into()));
            let _ = active.update(&self.db).await;
        }

        let tasks = tasks::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(tasks::Column::Status.eq(TaskStatus::Running))
                    .add(tasks::Column::Status.eq(TaskStatus::Paused)),
            )
            .all(&self.db)
            .await?;

        for task in tasks {
            let task_id = task.id;
            let mut active: tasks::ActiveModel = task.into();
            active.status = Set(TaskStatus::Failed);
            let _ = active.update(&self.db).await;

            self.log_system_event(
                task_id,
                "System restarted, marking zombie task as Failed.".to_string(),
            )
            .await;
        }

        Ok(())
    }
}
