use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Debug, ToSchema)]
pub struct FileSystemItem {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub mtime: Option<i64>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateFileRequest {
    pub path: String,
    pub content: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateDirectoryRequest {
    pub path: String,
}

#[derive(Deserialize, ToSchema)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}

#[derive(Deserialize, ToSchema)]
pub struct RenameItemRequest {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CopyItemRequest {
    pub from_path: String,
    pub to_path: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ExtractArchiveRequest {
    pub path: String,
    pub destination_path: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DownloadUrlRequest {
    pub url: String,
    pub path: String,
    pub filename: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct DownloadBatchRequest {
    pub paths: Vec<String>,
}
