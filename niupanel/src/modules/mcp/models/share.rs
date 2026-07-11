use super::*;

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShareStationStatsOutput {
    pub configured: bool,
    pub current_usage_bytes: u64,
    pub max_usage_bytes: u64,
    pub usage_percent: Option<String>,
    pub max_file_size_bytes: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShareStationFileOutput {
    pub token: String,
    pub file_key: String,
    pub size: u64,
    pub downloads_remaining: i32,
    pub delete_on_download: bool,
    pub expires_at: Option<i64>,
    pub note: Option<String>,
    pub uploaded_at: Option<i64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShareStationFilesOutput {
    pub total: usize,
    pub items: Vec<ShareStationFileOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShareImportSourceOutput {
    pub share_code: Option<String>,
    pub url: String,
    pub task_count: usize,
    pub task_names: Vec<String>,
    pub last_updated_at: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ShareImportSourcesOutput {
    pub total: usize,
    pub items: Vec<ShareImportSourceOutput>,
}
