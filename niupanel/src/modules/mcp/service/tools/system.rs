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
            core_version: versions.core_version,
            web_version: versions.web_version,
            api_contract: versions.api_contract,
            schema_epoch: versions.schema_epoch,
            schema_revision: versions.schema_revision,
            task_count,
            running_task_count,
        }))
    }

    #[tool(description = "List installed NiuPanel Core releases and rollback state")]
    async fn system_core_releases(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<CoreReleaseListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::OverviewRead)?;
        let releases = system::core_releases::list_releases().map_err(tool_error)?;
        let items = releases
            .releases
            .into_iter()
            .map(|release| CoreReleaseOutput {
                version: release.version,
                active: release.active,
                previous: release.previous,
                api_contract: release.api_contract,
                schema_epoch: release.schema_epoch,
                schema_revision: release.schema_revision,
                target: release.target,
                installed_at: release.installed_at,
            })
            .collect();
        self.audit(&user, "system_core_releases", None).await;
        Ok(Json(CoreReleaseListOutput {
            launcher_managed: releases.launcher_managed,
            active_version: releases.active_version,
            previous_version: releases.previous_version,
            pending_version: releases.pending_version,
            items,
        }))
    }

    #[tool(description = "List installed NiuPanel Web UI releases and compatibility status")]
    async fn system_web_releases(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<WebReleaseListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::OverviewRead)?;
        let releases = system::web_releases::list_releases().map_err(tool_error)?;
        let items = releases
            .releases
            .into_iter()
            .map(|release| WebReleaseOutput {
                version: release.version,
                active: release.active,
                previous: release.previous,
                managed: release.managed,
                compatible: release.compatible,
                compatibility_error: release.compatibility_error,
            })
            .collect();
        self.audit(&user, "system_web_releases", None).await;
        Ok(Json(WebReleaseListOutput {
            active_version: releases.active_version,
            previous_version: releases.previous_version,
            items,
        }))
    }

    #[tool(description = "Check the configured NiuPanel release channel for a Core update")]
    async fn system_core_update_check(
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
        self.audit(&user, "system_core_update_check", None).await;
        Ok(Json(UpdateCheckOutput {
            component: "core".to_string(),
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

    #[tool(description = "Check the configured NiuPanel release channel for a Web UI update")]
    async fn system_web_update_check(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<UpdateCheckOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::OverviewRead)?;
        let current_version = system::web_releases::list_releases()
            .map_err(tool_error)?
            .active_version;
        let output = match self
            .state
            .update_service
            .check_web_update(&current_version)
            .await
        {
            Ok(info) => UpdateCheckOutput {
                component: "web".to_string(),
                current_version: info.current_version,
                target_version: Some(info.version),
                channel: info.channel,
                prerelease: info.prerelease,
                update_available: info.update_available,
                size: info.size,
                release_url: (!info.html_url.is_empty()).then_some(info.html_url),
                summary: release_summary(&info.body),
            },
            Err(AppError::NotFound(message)) => UpdateCheckOutput {
                component: "web".to_string(),
                current_version,
                target_version: None,
                channel: "unknown".to_string(),
                prerelease: false,
                update_available: false,
                size: 0,
                release_url: None,
                summary: Some(message),
            },
            Err(error) => return Err(tool_error(error)),
        };
        self.audit(&user, "system_web_update_check", None).await;
        Ok(Json(output))
    }
}
