use crate::common::extractors::RealIp;
use crate::common::state::AppState;
pub use crate::modules::auth::models::{
    ForgotPasswordRequest, IdentifyRequest, LoginRequest, LoginResponse, RegisterRequest,
    ResetInfoResponse, ResetPasswordRequest, SetupStatus, UserInfo, VerifyCodeRequest,
    VerifyLogin2faRequest,
};
use crate::modules::auth::service::{AuthService, SessionService, USER_ID_KEY, USER_ROLE_KEY};
use crate::modules::user::service::UserService;
use axum::{Json, extract::State};
use chrono::Utc;
use niupanel_common::auth::permissions::UserRole;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_core::settings::{AUTH_MAX_SESSIONS, SettingsManager};
use niupanel_entity::users;
use sea_orm::EntityTrait;
use std::str::FromStr;
use tower_sessions::Session;

#[utoipa::path(
    post,
    path = "/api/v1/auth/identify-reset",
    request_body = IdentifyRequest,
    responses(
        (status = 200, description = "Identify user for password reset")
    ),
    tag = "Auth"
)]
pub async fn identify_reset(
    State(state): State<AppState>,
    Json(payload): Json<IdentifyRequest>,
) -> Result<ApiResponse<ResetInfoResponse>> {
    let info = AuthService::identify_reset(&state.db, payload).await?;
    Ok(ApiResponse::success(info))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, description = "Send password reset code to email")
    ),
    tag = "Auth"
)]
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<ApiResponse<()>> {
    AuthService::forgot_password(&state.db, payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-reset-code",
    request_body = VerifyCodeRequest,
    responses(
        (status = 200, description = "Verify reset code and get token")
    ),
    tag = "Auth"
)]
pub async fn verify_reset_code(
    State(state): State<AppState>,
    RealIp(ip): RealIp,
    Json(payload): Json<VerifyCodeRequest>,
) -> Result<ApiResponse<String>> {
    let token = AuthService::verify_reset_code(&state, &ip, payload).await?;
    Ok(ApiResponse::success(token))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 200, description = "Reset password with token")
    ),
    tag = "Auth"
)]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<ApiResponse<()>> {
    AuthService::reset_password(&state.db, payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = niupanel_common::response::ApiResponse<LoginResponse>),
        (status = 401, description = "Invalid credentials", body = niupanel_common::response::ApiResponse<Object>)
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    session: Session,
    RealIp(ip): RealIp,
    Json(payload): Json<LoginRequest>,
) -> Result<ApiResponse<LoginResponse>> {
    AuthService::check_rate_limit(&state, &ip, &payload.username).await?;

    let (user_id, role) =
        match AuthService::verify_user_credentials(&state.db, &payload.username, &payload.password)
            .await?
        {
            Some((id, r)) => {
                AuthService::record_login_attempt(&state, &ip, &payload.username, true).await?;
                (id, r)
            }
            None => {
                AuthService::record_login_attempt(&state, &ip, &payload.username, false).await?;

                AuditService::log(
                    &state.db,
                    None,
                    "User",
                    "LOGIN_FAILED",
                    "auth",
                    None,
                    Some(format!("用户 '{}' 登录失败", payload.username)),
                    Some(ip.clone()),
                )
                .await;

                AuthService::send_notification(
                    state.db.clone(),
                    state.event_bus.clone(),
                    "登录失败通知".into(),
                    format!("有人尝试使用'{}'在{}登录失败", payload.username, ip),
                    "warning",
                );

                return Err(AppError::Auth("用户名或密码错误".to_string()));
            }
        };

    let config_str = SettingsManager::get(&state.db, "plugin.telegram.config")
        .await
        .unwrap_or_default();
    if !config_str.is_empty() {
        if let Ok(config) =
            serde_json::from_str::<niupanel_bot::telegram::TelegramBotConfig>(&config_str)
        {
            if config.enabled && config.login_2fa {
                let ticket = nanoid::nanoid!(32);
                let code: String = (0..6).map(|_| fastrand::digit(10)).collect();

                state
                    .login_2fa_cache
                    .insert(ticket.clone(), (code.clone(), user_id))
                    .await;

                state
                    .event_bus
                    .publish(niupanel_core::event_bus::SystemEvent::Auth(
                        niupanel_core::event_bus::AuthEvent::LoginOtpRequest {
                            username: payload.username.clone(),
                            ip: ip.clone(),
                            code,
                            timestamp: Utc::now().timestamp() as u64,
                        },
                    ));

                AuditService::log(
                    &state.db,
                    Some(user_id),
                    "User",
                    "LOGIN_2FA_WAITING",
                    "auth",
                    None,
                    Some(format!("用户 '{}' 正在等待 2FA 验证", payload.username)),
                    Some(ip.clone()),
                )
                .await;

                return Ok(ApiResponse::success(LoginResponse::Requires2FA { ticket }));
            }
        }
    }

    let role_enum = UserRole::from_str(&role).unwrap_or(UserRole::User);
    let _ = UserService::record_login(&state.db, user_id, ip.clone()).await;

    let max_sessions_str = SettingsManager::get(&state.db, AUTH_MAX_SESSIONS).await?;
    let max_sessions = max_sessions_str.parse::<usize>().unwrap_or(2);
    if max_sessions > 0 {
        SessionService::cleanup_user_sessions(&state.db, user_id, max_sessions).await?;
    }

    session
        .flush()
        .await
        .map_err(|_| AppError::Auth("无法初始化会话".into()))?;
    session
        .insert(USER_ID_KEY, user_id)
        .await
        .map_err(|e| AppError::Auth(format!("登录失败: {}", e)))?;
    let _ = session
        .insert(USER_ROLE_KEY, role_enum.as_ref().to_string())
        .await;

    AuthService::send_notification(
        state.db.clone(),
        state.event_bus.clone(),
        "登录成功通知".into(),
        format!("用户'{}'在{}登录成功", payload.username, ip),
        "info",
    );

    AuditService::log(
        &state.db,
        Some(user_id),
        "User",
        "登录",
        "auth",
        None,
        Some(format!("用户 {} 登录成功", payload.username)),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(LoginResponse::Success(UserInfo {
        id: user_id,
        username: payload.username,
        role: role_enum.as_ref().to_string(),
    })))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-2fa",
    responses(
        (status = 200, description = "Verify 2FA code for login")
    ),
    tag = "Auth"
)]
pub async fn verify_login_2fa(
    State(state): State<AppState>,
    session: Session,
    RealIp(ip): RealIp,
    Json(payload): Json<VerifyLogin2faRequest>,
) -> Result<ApiResponse<UserInfo>> {
    if let Some((stored_code, user_id)) = state.login_2fa_cache.get(&payload.ticket).await {
        if stored_code == payload.code.trim() {
            state.login_2fa_cache.invalidate(&payload.ticket).await;

            let user = users::Entity::find_by_id(user_id)
                .one(&state.db)
                .await?
                .ok_or_else(|| AppError::Auth("用户不存在".to_string()))?;

            let role_enum = UserRole::from_str(&user.role).unwrap_or(UserRole::User);
            let _ = UserService::record_login(&state.db, user.id, ip.clone()).await;

            let max_sessions_str = SettingsManager::get(&state.db, AUTH_MAX_SESSIONS).await?;
            let max_sessions = max_sessions_str.parse::<usize>().unwrap_or(2);
            if max_sessions > 0 {
                SessionService::cleanup_user_sessions(&state.db, user.id, max_sessions).await?;
            }

            session
                .flush()
                .await
                .map_err(|_| AppError::Auth("无法初始化会话".into()))?;
            session
                .insert(USER_ID_KEY, user.id)
                .await
                .map_err(|e| AppError::Auth(format!("登录失败: {}", e)))?;
            let _ = session
                .insert(USER_ROLE_KEY, role_enum.as_ref().to_string())
                .await;

            AuthService::send_notification(
                state.db.clone(),
                state.event_bus.clone(),
                "二次验证登录成功".into(),
                format!("用户'{}'在{}完成二次验证并登录成功", user.username, ip),
                "info",
            );

            AuditService::log(
                &state.db,
                Some(user.id),
                "User",
                "登录",
                "auth",
                None,
                Some(format!("用户 {} 二次验证成功登录", user.username)),
                Some(ip),
            )
            .await;

            return Ok(ApiResponse::success(UserInfo {
                id: user.id,
                username: user.username,
                role: role_enum.as_ref().to_string(),
            }));
        } else {
            return Err(AppError::Auth("验证码不正确".to_string()));
        }
    }

    Err(AppError::Auth(
        "验证凭证无效或已过期，请重新登录".to_string(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/setup-status",
    responses(
        (status = 200, description = "Check if initial setup is needed")
    ),
    tag = "Auth"
)]
pub async fn get_setup_status(State(state): State<AppState>) -> Result<ApiResponse<SetupStatus>> {
    let status = AuthService::get_setup_status(&state.db).await?;
    Ok(ApiResponse::success(status))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register-admin",
    responses(
        (status = 200, description = "Register initial admin account")
    ),
    tag = "Auth"
)]
pub async fn register_admin(
    State(state): State<AppState>,
    RealIp(ip): RealIp,
    Json(payload): Json<RegisterRequest>,
) -> Result<ApiResponse<()>> {
    AuthService::register_admin(&state.db, ip, payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logout successful", body = niupanel_common::response::ApiResponse<Object>)
    ),
    tag = "Auth"
)]
pub async fn logout(
    State(state): State<AppState>,
    session: Session,
    RealIp(ip): RealIp,
) -> Result<ApiResponse<()>> {
    let user_id: Option<i32> = session.get(USER_ID_KEY).await.unwrap_or(None);

    if let Some(uid) = user_id {
        AuditService::log(
            &state.db,
            Some(uid),
            "User",
            "LOGOUT",
            "auth",
            None,
            Some("用户主动退出登录".to_string()),
            Some(ip),
        )
        .await;
    }

    if session.id().is_some() {
        let _ = session.delete().await;
    } else {
        tracing::warn!("Session ID is None, skipping session deletion.");
    }

    session
        .flush()
        .await
        .map_err(|e| AppError::Auth(format!("退出登录失败: {}", e)))?;

    Ok(ApiResponse::success(()))
}
