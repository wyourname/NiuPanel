use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

use super::{TaskService, infer_env_type};
use crate::modules::tasks::models::{CreateTaskRequest, UpdateTaskRequest};
use niupanel_common::error::{AppError, Result};
use niupanel_common::filesystem::resolve_path;
use niupanel_core::task::{
    CreateTaskInput, TaskVariableInput, create_task_record, delete_task_records,
    extract_script_path_from_command, normalize_cpu_limit, normalize_optional_limit,
    normalize_requirements, replace_task_variables, sanitize_schedule_fields,
};
use niupanel_entity::tasks;

impl From<CreateTaskRequest> for CreateTaskInput {
    fn from(value: CreateTaskRequest) -> Self {
        Self {
            name: value.name,
            path: value.path,
            command: value.command,
            description: value.description,
            env_type: value.env_type,
            env_version: value.env_version,
            tags: value.tags,
            cron_schedule: value.cron_schedule,
            requirements: value.requirements,
            variables: value.variables.map(|variables| {
                variables
                    .into_iter()
                    .map(|variable| TaskVariableInput {
                        key: variable.key,
                        value: variable.value,
                    })
                    .collect()
            }),
            notify: value.notify,
            cpu_limit: value.cpu_limit,
            timeout_sec: value.timeout_sec,
            memory_limit: value.memory_limit,
            trigger_next_tasks: value.trigger_next_tasks,
            random_config: value.random_config,
        }
    }
}

impl TaskService {
    pub async fn create_task(
        db: &DatabaseConnection,
        user_id: i32,
        payload: CreateTaskRequest,
    ) -> Result<tasks::Model> {
        create_task_record(db, user_id, payload.into()).await
    }

    pub async fn update_task(
        db: &DatabaseConnection,
        id: i32,
        payload: UpdateTaskRequest,
    ) -> Result<tasks::Model> {
        let task = tasks::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;

        let mut task_active_model: tasks::ActiveModel = task.into();

        if let Some(name) = payload.name {
            task_active_model.name = Set(name);
        }
        if let Some(command) = payload.command {
            let path_is_empty_or_none = payload
                .path
                .as_ref()
                .map(|path| path.is_empty())
                .unwrap_or(true);
            if path_is_empty_or_none {
                if let Some(guessed) = extract_script_path_from_command(&command) {
                    if resolve_path(&guessed).is_ok() {
                        task_active_model.path = Set(guessed);
                    }
                }
            }
            task_active_model.command = Set(Some(command));
        }

        if let Some(path) = payload.path {
            if !path.is_empty() {
                let _ = resolve_path(&path)?;
                task_active_model.path = Set(path);
            }
        }
        if let Some(description) = payload.description {
            task_active_model.description = Set(Some(description));
        }
        if let Some(env_type) = payload.env_type {
            task_active_model.env_type = Set(env_type);
        }
        if let Some(env_version) = payload.env_version {
            task_active_model.env_version = Set(Some(env_version));
        }
        if let Some(tags) = payload.tags {
            task_active_model.tags = Set(Some(tags));
        }
        let should_update_cron = payload.cron_schedule.is_some();
        let explicit_random_config = payload.random_config;
        let random_config_for_sanitization = explicit_random_config.clone().flatten();
        let (sanitized_cron, sanitized_random) =
            sanitize_schedule_fields(payload.cron_schedule, random_config_for_sanitization);

        if let Some(requirements) = payload.requirements {
            task_active_model.requirements = Set(normalize_requirements(Some(requirements)));
        }
        if let Some(notify) = payload.notify {
            task_active_model.notify = Set(notify);
        }
        if let Some(enabled) = payload.enabled {
            task_active_model.enabled = Set(enabled);
        }
        if payload.cpu_limit.is_some() {
            task_active_model.cpu_limit = Set(normalize_cpu_limit(payload.cpu_limit)?);
        }
        if payload.timeout_sec.is_some() {
            task_active_model.timeout_sec = Set(normalize_optional_limit(payload.timeout_sec));
        }
        if let Some(memory_limit) = payload.memory_limit {
            task_active_model.memory_limit = Set(normalize_optional_limit(Some(memory_limit)));
        }
        if let Some(triggers) = payload.trigger_next_tasks {
            task_active_model.trigger_next_tasks = Set(Some(
                serde_json::to_value(triggers).unwrap_or(serde_json::json!([])),
            ));
        }
        if should_update_cron || explicit_random_config.is_some() {
            task_active_model.cron_schedule = Set(sanitized_cron);
        }
        if let Some(random_config) = explicit_random_config {
            task_active_model.random_config = Set(match random_config {
                Some(_) => sanitized_random,
                None => None,
            });
        }

        let updated_task = task_active_model.update(db).await?;

        if let Some(vars) = payload.variables {
            let variables = vars
                .into_iter()
                .map(|variable| TaskVariableInput {
                    key: variable.key,
                    value: variable.value,
                })
                .collect();
            replace_task_variables(db, id, variables).await?;
        }

        Ok(updated_task)
    }

    pub async fn quick_create_task_from_url(
        db: &DatabaseConnection,
        http_client: &reqwest::Client,
        user_id: i32,
        url: String,
    ) -> Result<tasks::Model> {
        let path_part = url.split('?').next().unwrap_or("");
        let filename = path_part
            .split('/')
            .next_back()
            .unwrap_or("downloaded_task_script")
            .to_string();
        let target_path = resolve_path(&filename)?;

        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(AppError::Io)?;
            }
        }

        let response = http_client
            .get(&url)
            .send()
            .await
            .map_err(|error| AppError::Generic(error.to_string()))?;
        if !response.status().is_success() {
            return Err(AppError::Generic(format!(
                "Download failed: {}",
                response.status()
            )));
        }

        let content = response
            .bytes()
            .await
            .map_err(|error| AppError::Generic(error.to_string()))?;
        tokio::fs::write(&target_path, &content)
            .await
            .map_err(AppError::Io)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = tokio::fs::metadata(&target_path).await {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = tokio::fs::set_permissions(&target_path, perms).await;
            }
        }

        let env_type = infer_env_type(&filename, &content);
        let env_version = if env_type == "Python" {
            Some("3.11".to_string())
        } else {
            None
        };
        let min = (chrono::Utc::now().timestamp() % 60) as u8;
        let hour = (chrono::Utc::now().timestamp() / 60 % 24) as u8;
        let random_cron = format!("0 {} {} * * *", min, hour);

        let create_req = CreateTaskRequest {
            name: filename.clone(),
            path: Some(filename.clone()),
            command: None,
            description: Some(format!("Created from URL: {}", url)),
            env_type: env_type.to_string(),
            env_version,
            tags: None,
            cron_schedule: Some(random_cron),
            requirements: None,
            variables: None,
            notify: None,
            cpu_limit: None,
            timeout_sec: None,
            memory_limit: None,
            trigger_next_tasks: None,
            random_config: None,
        };

        Self::create_task(db, user_id, create_req).await
    }

    pub async fn batch_delete_tasks(
        db: &DatabaseConnection,
        ids: Vec<i32>,
        delete_script: bool,
        delete_var: bool,
    ) -> Result<()> {
        delete_task_records(db, ids, delete_script, delete_var).await
    }
}
