use sea_orm_migration::prelude::sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Upgrade older "User" role to "admin"
        manager
            .get_connection()
            .execute_unprepared("UPDATE users SET role = 'admin' WHERE role = 'User'")
            .await?;

        // 2. Ensure System User (ID 0) exists and is correctly configured
        // We use unprepared SQL because we might not have the full entity here,
        // and it's simpler for a one-time migration.

        let db = manager.get_connection();

        // Check if ID 0 exists
        let res = db
            .query_one_raw(Statement::from_string(
                db.get_database_backend(),
                "SELECT id FROM users WHERE id = 0".to_string(),
            ))
            .await?;

        if res.is_none() {
            db.execute_raw(Statement::from_string(
                 db.get_database_backend(),
                 "INSERT INTO users (id, username, password_hash, role) VALUES (0, 'api_client', 'disabled', 'system')".to_string(),
             )).await?;
        } else {
            db.execute_raw(Statement::from_string(
                db.get_database_backend(),
                "UPDATE users SET username = 'api_client', role = 'system' WHERE id = 0"
                    .to_string(),
            ))
            .await?;
        }

        // 3. Clear sessions to force re-login if roles changed
        // Check if tower_sessions table exists first (it won't on a brand new DB)
        let table_exists = manager.has_table("tower_sessions").await?;
        if table_exists {
            db.execute_unprepared("DELETE FROM tower_sessions").await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Migration is mostly one-way for role changes.
        Ok(())
    }
}
