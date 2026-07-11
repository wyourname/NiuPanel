use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StationFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StationFiles::Token)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(StationFiles::UserId).integer().not_null())
                    .col(ColumnDef::new(StationFiles::FileName).string().not_null())
                    .col(ColumnDef::new(StationFiles::Note).string().null())
                    .col(ColumnDef::new(StationFiles::Password).string().null())
                    .col(
                        ColumnDef::new(StationFiles::UploadedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StationFiles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum StationFiles {
    Table,
    Token,
    UserId,
    FileName,
    Note,
    Password,
    UploadedAt,
}
