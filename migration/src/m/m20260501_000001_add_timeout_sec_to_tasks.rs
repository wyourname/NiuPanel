use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_column("tasks", "timeout_sec").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Tasks::Table)
                        .add_column(ColumnDef::new(Tasks::TimeoutSec).integer().null())
                        .to_owned(),
                )
                .await?;
        }

        manager
            .get_connection()
            .execute_raw(Statement::from_string(
                manager.get_database_backend(),
                "UPDATE tasks SET timeout_sec = cpu_limit WHERE timeout_sec IS NULL AND cpu_limit IS NOT NULL"
                    .to_string(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::TimeoutSec)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Tasks {
    #[sea_orm(iden = "tasks")]
    Table,
    TimeoutSec,
}
