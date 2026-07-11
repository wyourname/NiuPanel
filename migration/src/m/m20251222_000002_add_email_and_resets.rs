use sea_orm_migration::prelude::sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[allow(dead_code)]
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add email column to users table (without UNIQUE constraint for SQLite compatibility)
        if !self.has_column(manager, "users", "email").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .add_column(ColumnDef::new(Users::Email).string())
                        .to_owned(),
                )
                .await?;
        }

        // 2. Create a unique index for the email column
        if !self.has_index(manager, "idx-users-email").await? {
            if self.has_duplicate_emails(manager).await? {
                eprintln!("Skipping idx-users-email creation because duplicate user emails exist");
            } else {
                manager
                    .create_index(
                        Index::create()
                            .name("idx-users-email")
                            .table(Users::Table)
                            .col(Users::Email)
                            .unique()
                            .to_owned(),
                    )
                    .await?;
            }
        }

        // 3. Create password_resets table
        manager
            .create_table(
                Table::create()
                    .table(PasswordResets::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PasswordResets::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PasswordResets::UserId).integer().not_null())
                    .col(
                        ColumnDef::new(PasswordResets::TokenHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PasswordResets::ExpiresAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PasswordResets::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-password_resets-user_id")
                            .from(PasswordResets::Table, PasswordResets::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PasswordResets::Table).to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx-users-email")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Email)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

impl Migration {
    async fn has_column(
        &self,
        manager: &SchemaManager<'_>,
        table: &str,
        column: &str,
    ) -> Result<bool, DbErr> {
        let db = manager.get_connection();
        let sql = format!("PRAGMA table_info({})", table);
        let rows = db
            .query_all(Statement::from_string(manager.get_database_backend(), sql))
            .await?;

        for row in rows {
            let name: String = row.try_get_by_index(1)?;
            if name == column {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn has_index(
        &self,
        manager: &SchemaManager<'_>,
        index_name: &str,
    ) -> Result<bool, DbErr> {
        let db = manager.get_connection();
        let sql = format!(
            "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = '{}'",
            index_name.replace('\'', "''")
        );
        let row = db
            .query_one(Statement::from_string(manager.get_database_backend(), sql))
            .await?;
        Ok(row.is_some())
    }

    async fn has_duplicate_emails(&self, manager: &SchemaManager<'_>) -> Result<bool, DbErr> {
        let db = manager.get_connection();
        let sql = "SELECT email FROM users WHERE email IS NOT NULL AND email != '' GROUP BY email HAVING COUNT(*) > 1 LIMIT 1".to_string();
        let row = db
            .query_one(Statement::from_string(manager.get_database_backend(), sql))
            .await?;
        Ok(row.is_some())
    }
}

#[allow(dead_code)]
#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
}

// #[allow(dead_code)]
#[derive(DeriveIden)]
enum PasswordResets {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    CreatedAt,
}
