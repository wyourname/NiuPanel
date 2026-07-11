use super::super::models::{NotificationSettingsRequest, TestNotifyRequest};
use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, State},
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_core::notification::service::NotificationService;
use niupanel_core::settings::{
    MAIL_HOST, MAIL_PASSWORD, MAIL_TO, MAIL_USERNAME, NOTIFY_EVENTS, NOTIFY_WEBHOOK_URL,
};

#[utoipa::path(
    put,
    path = "/api/v1/settings/notification",
    request_body = NotificationSettingsRequest,
    responses(
        (status = 200, description = "Update notification settings")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn update_notification_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<NotificationSettingsRequest>,
) -> Result<ApiResponse<()>> {
    state
        .settings
        .set(NOTIFY_WEBHOOK_URL, &req.webhook_url, Some("Notification"))
        .await?;
    state
        .settings
        .set(NOTIFY_EVENTS, &req.events, Some("Notification"))
        .await?;
    state
        .settings
        .set(MAIL_HOST, &req.mail_host, Some("Email"))
        .await?;
    state
        .settings
        .set(MAIL_USERNAME, &req.mail_username, Some("Email"))
        .await?;
    state
        .settings
        .set(MAIL_TO, &req.mail_to, Some("Email"))
        .await?;

    if !req.mail_password.is_empty() {
        state
            .settings
            .set(MAIL_PASSWORD, &req.mail_password, Some("Email"))
            .await?;
    }

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新通知设置",
        "settings",
        None,
        Some("更新了通知设置 (Webhook/邮件)".to_string()),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/notification/test",
    request_body = TestNotifyRequest,
    responses(
        (status = 200, description = "Send test notification")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn test_notification(
    State(state): State<AppState>,
    Json(payload): Json<TestNotifyRequest>,
) -> Result<ApiResponse<()>> {
    match payload.notify_type.as_str() {
        "webhook" => {
            let url = if let Some(u) = payload.webhook_url {
                if u.trim().is_empty() {
                    state.settings.get(NOTIFY_WEBHOOK_URL).await?
                } else {
                    u
                }
            } else {
                state.settings.get(NOTIFY_WEBHOOK_URL).await?
            };

            if url.trim().is_empty() {
                return Err(AppError::ValidationError(
                    "Webhook URL is missing".to_string(),
                ));
            }

            NotificationService::send_webhook(
                &state.http_client,
                &url,
                "Notification Test",
                "This is a test notification from NiuPanel.",
            )
            .await?;
        }
        "mail" => {
            let host = if let Some(h) = payload.mail_host {
                if h.trim().is_empty() {
                    state.settings.get(MAIL_HOST).await?
                } else {
                    h
                }
            } else {
                state.settings.get(MAIL_HOST).await?
            };
            let user = if let Some(u) = payload.mail_username {
                if u.trim().is_empty() {
                    state.settings.get(MAIL_USERNAME).await?
                } else {
                    u
                }
            } else {
                state.settings.get(MAIL_USERNAME).await?
            };
            let pass = if let Some(p) = payload.mail_password {
                if p.is_empty() {
                    state.settings.get(MAIL_PASSWORD).await?
                } else {
                    p
                }
            } else {
                state.settings.get(MAIL_PASSWORD).await?
            };
            let to = if let Some(t) = payload.mail_to {
                if t.trim().is_empty() {
                    state.settings.get(MAIL_TO).await?
                } else {
                    t
                }
            } else {
                state.settings.get(MAIL_TO).await?
            };

            if host.trim().is_empty() {
                return Err(AppError::ValidationError("SMTP host missing".to_string()));
            }
            if to.trim().is_empty() {
                return Err(AppError::ValidationError(
                    "Recipient email (Mail To) is missing".to_string(),
                ));
            }

            NotificationService::send_email(
                &host,
                &user,
                &pass,
                &to,
                "Notification Test",
                "This is a test email from NiuPanel.",
            )
            .await?;
        }
        _ => return Err(AppError::Generic("Invalid notification type".to_string())),
    }
    Ok(ApiResponse::success(()))
}
