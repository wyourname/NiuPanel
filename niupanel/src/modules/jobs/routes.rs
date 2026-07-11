use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    routing::{get, post},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // List
        .merge(
            Router::new()
                .route("/", get(handlers::list_jobs))
                .require(Permission::JobList),
        )
        // Read
        .merge(
            Router::new()
                .route("/{id}", get(handlers::get_job))
                .route("/{id}/logs", get(handlers::stream_job_logs))
                .route("/{id}/logs/latest", get(handlers::get_latest_job_log))
                .require(Permission::JobRead),
        )
        // Actions
        .merge(
            Router::new()
                .route("/{id}/cancel", post(handlers::cancel_job))
                .require(Permission::JobAll),
        )
}
