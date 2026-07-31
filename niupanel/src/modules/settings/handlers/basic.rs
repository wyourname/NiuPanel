use super::super::models::{
    GeneralSettingsRequest, SecuritySettingsRequest, SessionDto, SettingDto,
};
use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, Path, State},
};
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_core::settings::{
    AUTH_MAX_SESSIONS, NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR, SYSTEM_DEFAULT_NODE_VERSION,
    SYSTEM_DEFAULT_PYTHON_VERSION, SYSTEM_GITHUB_PROXY, SYSTEM_LOG_RETENTION,
    SYSTEM_MAX_CONCURRENCY, SYSTEM_ONBOARDING_DONE, SYSTEM_TIMEZONE, SYSTEM_UPDATE_CHANNEL,
    UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
};
use tower_sessions::Session;
// use axum::extract::ConnectInfo;
// use std::net::SocketAddr;

#[utoipa::path(
    get,
    path = "/api/v1/settings",
    responses(
        (status = 200, description = "Get all settings", body = ApiResponse<Vec<SettingDto>>)
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn get_settings(State(state): State<AppState>) -> Result<ApiResponse<Vec<SettingDto>>> {
    let settings = state.settings.get_all_public().await?;

    let mut dtos: Vec<_> = settings.into_iter().map(SettingDto::from).collect();
    if !dtos
        .iter()
        .any(|setting| setting.key == SYSTEM_UPDATE_CHANNEL)
    {
        dtos.push(SettingDto {
            key: SYSTEM_UPDATE_CHANNEL.to_string(),
            value: state
                .settings
                .get(SYSTEM_UPDATE_CHANNEL)
                .await
                .unwrap_or_else(|_| "stable".to_string()),
            category: "System".to_string(),
        });
    }
    Ok(ApiResponse::success(dtos))
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/sessions",
    responses(
        (status = 200, description = "Get all active sessions")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn get_active_sessions(
    State(state): State<AppState>,
    session: Session,
) -> Result<ApiResponse<Vec<SessionDto>>> {
    let current_id = session.id().map(|id| id.to_string()).unwrap_or_default();
    let now = chrono::Utc::now().timestamp();

    let session_list =
        crate::modules::auth::service::SessionService::get_all_active_sessions(&state.db).await?;
    let mut sessions = Vec::new();
    for (id, expiry) in session_list {
        if expiry > now {
            sessions.push(SessionDto {
                is_current: id == current_id,
                id,
                expiry,
            });
        }
    }

    sessions.sort_by(|a, b| {
        if a.is_current {
            std::cmp::Ordering::Less
        } else if b.is_current {
            std::cmp::Ordering::Greater
        } else {
            b.expiry.cmp(&a.expiry)
        }
    });

    Ok(ApiResponse::success(sessions))
}

#[utoipa::path(
    delete,
    path = "/api/v1/settings/sessions/{id}",
    params(
        ("id" = String, Path, description = "Session ID to delete")
    ),
    responses(
        (status = 200, description = "Delete a session")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn delete_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<ApiResponse<()>> {
    crate::modules::auth::service::SessionService::delete_session(&state.db, &id).await?;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    put,
    path = "/api/v1/settings/general",
    request_body = GeneralSettingsRequest,
    responses(
        (status = 200, description = "Update general settings")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn update_general_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<GeneralSettingsRequest>,
) -> Result<ApiResponse<()>> {
    state
        .settings
        .set("system.name", &req.name, Some("System"))
        .await?;
    state
        .settings
        .set("system.logo", &req.logo, Some("System"))
        .await?;
    state
        .settings
        .set(SYSTEM_TIMEZONE, &req.timezone, Some("System"))
        .await?;
    state
        .settings
        .set(
            SYSTEM_MAX_CONCURRENCY,
            &req.max_concurrency.to_string(),
            Some("System"),
        )
        .await?;
    state
        .settings
        .set(
            SYSTEM_LOG_RETENTION,
            &req.log_retention_days.to_string(),
            Some("System"),
        )
        .await?;
    state
        .settings
        .set(SYSTEM_GITHUB_PROXY, &req.github_proxy_url, Some("System"))
        .await?;
    state
        .settings
        .set(UV_PYTHON_MIRROR, &req.uv_python_mirror, Some("System"))
        .await?;
    state
        .settings
        .set(UV_PYPI_MIRROR, &req.uv_pypi_mirror, Some("System"))
        .await?;
    if let Some(pnpm_node_dist_mirror) = req.pnpm_node_dist_mirror.as_deref() {
        state
            .settings
            .set(PNPM_NODE_DIST_MIRROR, pnpm_node_dist_mirror, Some("System"))
            .await?;
    }
    if let Some(npm_registry_mirror) = req.npm_registry_mirror.as_deref() {
        state
            .settings
            .set(NPM_REGISTRY_MIRROR, npm_registry_mirror, Some("System"))
            .await?;
    }
    state
        .settings
        .set(
            SYSTEM_DEFAULT_PYTHON_VERSION,
            &req.default_python_version,
            Some("System"),
        )
        .await?;
    state
        .settings
        .set(
            SYSTEM_DEFAULT_NODE_VERSION,
            &req.default_node_version,
            Some("System"),
        )
        .await?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新常规设置",
        "settings",
        None,
        Some("更新了系统常规设置".to_string()),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    put,
    path = "/api/v1/settings/security",
    request_body = SecuritySettingsRequest,
    responses(
        (status = 200, description = "Update security settings")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn update_security_settings(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(req): Json<SecuritySettingsRequest>,
) -> Result<ApiResponse<()>> {
    state
        .settings
        .set(
            AUTH_MAX_SESSIONS,
            &req.max_sessions.to_string(),
            Some("Security"),
        )
        .await?;

    AuditService::log(
        &state.db,
        Some(user.id),
        "User",
        "更新安全设置",
        "settings",
        None,
        Some("更新了系统安全设置".to_string()),
        Some(ip),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/onboarding",
    responses(
        (status = 200, description = "Get onboarding status")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn get_onboarding_status(State(state): State<AppState>) -> Result<ApiResponse<bool>> {
    let done = state
        .settings
        .get(SYSTEM_ONBOARDING_DONE)
        .await
        .unwrap_or_default();

    if done == "true" {
        return Ok(ApiResponse::success(true));
    }

    // 兼容性检查：如果已经存在任意 Python 或 Node 环境目录，说明是老用户，自动跳过引导
    let runtimes_dir = &niupanel_common::config::Config::global().runtimes_dir;
    let python_dir = std::path::PathBuf::from(runtimes_dir).join("python");
    let node_dir = std::path::PathBuf::from(runtimes_dir).join("node");

    let has_existing = tokio::task::spawn_blocking(move || {
        let has_python = python_dir.exists()
            && std::fs::read_dir(&python_dir)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false);

        let has_node = node_dir.exists()
            && std::fs::read_dir(&node_dir)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false);

        // User wants onboarding to trigger if EITHER python or node is empty.
        // So we only skip onboarding if BOTH have environments.
        has_python && has_node
    })
    .await
    .unwrap_or(false);

    if has_existing {
        // 静默标记为完成，避免老用户每次登录都触发检查
        let _ = state
            .settings
            .set(SYSTEM_ONBOARDING_DONE, "true", Some("System"))
            .await;
        return Ok(ApiResponse::success(true));
    }

    Ok(ApiResponse::success(false))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/onboarding/complete",
    responses(
        (status = 200, description = "Mark onboarding as complete")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn complete_onboarding(State(state): State<AppState>) -> Result<ApiResponse<()>> {
    state
        .settings
        .set(SYSTEM_ONBOARDING_DONE, "true", Some("System"))
        .await?;
    Ok(ApiResponse::success(()))
}
