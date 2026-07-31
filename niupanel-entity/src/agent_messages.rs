use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "agent_messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub session_id: i32,
    pub role: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    #[schema(value_type = Object)]
    pub payload: Option<serde_json::Value>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeUtc,
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
