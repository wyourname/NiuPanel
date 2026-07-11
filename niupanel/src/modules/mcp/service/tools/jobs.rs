use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = job_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List recent NiuPanel system jobs")]
    async fn jobs_list(
        &self,
        Parameters(params): Parameters<JobListParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<JobListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::JobList)?;
        let limit = params.limit.unwrap_or(50).clamp(1, 100);
        let total = system_jobs::Entity::find()
            .count(&self.state.db)
            .await
            .map_err(tool_error)?;
        let jobs = system_jobs::Entity::find()
            .order_by_desc(system_jobs::Column::Id)
            .limit(limit)
            .all(&self.state.db)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "jobs_list", None).await;
        Ok(Json(JobListOutput {
            total,
            items: jobs.into_iter().map(job_output).collect(),
        }))
    }

    #[tool(description = "Get one NiuPanel system job")]
    async fn jobs_get(
        &self,
        Parameters(JobIdParams { job_id }): Parameters<JobIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<JobOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::JobRead)?;
        let job = system_jobs::Entity::find_by_id(job_id)
            .one(&self.state.db)
            .await
            .map_err(tool_error)?
            .ok_or_else(|| ErrorData::invalid_params("System job not found", None))?;
        self.audit(&user, "jobs_get", Some(job_id.to_string()))
            .await;
        Ok(Json(job_output(job)))
    }

    #[tool(description = "Read the tail of a NiuPanel system job log")]
    async fn jobs_get_logs(
        &self,
        Parameters(params): Parameters<JobLogParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<JobLogOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::JobRead)?;
        let limit = params.tail_bytes.unwrap_or(64 * 1024).clamp(1, 1024 * 1024);
        let (content, _) = self
            .state
            .task_manager
            .get_latest_log_content(params.job_id, true, Some(limit))
            .await
            .map_err(tool_error)?;
        self.audit(&user, "jobs_get_logs", Some(params.job_id.to_string()))
            .await;
        Ok(Json(JobLogOutput {
            job_id: params.job_id,
            content,
        }))
    }

    #[tool(description = "Cancel a running NiuPanel system job. This is destructive")]
    async fn jobs_cancel(
        &self,
        Parameters(JobIdParams { job_id }): Parameters<JobIdParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<JobActionOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::JobAll)?;
        self.state
            .task_manager
            .cancel_system_job(job_id)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "jobs_cancel", Some(job_id.to_string()))
            .await;
        Ok(Json(JobActionOutput {
            job_id,
            accepted: true,
            message: "System job cancellation accepted".to_string(),
        }))
    }
}
