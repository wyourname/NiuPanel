use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskListParams {
    #[schemars(description = "Maximum number of tasks to return, between 1 and 100")]
    pub limit: Option<u64>,
    #[schemars(description = "Optional search text matched against task name and metadata")]
    pub query: Option<String>,
    #[schemars(description = "Optional task status filter, for example Running or Failed")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskIdParams {
    pub task_id: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskCreateParams {
    pub name: String,
    pub env_type: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub description: Option<String>,
    pub env_version: Option<String>,
    pub cron_schedule: Option<String>,
    pub requirements: Option<String>,
    pub notify: Option<bool>,
    pub cpu_limit: Option<i32>,
    pub timeout_sec: Option<i32>,
    pub memory_limit: Option<i32>,
    pub trigger_next_tasks: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskUpdateParams {
    pub task_id: i32,
    pub name: Option<String>,
    pub path: Option<String>,
    pub command: Option<String>,
    pub description: Option<String>,
    pub env_type: Option<String>,
    pub env_version: Option<String>,
    pub cron_schedule: Option<String>,
    pub requirements: Option<String>,
    pub notify: Option<bool>,
    pub enabled: Option<bool>,
    pub cpu_limit: Option<i32>,
    pub timeout_sec: Option<i32>,
    pub memory_limit: Option<i32>,
    pub trigger_next_tasks: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskDeleteParams {
    pub task_id: i32,
    #[schemars(description = "Also delete the task script file. Defaults to false")]
    pub delete_script: Option<bool>,
    #[schemars(description = "Also delete task-scoped variables. Defaults to false")]
    pub delete_variables: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskLogParams {
    pub task_id: i32,
    #[schemars(description = "Number of bytes to read from the end of the latest log")]
    pub tail_bytes: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskHistoryParams {
    pub task_id: i32,
    #[schemars(description = "Maximum number of recent runs to return, between 1 and 100")]
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskRunLogParams {
    pub task_id: i32,
    pub run_id: i32,
    #[schemars(description = "Number of bytes to read from the end of the run log")]
    pub tail_bytes: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SystemStatusOutput {
    pub panel_version: String,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub task_count: u64,
    pub running_task_count: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskSummaryOutput {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub enabled: bool,
    pub env_type: String,
    pub env_version: Option<String>,
    pub path: String,
    pub next_run_at: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskListOutput {
    pub total: u64,
    pub items: Vec<TaskSummaryOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskDetailOutput {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub enabled: bool,
    pub path: String,
    pub command: Option<String>,
    pub env_type: String,
    pub env_version: Option<String>,
    pub cron_schedule: Option<String>,
    pub pid: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskLogOutput {
    pub task_id: i32,
    pub content: String,
    pub total_size: u64,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskActionOutput {
    pub task_id: i32,
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskRunOutput {
    pub id: i32,
    pub task_id: i32,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub has_log: bool,
    pub pid: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TaskHistoryOutput {
    pub total: u64,
    pub items: Vec<TaskRunOutput>,
}
