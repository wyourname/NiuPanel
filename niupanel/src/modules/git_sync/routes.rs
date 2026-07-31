use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route("/", get(handlers::list_repos))
                .route("/{id}/files", get(handlers::list_repo_files))
                .route("/{id}/scan", get(handlers::scan_repo_tasks))
                .require(Permission::GitRead),
        )
        .merge(
            Router::new()
                .route("/", post(handlers::create_repo))
                .route("/{id}", put(handlers::update_repo))
                .route("/{id}", delete(handlers::delete_repo))
                .require(Permission::GitConfig),
        )
        .merge(
            Router::new()
                .route("/{id}/sync", post(handlers::sync_repo))
                .route("/{id}/import", post(handlers::import_tasks))
                .require(Permission::GitSync),
        )
}
