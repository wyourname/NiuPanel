use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

fn deserialize_explicit_null<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct TaskVariable {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, Default, ToSchema, IntoParams)]
pub struct QueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub q: Option<String>,
    pub status: Option<String>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct LogPagination {
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct LogResponse {
    pub content: String,
    pub total_size: u64,
    pub offset: u64,
    pub length: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct DeleteTaskParams {
    pub delete_var: Option<bool>,
    pub delete_script: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
pub struct PaginationSettingRequest {
    pub page_size: u64,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateTaskRequest {
    pub name: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub description: Option<String>,
    pub env_type: String,
    pub env_version: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub tags: Option<serde_json::Value>,
    pub cron_schedule: Option<String>,
    pub requirements: Option<String>,
    pub variables: Option<Vec<TaskVariable>>,
    pub notify: Option<bool>,
    pub cpu_limit: Option<i32>,
    pub timeout_sec: Option<i32>,
    pub memory_limit: Option<i32>,
    pub trigger_next_tasks: Option<Vec<i32>>,
    #[schema(value_type = Option<Object>)]
    pub random_config: Option<serde_json::Value>,
}

#[derive(Deserialize, Default, ToSchema)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub path: Option<String>,
    pub command: Option<String>,
    pub description: Option<String>,
    pub env_type: Option<String>,
    pub env_version: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub tags: Option<serde_json::Value>,
    pub cron_schedule: Option<String>,
    pub requirements: Option<String>,
    pub variables: Option<Vec<TaskVariable>>,
    pub notify: Option<bool>,
    pub enabled: Option<bool>,
    pub cpu_limit: Option<i32>,
    pub timeout_sec: Option<i32>,
    pub memory_limit: Option<i32>,
    pub trigger_next_tasks: Option<Vec<i32>>,
    #[schema(value_type = Option<Object>)]
    #[serde(default, deserialize_with = "deserialize_explicit_null")]
    pub random_config: Option<Option<serde_json::Value>>,
}

#[derive(Deserialize, ToSchema)]
pub struct BatchIdRequest {
    pub ids: Vec<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct QuickCreateUrlRequest {
    pub url: String,
}
