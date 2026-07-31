use crate::common::state::AppState;
use axum::{Json, extract::State};
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;
use niupanel_core::event_bus::{SystemEvent, SystemNotification};
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct WebhookPushRequest {
    pub title: String,
    pub content: String,
    pub level: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/webhook/push",
    request_body = WebhookPushRequest,
    responses(
        (status = 200, description = "Send webhook notification")
    ),
    tag = "Webhook",
    security(("session_cookie" = []))
)]
pub async fn send_notification(
    State(state): State<AppState>,
    Json(payload): Json<WebhookPushRequest>,
) -> Result<ApiResponse<()>> {
    state
        .event_bus
        .publish(SystemEvent::System(SystemNotification::Notification {
            title: payload.title,
            content: payload.content,
            level: payload.level.unwrap_or_else(|| "info".to_string()),
        }));

    Ok(ApiResponse::success(()))
}
