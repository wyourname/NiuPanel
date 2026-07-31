use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create variables_tasks table
        manager
            .create_table(
                Table::create()
                    .table(VariablesTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VariablesTasks::VariableId)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(VariablesTasks::TaskId).integer().not_null())
                    .primary_key(
                        Index::create()
                            .col(VariablesTasks::VariableId)
                            .col(VariablesTasks::TaskId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_variables_tasks_variable_id")
                            .from(VariablesTasks::Table, VariablesTasks::VariableId)
                            .to(Variables::Table, Variables::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_variables_tasks_task_id")
                            .from(VariablesTasks::Table, VariablesTasks::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Migrate data: Insert into variables_tasks where scope='Script' and scope_id is not null
        let insert_query = r#"
            INSERT OR IGNORE INTO variables_tasks (variable_id, task_id)
            SELECT v.id, v.scope_id
            FROM variables v
            JOIN tasks t ON v.scope_id = t.id
            WHERE v.scope = 'Script' AND v.scope_id IS NOT NULL
        "#;
        manager
            .get_connection()
            .execute_raw(sea_orm::Statement::from_string(
                manager.get_database_backend(),
                insert_query.to_owned(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(VariablesTasks::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum VariablesTasks {
    Table,
    VariableId,
    TaskId,
}

#[derive(DeriveIden)]
enum Variables {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Tasks {
    Table,
    Id,
}
