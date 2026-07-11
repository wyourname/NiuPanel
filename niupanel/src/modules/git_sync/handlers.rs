use super::models::{DiscoveredTask, FileEntry, GitRepoRequest, ImportTasksRequest, SyncResult};
use super::service::GitService;
use crate::common::extractors::RealIp;
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use niupanel_common::error::Result;
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_entity::git_repositories;
use serde::Deserialize;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ListFilesQuery {
    path: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/git/repos",
    responses(
        (status = 200, description = "List all git repositories")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn list_repos(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<git_repositories::Model>>> {
    let repos = GitService::list_repos(&state.db).await?;
    Ok(ApiResponse::success(repos))
}

#[utoipa::path(
    get,
    path = "/api/v1/git/repos/{id}/files",
    params(
        ("id" = i32, Path, description = "Repository ID"),
        ("path" = Option<String>, Query, description = "Subdirectory path")
    ),
    responses(
        (status = 200, description = "List repository files")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn list_repo_files(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<ListFilesQuery>,
) -> Result<ApiResponse<Vec<FileEntry>>> {
    let files = GitService::list_repo_files(&state.db, id, query.path).await?;
    Ok(ApiResponse::success(files))
}

#[utoipa::path(
    post,
    path = "/api/v1/git/repos/{id}/scan",
    params(
        ("id" = i32, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Scan repository for tasks")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn scan_repo_tasks(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<Vec<DiscoveredTask>>> {
    let tasks = GitService::scan_repo_tasks(&state.db, id).await?;
    Ok(ApiResponse::success(tasks))
}

#[utoipa::path(
    post,
    path = "/api/v1/git/repos/{id}/import",
    params(
        ("id" = i32, Path, description = "Repository ID")
    ),
    request_body = ImportTasksRequest,
    responses(
        (status = 200, description = "Import tasks from repository")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn import_tasks(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<ImportTasksRequest>,
) -> Result<ApiResponse<()>> {
    GitService::import_tasks(&state.db, id, user.id, payload.tasks.clone()).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "批量导入任务",
        "git",
        Some(id.to_string()),
        Some(format!(
            "从 Git 仓库批量导入了 {} 个任务",
            payload.tasks.len()
        )),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/git/repos",
    request_body = GitRepoRequest,
    responses(
        (status = 200, description = "Create a git repository")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn create_repo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    RealIp(_ip): RealIp,
    Json(payload): Json<GitRepoRequest>,
) -> Result<ApiResponse<git_repositories::Model>> {
    let repo = GitService::create_repo(&state.db, payload).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "创建Git仓库",
        "git",
        Some(repo.id.to_string()),
        Some(format!("添加了 Git 仓库: {}", repo.name)),
    )
    .await;

    Ok(ApiResponse::success(repo))
}

#[utoipa::path(
    put,
    path = "/api/v1/git/repos/{id}",
    params(
        ("id" = i32, Path, description = "Repository ID")
    ),
    request_body = GitRepoRequest,
    responses(
        (status = 200, description = "Update a git repository")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn update_repo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<GitRepoRequest>,
) -> Result<ApiResponse<git_repositories::Model>> {
    let repo = GitService::update_repo(&state.db, id, payload).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "更新Git仓库",
        "git",
        Some(id.to_string()),
        Some(format!("更新了 Git 仓库: {}", repo.name)),
    )
    .await;

    Ok(ApiResponse::success(repo))
}

#[utoipa::path(
    delete,
    path = "/api/v1/git/repos/{id}",
    params(
        ("id" = i32, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Delete a git repository")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn delete_repo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<()>> {
    GitService::delete_repo(&state.db, id).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "删除Git仓库",
        "git",
        Some(id.to_string()),
        Some("删除了 Git 仓库配置".to_string()),
    )
    .await;

    Ok(ApiResponse::success(()))
}

#[utoipa::path(
    post,
    path = "/api/v1/git/repos/{id}/sync",
    params(
        ("id" = i32, Path, description = "Repository ID")
    ),
    responses(
        (status = 200, description = "Sync repository")
    ),
    tag = "Git Sync",
    security(("session_cookie" = []))
)]
pub async fn sync_repo(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<ApiResponse<SyncResult>> {
    let res = GitService::sync_repo(&state.db, id).await?;

    AuditService::log_user(
        &state.db,
        &user,
        "手动Git同步",
        "git",
        Some(id.to_string()),
        Some(format!("同步仓库 (ID: {}): {}", id, res.message)),
    )
    .await;

    Ok(ApiResponse::success(res))
}
