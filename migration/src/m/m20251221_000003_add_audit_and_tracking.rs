use sea_orm_migration::prelude::sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1. Add 'last_used_at' to 'api_keys'
        if manager.has_table("api_keys").await?
            && !self.has_column(manager, "api_keys", "last_used_at").await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ApiKeys::Table)
                        .add_column(
                            ColumnDef::new(ApiKeys::LastUsedAt)
                                .timestamp_with_time_zone()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 2. Create 'audit_logs' table
        manager
            .create_table(
                Table::create()
                    .table(AuditLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AuditLogs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuditLogs::UserId).integer().null()) // Nullable for system or failed auth? Or specific User ID 0?
                    .col(ColumnDef::new(AuditLogs::Action).string().not_null()) // e.g. "CREATE", "DELETE"
                    .col(ColumnDef::new(AuditLogs::Resource).string().not_null()) // e.g. "task", "setting"
                    .col(ColumnDef::new(AuditLogs::ResourceId).string().null())
                    .col(ColumnDef::new(AuditLogs::Details).text().null()) // JSON detail
                    .col(ColumnDef::new(AuditLogs::IpAddress).string().null())
                    .col(
                        ColumnDef::new(AuditLogs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for Audit Logs (Time based query is common)
        if !self.has_index(manager, "idx-audit_logs-created_at").await? {
            manager
                .create_index(
                    Index::create()
                        .name("idx-audit_logs-created_at")
                        .table(AuditLogs::Table)
                        .col(AuditLogs::CreatedAt)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditLogs::Table).to_owned())
            .await?;

        // SQLite cannot drop columns easily in older versions, but SeaORM handles it via temp table copy if needed
        // For simplicity in SQLite dev we might skip, but let's try.
        // manager.alter_table(Table::alter().table(ApiKeys::Table).drop_column(ApiKeys::LastUsedAt).to_owned()).await?;

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
            .query_all_raw(Statement::from_string(manager.get_database_backend(), sql))
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
            .query_one_raw(Statement::from_string(manager.get_database_backend(), sql))
            .await?;
        Ok(row.is_some())
    }
}

#[derive(Iden)]
enum ApiKeys {
    Table,
    LastUsedAt,
}

#[derive(Iden)]
enum AuditLogs {
    Table,
    Id,
    UserId,
    Action,
    Resource,
    ResourceId,
    Details,
    IpAddress,
    CreatedAt,
}
