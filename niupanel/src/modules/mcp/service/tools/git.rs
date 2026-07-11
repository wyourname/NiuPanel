use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = git_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List NiuPanel Git repositories and sync status without credentials")]
    async fn git_repos_list(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<GitRepoListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::GitRead)?;
        let repos = GitService::list_repos(&self.state.db)
            .await
            .map_err(tool_error)?;
        let items = repos.into_iter().map(git_repo_output).collect::<Vec<_>>();
        self.audit(&user, "git_repos_list", None).await;
        Ok(Json(GitRepoListOutput {
            total: items.len(),
            items,
        }))
    }

    #[tool(description = "List files in a configured NiuPanel Git repository")]
    async fn git_repo_files(
        &self,
        Parameters(params): Parameters<GitRepoFilesParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<GitFileListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::GitRead)?;
        let files = GitService::list_repo_files(&self.state.db, params.repo_id, params.path)
            .await
            .map_err(tool_error)?;
        let items = files
            .into_iter()
            .map(|file| GitFileOutput {
                name: file.name,
                path: file.path,
                script_path: file.full_path,
                is_directory: file.is_dir,
                size: file.size,
            })
            .collect();
        self.audit(&user, "git_repo_files", Some(params.repo_id.to_string()))
            .await;
        Ok(Json(GitFileListOutput {
            repo_id: params.repo_id,
            items,
        }))
    }

    #[tool(
        description = "Scan a NiuPanel Git repository for task definitions without importing them"
    )]
    async fn git_repo_scan_tasks(
        &self,
        Parameters(GitRepoIdParams { repo_id }): Parameters<GitRepoIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<GitDiscoveredTaskListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::GitRead)?;
        let tasks = GitService::scan_repo_tasks(&self.state.db, repo_id)
            .await
            .map_err(tool_error)?;
        let items = tasks
            .into_iter()
            .map(|task| GitDiscoveredTaskOutput {
                name: task.name,
                command: task.command,
                cron: task.cron,
                file_path: task.file_path,
                description: task.description,
            })
            .collect();
        self.audit(&user, "git_repo_scan_tasks", Some(repo_id.to_string()))
            .await;
        Ok(Json(GitDiscoveredTaskListOutput { repo_id, items }))
    }

    #[tool(description = "Synchronize one configured NiuPanel Git repository")]
    async fn git_repo_sync(
        &self,
        Parameters(GitRepoIdParams { repo_id }): Parameters<GitRepoIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<GitSyncOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::GitSync)?;
        let result = GitService::sync_repo(&self.state.db, repo_id)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "git_repo_sync", Some(repo_id.to_string()))
            .await;
        Ok(Json(GitSyncOutput {
            repo_id,
            success: result.success,
            message: result.message,
            changes: result.changes,
        }))
    }
}
