use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{Router, routing::get};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/ws", get(handlers::terminal_ws_handler))
        .require(Permission::TerminalAccess)
}
