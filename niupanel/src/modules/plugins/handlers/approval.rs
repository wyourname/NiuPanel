use super::*;
use crate::modules::agent_policy::{
    AgentApprovalPolicy, ApprovalGrantBinding, EffectiveApprovalMode, IssuedApprovalGrant,
    UpdateAgentApprovalPolicy,
};
use crate::modules::auth::middleware::ApiKeyAuthContext;
use niupanel_common::auth::permissions::UserRole;
use serde::Deserialize;
use std::time::Duration;
use utoipa::ToSchema;

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AgentApprovalPolicyView {
    pub policy: AgentApprovalPolicy,
    pub tools: Vec<AgentApprovalToolView>,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct AgentApprovalToolView {
    pub name: String,
    pub description: String,
    pub category: String,
    pub risk_level: crate::modules::mcp::models::McpToolRiskLevel,
    pub default_decision: &'static str,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApprovalGrantRequest {
    pub session_id: String,
    #[serde(default = "default_approval_grant_mode")]
    pub mode: EffectiveApprovalMode,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

fn default_approval_grant_mode() -> EffectiveApprovalMode {
    EffectiveApprovalMode::Yolo
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RevokeApprovalGrantRequest {
    pub session_id: String,
}

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct RevokedApprovalGrants {
    pub session_id: String,
    pub revoked: usize,
}

pub async fn get_agent_approval_policy(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
) -> Result<ApiResponse<AgentApprovalPolicyView>> {
    require_admin(&user)?;
    ensure_plugin_exists(&plugin_id)?;
    let policy = crate::modules::agent_policy::load_policy(&state, &plugin_id).await?;
    Ok(ApiResponse::success(approval_policy_view(
        &plugin_id, policy,
    )?))
}

pub async fn update_agent_approval_policy(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<UpdateAgentApprovalPolicy>,
) -> Result<ApiResponse<AgentApprovalPolicyView>> {
    require_admin(&user)?;
    ensure_plugin_exists(&plugin_id)?;
    let policy = crate::modules::agent_policy::update_policy(&state, &plugin_id, payload).await?;
    AuditService::log_user(
        &state.db,
        &user,
        "agent.approval_policy.updated",
        "plugin",
        Some(plugin_id.clone()),
        Some(
            serde_json::json!({
                "revision": policy.revision,
                "base_mode": policy.base_mode,
                "override_count": policy.tool_overrides.len(),
            })
            .to_string(),
        ),
    )
    .await;
    Ok(ApiResponse::success(approval_policy_view(
        &plugin_id, policy,
    )?))
}

pub async fn create_ui_approval_grant(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<ApprovalGrantRequest>,
) -> Result<ApiResponse<IssuedApprovalGrant>> {
    require_admin(&user)?;
    let binding = approval_binding(
        &plugin_id,
        format!("user:{}", user.id),
        "ui",
        &payload.session_id,
    )?;
    issue_and_audit_grant(&state, &user, binding, payload.mode, payload.ttl_seconds).await
}

pub async fn revoke_ui_approval_grants(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<RevokeApprovalGrantRequest>,
) -> Result<ApiResponse<RevokedApprovalGrants>> {
    require_admin(&user)?;
    let binding = approval_binding(
        &plugin_id,
        format!("user:{}", user.id),
        "ui",
        &payload.session_id,
    )?;
    revoke_and_audit_grants(&state, &user, binding).await
}

pub async fn create_open_approval_grant(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(api_key): Extension<ApiKeyAuthContext>,
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<ApprovalGrantRequest>,
) -> Result<ApiResponse<IssuedApprovalGrant>> {
    require_approval_permission(&user)?;
    let binding = approval_binding(
        &plugin_id,
        format!("api_key:{}", api_key.key_id),
        "api_key",
        &payload.session_id,
    )?;
    issue_and_audit_grant(&state, &user, binding, payload.mode, payload.ttl_seconds).await
}

pub async fn revoke_open_approval_grants(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    Extension(api_key): Extension<ApiKeyAuthContext>,
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<RevokeApprovalGrantRequest>,
) -> Result<ApiResponse<RevokedApprovalGrants>> {
    require_approval_permission(&user)?;
    let binding = approval_binding(
        &plugin_id,
        format!("api_key:{}", api_key.key_id),
        "api_key",
        &payload.session_id,
    )?;
    revoke_and_audit_grants(&state, &user, binding).await
}

fn require_admin(user: &AuthenticatedUser) -> Result<()> {
    if user.role != UserRole::Admin {
        return Err(AppError::Forbidden(
            "Only administrators can manage Agent approval policy or Web session modes".to_string(),
        ));
    }
    Ok(())
}

fn require_approval_permission(user: &AuthenticatedUser) -> Result<()> {
    if !user.has_permission(Permission::PluginApprove) {
        return Err(AppError::Forbidden(
            "API Key is missing plugin:approve permission".to_string(),
        ));
    }
    Ok(())
}

fn ensure_plugin_exists(plugin_id: &str) -> Result<()> {
    unified_plugin_service().get_plugin(plugin_id).map(|_| ())
}

fn approval_policy_view(
    plugin_id: &str,
    policy: AgentApprovalPolicy,
) -> Result<AgentApprovalPolicyView> {
    use crate::modules::agent_policy::ApprovalOutcome;
    let plugin = unified_plugin_service().get_plugin(plugin_id)?;
    let requested = plugin
        .manifest
        .tools
        .iter()
        .filter_map(requested_tool_name)
        .collect::<HashSet<_>>();
    let mut tools = crate::modules::mcp::models::McpInfoResponse::current()
        .tools
        .into_iter()
        .filter(|tool| requested.contains(tool.name))
        .map(|tool| AgentApprovalToolView {
            name: tool.name.to_string(),
            description: tool.description.to_string(),
            category: tool.category.to_string(),
            risk_level: tool.risk_level,
            default_decision: match policy.decision(tool.name, tool.risk_level, None) {
                ApprovalOutcome::Deny => "deny",
                ApprovalOutcome::Confirm => "confirm",
                ApprovalOutcome::Auto => "auto",
            },
        })
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| {
        left.category
            .cmp(&right.category)
            .then(left.name.cmp(&right.name))
    });
    Ok(AgentApprovalPolicyView { policy, tools })
}

fn requested_tool_name(value: &Value) -> Option<&str> {
    value
        .as_str()
        .or_else(|| value.get("name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn approval_binding(
    plugin_id: &str,
    principal: String,
    channel: &str,
    session_id: &str,
) -> Result<ApprovalGrantBinding> {
    let session_id = session_id.trim();
    if session_id.is_empty()
        || session_id.len() > 128
        || !session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(AppError::ValidationError(
            "Invalid Agent session_id".to_string(),
        ));
    }
    ensure_plugin_exists(plugin_id)?;
    Ok(ApprovalGrantBinding {
        plugin_id: plugin_id.to_string(),
        principal,
        channel: channel.to_string(),
        session_id: session_id.to_string(),
    })
}

async fn issue_and_audit_grant(
    state: &AppState,
    user: &AuthenticatedUser,
    binding: ApprovalGrantBinding,
    mode: EffectiveApprovalMode,
    ttl_seconds: Option<u64>,
) -> Result<ApiResponse<IssuedApprovalGrant>> {
    let ttl = ttl_seconds.map(Duration::from_secs);
    let issued = crate::modules::agent_policy::issue_grant(binding.clone(), mode, ttl).await;
    AuditService::log_user(
        &state.db,
        user,
        "agent.approval_grant.issued",
        "plugin",
        Some(binding.plugin_id),
        Some(
            serde_json::json!({
                "grant_id": issued.grant_id,
                "principal": binding.principal,
                "channel": binding.channel,
                "session_id": binding.session_id,
                "mode": issued.mode,
                "expires_in_seconds": issued.expires_in_seconds,
            })
            .to_string(),
        ),
    )
    .await;
    Ok(ApiResponse::success(issued))
}

async fn revoke_and_audit_grants(
    state: &AppState,
    user: &AuthenticatedUser,
    binding: ApprovalGrantBinding,
) -> Result<ApiResponse<RevokedApprovalGrants>> {
    let revoked = crate::modules::agent_policy::revoke_grants(&binding).await;
    AuditService::log_user(
        &state.db,
        user,
        "agent.approval_grant.revoked",
        "plugin",
        Some(binding.plugin_id),
        Some(
            serde_json::json!({
                "principal": binding.principal,
                "channel": binding.channel,
                "session_id": binding.session_id,
                "revoked": revoked,
            })
            .to_string(),
        ),
    )
    .await;
    Ok(ApiResponse::success(RevokedApprovalGrants {
        session_id: binding.session_id,
        revoked,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_binding_rejects_cross_path_session_ids() {
        assert!(approval_binding("agent", "user:1".to_string(), "ui", "../session").is_err());
    }
}
