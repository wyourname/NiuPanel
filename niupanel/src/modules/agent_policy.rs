use crate::common::state::AppState;
use crate::modules::mcp::models::McpToolRiskLevel;
use niupanel_common::error::{AppError, Result};
use niupanel_core::settings::SettingsManager;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use utoipa::ToSchema;

const POLICY_SETTING_SUFFIX: &str = "agent_approval_policy";
const POLICY_CATEGORY: &str = "Plugin Agent";
const DEFAULT_GRANT_TTL: Duration = Duration::from_secs(30 * 60);
const MAX_GRANT_TTL: Duration = Duration::from_secs(2 * 60 * 60);
const MAX_ACTIVE_GRANTS: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalBaseMode {
    ReadOnly,
    Safe,
    Balanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolApprovalDecision {
    Deny,
    Confirm,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectiveApprovalMode {
    ReadOnly,
    Safe,
    Balanced,
    Yolo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AgentApprovalPolicy {
    pub revision: u64,
    pub base_mode: ApprovalBaseMode,
    #[serde(default)]
    pub tool_overrides: BTreeMap<String, ToolApprovalDecision>,
}

impl Default for AgentApprovalPolicy {
    fn default() -> Self {
        Self {
            revision: 1,
            base_mode: ApprovalBaseMode::Safe,
            tool_overrides: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
pub struct UpdateAgentApprovalPolicy {
    pub revision: u64,
    pub base_mode: ApprovalBaseMode,
    #[serde(default)]
    pub tool_overrides: BTreeMap<String, ToolApprovalDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalOutcome {
    Deny,
    Confirm,
    Auto,
}

impl AgentApprovalPolicy {
    pub fn effective_mode(
        &self,
        session_mode: Option<EffectiveApprovalMode>,
    ) -> EffectiveApprovalMode {
        session_mode.unwrap_or_else(|| match self.base_mode {
            ApprovalBaseMode::ReadOnly => EffectiveApprovalMode::ReadOnly,
            ApprovalBaseMode::Safe => EffectiveApprovalMode::Safe,
            ApprovalBaseMode::Balanced => EffectiveApprovalMode::Balanced,
        })
    }

    pub fn decision(
        &self,
        tool: &str,
        risk: McpToolRiskLevel,
        session_mode: Option<EffectiveApprovalMode>,
    ) -> ApprovalOutcome {
        if self.tool_overrides.get(tool) == Some(&ToolApprovalDecision::Deny) {
            return ApprovalOutcome::Deny;
        }
        if let Some(mode) = session_mode {
            return match (mode, risk) {
                (_, McpToolRiskLevel::Read) | (EffectiveApprovalMode::Yolo, _) => {
                    ApprovalOutcome::Auto
                }
                (EffectiveApprovalMode::ReadOnly, _) => ApprovalOutcome::Deny,
                (EffectiveApprovalMode::Safe, _) => ApprovalOutcome::Confirm,
                (EffectiveApprovalMode::Balanced, McpToolRiskLevel::Low) => ApprovalOutcome::Auto,
                (EffectiveApprovalMode::Balanced, _) => ApprovalOutcome::Confirm,
            };
        }
        if let Some(decision) = self.tool_overrides.get(tool) {
            return match decision {
                ToolApprovalDecision::Deny => ApprovalOutcome::Deny,
                ToolApprovalDecision::Confirm => ApprovalOutcome::Confirm,
                ToolApprovalDecision::Auto => ApprovalOutcome::Auto,
            };
        }
        match (self.base_mode, risk) {
            (_, McpToolRiskLevel::Read) => ApprovalOutcome::Auto,
            (ApprovalBaseMode::ReadOnly, _) => ApprovalOutcome::Deny,
            (ApprovalBaseMode::Safe, _) => ApprovalOutcome::Confirm,
            (ApprovalBaseMode::Balanced, McpToolRiskLevel::Low) => ApprovalOutcome::Auto,
            (ApprovalBaseMode::Balanced, _) => ApprovalOutcome::Confirm,
        }
    }
}

pub async fn load_policy(state: &AppState, plugin_id: &str) -> Result<AgentApprovalPolicy> {
    let raw = SettingsManager::get(&state.db, &policy_setting_key(plugin_id)).await?;
    if raw.trim().is_empty() {
        return Ok(AgentApprovalPolicy::default());
    }
    let policy = serde_json::from_str::<AgentApprovalPolicy>(&raw).map_err(|error| {
        AppError::Internal(format!(
            "Stored approval policy for plugin '{plugin_id}' is invalid: {error}"
        ))
    })?;
    validate_policy(&policy)?;
    Ok(policy)
}

pub async fn update_policy(
    state: &AppState,
    plugin_id: &str,
    update: UpdateAgentApprovalPolicy,
) -> Result<AgentApprovalPolicy> {
    let _guard = policy_update_lock().lock().await;
    let current = load_policy(state, plugin_id).await?;
    if update.revision != current.revision {
        return Err(AppError::ValidationError(format!(
            "Approval policy revision conflict: expected {}, received {}",
            current.revision, update.revision
        )));
    }
    let policy = AgentApprovalPolicy {
        revision: current.revision.saturating_add(1),
        base_mode: update.base_mode,
        tool_overrides: update.tool_overrides,
    };
    validate_policy(&policy)?;
    let raw = serde_json::to_string(&policy)?;
    SettingsManager::set(
        &state.db,
        &policy_setting_key(plugin_id),
        &raw,
        Some(POLICY_CATEGORY),
    )
    .await?;
    Ok(policy)
}

fn validate_policy(policy: &AgentApprovalPolicy) -> Result<()> {
    if policy.revision == 0 {
        return Err(AppError::ValidationError(
            "Approval policy revision must be positive".to_string(),
        ));
    }
    for (tool, decision) in &policy.tool_overrides {
        let tool = tool.trim();
        if tool.is_empty()
            || tool.len() > 128
            || !tool
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(AppError::ValidationError(format!(
                "Invalid approval policy tool name '{tool}'"
            )));
        }
        let risk = crate::modules::mcp::models::tool_risk_level(tool);
        match (risk, decision) {
            (
                McpToolRiskLevel::Read,
                ToolApprovalDecision::Confirm | ToolApprovalDecision::Auto,
            ) => {
                return Err(AppError::ValidationError(format!(
                    "Read-only tool '{tool}' only supports a deny override"
                )));
            }
            (McpToolRiskLevel::High, ToolApprovalDecision::Auto) => {
                return Err(AppError::ValidationError(format!(
                    "High-risk tool '{tool}' cannot be configured for persistent auto approval"
                )));
            }
            _ => {}
        }
    }
    Ok(())
}

fn policy_setting_key(plugin_id: &str) -> String {
    format!("plugin.{plugin_id}.{POLICY_SETTING_SUFFIX}")
}

fn policy_update_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApprovalGrantBinding {
    pub plugin_id: String,
    pub principal: String,
    pub channel: String,
    pub session_id: String,
}

#[derive(Debug, Clone)]
struct ApprovalGrant {
    id: String,
    binding: ApprovalGrantBinding,
    mode: EffectiveApprovalMode,
    expires_at: Instant,
    created_at: Instant,
}

#[derive(Debug, Default)]
struct ApprovalGrantStore {
    grants: HashMap<String, ApprovalGrant>,
}

impl ApprovalGrantStore {
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.grants.retain(|_, grant| grant.expires_at > now);
        if self.grants.len() <= MAX_ACTIVE_GRANTS {
            return;
        }
        let mut oldest = self
            .grants
            .iter()
            .map(|(token, grant)| (token.clone(), grant.created_at))
            .collect::<Vec<_>>();
        oldest.sort_by_key(|(_, created_at)| *created_at);
        for (token, _) in oldest
            .into_iter()
            .take(self.grants.len() - MAX_ACTIVE_GRANTS)
        {
            self.grants.remove(&token);
        }
    }
}

fn approval_grant_store() -> &'static Mutex<ApprovalGrantStore> {
    static STORE: OnceLock<Mutex<ApprovalGrantStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(ApprovalGrantStore::default()))
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IssuedApprovalGrant {
    pub grant: String,
    pub grant_id: String,
    pub mode: EffectiveApprovalMode,
    pub session_id: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedApprovalGrant {
    pub id: String,
    pub mode: EffectiveApprovalMode,
}

pub async fn issue_grant(
    binding: ApprovalGrantBinding,
    mode: EffectiveApprovalMode,
    requested_ttl: Option<Duration>,
) -> IssuedApprovalGrant {
    let ttl = requested_ttl
        .unwrap_or(DEFAULT_GRANT_TTL)
        .clamp(Duration::from_secs(60), MAX_GRANT_TTL);
    let token = nanoid::nanoid!(64);
    let grant_id = nanoid::nanoid!(24);
    let grant = ApprovalGrant {
        id: grant_id.clone(),
        binding: binding.clone(),
        mode,
        expires_at: Instant::now() + ttl,
        created_at: Instant::now(),
    };
    let mut store = approval_grant_store().lock().await;
    store.cleanup();
    store.grants.retain(|_, grant| grant.binding != binding);
    store.grants.insert(token.clone(), grant);
    IssuedApprovalGrant {
        grant: token,
        grant_id,
        mode,
        session_id: binding.session_id,
        expires_in_seconds: ttl.as_secs(),
    }
}

pub async fn validate_grant(
    token: Option<&str>,
    binding: &ApprovalGrantBinding,
) -> Option<ValidatedApprovalGrant> {
    let token = token?.trim();
    if token.is_empty() {
        return None;
    }
    let mut store = approval_grant_store().lock().await;
    store.cleanup();
    store
        .grants
        .get(token)
        .filter(|grant| &grant.binding == binding)
        .map(|grant| ValidatedApprovalGrant {
            id: grant.id.clone(),
            mode: grant.mode,
        })
}

pub async fn has_bound_grant(binding: &ApprovalGrantBinding) -> Option<ValidatedApprovalGrant> {
    let mut store = approval_grant_store().lock().await;
    store.cleanup();
    store
        .grants
        .values()
        .find(|grant| &grant.binding == binding)
        .map(|grant| ValidatedApprovalGrant {
            id: grant.id.clone(),
            mode: grant.mode,
        })
}

pub async fn revoke_grants(binding: &ApprovalGrantBinding) -> usize {
    let mut store = approval_grant_store().lock().await;
    store.cleanup();
    let previous_len = store.grants.len();
    store.grants.retain(|_, grant| &grant.binding != binding);
    previous_len - store.grants.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_safe() {
        let policy = AgentApprovalPolicy::default();
        assert_eq!(
            serde_json::to_value(&policy).expect("serialize default policy")["tool_overrides"],
            serde_json::json!({})
        );
        assert_eq!(
            policy.decision("tasks_list", McpToolRiskLevel::Read, None),
            ApprovalOutcome::Auto
        );
        assert_eq!(
            policy.decision("tasks_create", McpToolRiskLevel::Medium, None),
            ApprovalOutcome::Confirm
        );
    }

    #[test]
    fn balanced_only_auto_approves_low_risk_writes() {
        let policy = AgentApprovalPolicy {
            base_mode: ApprovalBaseMode::Balanced,
            ..AgentApprovalPolicy::default()
        };
        assert_eq!(
            policy.decision("files_create_directory", McpToolRiskLevel::Low, None),
            ApprovalOutcome::Auto
        );
        assert_eq!(
            policy.decision("tasks_update", McpToolRiskLevel::Medium, None),
            ApprovalOutcome::Confirm
        );
        assert_eq!(
            policy.decision("files_delete", McpToolRiskLevel::High, None),
            ApprovalOutcome::Confirm
        );
    }

    #[test]
    fn deny_override_remains_absolute_in_yolo_mode() {
        let mut policy = AgentApprovalPolicy::default();
        policy
            .tool_overrides
            .insert("files_delete".to_string(), ToolApprovalDecision::Deny);
        assert_eq!(
            policy.decision(
                "files_delete",
                McpToolRiskLevel::High,
                Some(EffectiveApprovalMode::Yolo),
            ),
            ApprovalOutcome::Deny
        );
        assert_eq!(
            policy.decision(
                "tasks_run",
                McpToolRiskLevel::High,
                Some(EffectiveApprovalMode::Yolo),
            ),
            ApprovalOutcome::Auto
        );
    }

    #[test]
    fn persistent_high_risk_auto_override_is_rejected() {
        let mut policy = AgentApprovalPolicy::default();
        policy
            .tool_overrides
            .insert("files_delete".to_string(), ToolApprovalDecision::Auto);
        assert!(validate_policy(&policy).is_err());
    }

    #[test]
    fn session_mode_is_authoritative_while_global_deny_remains_absolute() {
        let mut policy = AgentApprovalPolicy::default();
        policy.tool_overrides.insert(
            "files_create_directory".to_string(),
            ToolApprovalDecision::Auto,
        );
        policy
            .tool_overrides
            .insert("files_delete".to_string(), ToolApprovalDecision::Deny);
        assert_eq!(
            policy.decision(
                "files_create_directory",
                McpToolRiskLevel::Low,
                Some(EffectiveApprovalMode::ReadOnly),
            ),
            ApprovalOutcome::Deny
        );
        assert_eq!(
            policy.decision(
                "files_create_directory",
                McpToolRiskLevel::Low,
                Some(EffectiveApprovalMode::Safe),
            ),
            ApprovalOutcome::Confirm
        );
        assert_eq!(
            policy.decision(
                "files_delete",
                McpToolRiskLevel::High,
                Some(EffectiveApprovalMode::Yolo),
            ),
            ApprovalOutcome::Deny
        );
    }

    #[tokio::test]
    async fn grants_are_bound_to_exact_principal_channel_and_session() {
        let binding = ApprovalGrantBinding {
            plugin_id: "agent".to_string(),
            principal: "user:1".to_string(),
            channel: "ui".to_string(),
            session_id: "session-a".to_string(),
        };
        let issued = issue_grant(
            binding.clone(),
            EffectiveApprovalMode::Balanced,
            Some(Duration::from_secs(60)),
        )
        .await;
        assert!(
            validate_grant(Some(&issued.grant), &binding)
                .await
                .is_some_and(|grant| grant.mode == EffectiveApprovalMode::Balanced)
        );
        let mut other = binding;
        other.session_id = "session-b".to_string();
        assert!(validate_grant(Some(&issued.grant), &other).await.is_none());
    }

    #[tokio::test]
    async fn issuing_a_new_session_mode_revokes_the_previous_token() {
        let binding = ApprovalGrantBinding {
            plugin_id: "agent-replace".to_string(),
            principal: "user:2".to_string(),
            channel: "ui".to_string(),
            session_id: "session-replace".to_string(),
        };
        let first = issue_grant(
            binding.clone(),
            EffectiveApprovalMode::Safe,
            Some(Duration::from_secs(60)),
        )
        .await;
        let second = issue_grant(
            binding.clone(),
            EffectiveApprovalMode::Balanced,
            Some(Duration::from_secs(60)),
        )
        .await;
        assert!(validate_grant(Some(&first.grant), &binding).await.is_none());
        assert!(
            validate_grant(Some(&second.grant), &binding)
                .await
                .is_some_and(|grant| grant.mode == EffectiveApprovalMode::Balanced)
        );
    }
}
