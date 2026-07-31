use sea_orm_migration::prelude::sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. 检查并补全 users 表的缺失字段
        let columns = vec![
            ("permissions", "TEXT"),
            ("email_verified", "BOOLEAN NOT NULL DEFAULT 0"),
            ("last_login_at", "DATETIME"),
            ("last_login_ip", "TEXT"),
            ("preferences", "JSON"),
        ];

        for (col_name, col_type) in columns {
            if !self.has_column(manager, "users", col_name).await? {
                db.execute(Statement::from_string(
                    manager.get_database_backend(),
                    format!("ALTER TABLE users ADD COLUMN {} {}", col_name, col_type),
                ))
                .await?;
            }
        }

        // 2. 检查并补全 audit_logs 表
        if !self.has_column(manager, "audit_logs", "actor_type").await? {
            db.execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE audit_logs ADD COLUMN actor_type TEXT NOT NULL DEFAULT 'User'"
                    .to_string(),
            ))
            .await?;
        }

        // 3. 检查并补全 api_keys 表
        if !self.has_column(manager, "api_keys", "last_used_ip").await? {
            db.execute(Statement::from_string(
                manager.get_database_backend(),
                "ALTER TABLE api_keys ADD COLUMN last_used_ip TEXT".to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
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
            // 在 SQLite 中，PRAGMA table_info 返回的列名在 "name" 字段中
            let name: String = row.try_get_by_index(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
