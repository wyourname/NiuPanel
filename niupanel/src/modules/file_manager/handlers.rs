pub use super::models::{
    CopyItemRequest, CreateDirectoryRequest, CreateFileRequest, DownloadBatchRequest,
    DownloadUrlRequest, ExtractArchiveRequest, FileSystemItem, RenameItemRequest, WriteFileRequest,
};
use super::service::{FileManagerService, no_cache_headers};
use crate::common::state::AppState;
use axum::{
    Json,
    extract::{Multipart, Path, Request, State},
    response::IntoResponse,
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::info;
use niupanel_common::response::ApiResponse;
use tower::ServiceExt;
use tower_http::services::ServeFile;

#[utoipa::path(
    get,
    path = "/api/v1/files/{path}",
    params(
        ("path" = String, Path, description = "Directory relative path"),
        ("q" = Option<String>, Query, description = "Search query")
    ),
    responses(
        (status = 200, description = "List directory contents")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn list_directory_contents(
    State(_state): State<AppState>,
    Path(relative_path): Path<String>,
    axum::extract::Query(query_params): axum::extract::Query<
        std::collections::HashMap<String, String>,
    >,
) -> Result<impl IntoResponse> {
    let query = query_params.get("q").cloned();
    info!("Listing directory contents for {}", relative_path);
    let items = FileManagerService::list_directory(relative_path, query).await?;
    Ok((no_cache_headers(), ApiResponse::success(items)))
}

#[utoipa::path(
    get,
    path = "/api/v1/files",
    responses(
        (status = 200, description = "List root directory contents")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn list_directory_root(
    State(_state): State<AppState>,
    axum::extract::Query(query_params): axum::extract::Query<
        std::collections::HashMap<String, String>,
    >,
) -> Result<impl IntoResponse> {
    let query = query_params.get("q").cloned();
    info!("Listing directory contents for root");
    let items = FileManagerService::list_directory("".to_string(), query).await?;
    Ok((no_cache_headers(), ApiResponse::success(items)))
}

#[utoipa::path(
    delete,
    path = "/api/v1/files",
    responses(
        (status = 200, description = "Delete root item (not allowed)")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn delete_root_item(State(_state): State<AppState>) -> Result<ApiResponse<()>> {
    Err(AppError::Generic(
        "Cannot delete root directory".to_string(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/files/content/{path}",
    params(
        ("path" = String, Path, description = "File relative path")
    ),
    responses(
        (status = 200, description = "Read file content")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn read_file_content(
    State(_state): State<AppState>,
    Path(relative_path): Path<String>,
) -> Result<impl IntoResponse> {
    let content = FileManagerService::read_file_content(relative_path).await?;
    Ok((no_cache_headers(), ApiResponse::success(content)))
}

#[utoipa::path(
    put,
    path = "/api/v1/files/content",
    request_body = WriteFileRequest,
    responses(
        (status = 200, description = "Write file content")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn write_file_content(
    State(_state): State<AppState>,
    Json(payload): Json<WriteFileRequest>,
) -> Result<ApiResponse<()>> {
    FileManagerService::write_file_content(payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/files/download/{path}",
    params(
        ("path" = String, Path, description = "File relative path")
    ),
    responses(
        (status = 200, description = "Download file")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn download_file(
    State(_state): State<AppState>,
    Path(relative_path): Path<String>,
    req: Request,
) -> Result<impl IntoResponse> {
    let (target_path, filename) = FileManagerService::resolve_download_file(relative_path).await?;
    let service = ServeFile::new(target_path);
    let mut res = service
        .oneshot(req)
        .await
        .map_err(|e| AppError::Generic(format!("File serving error: {}", e)))?;

    res.headers_mut().insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .map_err(|_| AppError::Generic("Invalid filename".to_string()))?,
    );

    Ok(res)
}

#[utoipa::path(
    post,
    path = "/api/v1/files/upload/{path}",
    params(
        ("path" = String, Path, description = "Target directory relative path")
    ),
    responses(
        (status = 200, description = "Upload file to path")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn upload_file(
    State(_state): State<AppState>,
    Path(relative_path): Path<String>,
    multipart: Multipart,
) -> Result<ApiResponse<()>> {
    FileManagerService::upload_file(relative_path, multipart).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/upload",
    responses(
        (status = 200, description = "Upload file to root")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn upload_file_root(
    State(_state): State<AppState>,
    multipart: Multipart,
) -> Result<ApiResponse<()>> {
    FileManagerService::upload_file("".to_string(), multipart).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/create",
    request_body = CreateFileRequest,
    responses(
        (status = 200, description = "Create new file")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn create_file(
    State(_state): State<AppState>,
    Json(payload): Json<CreateFileRequest>,
) -> Result<ApiResponse<()>> {
    FileManagerService::create_file(payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/mkdir",
    request_body = CreateDirectoryRequest,
    responses(
        (status = 200, description = "Create new directory")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn create_directory(
    State(_state): State<AppState>,
    Json(payload): Json<CreateDirectoryRequest>,
) -> Result<ApiResponse<()>> {
    FileManagerService::create_directory(payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/files/{path}",
    params(
        ("path" = String, Path, description = "File or directory relative path")
    ),
    responses(
        (status = 200, description = "Delete file or directory")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn delete_item(
    State(_state): State<AppState>,
    Path(relative_path): Path<String>,
) -> Result<ApiResponse<()>> {
    FileManagerService::delete_item(relative_path).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/rename",
    request_body = RenameItemRequest,
    responses(
        (status = 200, description = "Rename or move file/directory")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn rename_item(
    State(_state): State<AppState>,
    Json(payload): Json<RenameItemRequest>,
) -> Result<ApiResponse<()>> {
    FileManagerService::rename_item(payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/extract",
    request_body = ExtractArchiveRequest,
    responses(
        (status = 200, description = "Extract archive file")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn extract_archive(
    State(_state): State<AppState>,
    Json(payload): Json<ExtractArchiveRequest>,
) -> Result<ApiResponse<usize>> {
    let extracted = FileManagerService::extract_archive(payload).await?;
    Ok(ApiResponse::success(extracted))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/copy",
    request_body = CopyItemRequest,
    responses(
        (status = 200, description = "Copy file or directory")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn copy_item(
    State(_state): State<AppState>,
    Json(payload): Json<CopyItemRequest>,
) -> Result<ApiResponse<()>> {
    FileManagerService::copy_item(payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/download_url",
    request_body = DownloadUrlRequest,
    responses(
        (status = 200, description = "Download file from URL to server")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn download_from_url(
    State(state): State<AppState>,
    Json(payload): Json<DownloadUrlRequest>,
) -> Result<ApiResponse<()>> {
    FileManagerService::download_from_url(&state.http_client, payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/files/download_batch",
    request_body = DownloadBatchRequest,
    responses(
        (status = 200, description = "Download multiple files as TAR archive")
    ),
    tag = "Files",
    security(("session_cookie" = []))
)]
pub async fn download_batch(
    State(_state): State<AppState>,
    Json(payload): Json<DownloadBatchRequest>,
) -> Result<impl IntoResponse> {
    FileManagerService::download_batch(payload).await
}
