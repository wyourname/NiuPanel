use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "agent_memory_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub memory_id: String,
    pub scope: String,
    pub merge_key: String,
    pub action: String,
    pub updated_sequence: i64,
    pub actor_user_id: Option<i32>,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    #[sea_orm(column_type = "JsonBinary")]
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
