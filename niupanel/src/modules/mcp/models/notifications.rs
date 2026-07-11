use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WebhookPushParams {
    pub title: String,
    pub content: String,
    #[schemars(description = "Notification level: info, success, warning, or error")]
    pub level: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct WebhookPushOutput {
    pub accepted: bool,
    pub level: String,
}
