use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum TaskStatus {
    #[sea_orm(string_value = "Pending")]
    Pending,
    #[sea_orm(string_value = "Running")]
    Running,
    #[sea_orm(string_value = "Finished")]
    Finished,
    #[sea_orm(string_value = "Failed")]
    Failed,
    #[sea_orm(string_value = "Cancelled")]
    Cancelled,
    #[sea_orm(string_value = "Stopped")]
    Stopped,
    #[sea_orm(string_value = "Paused")]
    Paused,
    #[sea_orm(string_value = "Idle")]
    Idle,
}
