use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tg_workflows")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub event_type: String,
    pub action_type: String,
    #[sea_orm(column_type = "Text")]
    pub config_json: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
