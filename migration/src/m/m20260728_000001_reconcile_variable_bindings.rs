use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();

        // Older versions stored the primary task only in variables.scope_id.
        // Preserve that association before making variables_tasks authoritative.
        connection
            .execute_unprepared(
                r#"
                INSERT OR IGNORE INTO variables_tasks (variable_id, task_id, sort_order)
                SELECT v.id, v.scope_id, v.sort_order
                FROM variables v
                JOIN tasks t ON t.id = v.scope_id
                WHERE v.scope = 'Script'
                  AND v.scope_id IS NOT NULL
                "#,
            )
            .await?;

        // Global variables must never retain a task reference. For script
        // variables, keep scope_id as a compatibility projection of the first
        // variables_tasks relation until the legacy column can be removed.
        connection
            .execute_unprepared("UPDATE variables SET scope_id = NULL WHERE scope = 'Global'")
            .await?;
        connection
            .execute_unprepared(
                r#"
                UPDATE variables
                SET scope_id = (
                    SELECT vt.task_id
                    FROM variables_tasks vt
                    WHERE vt.variable_id = variables.id
                    ORDER BY vt.sort_order ASC, vt.task_id ASC
                    LIMIT 1
                )
                WHERE scope = 'Script'
                  AND EXISTS (
                    SELECT 1
                    FROM variables_tasks vt
                    WHERE vt.variable_id = variables.id
                )
                "#,
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_variables_scope_sort_id")
                    .table(Variables::Table)
                    .col(Variables::Scope)
                    .col(Variables::SortOrder)
                    .col(Variables::Id)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_variables_tasks_task_sort_variable")
                    .table(VariablesTasks::Table)
                    .col(VariablesTasks::TaskId)
                    .col(VariablesTasks::SortOrder)
                    .col(VariablesTasks::VariableId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_variables_tasks_task_sort_variable")
                    .table(VariablesTasks::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_variables_scope_sort_id")
                    .table(Variables::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Variables {
    Table,
    Id,
    Scope,
    SortOrder,
}

#[derive(DeriveIden)]
enum VariablesTasks {
    Table,
    VariableId,
    TaskId,
    SortOrder,
}
