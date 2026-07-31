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
        // List
        .merge(
            Router::new()
                .route("/scripts", get(handlers::list_directory_root))
                .route("/scripts/{*path}", get(handlers::list_directory_contents))
                .require(Permission::FileList),
        )
        // Read
        .merge(
            Router::new()
                .route("/file/{*path}", get(handlers::read_file_content))
                .route("/download/{*path}", get(handlers::download_file))
                .require(Permission::FileRead),
        )
        // Write / Create
        .merge(
            Router::new()
                .route(
                    "/file",
                    post(handlers::create_file).put(handlers::write_file_content),
                )
                .route("/directory", post(handlers::create_directory))
                .route("/rename", post(handlers::rename_item))
                .route("/copy", post(handlers::copy_item))
                .route("/extract", post(handlers::extract_archive))
                .route("/upload", post(handlers::upload_file_root))
                .route("/upload/{*path}", post(handlers::upload_file))
                .route("/download_url", post(handlers::download_from_url))
                .route("/download_batch", post(handlers::download_batch))
                .require(Permission::FileWrite),
        )
        // Delete
        .merge(
            Router::new()
                .route("/scripts", delete(handlers::delete_root_item))
                .route("/scripts/{*path}", delete(handlers::delete_item))
                .require(Permission::FileDelete),
        )
}
