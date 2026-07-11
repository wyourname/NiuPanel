use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Variables::Table)
                    .add_column(
                        ColumnDef::new(Variables::SortOrder)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("UPDATE variables SET sort_order = id WHERE sort_order = 0")
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Variables::Table)
                    .drop_column(Variables::SortOrder)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Variables {
    Table,
    SortOrder,
}
