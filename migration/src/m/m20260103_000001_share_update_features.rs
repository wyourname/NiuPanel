use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add package_config to station_files
        manager
            .alter_table(
                Table::alter()
                    .table(StationFiles::Table)
                    .add_column(
                        ColumnDef::new(StationFiles::PackageConfig)
                            .text() // JSON content
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add import_source to tasks
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .add_column(ColumnDef::new(Tasks::ImportSource).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Table)
                    .drop_column(Tasks::ImportSource)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(StationFiles::Table)
                    .drop_column(StationFiles::PackageConfig)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum StationFiles {
    Table,
    PackageConfig,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    ImportSource,
}
