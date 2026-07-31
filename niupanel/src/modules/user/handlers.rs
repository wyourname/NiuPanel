use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use argon2::{Argon2, PasswordHash, password_hash::PasswordVerifier};
use axum::{Extension, Json, extract::State};
use niupanel_common::auth::validate_password_strength;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_entity::users;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
// use std::net::SocketAddr;
use crate::modules::user::service::UserService;
use tower_sessions::Session;

// --- Unified User Management Handlers ---

#[derive(Serialize, ToSchema)]
pub struct UserProfileDto {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub role: String,
    pub permissions: Vec<String>,
    pub last_login_at: Option<i64>,
    pub last_login_ip: Option<String>,
    pub preferences: serde_json::Value,
}

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    responses(
        (status = 200, description = "Get current user profile", body = niupanel_common::response::ApiResponse<UserProfileDto>)
    ),
    tag = "User",
    security(
        ("session_cookie" = [])
    )
)]
pub async fn get_my_profile(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<UserProfileDto>> {
    let u = users::Entity::find_by_id(user.id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    Ok(ApiResponse::success(UserProfileDto {
        id: u.id,
        username: u.username,
        email: u.email,
        email_verified: u.email_verified,
        role: u.role,
        permissions: user.permissions.iter().map(|p| p.to_string()).collect(),
        last_login_at: u.last_login_at.map(|t| t.timestamp()),
        last_login_ip: u.last_login_ip,
        preferences: u.preferences.unwrap_or(serde_json::json!({})),
    }))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub password_confirm: String, // Modification requires current password
}

#[utoipa::path(
    put,
    path = "/api/v1/users/profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated successfully", body = niupanel_common::response::ApiResponse<Object>)
    ),
    tag = "User",
    security(
        ("session_cookie" = [])
    )
)]
pub async fn update_profile(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<ApiResponse<()>> {
    let u = users::Entity::find_by_id(user.id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    // Verify Password
    let parsed_hash = PasswordHash::new(&u.password_hash)
        .map_err(|e| AppError::Generic(format!("Invalid password hash: {}", e)))?;
    Argon2::default()
        .verify_password(payload.password_confirm.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Auth("Invalid password".to_string()))?;

    let mut active: users::ActiveModel = u.into();
    if let Some(new_name) = payload.username {
        let new_name = new_name.trim();
        if new_name.len() < 3 {
            return Err(AppError::ValidationError(
                "用户名长度必须至少为 3 个字符".into(),
            ));
        }
        active.username = Set(new_name.to_string());
    }
    active.update(&state.db).await?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "UPDATE_PROFILE",
        "user",
        Some(user.id.to_string()),
        Some("用户更新了个人基本资料".into()),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[derive(Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/users/change-password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Change password successfully")
    ),
    tag = "User",
    security(("session_cookie" = []))
)]
pub async fn change_password(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    session: Session,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<ApiResponse<()>> {
    let u = users::Entity::find_by_id(user.id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("User not found".into()))?;

    // Verify Old
    let parsed_hash = PasswordHash::new(&u.password_hash)
        .map_err(|e| AppError::Generic(format!("Invalid password hash: {}", e)))?;
    Argon2::default()
        .verify_password(payload.old_password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::Auth("Invalid old password".to_string()))?;

    validate_password_strength(&payload.new_password)?;

    UserService::update_password(&state.db, user.id, &payload.new_password).await?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "CHANGE_PASSWORD",
        "user",
        Some(user.id.to_string()),
        Some("用户修改了登录密码".into()),
        Some(ip),
    )
    .await;

    // Flush session to force logout
    session
        .flush()
        .await
        .map_err(|_| AppError::Auth("Failed to clear session".into()))?;

    Ok(ApiResponse::success(()))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePreferencesRequest {
    pub preferences: serde_json::Value,
}

#[utoipa::path(
    put,
    path = "/api/v1/users/preferences",
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "Update user preferences")
    ),
    tag = "User",
    security(("session_cookie" = []))
)]
pub async fn update_preferences(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<UpdatePreferencesRequest>,
) -> Result<ApiResponse<()>> {
    let mut active = users::ActiveModel {
        id: Set(user.id),
        ..Default::default()
    };
    active.preferences = Set(Some(payload.preferences));
    active.update(&state.db).await?;
    Ok(ApiResponse::success(()))
}

#[derive(Deserialize, ToSchema)]
#[allow(dead_code)]
pub struct EmailChangeRequest {
    pub new_email: String,
    pub password_confirm: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/users/email/verify-request",
    request_body = EmailChangeRequest,
    responses(
        (status = 200, description = "Request email change verification")
    ),
    tag = "User",
    security(("session_cookie" = []))
)]
pub async fn request_email_change(
    State(_state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Json(_payload): Json<EmailChangeRequest>,
) -> Result<ApiResponse<()>> {
    Err(AppError::Generic(
        "Email change verification not yet fully implemented".into(),
    ))
}

#[derive(Deserialize, ToSchema)]
#[allow(dead_code)]
pub struct ConfirmEmailChangeRequest {
    pub code: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/users/email/verify-confirm",
    request_body = ConfirmEmailChangeRequest,
    responses(
        (status = 200, description = "Confirm email change with verification code")
    ),
    tag = "User",
    security(("session_cookie" = []))
)]
pub async fn confirm_email_change(
    State(_state): State<AppState>,
    Extension(_user): Extension<AuthenticatedUser>,
    Json(_payload): Json<ConfirmEmailChangeRequest>,
) -> Result<ApiResponse<()>> {
    Err(AppError::Generic("Not implemented".into()))
}
