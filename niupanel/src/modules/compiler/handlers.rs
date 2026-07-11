use super::models::EncryptCodeRequest;
use super::service;
use axum::Json;
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;

#[utoipa::path(
    get,
    path = "/api/v1/compiler/versions",
    responses(
        (status = 200, description = "Get supported Python versions")
    ),
    tag = "Compiler",
    security(("session_cookie" = []))
)]
pub async fn get_supported_versions() -> Result<ApiResponse<Vec<String>>> {
    Ok(ApiResponse::success(service::supported_versions().await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/compiler/encrypt",
    request_body = EncryptCodeRequest,
    responses(
        (status = 200, description = "Encrypt Python code")
    ),
    tag = "Compiler",
    security(("session_cookie" = []))
)]
pub async fn encrypt_code(Json(req): Json<EncryptCodeRequest>) -> Result<ApiResponse<String>> {
    Ok(ApiResponse::success(service::encrypt(req).await?))
}
