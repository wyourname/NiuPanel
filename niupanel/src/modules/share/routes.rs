use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Read / List
        .merge(
            Router::new()
                .route("/station/list", get(handlers::list_station_files))
                .route("/station/stats", get(handlers::get_station_stats))
                .route("/import/history", get(handlers::list_imported_sources))
                .route(
                    "/import/{staging_id}/status",
                    get(handlers::get_import_status),
                )
                .route(
                    "/import/{staging_id}/preview",
                    get(handlers::get_import_preview),
                )
                .route("/market/sources", get(handlers::list_market_sources))
                .route("/market/scripts", get(handlers::list_market_scripts))
                .require(Permission::ShareList),
        )
        // Create
        .merge(
            Router::new()
                .route("/create", post(handlers::create_share))
                .route("/tasks/files", post(handlers::get_task_file_tree))
                .route("/import/submit", post(handlers::submit_import))
                .route(
                    "/import/{staging_id}/confirm",
                    post(handlers::confirm_import),
                )
                .route("/import/{staging_id}/retry", post(handlers::retry_import))
                .route("/station/config", post(handlers::save_station_config))
                .route("/market/sources", post(handlers::add_market_source))
                .route(
                    "/market/sources/{id}/sync",
                    post(handlers::sync_market_source),
                )
                .require(Permission::ShareCreate),
        )
        // Update
        .merge(
            Router::new()
                .route("/station/{token}", patch(handlers::update_station_file))
                .route(
                    "/station/{token}/content",
                    post(handlers::update_station_content),
                )
                .require(Permission::ShareUpdate),
        )
        // Delete
        .merge(
            Router::new()
                .route("/station/{token}", delete(handlers::delete_station_file))
                .route("/import/tasks", delete(handlers::delete_imported_tasks))
                .route(
                    "/market/sources/{id}",
                    delete(handlers::delete_market_source),
                )
                .require(Permission::ShareDelete),
        )
}
