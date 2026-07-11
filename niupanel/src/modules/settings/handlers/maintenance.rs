use super::super::models::{BackupOptions, CleanupLogsRequest};
use super::super::service;
use crate::common::state::AppState;
use axum::{
    Json,
    extract::{Multipart, Path, Query, Request, State},
    http::header,
    response::IntoResponse,
};
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use std::path::Path as StdPath;
use tokio::fs;
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[utoipa::path(
    get,
    path = "/api/v1/settings/version",
    responses(
        (status = 200, description = "Get system version")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn get_version(State(_state): State<AppState>) -> Result<ApiResponse<String>> {
    let version = env!("CARGO_PKG_VERSION");
    Ok(ApiResponse::success(version.to_string()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/backup",
    responses(
        (status = 200, description = "Trigger system backup")
    ),
    params(
        ("tasks" = Option<bool>, Query, description = "Include tasks"),
        ("variables" = Option<bool>, Query, description = "Include variables"),
        ("settings" = Option<bool>, Query, description = "Include settings"),
        ("environments" = Option<bool>, Query, description = "Include environments"),
        ("telegram" = Option<bool>, Query, description = "Include telegram config")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn backup_system(
    State(state): State<AppState>,
    Query(mut options): Query<BackupOptions>,
) -> Result<ApiResponse<()>> {
    if !options.tasks
        && !options.variables
        && !options.settings
        && !options.environments
        && !options.telegram
    {
        options.tasks = true;
        options.variables = true;
        options.settings = true;
        options.environments = true;
        options.telegram = true;
    }

    service::start_backup_task(state.db.clone(), options).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/maintenance/status",
    responses(
        (status = 200, description = "Get maintenance task status")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn get_maintenance_status(
    State(_state): State<AppState>,
) -> Result<ApiResponse<service::MaintenanceStatus>> {
    let status = service::get_maintenance_status();
    Ok(ApiResponse::success(status))
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/backup/{filename}",
    params(
        ("filename" = String, Path, description = "Backup filename to download")
    ),
    responses(
        (status = 200, description = "Download backup file")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn download_backup(
    State(_state): State<AppState>,
    Path(filename): Path<String>,
    req: Request,
) -> Result<impl IntoResponse> {
    let backup_path = StdPath::new(&Config::global().backup_dir).join(&filename);
    if !backup_path.exists() {
        return Err(AppError::NotFound("备份文件不存在".to_string()));
    }

    let service = ServeFile::new(&backup_path);
    let mut res = service
        .oneshot(req)
        .await
        .map_err(|e| AppError::Generic(format!("备份文件返回失败: {}", e)))?;

    res.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .map_err(|_| AppError::Generic("设置文件名失败".to_string()))?,
    );
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/gzip"),
    );

    let cleanup_path = backup_path.clone();
    let cleanup_filename = filename.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        match fs::remove_file(&cleanup_path).await {
            Ok(_) => niupanel_common::info!("备份文件已在延迟清理后删除: {}", cleanup_filename),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => niupanel_common::warn!("延迟清理备份文件失败: {}", e),
        }
    });

    Ok(res)
}

#[utoipa::path(
    get,
    path = "/api/v1/settings/backups",
    responses(
        (status = 200, description = "List available backups")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn list_backups(State(_state): State<AppState>) -> Result<ApiResponse<Vec<String>>> {
    let backups = service::list_backups().await?;
    Ok(ApiResponse::success(backups))
}

#[utoipa::path(
    delete,
    path = "/api/v1/settings/backups/{filename}",
    params(
        ("filename" = String, Path, description = "Backup filename to delete")
    ),
    responses(
        (status = 200, description = "Delete a backup file")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn delete_backup(
    State(_state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<ApiResponse<()>> {
    service::delete_backup(&filename).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/restore",
    responses(
        (status = 200, description = "Restore from uploaded backup")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn restore_backup(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<ApiResponse<()>> {
    let restore_dir = StdPath::new(&Config::global().restore_dir);
    if !restore_dir.exists() {
        fs::create_dir_all(restore_dir)
            .await
            .map_err(AppError::Io)?;
    }

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Generic(e.to_string()))?
    {
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::Generic(e.to_string()))?;
        let temp_tar_path = restore_dir.join("upload_restore.tar.gz");
        fs::write(&temp_tar_path, data)
            .await
            .map_err(AppError::Io)?;

        service::start_restore_task(state.db.clone(), temp_tar_path.clone()).await?;
        return Ok(ApiResponse::success(()));
    }

    Err(AppError::ValidationError("未提供备份文件".to_string()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/restore/{filename}",
    params(
        ("filename" = String, Path, description = "Local backup filename to restore")
    ),
    responses(
        (status = 200, description = "Restore from local backup")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn restore_from_local(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> Result<ApiResponse<()>> {
    let backup_path = StdPath::new(&Config::global().backup_dir).join(&filename);
    if !backup_path.exists() {
        return Err(AppError::NotFound("指定的备份文件不存在".to_string()));
    }

    service::start_restore_task(state.db.clone(), backup_path).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/cleanup-logs",
    request_body = CleanupLogsRequest,
    responses(
        (status = 200, description = "Cleanup old logs")
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn cleanup_logs(
    State(state): State<AppState>,
    Json(payload): Json<CleanupLogsRequest>,
) -> Result<ApiResponse<u64>> {
    let days = payload.days.unwrap_or(30);
    let count = service::cleanup_logs(&state.db, days).await?;
    Ok(ApiResponse::success(count))
}
