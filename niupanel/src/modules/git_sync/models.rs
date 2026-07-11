use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GitRepoRequest {
    pub name: String,
    pub repo_url: String,
    pub branch: Option<String>,
    pub auth_token: Option<String>,
    pub proxy_url: Option<String>,
    pub auto_sync: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct SyncResult {
    pub success: bool,
    pub message: String,
    pub changes: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub full_path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DiscoveredTask {
    pub name: String,
    pub command: String,
    pub cron: Option<String>,
    pub file_path: String,
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct ImportTasksRequest {
    pub tasks: Vec<DiscoveredTask>,
}
