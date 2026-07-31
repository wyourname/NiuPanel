use super::handlers;
use crate::common::state::AppState;
use crate::modules::auth::middleware::PermissionExt;
use axum::{
    Router,
    routing::{delete, get, post},
};
use niupanel_common::auth::permissions::Permission;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Read/List
        .merge(
            Router::new()
                .route("/", get(handlers::list_environments))
                .route("/versions", get(handlers::list_available_versions))
                .route(
                    "/python/{name}/packages",
                    get(handlers::list_python_packages),
                )
                .route("/node/{name}/packages", get(handlers::list_node_packages))
                .route("/shell/packages", get(handlers::list_shell_packages))
                .require(Permission::EnvRead),
        )
        // Create
        .merge(
            Router::new()
                .route("/python", post(handlers::create_python_env))
                .route("/node", post(handlers::create_node_env))
                .require(Permission::EnvCreate),
        )
        // Update / Install
        .merge(
            Router::new()
                .route("/mirror/{env_type}", post(handlers::set_mirror_source))
                .route(
                    "/python/{name}/packages",
                    post(handlers::install_python_packages),
                )
                .route(
                    "/node/{name}/packages",
                    post(handlers::install_node_packages),
                )
                .route("/node/{name}/set-default", post(handlers::set_node_default))
                .route("/shell/packages", post(handlers::install_shell_packages))
                .require(Permission::EnvUpdate),
        )
        // Delete
        .merge(
            Router::new()
                .route("/python/{name}", delete(handlers::delete_python_env))
                .route(
                    "/python/{name}/packages/{package}",
                    delete(handlers::uninstall_python_package),
                )
                .route("/node/{name}", delete(handlers::delete_node_env))
                .route(
                    "/node/{name}/packages/{package}",
                    delete(handlers::uninstall_node_package),
                )
                .route(
                    "/shell/packages/{package}",
                    delete(handlers::uninstall_shell_package),
                )
                .require(Permission::EnvDelete),
        )
}
