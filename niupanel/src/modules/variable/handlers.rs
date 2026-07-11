pub use super::models::{
    BatchIdRequest, BatchToggleRequest, KeyQuery, ReorderVariablesRequest, TaskSimpleResponse,
    VariableDto, VariableQueryParams, VariableRequest,
};
use super::service;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use niupanel_common::error::Result;
use niupanel_common::response::{ApiResponse, PaginatedData};
use niupanel_core::audit::service::AuditService;

#[utoipa::path(
    get,
    path = "/api/v1/variables",
    params(VariableQueryParams),
    responses(
        (status = 200, description = "List variables", body = ApiResponse<PaginatedData<VariableDto>>)
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn list_variables(
    State(state): State<AppState>,
    Query(params): Query<VariableQueryParams>,
) -> Result<ApiResponse<PaginatedData<VariableDto>>> {
    let data = service::list_variables(&state.db, &params).await?;
    Ok(ApiResponse::success(data))
}

#[utoipa::path(
    get,
    path = "/api/v1/variables/tasks",
    responses(
        (status = 200, description = "List tasks with simple info for variable binding")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn list_tasks_simple(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<TaskSimpleResponse>>> {
    let tasks = service::list_tasks_simple(&state.db).await?;
    Ok(ApiResponse::success(tasks))
}

#[utoipa::path(
    post,
    path = "/api/v1/variables",
    request_body = VariableRequest,
    responses(
        (status = 200, description = "Variable created", body = ApiResponse<VariableDto>)
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn create_variable(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<VariableRequest>,
) -> Result<ApiResponse<VariableDto>> {
    let key = payload.key.clone();
    let created = service::create_variable(&state.db, payload).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "创建变量",
        "variable",
        Some(created.inner.id.to_string()),
        Some(format!("创建了变量: {}", key)),
    )
    .await;

    Ok(ApiResponse::success(created))
}

#[utoipa::path(
    patch,
    path = "/api/v1/variables/{id}",
    request_body = VariableRequest,
    responses(
        (status = 200, description = "Variable updated", body = ApiResponse<VariableDto>)
    ),
    params(
        ("id" = i32, Path, description = "Variable ID")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn update_variable(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<VariableRequest>,
) -> Result<ApiResponse<VariableDto>> {
    let key = payload.key.clone();
    let updated = service::update_variable(&state.db, id, payload).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "更新变量",
        "variable",
        Some(id.to_string()),
        Some(format!("更新了变量: {}", key)),
    )
    .await;

    Ok(ApiResponse::success(updated))
}

#[utoipa::path(
    post,
    path = "/api/v1/variables/batch-toggle",
    request_body = BatchToggleRequest,
    responses(
        (status = 200, description = "Batch toggle variables enabled state")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn batch_toggle_variables(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchToggleRequest>,
) -> Result<ApiResponse<()>> {
    service::batch_toggle_variables(&state.db, &payload.ids, payload.enabled).await?;

    if !payload.ids.is_empty() {
        AuditService::log_user(
            &state.db,
            &user,
            "批量切换变量状态",
            "variable",
            None,
            Some(format!(
                "批量将 {} 个变量切换为 {}",
                payload.ids.len(),
                if payload.enabled { "启用" } else { "禁用" }
            )),
        )
        .await;
    }

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/variables/reorder",
    request_body = ReorderVariablesRequest,
    responses(
        (status = 200, description = "Batch reorder variables")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn batch_reorder_variables(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<ReorderVariablesRequest>,
) -> Result<ApiResponse<()>> {
    if payload.ids.is_empty() {
        return Ok(ApiResponse::success(()));
    }

    service::batch_reorder_variables(&state.db, payload.task_id, &payload.ids).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "变量排序",
        "variable",
        payload.task_id.map(|id| id.to_string()),
        Some(format!(
            "更新了 {} 个变量的排序 {}",
            payload.ids.len(),
            if let Some(task_id) = payload.task_id {
                format!("(任务 {})", task_id)
            } else {
                "(全局)".to_string()
            }
        )),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/variables/import",
    request_body = Vec<VariableRequest>,
    responses(
        (status = 200, description = "Import variables")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn import_variables(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<Vec<VariableRequest>>,
) -> Result<ApiResponse<usize>> {
    let count = service::import_variables(&state.db, payload).await?;

    if count > 0 {
        AuditService::log_user(
            &state.db,
            &user,
            "导入变量",
            "variable",
            None,
            Some(format!("导入了 {} 个变量", count)),
        )
        .await;
    }

    Ok(ApiResponse::success(count))
}

#[utoipa::path(
    get,
    path = "/api/v1/variables/by-key",
    params(KeyQuery),
    responses(
        (status = 200, description = "Get variable by key")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn get_variable_by_key(
    State(state): State<AppState>,
    Query(params): Query<KeyQuery>,
) -> Result<ApiResponse<Vec<VariableDto>>> {
    let data = service::get_variable_by_key(&state.db, &params.key).await?;
    Ok(ApiResponse::success(data))
}

#[utoipa::path(
    put,
    path = "/api/v1/variables/by-key",
    params(KeyQuery),
    request_body = VariableRequest,
    responses(
        (status = 200, description = "Update variable by key")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn update_variable_by_key(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<KeyQuery>,
    Json(payload): Json<VariableRequest>,
) -> Result<ApiResponse<Vec<VariableDto>>> {
    let target_key = params.key.clone();
    let updated = service::update_variable_by_key(&state.db, &params.key, payload).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "通过Key更新变量",
        "variable",
        None,
        Some(format!(
            "通过Key更新了 {} 个变量: {}",
            updated.len(),
            target_key
        )),
    )
    .await;

    Ok(ApiResponse::success(updated))
}

#[utoipa::path(
    post,
    path = "/api/v1/variables/batch-delete",
    responses(
        (status = 200, description = "Batch delete variables")
    ),
    tag = "Variables",
    security(("session_cookie" = []))
)]
pub async fn batch_delete_variables(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<()>> {
    if payload.ids.is_empty() {
        return Ok(ApiResponse::success(()));
    }

    let action_message =
        service::batch_delete_variables(&state.db, &payload.ids, payload.task_id).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "批量删除变量",
        "variable",
        payload.task_id.map(|id| id.to_string()),
        Some(action_message),
    )
    .await;

    Ok(ApiResponse::success(()))
}
