use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = audit_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "List recent NiuPanel audit records")]
    async fn audit_list(
        &self,
        Parameters(params): Parameters<AuditListParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<AuditListOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::AuditList)?;
        let limit = params.limit.unwrap_or(50).clamp(1, 100);
        let total = audit_logs::Entity::find()
            .count(&self.state.db)
            .await
            .map_err(tool_error)?;
        let records = audit_logs::Entity::find()
            .order_by_desc(audit_logs::Column::CreatedAt)
            .limit(limit)
            .all(&self.state.db)
            .await
            .map_err(tool_error)?;
        self.audit(&user, "audit_list", None).await;
        Ok(Json(AuditListOutput {
            total,
            items: records
                .into_iter()
                .map(|record| AuditOutput {
                    id: record.id,
                    user_id: record.user_id,
                    actor_type: record.actor_type,
                    action: record.action,
                    resource: record.resource,
                    resource_id: record.resource_id,
                    details: record.details,
                    ip_address: record.ip_address,
                    created_at: record.created_at.to_rfc3339(),
                })
                .collect(),
        }))
    }
}
