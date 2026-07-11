use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "shares")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub file_path: String,
    #[serde(skip)]
    pub password_hash: Option<String>,
    pub max_uses: Option<i32>,
    pub used_count: i32,
    pub expires_at: Option<DateTimeWithTimeZone>,
    pub burn_after_reading: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::share_logs::Entity")]
    ShareLogs,
}

impl Related<super::share_logs::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ShareLogs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
