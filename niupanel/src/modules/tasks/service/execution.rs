use futures::stream::{self, StreamExt};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use niupanel_core::task::{run_task_records, stop_task_records};
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::tasks;

use super::TaskService;

impl TaskService {
    pub async fn batch_run_tasks(
        db: &DatabaseConnection,
        task_manager: &TaskManagerService,
        ids: Vec<i32>,
    ) -> Vec<serde_json::Value> {
        run_task_records(db, task_manager, ids).await
    }

    pub async fn batch_stop_tasks(
        task_manager: &TaskManagerService,
        ids: Vec<i32>,
    ) -> Vec<serde_json::Value> {
        stop_task_records(task_manager, ids).await
    }

    pub async fn batch_pause_tasks(
        task_manager: &TaskManagerService,
        ids: Vec<i32>,
    ) -> Vec<serde_json::Value> {
        stream::iter(ids)
            .map(|id| {
                let task_manager = task_manager.clone();
                async move {
                    task_manager
                        .log_system_event(id, "###### 暂停任务 ######".to_string())
                        .await;
                    match task_manager.pause_task(id).await {
                        Ok(()) => serde_json::json!({
                            "task_id": id,
                            "status": "success",
                            "message": "Task paused"
                        }),
                        Err(error) => serde_json::json!({
                            "task_id": id,
                            "status": "error",
                            "message": error.to_string()
                        }),
                    }
                }
            })
            .buffer_unordered(5)
            .collect()
            .await
    }

    pub async fn batch_resume_tasks(
        task_manager: &TaskManagerService,
        ids: Vec<i32>,
    ) -> Vec<serde_json::Value> {
        stream::iter(ids)
            .map(|id| {
                let task_manager = task_manager.clone();
                async move {
                    task_manager
                        .log_system_event(id, "###### 恢复任务 ######".to_string())
                        .await;
                    match task_manager.resume_task(id).await {
                        Ok(()) => serde_json::json!({
                            "task_id": id,
                            "status": "success",
                            "message": "Task resumed"
                        }),
                        Err(error) => serde_json::json!({
                            "task_id": id,
                            "status": "error",
                            "message": error.to_string()
                        }),
                    }
                }
            })
            .buffer_unordered(5)
            .collect()
            .await
    }

    pub async fn batch_set_enabled(
        db: &DatabaseConnection,
        ids: &[i32],
        enabled: bool,
    ) -> niupanel_common::error::Result<Vec<tasks::Model>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        tasks::Entity::update_many()
            .col_expr(
                tasks::Column::Enabled,
                sea_orm::sea_query::Expr::value(enabled),
            )
            .filter(tasks::Column::Id.is_in(ids.iter().copied()))
            .exec(db)
            .await?;

        let task_models = tasks::Entity::find()
            .filter(tasks::Column::Id.is_in(ids.iter().copied()))
            .all(db)
            .await?;

        Ok(task_models)
    }
}
