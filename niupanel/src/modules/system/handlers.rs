use super::{
    models::{SystemHealth, SystemVersionInfo},
    panel_releases, service,
};
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::Json;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderValue, header};
use axum::response::{Html, IntoResponse, Response};
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;
use niupanel_common::version::{API_CONTRACT_VERSION, SCHEMA_EPOCH, SCHEMA_REVISION};
use niupanel_core::audit::service::AuditService;

pub async fn healthz() -> Json<SystemHealth> {
    let core_version = std::env::var("NIUPANEL_ACTIVE_CORE_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    Json(SystemHealth {
        status: "ok",
        panel_version: std::env::var("NIUPANEL_ACTIVE_PANEL_VERSION")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| core_version.clone()),
        core_version,
        api_contract: API_CONTRACT_VERSION,
        schema_epoch: SCHEMA_EPOCH,
        schema_revision: SCHEMA_REVISION,
    })
}

pub async fn recovery() -> Response {
    let mut response = Html(include_str!("recovery.html")).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; form-action 'self'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response
}

#[utoipa::path(
    get,
    path = "/api/v1/system/meta",
    responses((status = 200, description = "Get Panel runtime metadata")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn get_system_meta() -> Result<ApiResponse<SystemVersionInfo>> {
    Ok(ApiResponse::success(service::version_info()))
}

#[utoipa::path(
    get,
    path = "/api/v1/system/releases",
    responses((status = 200, description = "List installed Panel releases")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn list_panel_releases() -> Result<ApiResponse<panel_releases::PanelReleaseList>> {
    Ok(ApiResponse::success(panel_releases::list_releases().await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/system/releases/{version}/rollback",
    params(("version" = String, Path, description = "Panel release version")),
    request_body = panel_releases::RollbackPanelReleaseRequest,
    responses((status = 200, description = "Schedule an installed Panel release rollback")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn rollback_panel_release(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(version): Path<String>,
    Json(request): Json<panel_releases::RollbackPanelReleaseRequest>,
) -> Result<ApiResponse<panel_releases::PanelReleaseMutation>> {
    let mutation = panel_releases::rollback_release(&version, request.confirm_data_loss).await?;
    AuditService::log_user(
        &state.db,
        &user,
        "回退 Panel 版本",
        "system_release",
        Some(version),
        Some(format!(
            "transaction={}, restore_database={}",
            mutation.transaction_id, mutation.database_restore_required
        )),
    )
    .await;
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
        std::process::exit(0);
    });
    Ok(ApiResponse::success(mutation))
}
