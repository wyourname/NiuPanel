use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileListParams {
    #[schemars(description = "Directory path relative to the configured scripts directory")]
    pub path: Option<String>,
    #[schemars(description = "Optional recursive filename search text")]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FilePathParams {
    #[schemars(description = "Path relative to the configured scripts directory")]
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileWriteParams {
    #[schemars(description = "File path relative to the configured scripts directory")]
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FileItemOutput {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified_at_unix: Option<i64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FileListOutput {
    pub items: Vec<FileItemOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FileContentOutput {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FileActionOutput {
    pub path: String,
    pub accepted: bool,
    pub message: String,
}
