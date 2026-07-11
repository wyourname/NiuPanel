use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "agent_sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub agent_id: String,
    pub title: String,
    pub user_id: Option<i32>,
    pub task_id: Option<i32>,
    #[sea_orm(column_type = "Text", nullable)]
    pub goal: Option<String>,
    pub status: String,
    #[schema(value_type = String, format = DateTime)]
    pub last_message_at: DateTimeUtc,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeUtc,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::agent_messages::Entity")]
    AgentMessages,
    #[sea_orm(has_many = "super::agent_runs::Entity")]
    AgentRuns,
}

impl Related<super::agent_messages::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AgentMessages.def()
    }
}

impl Related<super::agent_runs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AgentRuns.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
