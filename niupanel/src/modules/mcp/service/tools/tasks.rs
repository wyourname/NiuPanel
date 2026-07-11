use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = tasks_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List NiuPanel tasks")]
    async fn tasks_list(
        &self,
        Parameters(params): Parameters<TaskListParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskList)?;
        let limit = params.limit.unwrap_or(50).clamp(1, 100);
        let result = TaskService::list_tasks(
            &self.state.db,
            QueryParams {
                page: Some(1),
                page_size: Some(limit),
                q: params.query,
                status: params.status,
            },
        )
        .await
        .map_err(tool_error)?;
        self.audit(&user, "tasks_list", None).await;
        Ok(Json(TaskListOutput {
            total: result.total,
            items: result.items.into_iter().map(task_summary).collect(),
        }))
    }

    #[tool(description = "Get one NiuPanel task by id")]
    async fn tasks_get(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskDetailOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskRead)?;
        let task = tasks::Entity::find_by_id(task_id)
            .one(&self.state.db)
            .await
            .map_err(tool_error)?
            .ok_or_else(|| ErrorData::invalid_params("Task not found", None))?;
        self.audit(&user, "tasks_get", Some(task_id.to_string()))
            .await;
        Ok(Json(task_detail(task)))
    }

    #[tool(description = "Read the tail of the latest task log")]
    async fn tasks_get_logs(
        &self,
        Parameters(params): Parameters<TaskLogParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskLogOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskRead)?;
        let tail_bytes = params.tail_bytes.unwrap_or(64 * 1024).clamp(1, 1024 * 1024);
        let log = TaskService::get_latest_log(
            &self.state.task_manager,
            params.task_id,
            LogPagination {
                offset: None,
                limit: Some(tail_bytes),
            },
        )
        .await
        .map_err(tool_error)?;
        self.audit(&user, "tasks_get_logs", Some(params.task_id.to_string()))
            .await;
        Ok(Json(TaskLogOutput {
            task_id: params.task_id,
            content: log.content,
            total_size: log.total_size,
            offset: log.offset,
            length: log.length,
        }))
    }

    #[tool(description = "List recent execution history for one NiuPanel task")]
    async fn tasks_history(
        &self,
        Parameters(params): Parameters<TaskHistoryParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskHistoryOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskRead)?;
        let limit = params.limit.unwrap_or(20).clamp(1, 100);
        let history = TaskService::list_task_history(
            &self.state.db,
            params.task_id,
            QueryParams {
                page: Some(1),
                page_size: Some(limit),
                q: None,
                status: None,
            },
        )
        .await
        .map_err(tool_error)?;
        self.audit(&user, "tasks_history", Some(params.task_id.to_string()))
            .await;
        Ok(Json(TaskHistoryOutput {
            total: history.total,
            items: history.items.into_iter().map(task_run).collect(),
        }))
    }

    #[tool(description = "Read the tail of a specific historical task run log")]
    async fn tasks_get_run_log(
        &self,
        Parameters(params): Parameters<TaskRunLogParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskLogOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskRead)?;
        let tail_bytes = params.tail_bytes.unwrap_or(64 * 1024).clamp(1, 1024 * 1024);
        let log = TaskService::get_task_run_log(
            &self.state.db,
            params.task_id,
            params.run_id,
            LogPagination {
                offset: None,
                limit: Some(tail_bytes),
            },
        )
        .await
        .map_err(tool_error)?;
        self.audit(
            &user,
            "tasks_get_run_log",
            Some(format!("{}:{}", params.task_id, params.run_id)),
        )
        .await;
        Ok(Json(TaskLogOutput {
            task_id: params.task_id,
            content: log.content,
            total_size: log.total_size,
            offset: log.offset,
            length: log.length,
        }))
    }

    #[tool(description = "Create a NiuPanel task")]
    async fn tasks_create(
        &self,
        Parameters(params): Parameters<TaskCreateParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskDetailOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskCreate)?;
        let task = TaskUseCase::from_state(&self.state)
            .create_task(
                &user,
                CreateTaskRequest {
                    name: params.name,
                    path: params.path,
                    command: params.command,
                    description: params.description,
                    env_type: params.env_type,
                    env_version: params.env_version,
                    tags: None,
                    cron_schedule: params.cron_schedule,
                    requirements: params.requirements,
                    variables: None,
                    notify: params.notify,
                    cpu_limit: params.cpu_limit,
                    timeout_sec: params.timeout_sec,
                    memory_limit: params.memory_limit,
                    trigger_next_tasks: params.trigger_next_tasks,
                    random_config: None,
                },
            )
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_create", Some(task.id.to_string()))
            .await;
        Ok(Json(task_detail(task)))
    }

    #[tool(description = "Update a NiuPanel task")]
    async fn tasks_update(
        &self,
        Parameters(params): Parameters<TaskUpdateParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskDetailOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskUpdate)?;
        let task_id = params.task_id;
        let task = TaskUseCase::from_state(&self.state)
            .update_task(
                &user,
                task_id,
                UpdateTaskRequest {
                    name: params.name,
                    path: params.path,
                    command: params.command,
                    description: params.description,
                    env_type: params.env_type,
                    env_version: params.env_version,
                    tags: None,
                    cron_schedule: params.cron_schedule,
                    requirements: params.requirements,
                    variables: None,
                    notify: params.notify,
                    enabled: params.enabled,
                    cpu_limit: params.cpu_limit,
                    timeout_sec: params.timeout_sec,
                    memory_limit: params.memory_limit,
                    trigger_next_tasks: params.trigger_next_tasks,
                    random_config: None,
                },
            )
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_update", Some(task_id.to_string()))
            .await;
        Ok(Json(task_detail(task)))
    }

    #[tool(description = "Delete a NiuPanel task. This is a destructive operation")]
    async fn tasks_delete(
        &self,
        Parameters(params): Parameters<TaskDeleteParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskDelete)?;
        TaskUseCase::from_state(&self.state)
            .batch_delete_tasks(
                &user,
                vec![params.task_id],
                params.delete_script.unwrap_or(false),
                params.delete_variables.unwrap_or(false),
            )
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_delete", Some(params.task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id: params.task_id,
            accepted: true,
            message: "Task deleted".to_string(),
        }))
    }

    #[tool(description = "Start a NiuPanel task")]
    async fn tasks_run(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskRun)?;
        TaskUseCase::from_state(&self.state)
            .batch_run_tasks(&user, vec![task_id])
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_run", Some(task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id,
            accepted: true,
            message: "Task start request accepted".to_string(),
        }))
    }

    #[tool(description = "Stop a running NiuPanel task")]
    async fn tasks_stop(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskStop)?;
        TaskUseCase::from_state(&self.state)
            .batch_stop_tasks(&user, vec![task_id])
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_stop", Some(task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id,
            accepted: true,
            message: "Task stop request accepted".to_string(),
        }))
    }

    #[tool(description = "Pause a running NiuPanel task")]
    async fn tasks_pause(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskStop)?;
        TaskUseCase::from_state(&self.state)
            .batch_pause_tasks(&user, vec![task_id])
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_pause", Some(task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id,
            accepted: true,
            message: "Task pause request accepted".to_string(),
        }))
    }

    #[tool(description = "Resume a paused NiuPanel task")]
    async fn tasks_resume(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskStop)?;
        TaskUseCase::from_state(&self.state)
            .batch_resume_tasks(&user, vec![task_id])
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_resume", Some(task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id,
            accepted: true,
            message: "Task resume request accepted".to_string(),
        }))
    }

    #[tool(description = "Enable scheduling for a NiuPanel task")]
    async fn tasks_enable(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskUpdate)?;
        TaskUseCase::from_state(&self.state)
            .batch_set_enabled(&user, vec![task_id], true)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_enable", Some(task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id,
            accepted: true,
            message: "Task enabled".to_string(),
        }))
    }

    #[tool(description = "Disable scheduling for a NiuPanel task")]
    async fn tasks_disable(
        &self,
        Parameters(TaskIdParams { task_id }): Parameters<TaskIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<TaskActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::TaskUpdate)?;
        TaskUseCase::from_state(&self.state)
            .batch_set_enabled(&user, vec![task_id], false)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "tasks_disable", Some(task_id.to_string()))
            .await;
        Ok(Json(TaskActionOutput {
            task_id,
            accepted: true,
            message: "Task disabled".to_string(),
        }))
    }
}
