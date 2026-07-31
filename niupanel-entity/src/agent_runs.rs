use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "agent_runs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub session_id: i32,
    pub agent_id: String,
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub goal: Option<String>,
    pub model_used: bool,
    pub closure_status: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub error: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    #[schema(value_type = Object)]
    pub result: Option<serde_json::Value>,
    #[schema(value_type = String, format = DateTime)]
    pub started_at: DateTimeUtc,
    #[schema(value_type = Option<String>, format = DateTime)]
    pub ended_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::agent_sessions::Entity",
        from = "Column::SessionId",
        to = "super::agent_sessions::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    AgentSessions,
}

impl Related<super::agent_sessions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AgentSessions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
