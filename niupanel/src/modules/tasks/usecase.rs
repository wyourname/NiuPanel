use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use niupanel_common::{
    error::{AppError, Result},
    warn,
};
use niupanel_core::audit::service::AuditService;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::tasks;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
};

use super::models::{CreateTaskRequest, QuickCreateUrlRequest, UpdateTaskRequest};
use super::service::TaskService;

static TASK_SCHEDULE_MUTATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone)]
pub struct TaskUseCase {
    db: DatabaseConnection,
    http_client: reqwest::Client,
    task_manager: TaskManagerService,
}

fn operation_counts(results: &[serde_json::Value]) -> (usize, usize) {
    let successes = results
        .iter()
        .filter(|result| result.get("status").and_then(|status| status.as_str()) == Some("success"))
        .count();
    (successes, results.len().saturating_sub(successes))
}

impl TaskUseCase {
    pub fn new(
        db: DatabaseConnection,
        http_client: reqwest::Client,
        task_manager: TaskManagerService,
    ) -> Self {
        Self {
            db,
            http_client,
            task_manager,
        }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(
            state.db.clone(),
            state.http_client.clone(),
            state.task_manager.clone(),
        )
    }

    async fn refresh_task_schedule(&self, task: &tasks::Model) -> Result<()> {
        self.task_manager.update_task_schedule(task).await
    }

    async fn remove_task_schedule(&self, task_id: i32) -> Result<()> {
        self.task_manager.remove_task_schedule(task_id).await
    }

    async fn restore_task_record(&self, task: &tasks::Model) -> Result<()> {
        let active = task.clone().into_active_model().reset_all();
        active.update(&self.db).await?;
        Ok(())
    }

    async fn rollback_created_task(
        &self,
        task_id: i32,
        delete_script: bool,
        cause: AppError,
    ) -> AppError {
        let schedule_cleanup = self.remove_task_schedule(task_id).await;
        let database_cleanup =
            niupanel_core::task::delete_task_records(&self.db, vec![task_id], delete_script, false)
                .await;
        if schedule_cleanup.is_ok() && database_cleanup.is_ok() {
            return cause;
        }

        AppError::Internal(format!(
            "{}；创建任务回滚不完整（调度清理: {}，数据库清理: {}）",
            cause,
            schedule_cleanup
                .err()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "成功".to_string()),
            database_cleanup
                .err()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "成功".to_string())
        ))
    }

    async fn rollback_updated_task(&self, previous: &tasks::Model, cause: AppError) -> AppError {
        let database_rollback = self.restore_task_record(previous).await;
        let schedule_rollback = self.refresh_task_schedule(previous).await;
        if database_rollback.is_ok() && schedule_rollback.is_ok() {
            return cause;
        }

        AppError::Internal(format!(
            "{}；更新任务回滚不完整（数据库: {}，调度: {}）",
            cause,
            database_rollback
                .err()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "成功".to_string()),
            schedule_rollback
                .err()
                .map(|err| err.to_string())
                .unwrap_or_else(|| "成功".to_string())
        ))
    }

    pub async fn create_task(
        &self,
        user: &AuthenticatedUser,
        payload: CreateTaskRequest,
    ) -> Result<tasks::Model> {
        let _mutation_guard = TASK_SCHEDULE_MUTATION_LOCK.lock().await;
        let task = TaskService::create_task(&self.db, user.id, payload).await?;
        if let Err(error) = self.refresh_task_schedule(&task).await {
            return Err(self.rollback_created_task(task.id, false, error).await);
        }

        AuditService::log_user(
            &self.db,
            user,
            "创建任务",
            "task",
            Some(task.id.to_string()),
            Some(format!("创建任务: {}", task.name)),
        )
        .await;

        Ok(task)
    }

    pub async fn update_task(
        &self,
        user: &AuthenticatedUser,
        id: i32,
        payload: UpdateTaskRequest,
    ) -> Result<tasks::Model> {
        let _mutation_guard = TASK_SCHEDULE_MUTATION_LOCK.lock().await;
        let previous = tasks::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("任务不存在".to_string()))?;
        let task = TaskService::update_task(&self.db, id, payload).await?;
        if let Err(error) = self.refresh_task_schedule(&task).await {
            return Err(self.rollback_updated_task(&previous, error).await);
        }

        AuditService::log_user(
            &self.db,
            user,
            "更新任务",
            "task",
            Some(id.to_string()),
            Some(format!("更新任务: {}", task.name)),
        )
        .await;

        Ok(task)
    }

    pub async fn quick_create_task_from_url(
        &self,
        user: &AuthenticatedUser,
        payload: QuickCreateUrlRequest,
    ) -> Result<tasks::Model> {
        let _mutation_guard = TASK_SCHEDULE_MUTATION_LOCK.lock().await;
        let url = payload.url;
        let task = TaskService::quick_create_task_from_url(
            &self.db,
            &self.http_client,
            user.id,
            url.clone(),
        )
        .await?;
        if let Err(error) = self.refresh_task_schedule(&task).await {
            return Err(self.rollback_created_task(task.id, true, error).await);
        }

        AuditService::log_user(
            &self.db,
            user,
            "通过URL快速创建任务",
            "task",
            Some(task.id.to_string()),
            Some(format!("创建任务: {} from {}", task.name, url)),
        )
        .await;

        Ok(task)
    }

    pub async fn batch_delete_tasks(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
        delete_script: bool,
        delete_var: bool,
    ) -> Result<()> {
        let _mutation_guard = TASK_SCHEDULE_MUTATION_LOCK.lock().await;
        let count = ids.len();
        let existing_tasks = tasks::Entity::find()
            .filter(tasks::Column::Id.is_in(ids.clone()))
            .all(&self.db)
            .await?;
        let mut removed_schedules = Vec::new();
        for task in &existing_tasks {
            if let Err(error) = self.remove_task_schedule(task.id).await {
                for removed in &removed_schedules {
                    if let Err(restore_error) = self.refresh_task_schedule(removed).await {
                        warn!(
                            "删除任务失败后恢复任务 {} 调度失败: {}",
                            removed.id, restore_error
                        );
                    }
                }
                return Err(error);
            }
            removed_schedules.push(task.clone());
        }

        if let Err(error) =
            TaskService::batch_delete_tasks(&self.db, ids.clone(), delete_script, delete_var).await
        {
            let remaining_ids = match tasks::Entity::find()
                .filter(tasks::Column::Id.is_in(ids.clone()))
                .all(&self.db)
                .await
            {
                Ok(tasks) => tasks
                    .into_iter()
                    .map(|task| task.id)
                    .collect::<std::collections::HashSet<_>>(),
                Err(check_error) => {
                    return Err(AppError::Internal(format!(
                        "{}；无法确认任务删除状态，调度未自动恢复: {}",
                        error, check_error
                    )));
                }
            };
            let mut restore_errors = Vec::new();
            for task in &removed_schedules {
                if remaining_ids.contains(&task.id)
                    && let Err(restore_error) = self.refresh_task_schedule(task).await
                {
                    restore_errors.push(format!("{}: {}", task.id, restore_error));
                }
            }
            return if restore_errors.is_empty() {
                Err(error)
            } else {
                Err(AppError::Internal(format!(
                    "{}；恢复任务调度失败: {}",
                    error,
                    restore_errors.join(", ")
                )))
            };
        }

        AuditService::log_user(
            &self.db,
            user,
            "批量删除任务",
            "task",
            None,
            Some(format!("批量删除了 {} 个任务", count)),
        )
        .await;

        Ok(())
    }

    pub async fn batch_set_pinned(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
        pinned: bool,
    ) -> Result<()> {
        TaskService::batch_set_pinned(&self.db, &ids, pinned).await?;

        let action = if pinned {
            "批量置顶任务"
        } else {
            "批量取消置顶任务"
        };
        let details = if pinned {
            format!("置顶任务: {:?}", ids)
        } else {
            format!("取消置顶任务: {:?}", ids)
        };

        AuditService::log_user(&self.db, user, action, "task", None, Some(details)).await;
        Ok(())
    }

    pub async fn batch_run_tasks(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
    ) -> Result<Vec<serde_json::Value>> {
        let results = TaskService::batch_run_tasks(&self.db, &self.task_manager, ids).await;

        AuditService::log_user(
            &self.db,
            user,
            "批量运行任务",
            "task",
            None,
            Some(format!("批量运行任务 {} 个", results.len())),
        )
        .await;

        Ok(results)
    }

    pub async fn batch_stop_tasks(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
    ) -> Result<Vec<serde_json::Value>> {
        let results = TaskService::batch_stop_tasks(&self.task_manager, ids).await;
        let (success_count, failure_count) = operation_counts(&results);

        AuditService::log_user(
            &self.db,
            user,
            "批量停止任务",
            "task",
            None,
            Some(format!(
                "批量停止任务：成功 {} 个，失败 {} 个",
                success_count, failure_count
            )),
        )
        .await;

        Ok(results)
    }

    pub async fn batch_pause_tasks(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
    ) -> Result<Vec<serde_json::Value>> {
        let results = TaskService::batch_pause_tasks(&self.task_manager, ids).await;
        let (success_count, failure_count) = operation_counts(&results);

        AuditService::log_user(
            &self.db,
            user,
            "批量暂停任务",
            "task",
            None,
            Some(format!(
                "批量暂停任务：成功 {} 个，失败 {} 个",
                success_count, failure_count
            )),
        )
        .await;

        Ok(results)
    }

    pub async fn batch_resume_tasks(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
    ) -> Result<Vec<serde_json::Value>> {
        let results = TaskService::batch_resume_tasks(&self.task_manager, ids).await;
        let (success_count, failure_count) = operation_counts(&results);

        AuditService::log_user(
            &self.db,
            user,
            "批量恢复任务",
            "task",
            None,
            Some(format!(
                "批量恢复任务：成功 {} 个，失败 {} 个",
                success_count, failure_count
            )),
        )
        .await;

        Ok(results)
    }

    pub async fn batch_set_enabled(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
        enabled: bool,
    ) -> Result<()> {
        let _mutation_guard = TASK_SCHEDULE_MUTATION_LOCK.lock().await;
        let previous_tasks = tasks::Entity::find()
            .filter(tasks::Column::Id.is_in(ids.clone()))
            .all(&self.db)
            .await?;
        let tasks = TaskService::batch_set_enabled(&self.db, &ids, enabled).await?;
        for task in tasks {
            if let Err(error) = self.refresh_task_schedule(&task).await {
                let mut rollback_errors = Vec::new();
                for previous in &previous_tasks {
                    if let Err(rollback_error) = self.restore_task_record(previous).await {
                        rollback_errors.push(format!(
                            "任务 {} 数据库回滚失败: {}",
                            previous.id, rollback_error
                        ));
                    }
                    if let Err(rollback_error) = self.refresh_task_schedule(previous).await {
                        rollback_errors.push(format!(
                            "任务 {} 调度回滚失败: {}",
                            previous.id, rollback_error
                        ));
                    }
                }
                return if rollback_errors.is_empty() {
                    Err(error)
                } else {
                    Err(AppError::Internal(format!(
                        "{}；批量启停回滚不完整: {}",
                        error,
                        rollback_errors.join(", ")
                    )))
                };
            }
        }

        let action = if enabled {
            "批量启用任务"
        } else {
            "批量禁用任务"
        };
        let details = if enabled {
            format!("批量启用了 {} 个任务", ids.len())
        } else {
            format!("批量禁用了 {} 个任务", ids.len())
        };

        AuditService::log_user(&self.db, user, action, "task", None, Some(details)).await;
        Ok(())
    }
}
