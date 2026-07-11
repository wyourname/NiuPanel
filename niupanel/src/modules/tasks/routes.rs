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
                .route("/", get(handlers::list_tasks))
                .require(Permission::TaskList),
        )
        // --- 2. 读取权限 ---
        .merge(
            Router::new()
                .route(
                    "/settings/pagination",
                    get(handlers::get_pagination_setting),
                )
                .route("/{id}/logs", get(handlers::stream_logs))
                .route("/{id}/logs/latest", get(handlers::get_latest_log))
                .route("/{id}/history", get(handlers::list_task_history))
                .route(
                    "/{id}/runs/{run_id}/logs",
                    get(handlers::stream_task_run_logs),
                )
                .route("/{id}/runs/{run_id}/log", get(handlers::get_task_run_log))
                .route(
                    "/{id}/history/{run_id}/log",
                    get(handlers::get_task_run_log),
                )
                .route("/status", get(handlers::stream_status))
                .require(Permission::TaskRead),
        )
        // --- 3. 创建权限 ---
        .merge(
            Router::new()
                .route("/", post(handlers::create_task))
                .route("/quick_create", post(handlers::quick_create_task_from_url))
                .require(Permission::TaskCreate),
        )
        // --- 4. 更新权限 ---
        .merge(
            Router::new()
                .route("/enable", post(handlers::batch_enable_tasks))
                .route("/disable", post(handlers::batch_disable_tasks))
                .route("/pin", post(handlers::batch_pin_tasks))
                .route("/unpin", post(handlers::batch_unpin_tasks))
                .route(
                    "/settings/pagination",
                    post(handlers::save_pagination_setting),
                )
                .route("/{id}", patch(handlers::update_task))
                .require(Permission::TaskUpdate),
        )
        // --- 5. 删除权限 ---
        .merge(
            Router::new()
                .route("/", delete(handlers::batch_delete_tasks))
                .route("/{id}/history/{run_id}", delete(handlers::delete_task_run))
                .require(Permission::TaskDelete),
        )
        // --- 6. 运行权限 ---
        .merge(
            Router::new()
                .route("/run", post(handlers::batch_run_tasks))
                .require(Permission::TaskRun),
        )
        // --- 7. 停止/操作权限 ---
        .merge(
            Router::new()
                .route("/stop", post(handlers::batch_stop_tasks))
                .route("/pause", post(handlers::batch_pause_tasks))
                .route("/resume", post(handlers::batch_resume_tasks))
                .require(Permission::TaskStop),
        )
}
