use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = share_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "Get NiuPanel share station capacity and configuration status")]
    async fn share_station_stats(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ShareStationStatsOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::ShareList)?;
        let stats = share_service::get_station_stats(&self.state.db, &self.state.http_client)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "share_station_stats", None).await;
        Ok(Json(ShareStationStatsOutput {
            configured: stats.is_configured,
            current_usage_bytes: stats.current_usage_bytes,
            max_usage_bytes: stats.max_usage_bytes,
            usage_percent: (!stats.usage_percent.trim().is_empty()).then_some(stats.usage_percent),
            max_file_size_bytes: stats.max_file_size_content,
        }))
    }

    #[tool(description = "List files in the configured NiuPanel share station without passwords")]
    async fn share_station_files(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ShareStationFilesOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::ShareList)?;
        let files = share_service::list_station_files(&self.state.db, &self.state.http_client)
            .await
            .map_err(tool_error)?;
        let items = files
            .into_iter()
            .map(|file| ShareStationFileOutput {
                token: file.token,
                file_key: file.file_key,
                size: file.size,
                downloads_remaining: file.downloads_remaining,
                delete_on_download: file.delete_on_download,
                expires_at: file.expires_at,
                note: file.note,
                uploaded_at: file.uploaded_at,
            })
            .collect::<Vec<_>>();
        self.audit(&user, "share_station_files", None).await;
        Ok(Json(ShareStationFilesOutput {
            total: items.len(),
            items,
        }))
    }

    #[tool(description = "List imported task source summaries in NiuPanel")]
    async fn share_import_sources(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ShareImportSourcesOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::ShareList)?;
        let sources = share_service::get_imported_sources_grouped(&self.state.db)
            .await
            .map_err(tool_error)?;
        let items = sources
            .into_iter()
            .map(|source| ShareImportSourceOutput {
                share_code: source.share_code,
                url: source.url,
                task_count: source.task_count,
                task_names: source
                    .tasks
                    .into_iter()
                    .map(|task| task.task_name)
                    .collect(),
                last_updated_at: source.last_updated_at,
            })
            .collect::<Vec<_>>();
        self.audit(&user, "share_import_sources", None).await;
        Ok(Json(ShareImportSourcesOutput {
            total: items.len(),
            items,
        }))
    }
}
