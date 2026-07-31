use super::{handlers, service::PanelMcpServer};
use crate::common::state::AppState;
use crate::modules::auth::middleware::{PermissionExt, token_auth_middleware};
use axum::{Router, middleware, routing::get};
use niupanel_common::auth::permissions::Permission;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::Arc;

pub fn create_management_router() -> Router<AppState> {
    Router::new()
        .route("/info", get(handlers::get_info))
        .require(Permission::KeyList)
}

pub fn create_protocol_router(state: AppState) -> Router<AppState> {
    let server_state = state.clone();
    let allowed_hosts = niupanel_common::config::Config::global()
        .mcp_allowed_hosts
        .clone();
    let config = if allowed_hosts.is_empty() {
        StreamableHttpServerConfig::default().disable_allowed_hosts()
    } else {
        StreamableHttpServerConfig::default().with_allowed_hosts(allowed_hosts)
    };
    let service = StreamableHttpService::new(
        move || Ok(PanelMcpServer::new(server_state.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );

    Router::<AppState>::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(state, token_auth_middleware))
}
