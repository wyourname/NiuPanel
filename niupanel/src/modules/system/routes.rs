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
        .merge(
            Router::new()
                .route("/meta", get(handlers::get_system_meta))
                .route("/releases", get(handlers::list_panel_releases))
                .require(Permission::OverviewRead),
        )
        .merge(
            Router::new()
                .route(
                    "/releases/{version}/rollback",
                    post(handlers::rollback_panel_release),
                )
                .require(Permission::SettingAll),
        )
}

pub fn create_public_router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/recovery", get(handlers::recovery))
}
