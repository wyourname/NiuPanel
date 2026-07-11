use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let stmt = sea_query::Query::update()
            .table(Tasks::Table)
            .values([(Tasks::EnvVersion, "".into())])
            .and_where(Expr::col(Tasks::EnvVersion).like("System Node%"))
            .to_owned();

        db.execute(manager.get_database_backend().build(&stmt))
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Can't reliably recover which tasks were "System Node", so this is a no-op down.
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    EnvVersion,
}
