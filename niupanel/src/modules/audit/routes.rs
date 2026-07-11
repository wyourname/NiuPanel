use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{Router, routing::get};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_audit_logs))
        .require(Permission::AuditList)
}
