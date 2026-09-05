use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Extension, Router,
    routing::{delete, get, post, put},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    let upload_store = handlers::plugin_upload_session_store();
    Router::new()
        .merge(
            Router::new()
                .route("/apps", get(handlers::list_plugin_apps))
                .route("/themes", get(handlers::list_plugin_themes))
                .route("/{id}/actions", get(handlers::list_ui_plugin_actions))
                .route(
                    "/{id}/approval-policy",
                    get(handlers::get_agent_approval_policy)
                        .put(handlers::update_agent_approval_policy),
                )
                .route(
                    "/{id}/approval-grants",
                    post(handlers::create_ui_approval_grant)
                        .delete(handlers::revoke_ui_approval_grants),
                )
                .route("/{id}/api", post(handlers::proxy_plugin_api_request))
                .route(
                    "/{id}/invoke",
                    post(handlers::invoke_plugin_action).layer(
                        axum::extract::DefaultBodyLimit::max(
                            niupanel_plugin::MAX_PLUGIN_INVOCATION_BODY_BYTES,
                        ),
                    ),
                )
                .route(
                    "/{id}/invoke/stream",
                    post(handlers::stream_plugin_action).layer(
                        axum::extract::DefaultBodyLimit::max(
                            niupanel_plugin::MAX_PLUGIN_INVOCATION_BODY_BYTES,
                        ),
                    ),
                )
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
                    "/upload-sessions",
                    post(handlers::create_plugin_upload_session)
                        .layer(axum::extract::DefaultBodyLimit::max(102 * 1024 * 1024)),
                )
                .route(
                    "/upload-sessions/{token}/commit",
                    post(handlers::commit_plugin_upload_session),
                )
                .route(
                    "/upload-sessions/{token}",
                    delete(handlers::delete_plugin_upload_session),
                )
                .route("/{id}/update", post(handlers::update_plugin))
                .route(
                    "/{id}/preview-update",
                    post(handlers::preview_update_plugin),
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
                .layer(Extension(upload_store))
                .require(Permission::SettingUpdate),
        )
}

pub fn create_open_router() -> Router<AppState> {
    Router::new()
        .route("/plugins", get(handlers::list_open_plugin_actions))
        .route(
            "/plugins/{plugin_id}/actions",
            get(handlers::list_open_plugin_actions_for_plugin),
        )
        .route(
            "/plugins/{plugin_id}/invoke",
            post(handlers::invoke_open_plugin_action).layer(axum::extract::DefaultBodyLimit::max(
                niupanel_plugin::MAX_PLUGIN_INVOCATION_BODY_BYTES,
            )),
        )
        .route(
            "/plugins/{plugin_id}/invoke/stream",
            post(handlers::stream_open_plugin_action).layer(axum::extract::DefaultBodyLimit::max(
                niupanel_plugin::MAX_PLUGIN_INVOCATION_BODY_BYTES,
            )),
        )
        .route(
            "/plugins/{plugin_id}/approval-grants",
            post(handlers::create_open_approval_grant)
                .delete(handlers::revoke_open_approval_grants),
        )
        .require(Permission::PluginInvoke)
}

pub fn create_root_router() -> Router<AppState> {
    Router::new()
        .route("/plugins", get(handlers::list_plugins))
        .require(Permission::SettingRead)
}

#[cfg(test)]
mod tests {
    use super::{create_open_router, create_root_router, create_router};

    #[test]
    fn plugin_management_routes_build_without_conflicts() {
        let _ = create_router();
        let _ = create_root_router();
        let _ = create_open_router();
    }
}
