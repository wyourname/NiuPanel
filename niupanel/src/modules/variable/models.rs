use niupanel_entity::variables;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, ToSchema, Clone)]
pub struct VariableRequest {
    pub key: String,
    pub value: String,
    pub scope: String,
    pub scope_id: Option<i32>,
    pub scope_ids: Option<Vec<i32>>,
    pub remarks: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct VariableDto {
    #[serde(flatten)]
    pub inner: variables::Model,
    pub task_ids: Vec<i32>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct VariableSummaryDto {
    pub id: i32,
    pub key: String,
    pub remarks: Option<String>,
    pub enabled: bool,
    pub scope: String,
    pub scope_id: Option<i32>,
    pub sort_order: i32,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub task_ids: Vec<i32>,
}

impl From<VariableDto> for VariableSummaryDto {
    fn from(value: VariableDto) -> Self {
        Self {
            id: value.inner.id,
            key: value.inner.key,
            remarks: value.inner.remarks,
            enabled: value.inner.enabled,
            scope: value.inner.scope,
            scope_id: value.inner.scope_id,
            sort_order: value.inner.sort_order,
            created_at: value.inner.created_at,
            updated_at: value.inner.updated_at,
            task_ids: value.task_ids,
        }
    }
}

#[derive(Serialize, ToSchema, Clone)]
pub struct VariableValueDto {
    pub id: i32,
    pub key: String,
    pub value: String,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct BatchIdRequest {
    pub ids: Vec<i32>,
    pub task_id: Option<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct BatchToggleRequest {
    pub ids: Vec<i32>,
    pub enabled: bool,
}

#[derive(Deserialize, ToSchema, Clone)]
pub struct VariableUpsertRequest {
    pub id: Option<i32>,
    pub variable: VariableRequest,
}

#[derive(Deserialize, ToSchema)]
pub struct BatchSaveVariablesRequest {
    pub task_id: i32,
    pub upserts: Vec<VariableUpsertRequest>,
    #[serde(default)]
    pub delete_ids: Vec<i32>,
}

#[derive(Deserialize, ToSchema)]
pub struct ReorderVariablesRequest {
    pub task_id: Option<i32>,
    pub scope: Option<String>,
    pub ids: Vec<i32>,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct VariableQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub scope: Option<String>,
    pub scope_id: Option<i32>,
    pub key: Option<String>,
}

#[derive(Serialize, ToSchema, FromQueryResult)]
pub struct TaskSimpleResponse {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct KeyQuery {
    pub key: String,
}
