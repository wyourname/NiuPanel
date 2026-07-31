use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TgCommands::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TgCommands::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(TgCommands::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(TgCommands::Script).text().not_null())
                    .col(
                        ColumnDef::new(TgCommands::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TgCommands::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TgWorkflows::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TgWorkflows::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TgWorkflows::EventType).string().not_null())
                    .col(ColumnDef::new(TgWorkflows::ActionType).string().not_null())
                    .col(ColumnDef::new(TgWorkflows::ConfigJson).text().not_null())
                    .col(
                        ColumnDef::new(TgWorkflows::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(TgWorkflows::UpdatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TgCommands::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TgWorkflows::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum TgCommands {
    Table,
    Id,
    Name,
    Script,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum TgWorkflows {
    Table,
    Id,
    EventType,
    ActionType,
    ConfigJson,
    CreatedAt,
    UpdatedAt,
}
