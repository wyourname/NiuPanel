use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use sea_orm::{EntityTrait, QueryOrder};
use std::convert::Infallible;
use tracing::info;

use crate::common::state::AppState;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_entity::system_jobs;

#[utoipa::path(
    get,
    path = "/api/v1/jobs",
    responses(
        (status = 200, description = "List system jobs", body = ApiResponse<Vec<niupanel_entity::system_jobs::Model>>)
    ),
    tag = "Jobs",
    security(("session_cookie" = []))
)]
pub async fn list_jobs(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<system_jobs::Model>>> {
    let jobs = system_jobs::Entity::find()
        .order_by_desc(system_jobs::Column::Id)
        .all(&state.db)
        .await?;

    Ok(ApiResponse::success(jobs))
}

#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}",
    params(
        ("id" = i32, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Get system job details")
    ),
    tag = "Jobs",
    security(("session_cookie" = []))
)]
pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<system_jobs::Model>> {
    let job = system_jobs::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("System job not found".to_string()))?;

    Ok(ApiResponse::success(job))
}

#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}/logs/stream",
    params(
        ("id" = i32, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Stream job logs via SSE", content_type = "text/event-stream")
    ),
    tag = "Jobs",
    security(("session_cookie" = []))
)]
pub async fn stream_job_logs(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    // 1. Subscribe first to ensure we don't miss any events while reading history
    // is_system = true
    let rx_result = state.task_manager.subscribe_logs(id, true);

    // 2. Get history and current file size
    // Increased history limit to 512KB for a better initial view
    let history_result = state
        .task_manager
        .get_latest_log_content(id, true, Some(512 * 1024))
        .await;

    let stream = async_stream::stream! {
        let mut current_offset = 0;

        // Yield History
        if let Ok((history, offset)) = history_result {
            current_offset = offset;
            if !history.is_empty() {
                yield Ok(Event::default().event("history").data(history));
            }
        }

        // Yield Live Updates
        match rx_result {
            Ok(mut rx) => {
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if event.offset > current_offset {
                                yield Ok(Event::default().event("log").data(serde_json::to_string(&event).unwrap_or_else(|_| serde_json::json!({ "content": event.content }).to_string())));
                                current_offset = event.offset;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            yield Ok(Event::default().event("log").data(serde_json::json!({ "kind": "system", "content": format!("[系统] 缓冲区溢出，跳过了 {} 条日志.", skipped) }).to_string()));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
            Err(_) => {
                // Job might be finished, or not running.
                // If we sent history, that's good. If not, and no stream, meaningful info is needed.
                 yield Ok(Event::default().event("log").data(serde_json::json!({ "kind": "system", "content": "[Info] Stream ended or not running." }).to_string()));
            }
        }
    };
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}/logs/latest",
    params(
        ("id" = i32, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Get latest job log content")
    ),
    tag = "Jobs",
    security(("session_cookie" = []))
)]
pub async fn get_latest_job_log(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<String>> {
    // is_system = true
    let (logs, _) = state
        .task_manager
        .get_latest_log_content(id, true, Some(1024 * 1024))
        .await?;

    Ok(ApiResponse::success(logs))
}

#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/cancel",
    params(
        ("id" = i32, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Cancel a system job")
    ),
    tag = "Jobs",
    security(("session_cookie" = []))
)]
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<()>> {
    info!("Cancel job: {}", id);
    state.task_manager.cancel_system_job(id).await?;

    Ok(ApiResponse::success(()))
}
