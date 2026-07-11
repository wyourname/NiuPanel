use super::task_status::TaskStatus;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "system_jobs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeUtc,
    #[schema(value_type = String, format = DateTime)]
    pub ended_at: Option<DateTimeUtc>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    #[schema(value_type = Object)]
    pub metadata: Option<serde_json::Value>,
    pub log_path: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
