use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct GitRepoRequest {
    pub name: String,
    pub repo_url: String,
    pub branch: Option<String>,
    pub auth_token: Option<String>,
    #[serde(default)]
    pub clear_auth_token: bool,
    pub proxy_url: Option<String>,
    pub auto_sync: Option<bool>,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct GitRepoResponse {
    pub id: i32,
    pub name: String,
    pub repo_url: String,
    pub branch: String,
    pub has_auth_token: bool,
    pub proxy_url: Option<String>,
    pub auto_sync: bool,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_sync_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_sync_status: Option<String>,
    pub last_sync_message: Option<String>,
    pub current_commit: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<niupanel_entity::git_repositories::Model> for GitRepoResponse {
    fn from(model: niupanel_entity::git_repositories::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            repo_url: model.repo_url,
            branch: model.branch,
            has_auth_token: model
                .auth_token
                .as_deref()
                .is_some_and(|token| !token.is_empty()),
            proxy_url: model.proxy_url,
            auto_sync: model.auto_sync,
            last_sync_at: model.last_sync_at,
            last_sync_status: model.last_sync_status,
            last_sync_message: model.last_sync_message,
            current_commit: model.current_commit,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
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
