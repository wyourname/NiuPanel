use sea_orm::{DatabaseConnection, EntityTrait};

use niupanel_common::error::{AppError, Result};
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::task_runs;

use super::{TaskService, read_log_response};
use crate::modules::tasks::models::{LogPagination, LogResponse};

impl TaskService {
    pub async fn get_latest_log(
        task_manager: &TaskManagerService,
        id: i32,
        pagination: LogPagination,
    ) -> Result<LogResponse> {
        let path = task_manager
            .get_latest_log_path(id, false)
            .await
            .ok_or_else(|| AppError::NotFound("日志文件不存在".to_string()))?;

        read_log_response(path, pagination, Some(1024 * 1024)).await
    }

    pub async fn get_task_run_log(
        db: &DatabaseConnection,
        id: i32,
        run_id: i32,
        pagination: LogPagination,
    ) -> Result<LogResponse> {
        let run = task_runs::Entity::find_by_id(run_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("任务日志不存在".to_string()))?;

        if run.task_id != id {
            return Err(AppError::NotFound(
                "该运行记录与指定任务不匹配。".to_string(),
            ));
        }

        let path = run
            .log_path
            .ok_or_else(|| AppError::NotFound("日志文件不存在".to_string()))?;
        read_log_response(path, pagination, None).await
    }

    pub async fn delete_task_run(db: &DatabaseConnection, id: i32, run_id: i32) -> Result<()> {
        let run = task_runs::Entity::find_by_id(run_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("记录不存在".to_string()))?;

        if run.task_id != id {
            return Err(AppError::Forbidden("无权操作此记录".to_string()));
        }

        if let Some(path) = run.log_path {
            let _ = tokio::fs::remove_file(path).await;
        }

        task_runs::Entity::delete_by_id(run_id).exec(db).await?;
        Ok(())
    }
}
