use sea_orm_migration::prelude::sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite 不支持在一条 ALTER TABLE 中添加多个字段，必须拆开执行

        // 1. 添加 RemoteTaskId
        if !self.has_column(manager, "tasks", "remote_task_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Tasks::Table)
                        .add_column(ColumnDef::new(Tasks::RemoteTaskId).string().null())
                        .to_owned(),
                )
                .await?;
        }

        // 2. 添加 ShareCode
        if !self.has_column(manager, "tasks", "share_code").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Tasks::Table)
                        .add_column(ColumnDef::new(Tasks::ShareCode).string().null())
                        .to_owned(),
                )
                .await?;
        }

        // 3. 创建索引
        if !self.has_index(manager, "idx-tasks-remote-task-id").await? {
            manager
                .create_index(
                    Index::create()
                        .name("idx-tasks-remote-task-id")
                        .table(Tasks::Table)
                        .col(Tasks::RemoteTaskId)
                        .to_owned(),
                )
                .await?;
        }

        if !self.has_index(manager, "idx-tasks-share-code").await? {
            manager
                .create_index(
                    Index::create()
                        .name("idx-tasks-share-code")
                        .table(Tasks::Table)
                        .col(Tasks::ShareCode)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 注意：SQLite 的 Drop Column 在某些旧版本中也不支持，
        // 但 SeaORM 会尝试通过创建临时表的方式来模拟。
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::RemoteTaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::ShareCode)
                    .to_owned(),
            )
            .await
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
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    RemoteTaskId,
    ShareCode,
}
