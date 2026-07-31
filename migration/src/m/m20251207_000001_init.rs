use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Username)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .col(
                        ColumnDef::new(Users::Role)
                            .string()
                            .not_null()
                            .default("User"),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Tasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tasks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tasks::Name).string().not_null())
                    .col(ColumnDef::new(Tasks::Path).string().not_null())
                    .col(ColumnDef::new(Tasks::Command).string().null())
                    .col(ColumnDef::new(Tasks::Description).string().null())
                    .col(ColumnDef::new(Tasks::CronSchedule).string().null())
                    .col(
                        ColumnDef::new(Tasks::Enabled)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Tasks::Status)
                            .string()
                            .not_null()
                            .default("Idle"),
                    )
                    .col(ColumnDef::new(Tasks::Pid).integer().null())
                    .col(
                        ColumnDef::new(Tasks::Notify)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Tasks::Tags).json().null())
                    .col(ColumnDef::new(Tasks::EnvType).string().not_null())
                    .col(ColumnDef::new(Tasks::EnvVersion).string().null())
                    .col(ColumnDef::new(Tasks::UserId).integer().not_null())
                    .col(ColumnDef::new(Tasks::Requirements).text().null())
                    .col(ColumnDef::new(Tasks::LastFinishedAt).timestamp())
                    .col(ColumnDef::new(Tasks::NextRunAt).timestamp())
                    .col(
                        ColumnDef::new(Tasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Tasks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-task-user_id")
                            .from(Tasks::Table, Tasks::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-tasks-user_id")
                    .table(Tasks::Table)
                    .col(Tasks::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-tasks-status")
                    .table(Tasks::Table)
                    .col(Tasks::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-tasks-next_run_at")
                    .table(Tasks::Table)
                    .col(Tasks::NextRunAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Variables::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Variables::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Variables::Key).string().not_null())
                    .col(ColumnDef::new(Variables::Value).string().not_null())
                    .col(ColumnDef::new(Variables::Remarks).string().null())
                    .col(
                        ColumnDef::new(Variables::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(Variables::Scope).string().not_null()) // 'Global' or 'Script'
                    .col(ColumnDef::new(Variables::ScopeId).integer().null())
                    .col(
                        ColumnDef::new(Variables::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Variables::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-variables-scope-scope_id")
                    .table(Variables::Table)
                    .col(Variables::Scope)
                    .col(Variables::ScopeId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(SystemJobs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SystemJobs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SystemJobs::Name).string().not_null())
                    .col(ColumnDef::new(SystemJobs::Description).string())
                    .col(ColumnDef::new(SystemJobs::Status).string().not_null())
                    .col(ColumnDef::new(SystemJobs::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(SystemJobs::EndedAt).timestamp())
                    .col(ColumnDef::new(SystemJobs::Metadata).json_binary())
                    .col(ColumnDef::new(SystemJobs::LogPath).string())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TaskRuns::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TaskRuns::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TaskRuns::TaskId).integer().not_null())
                    .col(ColumnDef::new(TaskRuns::Status).string().not_null())
                    .col(ColumnDef::new(TaskRuns::StartedAt).timestamp().not_null())
                    .col(ColumnDef::new(TaskRuns::EndedAt).timestamp())
                    .col(ColumnDef::new(TaskRuns::LogPath).string())
                    .col(ColumnDef::new(TaskRuns::Pid).integer())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-taskruns-task_id")
                            .from(TaskRuns::Table, TaskRuns::TaskId)
                            .to(Tasks::Table, Tasks::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-taskruns-task_id")
                    .table(TaskRuns::Table)
                    .col(TaskRuns::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx-taskruns-started_at")
                    .table(TaskRuns::Table)
                    .col(TaskRuns::StartedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TaskRuns::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(SystemJobs::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Variables::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tasks::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    #[sea_orm(iden = "users")]
    Table,
    Id,
    Username,
    PasswordHash,
    Role,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Tasks {
    #[sea_orm(iden = "tasks")]
    Table,
    Id,
    Name,
    Path,
    Command,
    Description,
    CronSchedule,
    Enabled,
    Status,
    Pid,
    Notify,
    Tags,
    EnvType,
    EnvVersion,
    UserId,
    Requirements,
    LastFinishedAt,
    NextRunAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Variables {
    #[sea_orm(iden = "variables")]
    Table,
    Id,
    Key,
    Value,
    Remarks,
    Enabled,
    Scope,
    ScopeId,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SystemJobs {
    #[sea_orm(iden = "system_jobs")]
    Table,
    Id,
    Name,
    Description,
    Status,
    CreatedAt,
    EndedAt,
    Metadata,
    LogPath,
}

#[derive(DeriveIden)]
enum TaskRuns {
    #[sea_orm(iden = "task_runs")]
    Table,
    Id,
    TaskId,
    Status,
    StartedAt,
    EndedAt,
    LogPath,
    Pid,
}
