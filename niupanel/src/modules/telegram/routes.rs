use super::{commands_api, handlers, workflows_api};
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::get_telegram_config))
        .route("/", put(handlers::update_telegram_config))
        .route("/test", post(handlers::test_telegram))
        .route("/latency", get(handlers::get_latency))
        .route("/metrics", get(handlers::get_bot_metrics))
        .route("/message", post(handlers::send_message))
        .route("/file", post(handlers::send_file))
        .route("/server-file", post(handlers::send_server_file))
        // Commands API
        .route("/commands", get(commands_api::list_commands))
        .route("/commands", post(commands_api::create_command))
        .route("/commands/{id}", put(commands_api::update_command))
        .route("/commands/{id}", delete(commands_api::delete_command))
        // Workflows API
        .route("/workflows", get(workflows_api::list_workflows))
        .route("/workflows", post(workflows_api::create_workflow))
        .route("/workflows/{id}", put(workflows_api::update_workflow))
        .route("/workflows/{id}", delete(workflows_api::delete_workflow))
        .require(Permission::SettingAll)
}
