use futures::stream::{self, StreamExt};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::task_manager::service::TaskManagerService;
use niupanel_common::error::{AppError, Result};
use niupanel_common::filesystem::resolve_path;
use niupanel_common::info;
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{tasks, variables, variables_tasks};

#[derive(Debug, Clone)]
pub struct TaskVariableInput {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    pub name: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub description: Option<String>,
    pub env_type: String,
    pub env_version: Option<String>,
    pub tags: Option<serde_json::Value>,
    pub cron_schedule: Option<String>,
    pub requirements: Option<String>,
    pub variables: Option<Vec<TaskVariableInput>>,
    pub notify: Option<bool>,
    pub cpu_limit: Option<i32>,
    pub timeout_sec: Option<i32>,
    pub memory_limit: Option<i32>,
    pub trigger_next_tasks: Option<Vec<i32>>,
    pub random_config: Option<serde_json::Value>,
}

pub async fn create_task_record(
    db: &DatabaseConnection,
    user_id: i32,
    payload: CreateTaskInput,
) -> Result<tasks::Model> {
    info!("创建任务: {}", payload.name);

    let effective_path = effective_task_path(payload.path.as_deref(), payload.command.as_deref());
    let requirements = normalize_requirements(payload.requirements);
    let cpu_limit = normalize_cpu_limit(payload.cpu_limit)?;
    let timeout_sec = normalize_optional_limit(payload.timeout_sec);
    let memory_limit = normalize_optional_limit(payload.memory_limit);
    let (cron_schedule, random_config) =
        sanitize_schedule_fields(payload.cron_schedule, payload.random_config);

    let new_task = tasks::ActiveModel {
        name: Set(payload.name),
        path: Set(effective_path),
        command: Set(payload.command),
        description: Set(payload.description),
        env_type: Set(payload.env_type),
        env_version: Set(payload.env_version),
        tags: Set(payload.tags),
        cron_schedule: Set(cron_schedule),
        requirements: Set(requirements),
        user_id: Set(user_id),
        enabled: Set(true),
        status: Set(TaskStatus::Idle),
        notify: Set(payload.notify.unwrap_or(false)),
        cpu_limit: Set(cpu_limit),
        timeout_sec: Set(timeout_sec),
        memory_limit: Set(memory_limit),
        trigger_next_tasks: Set(payload
            .trigger_next_tasks
            .map(|value| serde_json::to_value(value).unwrap_or(serde_json::json!([])))),
        random_config: Set(random_config),
        ..Default::default()
    };

    let created = new_task.insert(db).await?;

    if let Some(vars) = payload.variables {
        insert_task_variables(db, created.id, vars).await?;
    }

    Ok(created)
}

pub async fn delete_task_records(
    db: &DatabaseConnection,
    ids: Vec<i32>,
    delete_script: bool,
    delete_var: bool,
) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }

    if delete_script {
        let tasks_to_delete = tasks::Entity::find()
            .filter(tasks::Column::Id.is_in(ids.clone()))
            .all(db)
            .await?;
        for task in tasks_to_delete {
            if !task.path.is_empty() {
                if let Ok(path) = resolve_path(&task.path) {
                    let _ = tokio::fs::remove_file(path).await;
                }
            }
        }
    }

    if delete_var {
        let relation_rows = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::TaskId.is_in(ids.clone()))
            .all(db)
            .await?;
        let mut candidate_ids: Vec<i32> = relation_rows.iter().map(|row| row.variable_id).collect();
        let legacy_vars = variables::Entity::find()
            .filter(variables::Column::Scope.eq("Script"))
            .filter(variables::Column::ScopeId.is_in(ids.clone()))
            .all(db)
            .await?;
        candidate_ids.extend(legacy_vars.into_iter().map(|var| var.id));

        variables_tasks::Entity::delete_many()
            .filter(variables_tasks::Column::TaskId.is_in(ids.clone()))
            .exec(db)
            .await?;

        cleanup_orphan_script_variables(db, &candidate_ids).await?;
    }

    tasks::Entity::delete_many()
        .filter(tasks::Column::Id.is_in(ids))
        .exec(db)
        .await?;

    Ok(())
}

pub async fn replace_task_variables(
    db: &DatabaseConnection,
    task_id: i32,
    variables: Vec<TaskVariableInput>,
) -> Result<()> {
    let existing_relations = variables_tasks::Entity::find()
        .filter(variables_tasks::Column::TaskId.eq(task_id))
        .all(db)
        .await?;
    let existing_ids: Vec<i32> = existing_relations
        .iter()
        .map(|row| row.variable_id)
        .collect();

    variables_tasks::Entity::delete_many()
        .filter(variables_tasks::Column::TaskId.eq(task_id))
        .exec(db)
        .await?;
    cleanup_orphan_script_variables(db, &existing_ids).await?;
    insert_task_variables(db, task_id, variables).await
}

pub async fn run_task_records(
    db: &DatabaseConnection,
    task_manager: &TaskManagerService,
    ids: Vec<i32>,
) -> Vec<serde_json::Value> {
    stream::iter(ids)
        .map(|id| {
            let db = db.clone();
            let task_manager = task_manager.clone();
            async move {
                let task = tasks::Entity::find_by_id(id)
                    .one(&db)
                    .await
                    .ok()
                    .flatten()?;
                match task_manager.start_task(&task).await {
                    Ok((pid, run_id)) => Some(serde_json::json!({
                        "task_id": id,
                        "status": "success",
                        "pid": pid,
                        "run_id": run_id,
                        "message": "Task execution started"
                    })),
                    Err(error) => Some(serde_json::json!({
                        "task_id": id,
                        "status": "error",
                        "message": error.to_string()
                    })),
                }
            }
        })
        .buffer_unordered(5)
        .filter_map(|result| async move { result })
        .collect()
        .await
}

pub async fn stop_task_records(task_manager: &TaskManagerService, ids: Vec<i32>) {
    stream::iter(ids)
        .for_each_concurrent(5, |id| {
            let task_manager = task_manager.clone();
            async move {
                task_manager
                    .log_system_event(id, "###### 停止任务 ######".to_string())
                    .await;
                let _ = task_manager.stop_task(id).await;
            }
        })
        .await;
}

pub fn effective_task_path(path: Option<&str>, command: Option<&str>) -> String {
    if path.map(str::is_empty).unwrap_or(true) {
        if let Some(command_str) = command {
            let guessed = extract_script_path_from_command(command_str).unwrap_or_default();
            if resolve_path(&guessed).is_ok() {
                guessed
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        path.unwrap_or_default().to_string()
    }
}

pub fn normalize_requirements(requirements: Option<String>) -> Option<String> {
    requirements.and_then(|content| {
        let normalized = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    })
}

pub fn normalize_optional_limit(value: Option<i32>) -> Option<i32> {
    value.filter(|v| *v > 0)
}

pub fn normalize_cpu_limit(value: Option<i32>) -> Result<Option<i32>> {
    match value.filter(|v| *v > 0) {
        Some(v) if (1..=100).contains(&v) => Ok(Some(v)),
        Some(v) => Err(AppError::ValidationError(format!(
            "CPU 限制必须在 1 到 100 之间，当前值为 {}",
            v
        ))),
        None => Ok(None),
    }
}

pub fn sanitize_schedule_fields(
    cron_schedule: Option<String>,
    random_config: Option<serde_json::Value>,
) -> (Option<String>, Option<serde_json::Value>) {
    if let Some(config) = random_config {
        return (Some(String::new()), Some(config));
    }

    let normalized_cron = cron_schedule.map(|cron| cron.trim().to_string());
    (normalized_cron, None)
}

pub fn extract_script_path_from_command(command: &str) -> Option<String> {
    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut in_quote = false;
    let mut quote_char = '\0';

    for c in command.chars() {
        if in_quote {
            if c == quote_char {
                in_quote = false;
            } else {
                current_part.push(c);
            }
        } else if c == '"' || c == '\'' {
            in_quote = true;
            quote_char = c;
        } else if c.is_whitespace() {
            if !current_part.is_empty() {
                parts.push(current_part.clone());
                current_part.clear();
            }
        } else {
            current_part.push(c);
        }
    }
    if !current_part.is_empty() {
        parts.push(current_part);
    }

    let mut iter = parts.into_iter();
    while let Some(program) = iter.next() {
        if is_env_assignment(&program) || program == "env" || program.starts_with('-') {
            continue;
        }

        if is_script_interpreter(&program) {
            for arg in iter {
                if arg == "run" || arg.starts_with('-') || is_env_assignment(&arg) {
                    continue;
                }
                return Some(arg);
            }
            return Some(program);
        }

        return Some(program);
    }

    None
}

fn is_env_assignment(part: &str) -> bool {
    part.contains('=') && !part.starts_with('/') && !part.starts_with('.')
}

fn is_script_interpreter(program: &str) -> bool {
    [
        "python", "python3", "node", "sh", "bash", "ts-node", "uv", "uvx", "npx",
    ]
    .iter()
    .any(|item| program == *item || program.ends_with(&format!("/{item}")))
}

async fn insert_task_variables(
    db: &DatabaseConnection,
    task_id: i32,
    vars: Vec<TaskVariableInput>,
) -> Result<()> {
    for (index, var) in vars.into_iter().enumerate() {
        let created = variables::ActiveModel {
            key: Set(var.key),
            value: Set(var.value),
            scope: Set("Script".to_string()),
            scope_id: Set(Some(task_id)),
            enabled: Set(true),
            ..Default::default()
        }
        .insert(db)
        .await?;

        variables_tasks::ActiveModel {
            variable_id: Set(created.id),
            task_id: Set(task_id),
            sort_order: Set(index as i32 + 1),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn cleanup_orphan_script_variables(
    db: &DatabaseConnection,
    variable_ids: &[i32],
) -> Result<()> {
    for variable_id in variable_ids {
        let relation = variables_tasks::Entity::find()
            .filter(variables_tasks::Column::VariableId.eq(*variable_id))
            .order_by_asc(variables_tasks::Column::SortOrder)
            .one(db)
            .await?;

        match relation {
            Some(row) => {
                if let Some(var) = variables::Entity::find_by_id(*variable_id).one(db).await? {
                    if var.scope == "Script" && var.scope_id != Some(row.task_id) {
                        let mut active: variables::ActiveModel = var.into();
                        active.scope_id = Set(Some(row.task_id));
                        active.update(db).await?;
                    }
                }
            }
            None => {
                if let Some(var) = variables::Entity::find_by_id(*variable_id).one(db).await? {
                    if var.scope == "Script" {
                        variables::Entity::delete_by_id(*variable_id)
                            .exec(db)
                            .await?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_create_normalizes_requirements_and_limits() {
        assert_eq!(
            normalize_requirements(Some(" axios \n\njsdom\n ".to_string())),
            Some("axios\njsdom".to_string())
        );
        assert_eq!(normalize_requirements(Some(" \n ".to_string())), None);
        assert_eq!(normalize_optional_limit(Some(0)), None);
        assert_eq!(normalize_optional_limit(Some(30)), Some(30));
        assert_eq!(normalize_cpu_limit(Some(100)).expect("limit"), Some(100));
        assert!(normalize_cpu_limit(Some(101)).is_err());
    }

    #[test]
    fn task_create_sanitizes_schedule_fields() {
        assert_eq!(
            sanitize_schedule_fields(Some(" 0 * * * * ".to_string()), None),
            (Some("0 * * * *".to_string()), None)
        );

        let random = serde_json::json!({ "mode": "daily" });
        assert_eq!(
            sanitize_schedule_fields(Some("0 * * * *".to_string()), Some(random.clone())),
            (Some(String::new()), Some(random))
        );
    }

    #[test]
    fn extracts_script_path_from_common_commands() {
        assert_eq!(
            extract_script_path_from_command("node scripts/check.js --once").as_deref(),
            Some("scripts/check.js")
        );
        assert_eq!(
            extract_script_path_from_command("python3 'jobs/sync.py'").as_deref(),
            Some("jobs/sync.py")
        );
        assert_eq!(
            extract_script_path_from_command("env NODE_ENV=prod node app.mjs").as_deref(),
            Some("app.mjs")
        );
        assert_eq!(
            extract_script_path_from_command("/usr/bin/node scripts/check.js").as_deref(),
            Some("scripts/check.js")
        );
    }
}
