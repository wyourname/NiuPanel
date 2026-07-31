use super::models::*;
use super::service::TaskService;
use super::usecase::TaskUseCase;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    response::{
        IntoResponse,
        sse::{Event, Sse},
    },
};
use futures::stream::Stream;
use niupanel_common::error::Result;
use niupanel_common::response::{ApiResponse, PaginatedData};
use niupanel_entity::{task_runs, tasks};
use std::convert::Infallible;

fn no_cache_headers() -> axum::http::HeaderMap {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    headers.insert(
        axum::http::header::PRAGMA,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    headers.insert(
        axum::http::header::EXPIRES,
        axum::http::HeaderValue::from_static("0"),
    );
    headers
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks",
    params(QueryParams),
    responses(
        (status = 200, description = "List tasks successfully", body = ApiResponse<PaginatedData<niupanel_entity::tasks::Model>>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<impl IntoResponse> {
    let tasks = TaskService::list_tasks(&state.db, params).await?;
    Ok((no_cache_headers(), ApiResponse::success(tasks)))
}

#[utoipa::path(
    put,
    path = "/api/v1/tasks/pagination",
    request_body = PaginationSettingRequest,
    responses(
        (status = 200, description = "Save task pagination setting")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn save_pagination_setting(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<PaginationSettingRequest>,
) -> Result<ApiResponse<()>> {
    TaskService::save_pagination_setting(&state.db, user.id, payload.page_size).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/pagination",
    responses(
        (status = 200, description = "Get task pagination setting")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn get_pagination_setting(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<Option<u64>>> {
    let size = TaskService::get_pagination_setting(&state.db, user.id).await?;
    Ok(ApiResponse::success(size))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/delete",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch delete tasks successfully")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_delete_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<DeleteTaskParams>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<()>> {
    TaskUseCase::from_state(&state)
        .batch_delete_tasks(
            &user,
            payload.ids,
            params.delete_script.unwrap_or(false),
            params.delete_var.unwrap_or(false),
        )
        .await?;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/pin",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch pin tasks")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_pin_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<()>> {
    TaskUseCase::from_state(&state)
        .batch_set_pinned(&user, payload.ids, true)
        .await?;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/unpin",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch unpin tasks")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_unpin_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<()>> {
    TaskUseCase::from_state(&state)
        .batch_set_pinned(&user, payload.ids, false)
        .await?;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/run",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch run tasks successfully")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_run_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<Vec<serde_json::Value>>> {
    let results = TaskUseCase::from_state(&state)
        .batch_run_tasks(&user, payload.ids)
        .await?;

    Ok(ApiResponse::success(results))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/stop",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch stop tasks successfully")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_stop_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<Vec<serde_json::Value>>> {
    let results = TaskUseCase::from_state(&state)
        .batch_stop_tasks(&user, payload.ids)
        .await?;

    Ok(ApiResponse::success(results))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/pause",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch pause tasks")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_pause_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<Vec<serde_json::Value>>> {
    let results = TaskUseCase::from_state(&state)
        .batch_pause_tasks(&user, payload.ids)
        .await?;

    Ok(ApiResponse::success(results))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/resume",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch resume tasks")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_resume_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<Vec<serde_json::Value>>> {
    let results = TaskUseCase::from_state(&state)
        .batch_resume_tasks(&user, payload.ids)
        .await?;

    Ok(ApiResponse::success(results))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/enable",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch enable tasks successfully")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_enable_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<()>> {
    TaskUseCase::from_state(&state)
        .batch_set_enabled(&user, payload.ids, true)
        .await?;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/batch/disable",
    request_body = BatchIdRequest,
    responses(
        (status = 200, description = "Batch disable tasks successfully")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn batch_disable_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<BatchIdRequest>,
) -> Result<ApiResponse<()>> {
    TaskUseCase::from_state(&state)
        .batch_set_enabled(&user, payload.ids, false)
        .await?;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks",
    request_body = CreateTaskRequest,
    responses(
        (status = 200, description = "Task created successfully", body = ApiResponse<niupanel_entity::tasks::Model>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn create_task(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<ApiResponse<tasks::Model>> {
    let task = TaskUseCase::from_state(&state)
        .create_task(&user, payload)
        .await?;

    Ok(ApiResponse::success(task))
}

#[utoipa::path(
    put,
    path = "/api/v1/tasks/{id}",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    request_body = UpdateTaskRequest,
    responses(
        (status = 200, description = "Update task successfully", body = ApiResponse<niupanel_entity::tasks::Model>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn update_task(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<ApiResponse<tasks::Model>> {
    let task = TaskUseCase::from_state(&state)
        .update_task(&user, id, payload)
        .await?;

    Ok(ApiResponse::success(task))
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}/logs/stream",
    params(
        ("id" = i32, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Stream task logs via SSE", content_type = "text/event-stream")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn stream_logs(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let rx_result = state.task_manager.subscribe_logs(id, false);
    let history_result = state
        .task_manager
        .get_latest_log_content(id, false, Some(512 * 1024))
        .await;

    let stream = async_stream::stream! {
        let mut last_yielded_offset: Option<u64> = None;

        // 1. Yield History
        if let Ok((history, offset)) = history_result {
             if !history.is_empty() {
                last_yielded_offset = Some(offset);
                yield Ok(Event::default().event("history").data(history));
             }
        }

        let mut rx = match rx_result {
            Ok(r) => r,
            Err(_) => {
                yield Ok(Event::default().event("log").data(serde_json::json!("等待任务输出...").to_string()));
                return;
            }
        };

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let should_yield = match last_yielded_offset {
                        Some(last) => event.offset > last,
                        None => true,
                    };

                    if should_yield {
                        yield Ok(Event::default().event("log").data(serde_json::to_string(&event).unwrap_or_else(|_| serde_json::json!({ "content": event.content }).to_string())));
                        last_yielded_offset = Some(event.offset);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    yield Ok(Event::default().event("log").data(serde_json::json!({ "kind": "system", "content": format!("[系统] 缓冲区溢出，跳过了 {} 条日志.", skipped) }).to_string()));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}/runs/{run_id}/logs/stream",
    params(
        ("id" = i32, Path, description = "Task ID"),
        ("run_id" = i32, Path, description = "Task run ID")
    ),
    responses(
        (status = 200, description = "Stream task run logs via SSE", content_type = "text/event-stream")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn stream_task_run_logs(
    State(state): State<AppState>,
    Path((id, run_id)): Path<(i32, i32)>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let rx_result = state.task_manager.subscribe_task_run_logs(id, run_id);
    let history_result = TaskService::get_task_run_log(
        &state.db,
        id,
        run_id,
        LogPagination {
            offset: None,
            limit: Some(512 * 1024),
        },
    )
    .await;

    let stream = async_stream::stream! {
        let mut last_yielded_offset: Option<u64> = None;

        if let Ok(history) = history_result {
            if !history.content.is_empty() {
                last_yielded_offset = Some(history.total_size);
                yield Ok(Event::default().event("history").data(history.content));
            }
        }

        let mut rx = match rx_result {
            Ok(r) => r,
            Err(_) => return,
        };

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let should_yield = match last_yielded_offset {
                        Some(last) => event.offset > last,
                        None => true,
                    };

                    if should_yield {
                        yield Ok(Event::default().event("log").data(serde_json::to_string(&event).unwrap_or_else(|_| serde_json::json!({ "content": event.content }).to_string())));
                        last_yielded_offset = Some(event.offset);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    yield Ok(Event::default().event("log").data(serde_json::json!({ "kind": "system", "content": format!("[系统] 缓冲区溢出，跳过了 {} 条日志.", skipped) }).to_string()));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}/logs/latest",
    params(
        ("id" = i32, Path, description = "Task ID"),
        ("offset" = Option<u64>, Query, description = "Log offset"),
        ("limit" = Option<u64>, Query, description = "Log limit")
    ),
    responses(
        (status = 200, description = "Get latest task log", body = ApiResponse<LogResponse>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn get_latest_log(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(pagination): Query<LogPagination>,
) -> Result<impl IntoResponse> {
    let log = TaskService::get_latest_log(&state.task_manager, id, pagination).await?;
    Ok((no_cache_headers(), ApiResponse::success(log)))
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}/runs/{run_id}/log",
    params(
        ("id" = i32, Path, description = "Task ID"),
        ("run_id" = i32, Path, description = "Task run ID")
    ),
    responses(
        (status = 200, description = "Get task run log", body = ApiResponse<LogResponse>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn get_task_run_log(
    State(state): State<AppState>,
    Path((id, run_id)): Path<(i32, i32)>,
    Query(pagination): Query<LogPagination>,
) -> Result<impl IntoResponse> {
    let log = TaskService::get_task_run_log(&state.db, id, run_id, pagination).await?;
    Ok((no_cache_headers(), ApiResponse::success(log)))
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}/history",
    params(
        ("id" = i32, Path, description = "Task ID"),
        ("page" = Option<u64>, Query, description = "Page number"),
        ("page_size" = Option<u64>, Query, description = "Page size")
    ),
    responses(
        (status = 200, description = "List task run history", body = ApiResponse<PaginatedData<niupanel_entity::task_runs::Model>>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn list_task_history(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<QueryParams>,
) -> Result<ApiResponse<PaginatedData<task_runs::Model>>> {
    let history = TaskService::list_task_history(&state.db, id, params).await?;
    Ok(ApiResponse::success(history))
}

#[utoipa::path(
    delete,
    path = "/api/v1/tasks/{id}/runs/{run_id}",
    params(
        ("id" = i32, Path, description = "Task ID"),
        ("run_id" = i32, Path, description = "Task run ID")
    ),
    responses(
        (status = 200, description = "Delete task run record")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn delete_task_run(
    State(state): State<AppState>,
    Path((id, run_id)): Path<(i32, i32)>,
) -> Result<ApiResponse<()>> {
    TaskService::delete_task_run(&state.db, id, run_id).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/status/stream",
    responses(
        (status = 200, description = "Stream task status changes via SSE", content_type = "text/event-stream")
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn stream_status(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let mut rx = state.task_manager.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        yield Ok(Event::default().data(json));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

#[utoipa::path(
    post,
    path = "/api/v1/tasks/quick-create-url",
    request_body = QuickCreateUrlRequest,
    responses(
        (status = 200, description = "Quick create task from URL", body = ApiResponse<niupanel_entity::tasks::Model>)
    ),
    tag = "Tasks",
    security(("session_cookie" = []))
)]
pub async fn quick_create_task_from_url(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<QuickCreateUrlRequest>,
) -> Result<ApiResponse<tasks::Model>> {
    let task = TaskUseCase::from_state(&state)
        .quick_create_task_from_url(&user, payload)
        .await?;

    Ok(ApiResponse::success(task))
}
