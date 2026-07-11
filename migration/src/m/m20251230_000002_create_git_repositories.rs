use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GitRepositories::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GitRepositories::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GitRepositories::Name).string().not_null())
                    .col(ColumnDef::new(GitRepositories::RepoUrl).string().not_null())
                    .col(
                        ColumnDef::new(GitRepositories::Branch)
                            .string()
                            .not_null()
                            .default("main"),
                    )
                    .col(ColumnDef::new(GitRepositories::AuthToken).string().null())
                    .col(ColumnDef::new(GitRepositories::ProxyUrl).string().null())
                    .col(
                        ColumnDef::new(GitRepositories::AutoSync)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(GitRepositories::LastSyncAt)
                            .date_time()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GitRepositories::LastSyncStatus)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GitRepositories::LastSyncMessage)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GitRepositories::CurrentCommit)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GitRepositories::CreatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GitRepositories::UpdatedAt)
                            .date_time()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GitRepositories::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum GitRepositories {
    Table,
    Id,
    Name,
    RepoUrl,
    Branch,
    AuthToken,
    ProxyUrl,
    AutoSync,
    LastSyncAt,
    LastSyncStatus,
    LastSyncMessage,
    CurrentCommit,
    CreatedAt,
    UpdatedAt,
}
