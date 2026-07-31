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
        // Basic / List all
        .merge(
            Router::new()
                .route("/", get(handlers::basic::get_settings))
                .route("/sessions", get(handlers::basic::get_active_sessions))
                .route("/version", get(handlers::maintenance::get_version))
                .route("/update/stats", get(handlers::update::get_update_status))
                .route("/onboarding", get(handlers::basic::get_onboarding_status))
                .require(Permission::SettingRead),
        )
        // Onboarding complete (also needs auth but lower permission)
        .merge(
            Router::new()
                .route(
                    "/onboarding/done",
                    post(handlers::basic::complete_onboarding),
                )
                .require(Permission::SettingRead),
        )
        // Specialized Update Endpoints
        .merge(
            Router::new()
                .route("/general", put(handlers::basic::update_general_settings))
                .route("/security", put(handlers::basic::update_security_settings))
                .route(
                    "/notification",
                    put(handlers::notification::update_notification_settings),
                )
                .route("/update/channel", put(handlers::update::update_channel))
                .route("/sessions/{id}", delete(handlers::basic::delete_session))
                .route(
                    "/test_notify",
                    post(handlers::notification::test_notification),
                )
                .require(Permission::SettingUpdate),
        )
        // Maintenance / Critical
        .merge(
            Router::new()
                .route("/backup", get(handlers::maintenance::backup_system))
                .route(
                    "/maintenance/status",
                    get(handlers::maintenance::get_maintenance_status),
                )
                .route(
                    "/backup/download/{filename}",
                    get(handlers::maintenance::download_backup),
                )
                .route("/backups", get(handlers::maintenance::list_backups))
                .route(
                    "/backups/{filename}",
                    delete(handlers::maintenance::delete_backup),
                )
                .route(
                    "/backups/{filename}/restore",
                    post(handlers::maintenance::restore_from_local),
                )
                .route(
                    "/restore",
                    post(handlers::maintenance::restore_backup)
                        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
                )
                .route("/cleanup_logs", post(handlers::maintenance::cleanup_logs))
                .route("/update/check", get(handlers::update::check_update))
                .route("/update/execute", post(handlers::update::execute_update))
                .route("/update/cancel", post(handlers::update::cancel_update))
                .route(
                    "/update/upload",
                    post(handlers::update::upload_update)
                        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
                )
                .require(Permission::SettingAll),
        )
}
