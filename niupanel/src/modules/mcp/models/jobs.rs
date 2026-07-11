use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JobListParams {
    #[schemars(description = "Maximum number of recent jobs, between 1 and 100")]
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JobIdParams {
    pub job_id: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JobLogParams {
    pub job_id: i32,
    #[schemars(description = "Maximum log bytes to return, between 1 and 1048576")]
    pub tail_bytes: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct JobOutput {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub ended_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub has_log: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct JobListOutput {
    pub total: u64,
    pub items: Vec<JobOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct JobLogOutput {
    pub job_id: i32,
    pub content: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct JobActionOutput {
    pub job_id: i32,
    pub accepted: bool,
    pub message: String,
}
