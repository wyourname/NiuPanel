use super::handlers::{
    forgot_password, get_setup_status, identify_reset, login, logout, register_admin,
    reset_password, verify_login_2fa, verify_reset_code,
};
use super::middleware::auth_middleware;
use crate::common::state::AppState;
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn create_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/logout",
            get(logout).route_layer(middleware::from_fn_with_state(state, auth_middleware)),
        )
        .route("/status", get(get_setup_status))
        .route("/register", post(register_admin))
        .route("/login", post(login))
        .route("/login/verify-2fa", post(verify_login_2fa))
        .route("/forgot-password", post(forgot_password))
        .route("/verify-reset-code", post(verify_reset_code))
        .route("/identify-reset", post(identify_reset))
        .route("/reset-password", post(reset_password))
}
