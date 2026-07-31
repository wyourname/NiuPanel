use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "agent_memories")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub kind: String,
    pub scope: String,
    #[sea_orm(column_type = "Text")]
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub summary: String,
    pub merge_key: String,
    pub confidence: f32,
    pub status: String,
    pub risk_level: String,
    pub source: String,
    pub reinforcement_count: i32,
    pub created_sequence: i64,
    pub updated_sequence: i64,
    #[sea_orm(column_type = "JsonBinary")]
    #[schema(value_type = Object)]
    pub evidence: serde_json::Value,
    pub last_governance_action: Option<String>,
    pub last_governed_by_user_id: Option<i32>,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub last_governed_at: Option<DateTimeUtc>,
    #[sea_orm(column_type = "Text", nullable)]
    pub last_governance_note: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeUtc,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
