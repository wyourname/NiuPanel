use super::super::*;
use rmcp::{tool, tool_router};

#[tool_router(router = notification_tool_router, vis = "pub(crate)")]
impl PanelMcpServer {
    #[tool(description = "Push a notification through configured NiuPanel notification channels")]
    async fn webhook_push(
        &self,
        Parameters(params): Parameters<WebhookPushParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<WebhookPushOutput>, ErrorData> {
        let user = Self::user_for(&context, Permission::WebhookPush)?;
        let level = params
            .level
            .unwrap_or_else(|| "info".to_string())
            .trim()
            .to_ascii_lowercase();
        if !matches!(level.as_str(), "info" | "success" | "warning" | "error") {
            return Err(ErrorData::invalid_params(
                "Notification level must be info, success, warning, or error",
                None,
            ));
        }
        self.state
            .event_bus
            .publish(SystemEvent::System(SystemNotification::Notification {
                title: params.title,
                content: params.content,
                level: level.clone(),
            }));
        self.audit(&user, "webhook_push", None).await;
        Ok(Json(WebhookPushOutput {
            accepted: true,
            level,
        }))
    }
}
