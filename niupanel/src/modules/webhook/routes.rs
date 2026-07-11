use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{Router, routing::post};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/push", post(handlers::send_notification))
        .require(Permission::WebhookPush)
}
