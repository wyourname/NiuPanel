use crate::common::extractors::RealIp;
use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::path::PathBuf;
use walkdir::WalkDir;

use super::models::{
    ConfirmImportRequest, CreateMarketSourceRequest, CreateShareRequest, CreateShareResponse,
    DeleteImportQuery, FileNode, ImportSourceGroup, ImportStatusResponse, MarketScriptAggregated,
    NiuPackage, StationConfigPayload, StationFile, StationStats, SubmitImportRequest,
    SubmitImportResponse, TaskFileMapping, TaskFileTreeData, UpdateStationFileRequest,
};
use super::service;
use crate::{
    common::state::AppState,
    modules::{auth::service::AuthenticatedUser, share::models::ImportSummary},
};
use niupanel_common::error::{AppError, Result};
use niupanel_common::filesystem::resolve_path;
use niupanel_common::response::ApiResponse;
use niupanel_entity::market_sources;
use niupanel_entity::tasks;

// --- Handlers ---

#[utoipa::path(
    post,
    path = "/api/v1/share/file-tree",
    request_body = Vec<i32>,
    responses(
        (status = 200, description = "Get task file tree for sharing")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn get_task_file_tree(
    State(state): State<AppState>,
    Json(task_ids): Json<Vec<i32>>,
) -> Result<ApiResponse<TaskFileTreeData>> {
    let tasks = tasks::Entity::find()
        .filter(tasks::Column::Id.is_in(task_ids))
        .all(&state.db)
        .await?;

    if tasks.is_empty() {
        return Ok(ApiResponse::success(TaskFileTreeData {
            tree: Vec::new(),
            tasks: Vec::new(),
            suggestions: Vec::new(),
        }));
    }

    let scripts_root_abs = match resolve_path("") {
        Ok(p) => p,
        Err(_) => {
            return Ok(ApiResponse::success(TaskFileTreeData {
                tree: Vec::new(),
                tasks: Vec::new(),
                suggestions: Vec::new(),
            }));
        }
    };

    if !scripts_root_abs.exists() || !scripts_root_abs.is_dir() {
        return Ok(ApiResponse::success(TaskFileTreeData {
            tree: Vec::new(),
            tasks: Vec::new(),
            suggestions: Vec::new(),
        }));
    }

    let task_files_base_dir_abs = scripts_root_abs.clone();
    let current_scan_dir = scripts_root_abs.clone();

    let (base_tree, mut base_suggestions) = tokio::task::spawn_blocking(move || {
        scan_directory(&task_files_base_dir_abs, &current_scan_dir)
    })
    .await
    .map_err(|e| AppError::Generic(e.to_string()))??;

    let mut task_mappings = Vec::new();

    for task in tasks {
        let filename = std::path::Path::new(&task.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        if !base_suggestions.contains(&task.path) {
            base_suggestions.push(task.path.clone());
        }

        task_mappings.push(TaskFileMapping {
            task_id: task.id,
            path: task.path,
            file: filename,
            name: task.name,
        });
    }

    Ok(ApiResponse::success(TaskFileTreeData {
        tree: base_tree,
        tasks: task_mappings,
        suggestions: base_suggestions,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/create",
    request_body = CreateShareRequest,
    responses(
        (status = 200, description = "Create a new share")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn create_share(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateShareRequest>,
) -> Result<ApiResponse<CreateShareResponse>> {
    let share_url: String =
        service::create_and_upload_share(&state.db, &state.http_client, &user, payload).await?;

    Ok(ApiResponse::success(CreateShareResponse {
        // token: token.clone(),
        link: share_url,
    }))
}

// --- Import Flow Handlers ---

#[utoipa::path(
    post,
    path = "/api/v1/share/import/submit",
    request_body = SubmitImportRequest,
    responses(
        (status = 200, description = "Submit import from URL")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn submit_import(
    State(state): State<AppState>,
    Json(payload): Json<SubmitImportRequest>,
) -> Result<ApiResponse<SubmitImportResponse>> {
    let staging_id =
        service::stage_import(&state.http_client, &payload.url, payload.password).await?;
    Ok(ApiResponse::success(SubmitImportResponse { staging_id }))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/import/{staging_id}/retry",
    params(
        ("staging_id" = String, Path, description = "Staging ID")
    ),
    request_body = SubmitImportRequest,
    responses(
        (status = 200, description = "Retry failed import")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn retry_import(
    State(state): State<AppState>,
    Path(staging_id): Path<String>,
    Json(payload): Json<SubmitImportRequest>,
) -> Result<ApiResponse<()>> {
    service::retry_import(
        &state.http_client,
        &staging_id,
        &payload.url,
        payload.password,
    )
    .await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/share/import/{staging_id}/status",
    params(
        ("staging_id" = String, Path, description = "Staging ID")
    ),
    responses(
        (status = 200, description = "Get import status")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn get_import_status(
    State(_state): State<AppState>,
    Path(staging_id): Path<String>,
) -> Result<ApiResponse<ImportStatusResponse>> {
    let status = service::get_import_status(&staging_id).await?;
    Ok(ApiResponse::success(status))
}

#[utoipa::path(
    get,
    path = "/api/v1/share/import/{staging_id}/preview",
    params(
        ("staging_id" = String, Path, description = "Staging ID")
    ),
    responses(
        (status = 200, description = "Get import preview")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn get_import_preview(
    State(_state): State<AppState>,
    Path(staging_id): Path<String>,
) -> Result<ApiResponse<NiuPackage>> {
    let package = service::get_staged_package(&staging_id).await?;
    Ok(ApiResponse::success(package))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/import/{staging_id}/confirm",
    params(
        ("staging_id" = String, Path, description = "Staging ID")
    ),
    request_body = ConfirmImportRequest,
    responses(
        (status = 200, description = "Confirm and finalize import")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn confirm_import(
    State(state): State<AppState>,
    RealIp(ip): RealIp,
    Path(staging_id): Path<String>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<ConfirmImportRequest>,
) -> Result<ApiResponse<ImportSummary>> {
    let summary = service::finalize_import(
        &state.db,
        &staging_id,
        payload.selected_tasks,
        &user,
        Some(ip),
        payload.update_existing.unwrap_or(false),
    )
    .await?;
    Ok(ApiResponse::success(summary))
}

// --- Import Management ---

#[utoipa::path(
    delete,
    path = "/api/v1/share/imported",
    params(
        ("task_id" = Option<i32>, Query, description = "Task ID"),
        ("share_code" = Option<String>, Query, description = "Share code"),
        ("import_source" = Option<String>, Query, description = "Import source URL")
    ),
    responses(
        (status = 200, description = "Delete imported tasks")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn delete_imported_tasks(
    State(state): State<AppState>,
    Query(query): Query<DeleteImportQuery>,
) -> Result<ApiResponse<u64>> {
    if let Some(share_code) = query.share_code {
        let count = service::delete_imported_by_share_code(&state.db, &share_code).await?;
        Ok(ApiResponse::success(count))
    } else if let Some(source_url) = query.import_source {
        let count = service::delete_imported_by_source(&state.db, &source_url).await?;
        Ok(ApiResponse::success(count))
    } else if let Some(task_id) = query.task_id {
        let count = service::delete_imported_tasks(&state.db, task_id).await?;
        Ok(ApiResponse::success(count))
    } else {
        Err(AppError::Generic(
            "Either task_id, share_code, or import_source must be provided".to_string(),
        ))
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/share/imported/sources",
    responses(
        (status = 200, description = "List imported sources")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn list_imported_sources(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<ImportSourceGroup>>> {
    let sources = service::get_imported_sources_grouped(&state.db).await?;
    Ok(ApiResponse::success(sources))
}

// --- Station Handlers ---

#[utoipa::path(
    get,
    path = "/api/v1/share/station/files",
    responses(
        (status = 200, description = "List station files")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn list_station_files(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<StationFile>>> {
    let list = service::list_station_files(&state.db, &state.http_client).await?;
    Ok(ApiResponse::success(list))
}

#[utoipa::path(
    get,
    path = "/api/v1/share/station/stats",
    responses(
        (status = 200, description = "Get station storage stats")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn get_station_stats(State(state): State<AppState>) -> Result<ApiResponse<StationStats>> {
    let stats = service::get_station_stats(&state.db, &state.http_client).await?;
    Ok(ApiResponse::success(stats))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/station/config",
    request_body = StationConfigPayload,
    responses(
        (status = 200, description = "Save station config")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn save_station_config(
    State(state): State<AppState>,
    Json(payload): Json<StationConfigPayload>,
) -> Result<ApiResponse<()>> {
    service::save_station_config(&state.db, payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    put,
    path = "/api/v1/share/station/{token}",
    params(
        ("token" = String, Path, description = "Station file token")
    ),
    request_body = UpdateStationFileRequest,
    responses(
        (status = 200, description = "Update station file")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn update_station_file(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(payload): Json<UpdateStationFileRequest>,
) -> Result<ApiResponse<()>> {
    service::update_station_file(&state.db, &state.http_client, &token, payload).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/station/{token}/sync",
    params(
        ("token" = String, Path, description = "Station file token")
    ),
    responses(
        (status = 200, description = "Sync station file content")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn update_station_content(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<ApiResponse<()>> {
    service::update_station_content(&state.db, &state.http_client, &token).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/share/station/{token}",
    params(
        ("token" = String, Path, description = "Station file token")
    ),
    responses(
        (status = 200, description = "Delete station file")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn delete_station_file(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<ApiResponse<()>> {
    service::delete_station_file(&state.db, &state.http_client, &token).await?;
    Ok(ApiResponse::success(()))
}

// Helpers

fn scan_directory(root: &PathBuf, current: &PathBuf) -> Result<(Vec<FileNode>, Vec<String>)> {
    let mut nodes = Vec::new();
    let mut suggestions = Vec::new();

    let blacklist_folders = [
        "venv",
        "node_modules",
        ".git",
        "__pycache__",
        ".DS_Store",
        ".idea",
        ".vscode",
        "target",
        "logs",
    ];
    let whitelist_ext = [
        "py", "js", "ts", "sh", "json", "yaml", "yml", "md", "txt", "toml", "env", "sql", "ini",
        "conf",
    ];
    let whitelist_filenames = ["requirements.txt", "package.json"];

    let walker = WalkDir::new(current)
        .min_depth(1)
        .max_depth(1)
        .sort_by_file_name();

    for entry_result in walker {
        let entry = entry_result.map_err(|e| AppError::Io(e.into()))?;
        let path = entry.path().to_path_buf();
        let name = entry.file_name().to_string_lossy().to_string();

        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| {
                AppError::Generic(format!(
                    "Path error: could not strip prefix {} from {}",
                    root.display(),
                    path.display()
                ))
            })?
            .to_string_lossy()
            .to_string();

        if name.starts_with('.') && !whitelist_filenames.contains(&name.as_str()) {
            continue;
        }

        let is_dir = path.is_dir();

        if is_dir && blacklist_folders.contains(&name.as_str()) {
            continue;
        }

        let mut children = None;
        let size = if is_dir {
            0
        } else {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };

        if is_dir {
            let (child_nodes, child_suggestions) = scan_directory(root, &path)?;
            children = Some(child_nodes);
            suggestions.extend(child_suggestions);
        } else {
            let mut is_suggested = false;
            if whitelist_filenames.contains(&name.as_str()) {
                is_suggested = true;
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if whitelist_ext.contains(&ext) {
                    is_suggested = true;
                }
            }
            if is_suggested {
                suggestions.push(relative_path.clone());
            }
        }

        nodes.push(FileNode {
            name: name.clone(),
            path: relative_path.clone(),
            is_dir,
            size,
            children,
            task_id: None,
        });
    }

    nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok((nodes, suggestions))
}

// --- Market Handlers ---

#[utoipa::path(
    get,
    path = "/api/v1/share/market/sources",
    responses(
        (status = 200, description = "List subscribed market sources")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn list_market_sources(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<market_sources::Model>>> {
    let list = service::list_market_sources(&state.db).await?;
    Ok(ApiResponse::success(list))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/market/sources",
    request_body = CreateMarketSourceRequest,
    responses(
        (status = 200, description = "Add a new market source")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn add_market_source(
    State(state): State<AppState>,
    Json(payload): Json<CreateMarketSourceRequest>,
) -> Result<ApiResponse<market_sources::Model>> {
    let source = service::add_market_source(&state.db, &state.http_client, payload).await?;
    Ok(ApiResponse::success(source))
}

#[utoipa::path(
    delete,
    path = "/api/v1/share/market/sources/{id}",
    params(
        ("id" = i32, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Delete a market source")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn delete_market_source(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<u64>> {
    let count = service::delete_market_source(&state.db, id).await?;
    Ok(ApiResponse::success(count))
}

#[utoipa::path(
    post,
    path = "/api/v1/share/market/sources/{id}/sync",
    params(
        ("id" = i32, Path, description = "Source ID")
    ),
    responses(
        (status = 200, description = "Manually sync a market source")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn sync_market_source(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<()>> {
    service::sync_market_source(&state.db, &state.http_client, id).await?;
    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    get,
    path = "/api/v1/share/market/scripts",
    responses(
        (status = 200, description = "List all scripts from subscribed sources")
    ),
    tag = "Share",
    security(("session_cookie" = []))
)]
pub async fn list_market_scripts(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<MarketScriptAggregated>>> {
    let scripts = service::list_aggregated_market_scripts(&state.db, &state.http_client).await?;
    Ok(ApiResponse::success(scripts))
}
