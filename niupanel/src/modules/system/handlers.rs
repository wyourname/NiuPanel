use super::{
    core_releases,
    models::{SystemHealth, SystemVersionInfo},
    service, web_releases,
};
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::Json;
use axum::extract::{Extension, Multipart, Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_common::version::{API_CONTRACT_VERSION, SCHEMA_EPOCH, SCHEMA_REVISION};
use niupanel_core::audit::service::AuditService;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct InstallWebReleaseQuery {
    #[serde(default = "default_activate")]
    activate: bool,
}

fn default_activate() -> bool {
    true
}

pub async fn healthz() -> Json<SystemHealth> {
    Json(SystemHealth {
        status: "ok",
        core_version: env!("CARGO_PKG_VERSION").to_string(),
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
    responses((status = 200, description = "Get Core, Web, API and schema version contract")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn get_system_meta() -> Result<ApiResponse<SystemVersionInfo>> {
    Ok(ApiResponse::success(service::version_info()))
}

#[utoipa::path(
    get,
    path = "/api/v1/system/web/releases",
    responses((status = 200, description = "List installed Web UI releases")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn list_web_releases() -> Result<ApiResponse<web_releases::WebReleaseList>> {
    Ok(ApiResponse::success(web_releases::list_releases()?))
}

#[utoipa::path(
    post,
    path = "/api/v1/system/web/releases/upload",
    responses((status = 200, description = "Install a Web UI release package")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn upload_web_release(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(options): Query<InstallWebReleaseQuery>,
    mut multipart: Multipart,
) -> Result<ApiResponse<web_releases::WebReleaseMutation>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::ValidationError(error.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|error| AppError::ValidationError(error.to_string()))?;
        let temp = tempfile::Builder::new()
            .prefix("niupanel-web-release-")
            .suffix(".tar.gz")
            .tempfile()
            .map_err(AppError::Io)?;
        std::fs::write(temp.path(), bytes).map_err(AppError::Io)?;
        let path = temp.into_temp_path();
        let mutation = tokio::task::spawn_blocking(move || {
            web_releases::install_release(&path, options.activate)
        })
        .await
        .map_err(|error| AppError::Internal(format!("Web release worker failed: {error}")))??;
        AuditService::log_user(
            &state.db,
            &user,
            "安装 Web UI 版本",
            "system_release",
            Some(mutation.version.clone()),
            Some(format!("active={}", mutation.active)),
        )
        .await;
        return Ok(ApiResponse::success(mutation));
    }
    Err(AppError::ValidationError(
        "No Web release package was provided".to_string(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/system/web/releases/{version}/activate",
    params(("version" = String, Path, description = "Web release version")),
    responses((status = 200, description = "Activate an installed Web UI release")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn activate_web_release(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(version): Path<String>,
) -> Result<ApiResponse<web_releases::WebReleaseMutation>> {
    let mutation = web_releases::activate_release(&version)?;
    AuditService::log_user(
        &state.db,
        &user,
        "切换 Web UI 版本",
        "system_release",
        Some(version),
        None,
    )
    .await;
    Ok(ApiResponse::success(mutation))
}

#[utoipa::path(
    post,
    path = "/api/v1/system/web/rollback",
    responses((status = 200, description = "Switch back to the previous Web UI release")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn rollback_web_release(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<web_releases::WebReleaseMutation>> {
    let mutation = web_releases::rollback_release()?;
    AuditService::log_user(
        &state.db,
        &user,
        "回退 Web UI 版本",
        "system_release",
        Some(mutation.version.clone()),
        None,
    )
    .await;
    Ok(ApiResponse::success(mutation))
}

#[utoipa::path(
    get,
    path = "/api/v1/system/web/update/check",
    responses((status = 200, description = "Check the selected release channel for a Web UI update")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn check_web_update(State(state): State<AppState>) -> Result<Response> {
    let current = web_releases::list_releases()?.active_version;
    web_update_check_response(state.update_service.check_web_update(&current).await)
}

fn web_update_check_response(
    result: Result<niupanel_common::models::update::WebReleaseInfo>,
) -> Result<Response> {
    match result {
        Ok(info) => Ok(ApiResponse::success(info).into_response()),
        Err(AppError::NotFound(message)) => Ok((
            StatusCode::OK,
            Json(
                ApiResponse::<niupanel_common::models::update::WebReleaseInfo>::error(404, message),
            ),
        )
            .into_response()),
        Err(error) => Err(error),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/system/web/update/install",
    responses((status = 200, description = "Download, verify and activate the Web UI release from the selected channel")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn install_web_update(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<web_releases::WebReleaseMutation>> {
    let current = web_releases::list_releases()?.active_version;
    let download = state.update_service.download_web_update(&current).await?;
    let expected_version = download.info.version.clone();
    let package_path = download.package_path.clone();
    let mutation = tokio::task::spawn_blocking(move || {
        let _temp_dir = download.temp_dir;
        web_releases::install_release_expected(&package_path, true, Some(&expected_version))
    })
    .await
    .map_err(|error| AppError::Internal(format!("Web update worker failed: {error}")))??;
    AuditService::log_user(
        &state.db,
        &user,
        "在线更新 Web UI",
        "system_release",
        Some(mutation.version.clone()),
        Some(format!("from={current}")),
    )
    .await;
    Ok(ApiResponse::success(mutation))
}

#[utoipa::path(
    get,
    path = "/api/v1/system/core/releases",
    responses((status = 200, description = "List installed Core releases")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn list_core_releases() -> Result<ApiResponse<core_releases::CoreReleaseList>> {
    Ok(ApiResponse::success(core_releases::list_releases()?))
}

#[utoipa::path(
    post,
    path = "/api/v1/system/core/releases/{version}/activate",
    params(("version" = String, Path, description = "Core release version")),
    request_body = core_releases::ActivateCoreReleaseRequest,
    responses((status = 200, description = "Schedule an installed Core release for activation")),
    tag = "System",
    security(("session_cookie" = []))
)]
pub async fn activate_core_release(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(version): Path<String>,
    Json(request): Json<core_releases::ActivateCoreReleaseRequest>,
) -> Result<ApiResponse<core_releases::CoreReleaseMutation>> {
    let mutation = core_releases::activate_release(&version, request.confirm_data_loss)?;
    AuditService::log_user(
        &state.db,
        &user,
        "切换 Core 版本",
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

#[cfg(test)]
mod tests {
    use super::web_update_check_response;
    use axum::http::StatusCode;
    use niupanel_common::error::AppError;

    #[tokio::test]
    async fn missing_web_release_uses_http_200_with_business_404() {
        let response =
            web_update_check_response(Err(AppError::NotFound("no web release".to_string())))
                .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], 404);
    }
}
