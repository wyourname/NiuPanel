use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitRepoIdParams {
    pub repo_id: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitRepoFilesParams {
    pub repo_id: i32,
    #[schemars(description = "Optional path relative to the repository root")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitRepoOutput {
    pub id: i32,
    pub name: String,
    pub repo_url: String,
    pub branch: String,
    pub auto_sync: bool,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_sync_message: Option<String>,
    pub current_commit: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitRepoListOutput {
    pub total: usize,
    pub items: Vec<GitRepoOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitFileOutput {
    pub name: String,
    pub path: String,
    pub script_path: String,
    pub is_directory: bool,
    pub size: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitFileListOutput {
    pub repo_id: i32,
    pub items: Vec<GitFileOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitDiscoveredTaskOutput {
    pub name: String,
    pub command: String,
    pub cron: Option<String>,
    pub file_path: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitDiscoveredTaskListOutput {
    pub repo_id: i32,
    pub items: Vec<GitDiscoveredTaskOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GitSyncOutput {
    pub repo_id: i32,
    pub success: bool,
    pub message: String,
    pub changes: Vec<String>,
}
