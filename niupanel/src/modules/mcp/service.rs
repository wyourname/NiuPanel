use super::models::{
    AuditListOutput, AuditListParams, AuditOutput, CoreReleaseListOutput, CoreReleaseOutput,
    EnvironmentActionOutput, EnvironmentCreateParams, EnvironmentDeleteParams,
    EnvironmentGetParams, EnvironmentInstallPackagesParams, EnvironmentJobOutput,
    EnvironmentListOutput, EnvironmentOutput, EnvironmentPackagesOutput, EnvironmentPackagesParams,
    EnvironmentSetDefaultParams, EnvironmentUninstallPackageParams, EnvironmentVersionsOutput,
    FileActionOutput, FileContentOutput, FileItemOutput, FileListOutput, FileListParams,
    FilePathParams, FileWriteParams, GitDiscoveredTaskListOutput, GitDiscoveredTaskOutput,
    GitFileListOutput, GitFileOutput, GitRepoFilesParams, GitRepoIdParams, GitRepoListOutput,
    GitRepoOutput, GitSyncOutput, JobActionOutput, JobIdParams, JobListOutput, JobListParams,
    JobLogOutput, JobLogParams, JobOutput, ManagedEnvironmentRuntime, PackageRuntime,
    ShareImportSourceOutput, ShareImportSourcesOutput, ShareStationFileOutput,
    ShareStationFilesOutput, ShareStationStatsOutput, SystemStatusOutput, TaskActionOutput,
    TaskCreateParams, TaskDeleteParams, TaskDetailOutput, TaskHistoryOutput, TaskHistoryParams,
    TaskIdParams, TaskListOutput, TaskListParams, TaskLogOutput, TaskLogParams, TaskRunLogParams,
    TaskRunOutput, TaskSummaryOutput, TaskUpdateParams, UpdateCheckOutput, VariableActionOutput,
    VariableCreateParams, VariableIdParams, VariableListOutput, VariableListParams, VariableOutput,
    VariableUpdateParams, WebReleaseListOutput, WebReleaseOutput, WebhookPushOutput,
    WebhookPushParams,
};
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use crate::modules::environment::models::{CreateEnvRequest, InstallPackageRequest};
use crate::modules::environment::usecase::EnvironmentUseCase;
use crate::modules::file_manager::{
    models::{CreateDirectoryRequest, WriteFileRequest},
    service::FileManagerService,
};
use crate::modules::git_sync::service::GitService;
use crate::modules::share::service as share_service;
use crate::modules::system;
use crate::modules::tasks::models::{
    CreateTaskRequest, LogPagination, QueryParams, UpdateTaskRequest,
};
use crate::modules::tasks::service::TaskService;
use crate::modules::tasks::usecase::TaskUseCase;
use crate::modules::variable::service as variable_service;
use crate::modules::variable::{models::VariableQueryParams, models::VariableRequest};
use niupanel_common::auth::permissions::Permission;
use niupanel_common::error::AppError;
use niupanel_core::audit::service::AuditService;
use niupanel_core::event_bus::{SystemEvent, SystemNotification};
use niupanel_entity::{
    audit_logs, system_jobs, task_runs, task_status::TaskStatus, tasks, variables, variables_tasks,
};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{ServerCapabilities, ServerInfo},
    service::RequestContext,
    tool_handler,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

#[derive(Clone)]
pub struct PanelMcpServer {
    state: AppState,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl PanelMcpServer {
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    fn user_for(
        context: &RequestContext<RoleServer>,
        permission: Permission,
    ) -> Result<AuthenticatedUser, ErrorData> {
        let parts = context
            .extensions
            .get::<http::request::Parts>()
            .ok_or_else(|| ErrorData::internal_error("Missing HTTP request context", None))?;
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| ErrorData::invalid_request("MCP authentication is required", None))?;
        if !user.has_permission(Permission::McpConnect) {
            return Err(ErrorData::invalid_request(
                "API key is missing mcp:connect permission",
                None,
            ));
        }
        if !user.has_permission(permission) {
            return Err(ErrorData::invalid_request(
                format!("API key is missing {} permission", permission),
                None,
            ));
        }
        Ok(user)
    }

    async fn audit(&self, user: &AuthenticatedUser, tool: &str, resource_id: Option<String>) {
        AuditService::log_user(
            &self.state.db,
            user,
            "MCP工具调用",
            "mcp",
            resource_id,
            Some(format!("调用工具: {tool}")),
        )
        .await;
    }
}

fn task_status(status: TaskStatus) -> String {
    match status {
        TaskStatus::Pending => "Pending",
        TaskStatus::Running => "Running",
        TaskStatus::Finished => "Finished",
        TaskStatus::Failed => "Failed",
        TaskStatus::Cancelled => "Cancelled",
        TaskStatus::Stopped => "Stopped",
        TaskStatus::Paused => "Paused",
        TaskStatus::Idle => "Idle",
    }
    .to_string()
}

fn task_summary(task: tasks::Model) -> TaskSummaryOutput {
    TaskSummaryOutput {
        id: task.id,
        name: task.name,
        status: task_status(task.status),
        enabled: task.enabled,
        env_type: task.env_type,
        env_version: task.env_version,
        path: task.path,
        next_run_at: task.next_run_at.map(|value| value.to_rfc3339()),
    }
}

fn task_detail(task: tasks::Model) -> TaskDetailOutput {
    TaskDetailOutput {
        id: task.id,
        name: task.name,
        description: task.description,
        status: task_status(task.status),
        enabled: task.enabled,
        path: task.path,
        command: task.command,
        env_type: task.env_type,
        env_version: task.env_version,
        cron_schedule: task.cron_schedule,
        pid: task.pid,
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
    }
}

fn variable_output(
    variable: variables::Model,
    task_ids: Vec<i32>,
    include_value: bool,
) -> VariableOutput {
    VariableOutput {
        id: variable.id,
        key: variable.key,
        value: include_value.then_some(variable.value),
        remarks: variable.remarks,
        enabled: variable.enabled,
        scope: variable.scope,
        task_ids,
        created_at: variable.created_at.to_rfc3339(),
        updated_at: variable.updated_at.to_rfc3339(),
    }
}

fn task_run(run: task_runs::Model) -> TaskRunOutput {
    TaskRunOutput {
        id: run.id,
        task_id: run.task_id,
        status: task_status(run.status),
        started_at: run.started_at.to_rfc3339(),
        ended_at: run.ended_at.map(|value| value.to_rfc3339()),
        has_log: run.log_path.is_some(),
        pid: run.pid,
    }
}

fn job_output(job: system_jobs::Model) -> JobOutput {
    JobOutput {
        id: job.id,
        name: job.name,
        description: job.description,
        status: task_status(job.status),
        created_at: job.created_at.to_rfc3339(),
        ended_at: job.ended_at.map(|value| value.to_rfc3339()),
        metadata: job.metadata,
        has_log: job.log_path.is_some(),
    }
}

fn git_repo_output(repo: niupanel_entity::git_repositories::Model) -> GitRepoOutput {
    GitRepoOutput {
        id: repo.id,
        name: repo.name,
        repo_url: repo.repo_url,
        branch: repo.branch,
        auto_sync: repo.auto_sync,
        last_sync_at: repo.last_sync_at.map(|value| value.to_rfc3339()),
        last_sync_status: repo.last_sync_status,
        last_sync_message: repo.last_sync_message,
        current_commit: repo.current_commit,
    }
}

fn release_summary(body: &str) -> Option<String> {
    let body = body.trim();
    if body.is_empty() {
        return None;
    }
    Some(body.chars().take(2_000).collect())
}

fn tool_error(error: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(error.to_string(), None)
}

fn required_environment_name(name: &Option<String>) -> Result<&str, ErrorData> {
    name.as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            ErrorData::invalid_params("name is required for python and node environments", None)
        })
}

fn required_value(value: String, field: &str) -> Result<String, ErrorData> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ErrorData::invalid_params(
            format!("{field} must not be empty"),
            None,
        ));
    }
    Ok(value.to_string())
}

fn required_packages(packages: Vec<String>) -> Result<Vec<String>, ErrorData> {
    let packages = packages
        .into_iter()
        .map(|package| package.trim().to_string())
        .filter(|package| !package.is_empty())
        .collect::<Vec<_>>();
    if packages.is_empty() {
        return Err(ErrorData::invalid_params(
            "packages must contain at least one package",
            None,
        ));
    }
    Ok(packages)
}

mod tools;

impl PanelMcpServer {
    fn tool_router() -> ToolRouter<Self> {
        Self::system_tool_router()
            + Self::tasks_tool_router()
            + Self::environment_tool_router()
            + Self::variable_tool_router()
            + Self::audit_tool_router()
            + Self::file_tool_router()
            + Self::job_tool_router()
            + Self::notification_tool_router()
            + Self::share_tool_router()
            + Self::git_tool_router()
    }
}

#[tool_handler]
impl ServerHandler for PanelMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Manage this NiuPanel instance through task, variable, script file, environment, share, Git, release, system job, audit, and notification tools. Every request requires X-API-Key, mcp:connect, and the permission declared by the selected tool. Destructive tools should only be called after explicit user confirmation.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::PanelMcpServer;
    use crate::modules::mcp::models::McpInfoResponse;
    use std::collections::HashSet;

    #[test]
    fn advertised_tools_match_protocol_router() {
        let advertised = McpInfoResponse::current()
            .tools
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<HashSet<_>>();
        let routed = PanelMcpServer::tool_router()
            .map
            .keys()
            .map(|name| name.to_string())
            .collect::<HashSet<_>>();

        assert_eq!(advertised, routed);
    }
}
