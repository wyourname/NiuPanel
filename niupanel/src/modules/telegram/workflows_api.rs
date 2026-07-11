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
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).max(1);

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
    let now = chrono::Utc::now().naive_utc();
    let model = tg_workflows::ActiveModel {
        event_type: Set(req.event_type.clone()),
        action_type: Set(req.action_type.clone()),
        config_json: Set(req.config_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|e| AppError::Generic(e.to_string()))?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "创建TG自动化工作流",
        "telegram",
        Some(model.id.to_string()),
        Some(req.event_type),
        Some(ip),
    )
    .await;

    let wf_model = model.clone().try_into_model().unwrap();
    let _ = state.task_manager.update_workflow_schedule(&wf_model).await;

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
    let mut am: tg_workflows::ActiveModel = tg_workflows::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::Generic("工作流不存在".into()))?
        .into();

    am.event_type = Set(req.event_type);
    am.action_type = Set(req.action_type);
    am.config_json = Set(req.config_json);
    am.updated_at = Set(chrono::Utc::now().naive_utc());

    let model = am
        .update(&state.db)
        .await
        .map_err(|e| AppError::Generic(e.to_string()))?;

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

    let _ = state.task_manager.update_workflow_schedule(&model).await;

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
    let result = tg_workflows::Entity::delete_by_id(id)
        .exec(&state.db)
        .await?;
    if result.rows_affected > 0 {
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

        let _ = state.task_manager.remove_workflow_schedule(id).await;
    }
    Ok(ApiResponse::success(()))
}
