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
        .merge(
            Router::new()
                .route("/", get(handlers::list_api_keys))
                .require(Permission::KeyList),
        )
        .merge(
            Router::new()
                .route("/", post(handlers::create_api_key))
                .require(Permission::KeyCreate),
        )
        .merge(
            Router::new()
                .route("/{id}", patch(handlers::update_api_key))
                .require(Permission::KeyUpdate),
        )
        .merge(
            Router::new()
                .route("/{id}", delete(handlers::delete_api_key))
                .require(Permission::KeyDelete),
        )
}
