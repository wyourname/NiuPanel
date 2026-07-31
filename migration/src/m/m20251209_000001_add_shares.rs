use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create shares table
        manager
            .create_table(
                Table::create()
                    .table(Shares::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Shares::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Shares::Name).string().not_null())
                    .col(ColumnDef::new(Shares::FilePath).string().not_null())
                    .col(ColumnDef::new(Shares::PasswordHash).string())
                    .col(ColumnDef::new(Shares::MaxUses).integer())
                    .col(
                        ColumnDef::new(Shares::UsedCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Shares::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Shares::BurnAfterReading)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Shares::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create share_logs table
        manager
            .create_table(
                Table::create()
                    .table(ShareLogs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShareLogs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ShareLogs::ShareId).string().not_null())
                    .col(ColumnDef::new(ShareLogs::IpAddress).string())
                    .col(
                        ColumnDef::new(ShareLogs::ImportedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_logs_share_id")
                            .from(ShareLogs::Table, ShareLogs::ShareId)
                            .to(Shares::Table, Shares::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShareLogs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Shares::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum Shares {
    Table,
    Id,
    Name, // Friendly name for the share
    FilePath,
    PasswordHash,
    MaxUses,
    UsedCount,
    ExpiresAt,
    BurnAfterReading,
    CreatedAt,
}

#[derive(Iden)]
enum ShareLogs {
    Table,
    Id,
    ShareId,
    IpAddress,
    ImportedAt,
}
