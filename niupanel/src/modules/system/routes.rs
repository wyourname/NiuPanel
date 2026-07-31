use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route("/meta", get(handlers::get_system_meta))
                .route("/core/releases", get(handlers::list_core_releases))
                .route("/web/releases", get(handlers::list_web_releases))
                .route("/web/update/check", get(handlers::check_web_update))
                .require(Permission::OverviewRead),
        )
        .merge(
            Router::new()
                .route(
                    "/web/releases/upload",
                    post(handlers::upload_web_release)
                        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024)),
                )
                .route(
                    "/web/releases/{version}/activate",
                    post(handlers::activate_web_release),
                )
                .route("/web/rollback", post(handlers::rollback_web_release))
                .route("/web/update/install", post(handlers::install_web_update))
                .route(
                    "/core/releases/{version}/activate",
                    post(handlers::activate_core_release),
                )
                .require(Permission::SettingAll),
        )
}

pub fn create_public_router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/recovery", get(handlers::recovery))
}
