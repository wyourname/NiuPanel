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
                .route("/apps", get(handlers::list_plugin_apps))
                .route("/themes", get(handlers::list_plugin_themes))
                .route("/{id}/api", post(handlers::proxy_plugin_api_request))
                .route("/{id}/invoke", post(handlers::invoke_agent_plugin))
                .route(
                    "/{id}/ui/{*asset_path}",
                    get(handlers::serve_plugin_ui_asset),
                ),
        )
        .merge(
            Router::new()
                .route("/{id}/versions", get(handlers::list_plugin_versions))
                .route("/health", get(handlers::list_plugin_health))
                .route("/market", get(handlers::get_plugin_market))
                .route("/market/sources", get(handlers::list_plugin_market_sources))
                .route(
                    "/market/updates",
                    get(handlers::check_plugin_market_updates),
                )
                .require(Permission::SettingRead),
        )
        .merge(
            Router::new()
                .route("/preview", post(handlers::preview_install_plugin))
                .route("/install", post(handlers::install_plugin))
                .route(
                    "/upload",
                    post(handlers::upload_install_plugin)
                        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
                )
                .route(
                    "/preview-upload",
                    post(handlers::preview_upload_install_plugin)
                        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
                )
                .route("/{id}/update", post(handlers::update_plugin))
                .route(
                    "/{id}/preview-update",
                    post(handlers::preview_update_plugin),
                )
                .route(
                    "/{id}/upload-update",
                    post(handlers::upload_update_plugin)
                        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
                )
                .route(
                    "/{id}/preview-upload-update",
                    post(handlers::preview_upload_update_plugin)
                        .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
                )
                .route(
                    "/{id}/rollback/{version_id}",
                    post(handlers::rollback_plugin),
                )
                .route("/{id}/enable", post(handlers::enable_plugin))
                .route("/{id}/disable", post(handlers::disable_plugin))
                .route("/{id}", delete(handlers::uninstall_plugin))
                .route(
                    "/market/install",
                    post(handlers::install_plugin_from_market),
                )
                .route(
                    "/market/preview",
                    post(handlers::preview_plugin_from_market),
                )
                .route(
                    "/market/sources",
                    put(handlers::update_plugin_market_sources),
                )
                .require(Permission::SettingUpdate),
        )
}

pub fn create_root_router() -> Router<AppState> {
    Router::new()
        .route("/plugins", get(handlers::list_plugins))
        .require(Permission::SettingRead)
}

#[cfg(test)]
mod tests {
    use super::{create_root_router, create_router};

    #[test]
    fn plugin_management_routes_build_without_conflicts() {
        let _ = create_router();
        let _ = create_root_router();
    }
}
