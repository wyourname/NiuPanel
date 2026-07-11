use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::{PermissionExt, auth_middleware};
use axum::{
    Router, middleware,
    routing::{get, post, put},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me", get(handlers::get_my_profile))
        .route("/profile", put(handlers::update_profile))
        .route("/password", put(handlers::change_password))
        .route("/preferences", put(handlers::update_preferences))
        .route(
            "/email/verify-request",
            post(handlers::request_email_change),
        )
        .route(
            "/email/verify-confirm",
            post(handlers::confirm_email_change),
        )
        .require(Permission::ProfileUpdate)
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}
