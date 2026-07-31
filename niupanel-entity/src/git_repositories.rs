use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Deserialize, Serialize, ToSchema)]
#[sea_orm(table_name = "git_repositories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub repo_url: String,
    pub branch: String,
    pub auth_token: Option<String>,
    pub proxy_url: Option<String>,
    pub auto_sync: bool,
    #[schema(value_type = String, format = DateTime)]
    pub last_sync_at: Option<DateTimeUtc>,
    pub last_sync_status: Option<String>,
    pub last_sync_message: Option<String>,
    pub current_commit: Option<String>,
    #[schema(value_type = String, format = DateTime)]
    pub created_at: DateTimeUtc,
    #[schema(value_type = String, format = DateTime)]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        use sea_orm::Set;
        let mut model = <Self as sea_orm::ActiveModelTrait>::default();
        model.created_at = Set(chrono::Utc::now().into());
        model.updated_at = Set(chrono::Utc::now().into());
        model
    }

    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        use sea_orm::Set;
        self.updated_at = Set(chrono::Utc::now().into());
        Ok(self)
    }
}
