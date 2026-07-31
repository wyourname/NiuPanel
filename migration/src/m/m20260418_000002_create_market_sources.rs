use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MarketSources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MarketSources::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MarketSources::Name).string().not_null())
                    .col(ColumnDef::new(MarketSources::Url).string().not_null())
                    .col(ColumnDef::new(MarketSources::Icon).string().null())
                    .col(ColumnDef::new(MarketSources::Description).string().null())
                    .col(
                        ColumnDef::new(MarketSources::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(MarketSources::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(MarketSources::LastSyncedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MarketSources::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MarketSources {
    Table,
    Id,
    Name,
    Url,
    Icon,
    Description,
    Enabled,
    CreatedAt,
    LastSyncedAt,
}
