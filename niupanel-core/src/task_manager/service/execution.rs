use super::{TaskManagerService, filter_args, task_run_log_key};
use crate::script::exec::ScriptType;
use crate::script::interpreter::node::NodeEnvironment;
use crate::settings::SettingsManager;
use crate::task_log::LogSession;
use niupanel_common::config::Config;
use niupanel_common::constants::settings::{
    FNM_NODE_DIST_MIRROR, NPM_REGISTRY_MIRROR, SYSTEM_DEFAULT_NODE_VERSION,
    SYSTEM_DEFAULT_PYTHON_VERSION, UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
};
use niupanel_common::error::AppError;
use niupanel_common::filesystem::resolve_path;
use niupanel_common::{debug, error as error_msg, info, warn};
use niupanel_entity::task_status::TaskStatus;
use niupanel_entity::{task_runs, tasks};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::HashMap;

impl TaskManagerService {
    pub async fn start_task(&self, task: &tasks::Model) -> Result<(u32, i32), AppError> {
        if self.process_manager.is_running(task.id) {
            return Err(AppError::Generic(format!(
                "任务 '{}' (ID: {}) 已经在运行中，请等待任务结束",
                task.name, task.id
            )));
        }

        let permit = self
            .running_permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                AppError::ConcurrencyLimitExceeded(
                    "到达系统设定的并发限制数量,可以前往系统设置进行修改".to_string(),
                )
            })?;
        debug!(
            "当前 可允许的并发数: {}",
            self.running_permits.available_permits()
        );

        let new_run = task_runs::ActiveModel {
            task_id: Set(task.id),
            started_at: Set(chrono::Utc::now().into()),
            status: Set(TaskStatus::Running),
            ..Default::default()
        };
        let mut run_record = new_run.insert(&self.db).await?;

        let log_session =
            match LogSession::create_task_run(&Config::global().logs_dir, task.id, run_record.id)
                .await
            {
                Ok(session) => session,
                Err(e) => {
                    let run_update = task_runs::ActiveModel {
                        id: Set(run_record.id),
                        status: Set(TaskStatus::Failed),
                        ended_at: Set(Some(chrono::Utc::now().into())),
                        ..Default::default()
                    };
                    let _ = run_update.update(&self.db).await;
                    return Err(e);
                }
            };
        self.log_sessions
            .insert(task_run_log_key(run_record.id), log_session.clone());
        self.active_task_runs.insert(task.id, run_record.id);

        let run_update = task_runs::ActiveModel {
            id: Set(run_record.id),
            log_path: Set(Some(log_session.path_string())),
            ..Default::default()
        };
        run_record = run_update.update(&self.db).await?;
        self.log_system_event(task.id, "###### 启动任务 ######".to_string())
            .await;

        let env_vars = match self.prepare_task_env_vars(task.id).await {
            Ok(v) => v,
            Err(e) => {
                self.handle_execution_error(task.id, e.to_string(), Some(run_record.id))
                    .await;
                return Err(e);
            }
        };

        self.update_task_status(task.id, TaskStatus::Running, Some(run_record.id))
            .await?;

        let (script_path, args) = match self.resolve_execution_config(task) {
            Ok(res) => res,
            Err(e) => {
                self.handle_execution_error(task.id, e.to_string(), Some(run_record.id))
                    .await;
                return Err(e);
            }
        };
        debug!(
            "Executing task id:{} - task name:{} args:{:?}",
            task.id, task.name, args
        );

        let mirrors = self.runtime_mirrors().await;
        let env_version = match self
            .resolve_task_runtime_version(&task.env_type, task.env_version.as_deref())
            .await
        {
            Ok(version) => version,
            Err(e) => {
                self.handle_execution_error(task.id, e.to_string(), Some(run_record.id))
                    .await;
                return Err(e);
            }
        };

        let executor = crate::script::exec::ScriptExecutor::new(
            crate::script::exec::ExecutionContext {
                task_id: task.id,
                task_run_id: Some(run_record.id),
                process_manager: self.process_manager.clone(),
                permit,
                log_session: log_session.clone(),
            },
            crate::script::exec::ResourceLimits {
                cpu_limit: task.cpu_limit,
                timeout_sec: task.timeout_sec,
                memory: task.memory_limit,
            },
            Some(mirrors),
        );

        let config = crate::script::exec::ScriptConfig {
            path: script_path,
            args,
            env_type: task.env_type.clone(),
            env_version,
            requirements: task.requirements.clone(),
            env_vars,
        };

        let pid = match executor.execute(config).await {
            Ok(res) => res,
            Err(e) => {
                tracing::error!("启动任务 {} 失败: {}", task.id, e);
                self.handle_execution_error(task.id, e.to_string(), Some(run_record.id))
                    .await;
                return Err(e);
            }
        };

        let run_id = run_record.id;
        let mut run_active: task_runs::ActiveModel = run_record.into();
        run_active.pid = Set(Some(pid as i32));
        run_active.update(&self.db).await?;

        tracing::info!("任务 {} 已启动，PID {}", task.id, pid);
        Ok((pid, run_id))
    }

    async fn prepare_task_env_vars(
        &self,
        task_id: i32,
    ) -> Result<HashMap<String, String>, AppError> {
        let global_vars = niupanel_entity::variables::Entity::find()
            .filter(niupanel_entity::variables::Column::Scope.eq("Global"))
            .filter(niupanel_entity::variables::Column::ScopeId.is_null())
            .order_by_asc(niupanel_entity::variables::Column::Key)
            .order_by_asc(niupanel_entity::variables::Column::Id)
            .all(&self.db)
            .await?;
        let relation_rows = niupanel_entity::variables_tasks::Entity::find()
            .filter(niupanel_entity::variables_tasks::Column::TaskId.eq(task_id))
            .order_by_asc(niupanel_entity::variables_tasks::Column::SortOrder)
            .order_by_asc(niupanel_entity::variables_tasks::Column::VariableId)
            .all(&self.db)
            .await?;
        let relation_ids: Vec<i32> = relation_rows.iter().map(|row| row.variable_id).collect();
        let linked_vars = if relation_ids.is_empty() {
            vec![]
        } else {
            niupanel_entity::variables::Entity::find()
                .filter(niupanel_entity::variables::Column::Id.is_in(relation_ids.clone()))
                .all(&self.db)
                .await?
        };
        let mut linked_map: HashMap<i32, niupanel_entity::variables::Model> =
            linked_vars.into_iter().map(|v| (v.id, v)).collect();
        let mut script_vars = Vec::new();
        for variable_id in relation_ids {
            if let Some(var) = linked_map.remove(&variable_id) {
                script_vars.push(var);
            }
        }
        let mut legacy_query = niupanel_entity::variables::Entity::find()
            .filter(niupanel_entity::variables::Column::Scope.eq("Script"))
            .filter(niupanel_entity::variables::Column::ScopeId.eq(Some(task_id)));
        if !script_vars.is_empty() {
            let existing_ids: Vec<i32> = script_vars.iter().map(|v| v.id).collect();
            legacy_query =
                legacy_query.filter(niupanel_entity::variables::Column::Id.is_not_in(existing_ids));
        }
        let legacy_vars = legacy_query
            .order_by_asc(niupanel_entity::variables::Column::Id)
            .all(&self.db)
            .await?;
        script_vars.extend(legacy_vars);

        let mut env_map = HashMap::new();
        let mut insert_or_append = |key: String, value: String| {
            env_map
                .entry(key)
                .and_modify(|v: &mut String| *v = format!("{}\n{}", v, value))
                .or_insert(value);
        };

        for var in global_vars {
            if var.enabled {
                insert_or_append(var.key, var.value);
            }
        }

        for var in script_vars {
            if var.enabled {
                insert_or_append(var.key, var.value);
            }
        }

        Ok(env_map)
    }

    async fn runtime_mirrors(&self) -> HashMap<String, String> {
        let mut mirrors = HashMap::new();

        for (setting_key, env_key) in [
            (UV_PYTHON_MIRROR, "UV_PYTHON_INSTALL_MIRROR"),
            (UV_PYPI_MIRROR, "UV_INDEX_URL"),
            (FNM_NODE_DIST_MIRROR, "FNM_NODE_DIST_MIRROR"),
            (NPM_REGISTRY_MIRROR, "npm_config_registry"),
        ] {
            if let Ok(value) = SettingsManager::get(&self.db, setting_key).await {
                if !value.trim().is_empty() {
                    mirrors.insert(env_key.to_string(), value);
                }
            }
        }

        mirrors
    }

    async fn resolve_task_runtime_version(
        &self,
        env_type: &str,
        task_version: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        match ScriptType::try_from_str(env_type)? {
            ScriptType::Python => self.resolve_python_version(task_version).await.map(Some),
            ScriptType::NodeJs => self.resolve_node_version(task_version).await.map(Some),
            ScriptType::Shell => Ok(None),
        }
    }

    async fn resolve_python_version(&self, task_version: Option<&str>) -> Result<String, AppError> {
        if let Some(version) = normalize_python_version(task_version) {
            return Ok(version);
        }

        let default_version = SettingsManager::get(&self.db, SYSTEM_DEFAULT_PYTHON_VERSION).await?;
        normalize_python_version(Some(&default_version)).ok_or_else(|| {
            AppError::ValidationError(
                "Python version must be specified. Set a task version or system default Python version."
                    .to_string(),
            )
        })
    }

    async fn resolve_node_version(&self, task_version: Option<&str>) -> Result<String, AppError> {
        if let Some(version) = task_version.and_then(NodeEnvironment::normalize_version) {
            return Ok(version);
        }

        let default_version = SettingsManager::get(&self.db, SYSTEM_DEFAULT_NODE_VERSION).await?;
        NodeEnvironment::normalize_version(&default_version).ok_or_else(|| {
            AppError::ValidationError(
                "Node version must be specified. Set a task version or system default Node version."
                    .to_string(),
            )
        })
    }

    fn resolve_execution_config(
        &self,
        task: &tasks::Model,
    ) -> Result<(std::path::PathBuf, Vec<String>), AppError> {
        debug!("Resolving path for script: {}", task.path);
        let script_path = resolve_path(&task.path).map_err(|e| {
            tracing::error!("Failed to resolve script path for task {}: {}", task.id, e);
            AppError::Generic(format!("无法找到脚本文件: {} ({})", task.path, e))
        })?;
        debug!("Script path: {}", script_path.display());

        let args = if let Some(cmd_str) = &task.command {
            let parsed_args = shell_words::split(cmd_str).map_err(|e| {
                tracing::error!(
                    "Failed to parse command arguments for task id:{} - task name:{}【{}】",
                    task.id,
                    task.name,
                    e
                );
                AppError::Generic(format!("Command argument parsing failed: {}", e))
            })?;
            filter_args(&parsed_args, &script_path)
        } else {
            Vec::new()
        };

        Ok((script_path, args))
    }

    pub async fn stop_task(&self, task_id: i32) -> Result<(), AppError> {
        match self.process_manager.stop(task_id, None).await {
            Ok(_) => Ok(()),
            Err(AppError::NotFound(_)) => {
                warn!("停止任务时未找到ID {}，标记为已停止。", task_id);
                self.update_task_status(task_id, TaskStatus::Stopped, None)
                    .await
            }
            Err(e) => Err(e),
        }
    }

    pub async fn pause_task(&self, task_id: i32) -> Result<(), AppError> {
        self.process_manager.pause(task_id).await?;
        Ok(())
    }

    pub async fn resume_task(&self, task_id: i32) -> Result<(), AppError> {
        self.process_manager.resume(task_id).await?;
        Ok(())
    }

    async fn handle_execution_error(
        &self,
        task_id: i32,
        error_msg_input: impl Into<String>,
        run_id: Option<i32>,
    ) {
        let msg = error_msg_input.into();
        error_msg!("任务 {} 执行失败: {}", task_id, msg);

        let formatted_msg = format!(
            "Startup Failed: {}\n时间:{}\n",
            msg,
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        let fallback_log_path = if self
            .write_task_log_raw(task_id, run_id, formatted_msg.clone())
            .await
            .is_some()
        {
            None
        } else {
            let timestamp_date = chrono::Local::now()
                .format("%Y-%m-%d_%H-%M-%S%.3f")
                .to_string();
            let log_dir =
                std::path::Path::new(&Config::global().logs_dir).join(task_id.to_string());
            let _ = tokio::fs::create_dir_all(&log_dir).await;
            let log_file_path = match run_id {
                Some(rid) => log_dir.join(format!("{}_run-{}.log", timestamp_date, rid)),
                None => log_dir.join(format!("{}_startup-failed.log", timestamp_date)),
            };

            if let Ok(mut file) = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)
                .await
            {
                use tokio::io::AsyncWriteExt;
                let _ = file.write_all(formatted_msg.as_bytes()).await;
                let _ = file.flush().await;
            }

            Some(log_file_path)
        };

        if let (Some(rid), Some(log_file_path)) = (run_id, fallback_log_path) {
            let run_update = task_runs::ActiveModel {
                id: Set(rid),
                log_path: Set(Some(log_file_path.to_string_lossy().to_string())),
                ..Default::default()
            };
            let _ = run_update.update(&self.db).await;
        }

        let _ = self
            .update_task_status(task_id, TaskStatus::Failed, run_id)
            .await;
    }

    pub(crate) async fn check_and_trigger_next_tasks(&self, task_id: i32) -> Result<(), AppError> {
        let task = tasks::Entity::find_by_id(task_id).one(&self.db).await?;

        if let Some(t) = task {
            if let Some(triggers_json) = t.trigger_next_tasks {
                if let Ok(next_ids) = serde_json::from_value::<Vec<i32>>(triggers_json) {
                    for next_id in next_ids {
                        if let Some(next_task) =
                            tasks::Entity::find_by_id(next_id).one(&self.db).await?
                        {
                            if next_task.enabled {
                                info!(
                                    "任务 {} 完成，自动触发后续任务: {} ({})",
                                    t.name, next_task.name, next_id
                                );
                                if let Err(e) = self.start_task(&next_task).await {
                                    error_msg!("自动触发任务 {} 失败: {}", next_id, e);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn normalize_python_version(version: Option<&str>) -> Option<String> {
    let trimmed = version?.trim();
    if trimmed.is_empty()
        || trimmed == "system"
        || trimmed == "default"
        || trimmed == "venv_default"
    {
        return None;
    }

    Some(
        trimmed
            .strip_prefix("venv_")
            .unwrap_or(trimmed)
            .split_whitespace()
            .next()
            .unwrap_or(trimmed)
            .to_string(),
    )
    .filter(|value| !value.is_empty())
}
