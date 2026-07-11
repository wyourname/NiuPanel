use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::Router;
use axum::routing::{get, post};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route("/versions", get(handlers::get_supported_versions))
                .require(Permission::CompilerRead),
        )
        .merge(
            Router::new()
                .route("/encrypt", post(handlers::encrypt_code))
                .require(Permission::CompilerRun),
        )
}
