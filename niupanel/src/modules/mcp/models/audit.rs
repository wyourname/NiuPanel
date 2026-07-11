use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditListParams {
    #[schemars(description = "Maximum number of recent audit records, between 1 and 100")]
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AuditOutput {
    pub id: i32,
    pub user_id: Option<i32>,
    pub actor_type: String,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AuditListOutput {
    pub total: u64,
    pub items: Vec<AuditOutput>,
}
