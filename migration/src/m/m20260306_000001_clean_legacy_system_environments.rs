use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Delete legacy sandboxes for Node and Python, which were used in older architectures.
        let stmt = sea_query::Query::delete()
            .from_table(Environments::Table)
            .and_where(Expr::col(Environments::Name).is_in(["System Node", "System Python"]))
            .to_owned();

        db.execute(manager.get_database_backend().build(&stmt))
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Cannot reliably recreate the legacy fake sandboxes.
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Environments {
    Table,
    Name,
}
