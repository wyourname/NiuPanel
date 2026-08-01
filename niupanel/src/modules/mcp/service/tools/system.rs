use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = system_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "Get NiuPanel versions and task runtime summary")]
    async fn system_status(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<SystemStatusOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::OverviewRead)?;
        let task_count = tasks::Entity::find()
            .count(&self.state.db)
            .await
            .map_err(tool_error)?;
        let running_task_count = tasks::Entity::find()
            .filter(tasks::Column::Status.eq(TaskStatus::Running))
            .count(&self.state.db)
            .await
            .map_err(tool_error)?;
        let versions = system::service::version_info();
        self.audit(&user, "system_status", None).await;
        Ok(Json(SystemStatusOutput {
            panel_version: versions.panel_version,
            api_contract: versions.api_contract,
            schema_epoch: versions.schema_epoch,
            schema_revision: versions.schema_revision,
            task_count,
            running_task_count,
        }))
    }

    #[tool(description = "List installed NiuPanel releases and rollback state")]
    async fn system_releases(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<PanelReleaseListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::OverviewRead)?;
        let releases = system::panel_releases::list_releases()
            .await
            .map_err(tool_error)?;
        let items = releases
            .releases
            .into_iter()
            .map(|release| PanelReleaseOutput {
                version: release.version,
                active: release.active,
                previous: release.previous,
                rollback_available: release.rollback_available,
                installed_at: release.installed_at,
            })
            .collect();
        self.audit(&user, "system_releases", None).await;
        Ok(Json(PanelReleaseListOutput {
            launcher_managed: releases.launcher_managed,
            active_version: releases.active_version,
            previous_version: releases.previous_version,
            pending_version: releases.pending_version,
            items,
        }))
    }

    #[tool(description = "Check the configured stable or preview channel for a NiuPanel update")]
    async fn system_update_check(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<UpdateCheckOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::OverviewRead)?;
        let info = self
            .state
            .update_service
            .check_update()
            .await
            .map_err(tool_error)?;
        self.audit(&user, "system_update_check", None).await;
        Ok(Json(UpdateCheckOutput {
            component: "panel".to_string(),
            current_version: info.current_version,
            target_version: (info.tag_name != "unknown").then_some(info.tag_name),
            channel: info.channel,
            prerelease: info.prerelease,
            update_available: info.update_available,
            size: info.size,
            release_url: (!info.html_url.is_empty()).then_some(info.html_url),
            summary: release_summary(&info.body),
        }))
    }
}
