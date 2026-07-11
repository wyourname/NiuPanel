use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use niupanel_common::{error::Result, warn};
use niupanel_core::audit::service::AuditService;
use niupanel_core::task_manager::service::TaskManagerService;
use niupanel_entity::tasks;
use sea_orm::DatabaseConnection;

use super::models::{CreateTaskRequest, QuickCreateUrlRequest, UpdateTaskRequest};
use super::service::TaskService;

#[derive(Clone)]
pub struct TaskUseCase {
    db: DatabaseConnection,
    http_client: reqwest::Client,
    task_manager: TaskManagerService,
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

    async fn refresh_task_schedule(&self, task: &tasks::Model) {
        if let Err(error) = self.task_manager.update_task_schedule(task).await {
            warn!("刷新任务 {} 调度失败: {}", task.id, error);
        }
    }

    async fn remove_task_schedule(&self, task_id: i32) {
        if let Err(error) = self.task_manager.remove_task_schedule(task_id).await {
            warn!("移除任务 {} 调度失败: {}", task_id, error);
        }
    }

    pub async fn create_task(
        &self,
        user: &AuthenticatedUser,
        payload: CreateTaskRequest,
    ) -> Result<tasks::Model> {
        let task = TaskService::create_task(&self.db, user.id, payload).await?;
        self.refresh_task_schedule(&task).await;

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
        let task = TaskService::update_task(&self.db, id, payload).await?;
        self.refresh_task_schedule(&task).await;

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
        let url = payload.url;
        let task = TaskService::quick_create_task_from_url(
            &self.db,
            &self.http_client,
            user.id,
            url.clone(),
        )
        .await?;
        self.refresh_task_schedule(&task).await;

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
        let count = ids.len();
        TaskService::batch_delete_tasks(&self.db, ids.clone(), delete_script, delete_var).await?;

        for id in ids {
            self.remove_task_schedule(id).await;
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

    pub async fn batch_stop_tasks(&self, user: &AuthenticatedUser, ids: Vec<i32>) -> Result<()> {
        let count = ids.len();
        TaskService::batch_stop_tasks(&self.task_manager, ids).await;

        AuditService::log_user(
            &self.db,
            user,
            "批量停止任务",
            "task",
            None,
            Some(format!("批量停止了 {} 个任务", count)),
        )
        .await;

        Ok(())
    }

    pub async fn batch_pause_tasks(&self, user: &AuthenticatedUser, ids: Vec<i32>) -> Result<()> {
        let count = ids.len();
        TaskService::batch_pause_tasks(&self.task_manager, ids).await;

        AuditService::log_user(
            &self.db,
            user,
            "批量暂停任务",
            "task",
            None,
            Some(format!("批量暂停了 {} 个任务", count)),
        )
        .await;

        Ok(())
    }

    pub async fn batch_resume_tasks(&self, user: &AuthenticatedUser, ids: Vec<i32>) -> Result<()> {
        let count = ids.len();
        TaskService::batch_resume_tasks(&self.task_manager, ids).await;

        AuditService::log_user(
            &self.db,
            user,
            "批量恢复任务",
            "task",
            None,
            Some(format!("批量恢复了 {} 个任务", count)),
        )
        .await;

        Ok(())
    }

    pub async fn batch_set_enabled(
        &self,
        user: &AuthenticatedUser,
        ids: Vec<i32>,
        enabled: bool,
    ) -> Result<()> {
        let tasks = TaskService::batch_set_enabled(&self.db, &ids, enabled).await?;
        for task in tasks {
            self.refresh_task_schedule(&task).await;
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
