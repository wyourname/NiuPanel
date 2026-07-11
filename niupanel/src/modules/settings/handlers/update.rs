use crate::common::state::AppState;
use axum::{Json, extract::State};
use niupanel_common::error::Result;
use niupanel_common::models::update::ReleaseInfo;
use niupanel_common::models::update::UpdateStatus;
use niupanel_common::response::ApiResponse;
use niupanel_core::settings::SYSTEM_UPDATE_CHANNEL;

use crate::modules::settings::models::UpdateChannelRequest;

#[utoipa::path(
    get,
    path = "/api/v1/settings/update/check",
    responses(
        (status = 200, description = "Check for system updates")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn check_update(State(state): State<AppState>) -> Result<ApiResponse<ReleaseInfo>> {
    let release_info = state.update_service.check_update().await?;
    Ok(ApiResponse::success(release_info))
}

#[utoipa::path(
    put,
    path = "/api/v1/settings/update/channel",
    request_body = UpdateChannelRequest,
    responses(
        (status = 200, description = "Update system update channel")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn update_channel(
    State(state): State<AppState>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<ApiResponse<()>> {
    state
        .settings
        .set(SYSTEM_UPDATE_CHANNEL, &req.channel, Some("System"))
        .await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/update/stats",
    responses(
        (status = 200, description = "Get current update status")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn get_update_status(State(state): State<AppState>) -> Result<ApiResponse<UpdateStatus>> {
    let status = state.update_service.get_status()?;
    Ok(ApiResponse::success(status))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/update/cancel",
    responses(
        (status = 200, description = "Cancel ongoing update")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn cancel_update(State(state): State<AppState>) -> Result<ApiResponse<()>> {
    state.update_service.cancel_update().await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/update/execute",
    responses(
        (status = 200, description = "Execute system update")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn execute_update(State(state): State<AppState>) -> Result<ApiResponse<()>> {
    state.update_service.execute_update().await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/update/upload",
    responses(
        (status = 200, description = "Upload and apply local update package")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn upload_update(
    State(state): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Result<ApiResponse<()>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| niupanel_common::error::AppError::Generic(e.to_string()))?
    {
        let _file_name = field.file_name().unwrap_or("update.tar.gz").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|e| niupanel_common::error::AppError::Generic(e.to_string()))?;

        state
            .update_service
            .execute_local_update(data.to_vec())
            .await?;
        return Ok(ApiResponse::success(()));
    }

    Err(niupanel_common::error::AppError::Generic(
        "No file provided".to_string(),
    ))
}
