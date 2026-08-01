use super::super::models::{BackupOptions, CleanupLogsRequest, LogCleanupReport};
use super::super::service;
use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, Multipart, Path, Query, Request, State},
    http::header,
    response::IntoResponse,
};
use niupanel_common::constants::defaults;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_common::upload::{TempUploadOptions, stream_field_to_temp_file};
use niupanel_core::audit::service::AuditService;
use tokio::fs;
use tower::ServiceExt;
use tower_http::services::ServeFile;

const MAX_BACKUP_UPLOAD_SIZE: u64 = 512 * 1024 * 1024;

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
    let backup_path = service::resolve_backup_path(&filename).await?;

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
    let restore_dir = &niupanel_common::config::Config::global().restore_dir;
    if !restore_dir.exists() {
        fs::create_dir_all(restore_dir)
            .await
            .map_err(AppError::Io)?;
    }

    let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Generic(e.to_string()))?
    else {
        return Err(AppError::ValidationError("未提供备份文件".to_string()));
    };
    let upload = stream_field_to_temp_file(
        &mut field,
        TempUploadOptions::new(restore_dir, "niupanel-restore-")
            .with_max_size(MAX_BACKUP_UPLOAD_SIZE),
    )
    .await?;
    if upload.size == 0 {
        return Err(AppError::ValidationError("备份文件不能为空".to_string()));
    }
    let temp_tar_path = upload.into_persisted_path()?;

    if let Err(err) =
        service::start_restore_task(state.db.clone(), temp_tar_path.clone(), true).await
    {
        let _ = fs::remove_file(&temp_tar_path).await;
        return Err(err);
    }
    Ok(ApiResponse::success(()))
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
    let backup_path = service::resolve_backup_path(&filename).await?;

    service::start_restore_task(state.db.clone(), backup_path, false).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/settings/cleanup_logs",
    request_body = CleanupLogsRequest,
    responses(
        (status = 200, description = "Preview or clean expired logs", body = ApiResponse<LogCleanupReport>)
    ),
    tag = "Settings",
    security(("session_cookie" = []))
)]
pub async fn cleanup_logs(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(ip): RealIp,
    Json(payload): Json<CleanupLogsRequest>,
) -> Result<ApiResponse<LogCleanupReport>> {
    let days = payload.days.unwrap_or(defaults::LOG_RETENTION_DAYS);
    let report = if payload.dry_run {
        service::preview_log_cleanup(&state.db, days).await?
    } else {
        service::cleanup_logs(&state.db, days).await?
    };

    if !payload.dry_run {
        let details = serde_json::to_string(&report).ok();
        AuditService::log(
            &state.db,
            Some(user.id),
            "User",
            "清理日志",
            "settings",
            None,
            details,
            Some(ip),
        )
        .await;
    }

    Ok(ApiResponse::success(report))
}
