use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Environments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Environments::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Environments::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Environments::EnvType).string().not_null())
                    .col(ColumnDef::new(Environments::Version).string().not_null())
                    .col(ColumnDef::new(Environments::Requirements).text())
                    .col(
                        ColumnDef::new(Environments::CreatedAt)
                            .date_time()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Environments::UpdatedAt)
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
            .drop_table(Table::drop().table(Environments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Environments {
    Table,
    Id,
    Name,
    EnvType,
    Version,
    Requirements,
    CreatedAt,
    UpdatedAt,
}
