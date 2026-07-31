use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "share_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub share_id: String,
    pub ip_address: Option<String>,
    pub imported_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::shares::Entity",
        from = "Column::ShareId",
        to = "super::shares::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Shares,
}

impl Related<super::shares::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Shares.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
