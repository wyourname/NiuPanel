//! `SeaORM` Entity

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize, Serialize)]
#[sea_orm(table_name = "login_attempts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub ip_address: String,
    pub username: String,
    pub attempt_count: i32,
    pub last_attempt_at: DateTimeUtc,
    pub banned_until: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
