use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    routing::{get, post, put},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .merge(
            Router::new()
                .route("/", get(handlers::get_telegram_config))
                .require(Permission::SettingRead),
        )
        .merge(
            Router::new()
                .route("/", put(handlers::update_telegram_config))
                .route("/users", get(handlers::list_telegram_users))
                .route("/test", post(handlers::test_telegram))
                .require(Permission::SettingUpdate),
        )
}
