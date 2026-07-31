use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::{ApiResponse, PaginatedData};
use niupanel_core::audit::service::AuditService;
use niupanel_entity::tg_workflows;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct QueryParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateWorkflowReq {
    pub event_type: String,
    pub action_type: String,
    pub config_json: String,
}

const MAX_WORKFLOW_CONFIG_BYTES: usize = 1024 * 1024;
const SUPPORTED_EVENT_TYPES: &[&str] = &["failed", "success", "alert", "cron"];
const SUPPORTED_ACTION_TYPES: &[&str] = &["notify", "shell", "approval"];
static WORKFLOW_MUTATION_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn validate_workflow_request(event_type: &str, action_type: &str, config_json: &str) -> Result<()> {
    if !SUPPORTED_EVENT_TYPES.contains(&event_type) {
        return Err(AppError::ValidationError(format!(
            "不支持的事件类型: {}",
            event_type
        )));
    }
    if !SUPPORTED_ACTION_TYPES.contains(&action_type) {
        return Err(AppError::ValidationError(format!(
            "不支持的动作类型: {}",
            action_type
        )));
    }
    if config_json.len() > MAX_WORKFLOW_CONFIG_BYTES {
        return Err(AppError::ValidationError("工作流配置不能超过 1 MB".into()));
    }

    let parsed_config = if event_type == "cron"
        || matches!(action_type, "notify" | "approval")
        || config_json.trim_start().starts_with('{')
        || config_json.trim_start().starts_with('[')
    {
        Some(
            serde_json::from_str::<serde_json::Value>(config_json).map_err(|err| {
                AppError::ValidationError(format!("工作流配置不是有效 JSON: {}", err))
            })?,
        )
    } else {
        None
    };

    if event_type == "cron" {
        let cron = parsed_config
            .as_ref()
            .and_then(|config| config.get("cron"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if cron.is_none() {
            return Err(AppError::ValidationError(
                "Cron 工作流必须提供非空的 cron 字段".into(),
            ));
        }
    }

    if action_type == "approval" {
        let script = parsed_config
            .as_ref()
            .and_then(|config| config.get("script"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if script.is_none() {
            return Err(AppError::ValidationError(
                "审批工作流必须提供非空的 script 字段".into(),
            ));
        }
    } else if action_type == "shell" {
        let script = parsed_config
            .as_ref()
            .and_then(|config| config.get("script"))
            .and_then(|value| value.as_str())
            .unwrap_or(config_json)
            .trim();
        if script.is_empty() {
            return Err(AppError::ValidationError("Shell 工作流脚本不能为空".into()));
        }
    }

    Ok(())
}

#[derive(Serialize, ToSchema)]
pub struct WorkflowDto {
    pub id: i32,
    pub event_type: String,
    pub action_type: String,
    pub config_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<tg_workflows::Model> for WorkflowDto {
    fn from(m: tg_workflows::Model) -> Self {
        Self {
            id: m.id,
            event_type: m.event_type,
            action_type: m.action_type,
            config_json: m.config_json,
            created_at: m.created_at.and_utc().timestamp(),
            updated_at: m.updated_at.and_utc().timestamp(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/bot/telegram/workflows",
    responses(
        (status = 200, description = "List Telegram workflows")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn list_workflows(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<QueryParams>,
) -> Result<ApiResponse<PaginatedData<WorkflowDto>>> {
    let page = std::cmp::max(params.page.unwrap_or(1), 1);
    let limit = params.limit.unwrap_or(20).clamp(1, 200);

    let paginator = tg_workflows::Entity::find()
        .order_by_desc(tg_workflows::Column::UpdatedAt)
        .paginate(&state.db, limit);

    let total = paginator.num_items().await?;
    let models = paginator.fetch_page(page - 1).await?;
    let dtos = models.into_iter().map(WorkflowDto::from).collect();

    Ok(ApiResponse::success(PaginatedData { items: dtos, total }))
}

#[utoipa::path(
    post,
    path = "/api/v1/bot/telegram/workflows",
    request_body = CreateWorkflowReq,
    responses(
        (status = 200, description = "Create Telegram workflow")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn create_workflow(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<CreateWorkflowReq>,
) -> Result<ApiResponse<WorkflowDto>> {
    let event_type = req.event_type.trim().to_string();
    let action_type = req.action_type.trim().to_string();
    validate_workflow_request(&event_type, &action_type, &req.config_json)?;
    let _mutation_guard = WORKFLOW_MUTATION_LOCK.lock().await;

    let now = chrono::Utc::now().naive_utc();
    let wf_model = tg_workflows::ActiveModel {
        event_type: Set(event_type.clone()),
        action_type: Set(action_type),
        config_json: Set(req.config_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|e| AppError::Generic(e.to_string()))?;

    if let Err(schedule_err) = state.task_manager.update_workflow_schedule(&wf_model).await {
        let rollback_result = tg_workflows::Entity::delete_by_id(wf_model.id)
            .exec(&state.db)
            .await;
        return match rollback_result {
            Ok(_) => Err(schedule_err),
            Err(rollback_err) => Err(AppError::Internal(format!(
                "{}；数据库回滚失败: {}",
                schedule_err, rollback_err
            ))),
        };
    }

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "创建TG自动化工作流",
        "telegram",
        Some(wf_model.id.to_string()),
        Some(event_type),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(wf_model.into()))
}

#[utoipa::path(
    put,
    path = "/api/v1/bot/telegram/workflows/{id}",
    params(
        ("id" = i32, Path, description = "Workflow ID")
    ),
    request_body = CreateWorkflowReq,
    responses(
        (status = 200, description = "Update Telegram workflow")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn update_workflow(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<CreateWorkflowReq>,
) -> Result<ApiResponse<WorkflowDto>> {
    let event_type = req.event_type.trim().to_string();
    let action_type = req.action_type.trim().to_string();
    validate_workflow_request(&event_type, &action_type, &req.config_json)?;
    let _mutation_guard = WORKFLOW_MUTATION_LOCK.lock().await;

    let previous = tg_workflows::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("工作流不存在".into()))?;
    let mut am: tg_workflows::ActiveModel = previous.clone().into();

    am.event_type = Set(event_type);
    am.action_type = Set(action_type);
    am.config_json = Set(req.config_json);
    am.updated_at = Set(chrono::Utc::now().naive_utc());

    let model = am
        .update(&state.db)
        .await
        .map_err(|e| AppError::Generic(e.to_string()))?;

    if let Err(schedule_err) = state.task_manager.update_workflow_schedule(&model).await {
        let mut rollback: tg_workflows::ActiveModel = previous.into();
        rollback.event_type = Set(rollback.event_type.take().unwrap_or_default());
        rollback.action_type = Set(rollback.action_type.take().unwrap_or_default());
        rollback.config_json = Set(rollback.config_json.take().unwrap_or_default());
        rollback.updated_at = Set(rollback.updated_at.take().unwrap_or_default());
        let rollback_result = rollback.update(&state.db).await;
        return match rollback_result {
            Ok(_) => Err(schedule_err),
            Err(rollback_err) => Err(AppError::Internal(format!(
                "{}；数据库回滚失败: {}",
                schedule_err, rollback_err
            ))),
        };
    }

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新TG自动化工作流",
        "telegram",
        Some(id.to_string()),
        None,
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(model.into()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/bot/telegram/workflows/{id}",
    params(
        ("id" = i32, Path, description = "Workflow ID")
    ),
    responses(
        (status = 200, description = "Delete Telegram workflow")
    ),
    tag = "Telegram",
    security(("session_cookie" = []))
)]
pub async fn delete_workflow(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
) -> Result<ApiResponse<()>> {
    let _mutation_guard = WORKFLOW_MUTATION_LOCK.lock().await;
    let workflow = tg_workflows::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("工作流不存在".into()))?;

    state.task_manager.remove_workflow_schedule(id).await?;

    let result = tg_workflows::Entity::delete_by_id(id).exec(&state.db).await;
    let result = match result {
        Ok(result) => result,
        Err(delete_err) => {
            let restore_result = state.task_manager.update_workflow_schedule(&workflow).await;
            return match restore_result {
                Ok(_) => Err(delete_err.into()),
                Err(restore_err) => Err(AppError::Internal(format!(
                    "删除工作流失败: {}；恢复调度也失败: {}",
                    delete_err, restore_err
                ))),
            };
        }
    };
    if result.rows_affected == 0 {
        return Err(AppError::NotFound("工作流不存在".into()));
    }

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "删除TG自动化工作流",
        "telegram",
        Some(id.to_string()),
        None,
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[cfg(test)]
mod tests {
    use super::validate_workflow_request;

    #[test]
    fn validates_supported_workflow_types() {
        assert!(validate_workflow_request("failed", "notify", "{}").is_ok());
        assert!(validate_workflow_request("timeout", "notify", "{}").is_err());
        assert!(validate_workflow_request("failed", "webhook", "{}").is_err());
    }

    #[test]
    fn validates_cron_and_approval_config() {
        assert!(
            validate_workflow_request(
                "cron",
                "shell",
                r#"{"cron":"0 8 * * *","script":"echo ok"}"#
            )
            .is_ok()
        );
        assert!(validate_workflow_request("cron", "shell", r#"{"script":"echo ok"}"#).is_err());
        assert!(validate_workflow_request("alert", "approval", r#"{"message":"ok"}"#).is_err());
    }
}
