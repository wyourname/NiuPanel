use sea_orm_migration::prelude::sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逐个添加列，确保 SQLite 兼容性
        let table_users = Alias::new("users");

        // 1. permissions
        if !self.has_column(manager, "users", "permissions").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table_users.clone())
                        .add_column(ColumnDef::new(Alias::new("permissions")).string().null())
                        .to_owned(),
                )
                .await?;
        }

        // 2. email_verified
        if !self.has_column(manager, "users", "email_verified").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table_users.clone())
                        .add_column(
                            ColumnDef::new(Alias::new("email_verified"))
                                .boolean()
                                .not_null()
                                .default(false),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 3. last_login_at
        if !self.has_column(manager, "users", "last_login_at").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table_users.clone())
                        .add_column(
                            ColumnDef::new(Alias::new("last_login_at"))
                                .date_time()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 4. last_login_ip
        if !self.has_column(manager, "users", "last_login_ip").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table_users.clone())
                        .add_column(ColumnDef::new(Alias::new("last_login_ip")).string().null())
                        .to_owned(),
                )
                .await?;
        }

        // 5. preferences
        if !self.has_column(manager, "users", "preferences").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(table_users.clone())
                        .add_column(ColumnDef::new(Alias::new("preferences")).json().null())
                        .to_owned(),
                )
                .await?;
        }

        // 6. Add actor_type to audit_logs
        if manager.has_table("audit_logs").await?
            && !self.has_column(manager, "audit_logs", "actor_type").await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new("audit_logs"))
                        .add_column(
                            ColumnDef::new(Alias::new("actor_type"))
                                .string()
                                .not_null()
                                .default("User"),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 7. Add last_used_ip to api_keys
        if manager.has_table("api_keys").await?
            && !self.has_column(manager, "api_keys", "last_used_ip").await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Alias::new("api_keys"))
                        .add_column(ColumnDef::new(Alias::new("last_used_ip")).string().null())
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite 不支持一次性 DROP 多个列，通常需要通过重建表实现，
        // 这里为了简单，我们仅定义基本的回滚逻辑
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
}
