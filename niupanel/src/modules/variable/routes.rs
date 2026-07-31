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
        // --- 1. 列表权限 ---
        .merge(
            Router::new()
                .route("/", get(handlers::list_variables))
                .route("/tasks/all", get(handlers::list_tasks_simple))
                .require(Permission::VarList),
        )
        // Sensitive values are intentionally isolated from metadata listing.
        .merge(
            Router::new()
                .route("/with-values", get(handlers::list_variables_with_values))
                .route("/{id}/value", get(handlers::get_variable_value))
                .route("/by-key", get(handlers::get_variable_by_key))
                .require(Permission::VarRead),
        )
        // --- 2. 创建/导入权限 ---
        .merge(
            Router::new()
                .route("/", post(handlers::create_variable))
                .route("/import", post(handlers::import_variables))
                .require(Permission::VarCreate),
        )
        // --- 3. 更新权限 ---
        .merge(
            Router::new()
                .route("/by-key", patch(handlers::update_variable_by_key))
                .route("/toggle", post(handlers::batch_toggle_variables))
                .route("/batch-save", post(handlers::batch_save_variables))
                .route("/reorder", post(handlers::batch_reorder_variables))
                .route("/{id}", patch(handlers::update_variable))
                .require(Permission::VarUpdate),
        )
        // --- 4. 删除权限 ---
        .merge(
            Router::new()
                .route("/", delete(handlers::batch_delete_variables))
                .require(Permission::VarDelete),
        )
}
