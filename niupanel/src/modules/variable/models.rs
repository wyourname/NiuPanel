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

#[derive(Deserialize, ToSchema)]
pub struct ReorderVariablesRequest {
    pub task_id: Option<i32>,
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
