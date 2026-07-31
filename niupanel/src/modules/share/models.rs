use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    #[schema(value_type = Option<Vec<Object>>)]
    pub children: Option<Vec<FileNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i32>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct TaskFileTreeData {
    pub tree: Vec<FileNode>,
    pub tasks: Vec<TaskFileMapping>,
    pub suggestions: Vec<String>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct TaskFileMapping {
    pub task_id: i32,
    pub path: String,
    pub file: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Default, Clone, ToSchema)]
pub struct CreateShareRequest {
    pub tasks: Vec<TaskExportSpec>,
    pub password: Option<String>,
    pub max_uses: Option<i32>,
    pub expires_in_hours: Option<i64>,
    pub burn_after_reading: Option<bool>,
    pub note: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct TaskExportSpec {
    pub task_id: i32,
    pub files: Vec<FileAssociations>,
    pub include_envs: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct FileAssociations {
    pub main_file: String,
    pub dependencies: Vec<String>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct CreateShareResponse {
    pub link: String,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct SubmitImportRequest {
    pub url: String,
    pub password: Option<String>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct SubmitImportResponse {
    pub staging_id: String,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ImportStatusResponse {
    pub state: ImportState,
    pub message: Option<String>,
    pub progress: u8,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportState {
    Pending,
    Downloading,
    Ready,
    Error,
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct ConfirmImportRequest {
    pub selected_tasks: Option<Vec<String>>,
    pub update_existing: Option<bool>,
}

#[derive(Serialize, Debug, Default, ToSchema)]
pub struct ImportSummary {
    pub success_count: usize,
    pub skip_count: usize,
    pub failure_count: usize,
    pub failures: Vec<ImportFailure>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct ImportFailure {
    pub task_name: String,
    pub error: String,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct ImportSourceGroup {
    pub share_code: Option<String>,
    pub url: String,
    pub tasks: Vec<ImportHistoryItem>,
    pub task_count: usize,
    pub last_updated_at: i64,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct ImportHistoryItem {
    pub id: i32,
    pub url: String,
    pub share_code: Option<String>,
    pub task_name: String,
    pub note: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteImportQuery {
    pub task_id: Option<i32>,
    pub share_code: Option<String>,
    pub import_source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StationFilesResponse {
    pub files: Vec<StationFile>,
    pub cursor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StationFile {
    pub token: String,
    #[serde(rename = "fileKey")]
    pub file_key: String,
    pub size: u64,
    #[serde(rename = "downloadsRemaining")]
    pub downloads_remaining: i32,
    #[serde(rename = "deleteOnDownload")]
    pub delete_on_download: bool,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub uploaded_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Default, ToSchema)]
pub struct StationStats {
    #[serde(rename = "currentUsageBytes")]
    pub current_usage_bytes: u64,
    #[serde(rename = "maxUsageBytes")]
    pub max_usage_bytes: u64,
    #[serde(rename = "usagePercent")]
    pub usage_percent: String,
    #[serde(rename = "maxFileSizeContent")]
    pub max_file_size_content: u64,
    #[serde(rename = "isConfigured", default)]
    pub is_configured: bool,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct StationConfigPayload {
    pub url: String,
    pub token: String,
}

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct UpdateStationFileRequest {
    #[serde(rename = "downloadsRemaining")]
    pub downloads_remaining: Option<i32>,
    #[serde(rename = "deleteOnDownload")]
    pub delete_on_download: Option<bool>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<i64>,
    pub password: Option<String>,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NiuPackage {
    pub version: u8,
    pub created_at: i64,
    pub tasks: Vec<TaskData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskData {
    pub meta: TaskMeta,
    pub main_file: Option<String>,
    pub files: Vec<FileEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskMeta {
    pub name: String,
    pub description: Option<String>,
    pub env_type: String,
    pub env_version: Option<String>,
    pub cron_schedule: Option<String>,
    pub command: Option<String>,
    pub requirements: Option<String>,
    pub notify: bool,
    #[serde(default)]
    pub variables: Option<Vec<ShareVariable>>,
    #[serde(default)]
    pub remote_task_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ShareVariable {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub mode: u32,
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StagedImportMeta {
    pub source_url: String,
    pub staging_id: String,
    pub created_at: i64,
}

// --- Market Related Models ---

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct MarketSourceInfo {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub scripts: Vec<MarketScriptItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct MarketScriptItem {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub url: String, // 脚本包下载地址 (.npack)
    pub icon: Option<String>,
    pub tags: Vec<String>,
    pub updated_at: i64,
    pub is_encrypted: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateMarketSourceRequest {
    pub name: String,
    pub url: String,
    pub description: Option<String>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct MarketScriptAggregated {
    pub source_id: i32,
    pub source_name: String,
    pub script: MarketScriptItem,
}
