use crate::common::state::AppState;
use crate::modules::agent_policy::{
    AgentApprovalPolicy, ApprovalGrantBinding, ApprovalOutcome, EffectiveApprovalMode,
    ValidatedApprovalGrant,
};
use crate::modules::auth::service::AuthenticatedUser;
use crate::modules::mcp::models::{McpInfoResponse, McpToolInfo, McpToolRiskLevel};
use crate::modules::mcp::service::PanelMcpServer;
use niupanel_common::auth::permissions::Permission;
use niupanel_common::error::{AppError, Result};
use niupanel_core::audit::service::AuditService;
use niupanel_plugin::{PluginToolFuture, ProcessPluginToolCall};
use rmcp::RoleServer;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{CallToolRequestParams, CallToolResponse};
use rmcp::service::serve_directly;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const CONFIRM_TOOL: &str = "niupanel_confirm_operation";
const CANCEL_TOOL: &str = "niupanel_cancel_operation";
const PENDING_TOOL: &str = "niupanel_pending_operations";
const EXECUTE_PLAN_TOOL: &str = "niupanel_execute_plan";
const CONFIRMATION_TTL: Duration = Duration::from_secs(10 * 60);
const COMPLETED_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_PLAN_STEPS: usize = 8;
const MAX_PLAN_INPUT_BYTES: usize = 512 * 1024;

#[derive(Clone)]
pub struct AgentToolGateway {
    state: AppState,
    user: AuthenticatedUser,
    plugin_id: String,
    actor_scope: String,
    allowed_tools: HashSet<String>,
    approval_policy: AgentApprovalPolicy,
    effective_mode: EffectiveApprovalMode,
    approval_grant: Option<ValidatedApprovalGrant>,
    principal: String,
    channel: String,
    session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentInvocationIdentity {
    pub principal: String,
    pub channel: String,
    pub session_id: Option<String>,
    pub presented_grant: Option<String>,
    pub allow_implicit_grant: bool,
}

#[derive(Clone)]
struct PendingOperation {
    id: String,
    actor_scope: String,
    plugin_id: String,
    tool: String,
    risk_level: McpToolRiskLevel,
    input: Value,
    redacted_input: Value,
    target_summary: String,
    created_at: Instant,
    expires_at: Instant,
}

enum StoredOperation {
    Pending(PendingOperation),
    Executing(PendingOperation),
    Terminal {
        operation: PendingOperation,
        status: OperationTerminalStatus,
        output: Option<Value>,
        error: Option<String>,
        expires_at: Instant,
    },
}

#[derive(Clone, Copy)]
enum OperationTerminalStatus {
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationPlan {
    summary: String,
    steps: Vec<OperationPlanStep>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationPlanStep {
    id: String,
    tool: String,
    input: Value,
}

impl OperationTerminalStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Default)]
struct ConfirmationStore {
    operations: HashMap<String, StoredOperation>,
}

fn confirmation_store() -> &'static Mutex<ConfirmationStore> {
    static STORE: OnceLock<Mutex<ConfirmationStore>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(ConfirmationStore::default()))
}

impl ConfirmationStore {
    fn cleanup(&mut self) {
        let now = Instant::now();
        self.operations.retain(|_, operation| match operation {
            StoredOperation::Pending(operation) | StoredOperation::Executing(operation) => {
                operation.expires_at > now
            }
            StoredOperation::Terminal { expires_at, .. } => *expires_at > now,
        });
    }

    fn take_pending_operations(
        &mut self,
        actor_scope: &str,
        plugin_id: &str,
    ) -> Vec<(String, String)> {
        self.cleanup();
        let cancelled = self
            .operations
            .iter()
            .filter_map(|(operation_id, operation)| match operation {
                StoredOperation::Pending(operation)
                    if operation.actor_scope == actor_scope && operation.plugin_id == plugin_id =>
                {
                    Some((operation_id.clone(), operation.tool.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for (operation_id, _) in &cancelled {
            self.operations.remove(operation_id);
        }
        cancelled
    }
}

impl AgentToolGateway {
    pub async fn for_plugin(
        state: AppState,
        user: AuthenticatedUser,
        plugin_id: &str,
        identity: AgentInvocationIdentity,
    ) -> Result<Self> {
        let plugin =
            crate::modules::plugins::service::unified_plugin_service().get_plugin(plugin_id)?;
        let allowed_tools = plugin
            .manifest
            .tools
            .iter()
            .filter_map(requested_tool_name)
            .collect::<HashSet<_>>();
        let principal = identity.principal.trim();
        let channel = identity.channel.trim();
        if principal.is_empty() || channel.is_empty() {
            return Err(AppError::Internal(
                "Agent invocation identity is incomplete".to_string(),
            ));
        }
        let approval_policy = crate::modules::agent_policy::load_policy(&state, plugin_id).await?;
        let binding = identity
            .session_id
            .as_deref()
            .map(|session_id| ApprovalGrantBinding {
                plugin_id: plugin_id.to_string(),
                principal: principal.to_string(),
                channel: channel.to_string(),
                session_id: session_id.to_string(),
            });
        let approval_grant = if let Some(binding) = binding.as_ref() {
            let presented = crate::modules::agent_policy::validate_grant(
                identity.presented_grant.as_deref(),
                binding,
            )
            .await;
            if presented.is_some() || !identity.allow_implicit_grant {
                presented
            } else {
                crate::modules::agent_policy::has_bound_grant(binding).await
            }
        } else {
            None
        };
        let effective_mode =
            approval_policy.effective_mode(approval_grant.as_ref().map(|grant| grant.mode));
        let actor_scope = operation_actor_scope(
            plugin_id,
            principal,
            channel,
            identity.session_id.as_deref(),
        );
        Ok(Self {
            state,
            user,
            plugin_id: plugin_id.to_string(),
            actor_scope,
            allowed_tools,
            approval_policy,
            effective_mode,
            approval_grant,
            principal: principal.to_string(),
            channel: channel.to_string(),
            session_id: identity.session_id,
        })
    }

    pub fn effective_mode(&self) -> EffectiveApprovalMode {
        self.effective_mode
    }

    pub fn approval_grant_id(&self) -> Option<&str> {
        self.approval_grant.as_ref().map(|grant| grant.id.as_str())
    }

    pub fn invocation_context(
        &self,
        caller: niupanel_plugin::PluginActionCaller,
        client_request_id: Option<&str>,
    ) -> niupanel_plugin::PluginInvocationContext {
        use niupanel_plugin::PluginInvocationPrincipalKind;
        let (principal_kind, principal_id) = self
            .principal
            .split_once(':')
            .map(|(kind, id)| {
                let kind = match kind {
                    "user" => PluginInvocationPrincipalKind::User,
                    "api_key" => PluginInvocationPrincipalKind::ApiKey,
                    "task" => PluginInvocationPrincipalKind::Task,
                    _ => PluginInvocationPrincipalKind::System,
                };
                (kind, id.to_string())
            })
            .unwrap_or((
                PluginInvocationPrincipalKind::System,
                self.principal.clone(),
            ));
        niupanel_plugin::PluginInvocationContext {
            caller,
            principal_kind,
            principal_id,
            administrator: self.user.role == niupanel_common::auth::permissions::UserRole::Admin,
            channel: self.channel.clone(),
            session_id: self.session_id.clone(),
            client_request_id: client_request_id.map(ToString::to_string),
            approval_mode: serde_json::to_value(self.effective_mode())
                .ok()
                .and_then(|value| value.as_str().map(ToString::to_string))
                .unwrap_or_else(|| "safe".to_string()),
            approval_policy_revision: self.approval_policy.revision,
            approval_grant_id: self.approval_grant_id().map(ToString::to_string),
        }
    }

    pub fn coordination_scope(&self) -> String {
        format!(
            "{}:{}:{}",
            self.plugin_id,
            self.principal,
            self.session_id.as_deref().unwrap_or("new-session")
        )
    }

    pub fn definitions(&self) -> Vec<Value> {
        let catalog = tool_catalog();
        let mut definitions = PanelMcpServer::agent_tool_router()
            .list_all()
            .into_iter()
            .filter_map(|tool| {
                let info = catalog.get(tool.name.as_ref())?;
                if !self.allowed_tools.contains(tool.name.as_ref())
                    || !user_has_catalog_permission(&self.user, info)
                {
                    return None;
                }
                let decision = self.approval_decision(tool.name.as_ref(), info);
                if decision == ApprovalOutcome::Deny {
                    return None;
                }
                let read_only = info.risk_level == McpToolRiskLevel::Read;
                Some(serde_json::json!({
                    "name": tool.name,
                    "description": tool.description.unwrap_or(Cow::Borrowed(info.description)),
                    "input_schema": Value::Object(tool.input_schema.as_ref().clone()),
                    "access": if read_only { "read" } else { "write" },
                    "requires_confirmation": decision == ApprovalOutcome::Confirm,
                    "destructive": info.destructive,
                    "category": info.category,
                    "risk_level": info.risk_level,
                    "approval_mode": self.effective_mode,
                }))
            })
            .collect::<Vec<_>>();

        if definitions
            .iter()
            .any(|tool| tool.get("access").and_then(Value::as_str) == Some("write"))
        {
            definitions.extend(control_tool_definitions());
        }
        definitions
    }

    pub fn handler(
        &self,
    ) -> impl Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync + 'static {
        let gateway = self.clone();
        move |call| {
            let gateway = gateway.clone();
            Box::pin(async move { gateway.handle(call).await })
        }
    }

    pub async fn cancel_pending_operations(&self) -> usize {
        let cancelled = {
            let mut store = confirmation_store().lock().await;
            store.take_pending_operations(&self.actor_scope, &self.plugin_id)
        };

        for (operation_id, tool) in &cancelled {
            self.audit(
                "agent.operation.cancelled",
                Some(operation_id.clone()),
                serde_json::json!({
                    "tool": tool,
                    "reason": "agent_session_reset",
                }),
            )
            .await;
        }
        cancelled.len()
    }

    async fn handle(&self, call: ProcessPluginToolCall) -> Result<Value> {
        match call.tool.as_str() {
            CONFIRM_TOOL => self.confirm_operation(&call.input).await,
            CANCEL_TOOL => self.cancel_operation(&call.input).await,
            PENDING_TOOL => self.pending_operations(&call.input).await,
            EXECUTE_PLAN_TOOL => self.execute_or_stage_plan(call.input).await,
            tool => {
                let info = self.authorized_tool(tool)?;
                match self.approval_decision(tool, info) {
                    ApprovalOutcome::Deny => Err(AppError::Forbidden(format!(
                        "Agent approval policy denies host tool '{tool}'"
                    ))),
                    ApprovalOutcome::Confirm => self.stage_operation(tool, info, call.input).await,
                    ApprovalOutcome::Auto => self.execute_tool(tool, call.input).await,
                }
            }
        }
    }

    fn authorized_tool(&self, tool: &str) -> Result<&'static McpToolInfo> {
        if !self.allowed_tools.contains(tool) {
            return Err(AppError::Forbidden(format!(
                "Plugin '{}' did not request host tool '{}'",
                self.plugin_id, tool
            )));
        }
        let info = tool_catalog()
            .get(tool)
            .ok_or_else(|| AppError::NotFound(format!("Unknown Agent host tool '{tool}'")))?;
        if !user_has_catalog_permission(&self.user, info) {
            return Err(AppError::Forbidden(format!(
                "Agent actor is missing {} permission",
                info.permission
            )));
        }
        Ok(info)
    }

    fn approval_decision(&self, tool: &str, info: &McpToolInfo) -> ApprovalOutcome {
        self.approval_policy.decision(
            tool,
            info.risk_level,
            self.approval_grant.as_ref().map(|grant| grant.mode),
        )
    }

    async fn stage_operation(&self, tool: &str, info: &McpToolInfo, input: Value) -> Result<Value> {
        self.authorized_tool(tool)?;
        self.stage_operation_with_metadata(
            tool,
            info.risk_level,
            operation_target_summary(tool, &input),
            input,
        )
        .await
    }

    async fn stage_operation_with_metadata(
        &self,
        tool: &str,
        risk_level: McpToolRiskLevel,
        target_summary: String,
        input: Value,
    ) -> Result<Value> {
        let mut store = confirmation_store().lock().await;
        store.cleanup();
        if let Some(operation) = store
            .operations
            .values()
            .find_map(|operation| match operation {
                StoredOperation::Pending(operation)
                    if operation.actor_scope == self.actor_scope
                        && operation.plugin_id == self.plugin_id
                        && operation.tool == tool
                        && operation.input == input =>
                {
                    Some(operation.clone())
                }
                _ => None,
            })
        {
            return Ok(staged_response(&operation));
        }

        let operation = PendingOperation {
            id: nanoid::nanoid!(32),
            actor_scope: self.actor_scope.clone(),
            plugin_id: self.plugin_id.clone(),
            tool: tool.to_string(),
            risk_level,
            redacted_input: redact_sensitive_value(&input),
            target_summary,
            input,
            created_at: Instant::now(),
            expires_at: Instant::now() + CONFIRMATION_TTL,
        };
        let response = staged_response(&operation);
        store.operations.insert(
            operation.id.clone(),
            StoredOperation::Pending(operation.clone()),
        );
        drop(store);
        self.audit(
            "agent.operation.staged",
            Some(operation.id),
            serde_json::json!({
                "tool": tool,
                "risk_level": risk_level,
                "target": operation.target_summary,
                "input": operation.redacted_input,
            }),
        )
        .await;
        Ok(response)
    }

    async fn execute_or_stage_plan(&self, input: Value) -> Result<Value> {
        let (plan, risk_level, requires_confirmation) = self.validate_plan(&input)?;
        if requires_confirmation {
            return self
                .stage_operation_with_metadata(
                    EXECUTE_PLAN_TOOL,
                    risk_level,
                    plan.summary.trim().to_string(),
                    input,
                )
                .await;
        }
        self.execute_plan(plan).await
    }

    fn validate_plan(&self, input: &Value) -> Result<(OperationPlan, McpToolRiskLevel, bool)> {
        if serde_json::to_vec(input)?.len() > MAX_PLAN_INPUT_BYTES {
            return Err(AppError::ValidationError(format!(
                "Agent operation plan exceeds {MAX_PLAN_INPUT_BYTES} bytes"
            )));
        }
        let plan: OperationPlan = serde_json::from_value(input.clone()).map_err(|error| {
            AppError::ValidationError(format!("Invalid Agent operation plan: {error}"))
        })?;
        let summary = plan.summary.trim();
        if summary.is_empty() || summary.chars().count() > 240 {
            return Err(AppError::ValidationError(
                "Agent operation plan summary must contain 1 to 240 characters".to_string(),
            ));
        }
        if plan.steps.is_empty() || plan.steps.len() > MAX_PLAN_STEPS {
            return Err(AppError::ValidationError(format!(
                "Agent operation plan must contain 1 to {MAX_PLAN_STEPS} steps"
            )));
        }

        let mut prior_steps = HashSet::new();
        let mut risk_level = McpToolRiskLevel::Read;
        let mut requires_confirmation = false;
        for step in &plan.steps {
            validate_plan_step_id(&step.id)?;
            if prior_steps.contains(&step.id) {
                return Err(AppError::ValidationError(format!(
                    "Duplicate Agent operation plan step id '{}'",
                    step.id
                )));
            }
            if matches!(
                step.tool.as_str(),
                CONFIRM_TOOL | CANCEL_TOOL | PENDING_TOOL | EXECUTE_PLAN_TOOL
            ) {
                return Err(AppError::ValidationError(format!(
                    "Agent operation plan cannot contain control tool '{}'",
                    step.tool
                )));
            }
            if !step.input.is_object() {
                return Err(AppError::ValidationError(format!(
                    "Agent operation plan step '{}' input must be an object",
                    step.id
                )));
            }
            validate_plan_references(&step.input, &prior_steps, &step.id)?;
            prior_steps.insert(step.id.clone());

            let info = self.authorized_tool(&step.tool)?;
            risk_level = max_risk_level(risk_level, info.risk_level);
            match self.approval_decision(&step.tool, info) {
                ApprovalOutcome::Deny => {
                    return Err(AppError::Forbidden(format!(
                        "Agent approval policy denies host tool '{}'",
                        step.tool
                    )));
                }
                ApprovalOutcome::Confirm => requires_confirmation = true,
                ApprovalOutcome::Auto => {}
            }
        }
        Ok((plan, risk_level, requires_confirmation))
    }

    async fn execute_plan(&self, plan: OperationPlan) -> Result<Value> {
        let mut outputs = HashMap::<String, Value>::new();
        let mut completed_steps = Vec::with_capacity(plan.steps.len());
        for step in plan.steps {
            let input = match resolve_plan_references(&step.input, &outputs, &step.id) {
                Ok(input) => input,
                Err(error) => {
                    return Ok(failed_plan_output(
                        &plan.summary,
                        completed_steps,
                        &step,
                        error.to_string(),
                    ));
                }
            };
            let output = match self.execute_tool(&step.tool, input).await {
                Ok(output) => output,
                Err(error) => {
                    return Ok(failed_plan_output(
                        &plan.summary,
                        completed_steps,
                        &step,
                        error.to_string(),
                    ));
                }
            };
            outputs.insert(step.id.clone(), output.clone());
            completed_steps.push(serde_json::json!({
                "id": step.id,
                "tool": step.tool,
                "status": "completed",
                "output": output,
            }));
        }
        Ok(serde_json::json!({
            "status": "completed",
            "summary": plan.summary.trim(),
            "steps": completed_steps,
        }))
    }

    async fn confirm_operation(&self, input: &Value) -> Result<Value> {
        let operation_id = operation_id(input)?;
        let operation = {
            let mut store = confirmation_store().lock().await;
            store.cleanup();
            match store.operations.get(operation_id) {
                Some(StoredOperation::Terminal {
                    operation,
                    status,
                    output,
                    error,
                    ..
                }) if operation.actor_scope == self.actor_scope
                    && operation.plugin_id == self.plugin_id =>
                {
                    return Ok(serde_json::json!({
                        "status": status.as_str(),
                        "operation_id": operation_id,
                        "tool": operation.tool,
                        "risk_level": operation.risk_level,
                        "target": operation.target_summary,
                        "output": output,
                        "error": error,
                        "replayed": true,
                    }));
                }
                Some(StoredOperation::Executing(operation))
                    if operation.actor_scope == self.actor_scope =>
                {
                    return Ok(serde_json::json!({
                        "status": "executing",
                        "operation_id": operation_id,
                    }));
                }
                Some(StoredOperation::Pending(operation))
                    if operation.actor_scope == self.actor_scope
                        && operation.plugin_id == self.plugin_id =>
                {
                    operation.clone()
                }
                Some(_) => {
                    return Err(AppError::Forbidden(
                        "Confirmation belongs to a different Agent conversation".to_string(),
                    ));
                }
                None => {
                    return Err(AppError::NotFound(
                        "Confirmation is unknown or expired".to_string(),
                    ));
                }
            }
        };

        if operation.tool == EXECUTE_PLAN_TOOL {
            self.validate_plan(&operation.input)?;
        } else {
            self.authorized_tool(&operation.tool)?;
        }
        {
            let mut store = confirmation_store().lock().await;
            store.operations.insert(
                operation.id.clone(),
                StoredOperation::Executing(operation.clone()),
            );
        }

        // A confirmed plan may wait for a task run. Detach it from the plugin RPC so a
        // browser disconnect or request timeout cannot cancel already-authorized work.
        let confirmed_operation_id = operation.id.clone();
        let recovery_operation = operation.clone();
        let gateway = self.clone();
        tokio::spawn(async move {
            if let Err(error) = gateway.finish_confirmed_operation(operation).await {
                let mut store = confirmation_store().lock().await;
                store.operations.insert(
                    recovery_operation.id.clone(),
                    terminal_operation(
                        &recovery_operation,
                        OperationTerminalStatus::Interrupted,
                        None,
                        Some(error.to_string()),
                    ),
                );
            }
        });
        Ok(serde_json::json!({
            "status": "executing",
            "operation_id": confirmed_operation_id,
        }))
    }

    async fn finish_confirmed_operation(&self, operation: PendingOperation) -> Result<Value> {
        let result = if operation.tool == EXECUTE_PLAN_TOOL {
            match self.validate_plan(&operation.input) {
                Ok((plan, _, _)) => self.execute_plan(plan).await,
                Err(error) => Err(error),
            }
        } else {
            self.execute_tool(&operation.tool, operation.input.clone())
                .await
        };
        let mut store = confirmation_store().lock().await;
        match result {
            Ok(output)
                if operation.tool == EXECUTE_PLAN_TOOL
                    && output["status"].as_str() == Some("failed") =>
            {
                let message = output["error"]
                    .as_str()
                    .unwrap_or("Agent operation plan failed")
                    .to_string();
                store.operations.insert(
                    operation.id.clone(),
                    terminal_operation(
                        &operation,
                        OperationTerminalStatus::Failed,
                        Some(output),
                        Some(message.clone()),
                    ),
                );
                drop(store);
                self.audit(
                    "agent.operation.failed",
                    Some(operation.id.clone()),
                    serde_json::json!({ "tool": operation.tool, "error": message }),
                )
                .await;
                Ok(serde_json::json!({
                    "status": "failed",
                    "operation_id": operation.id,
                    "error": message,
                    "replayed": false,
                }))
            }
            Ok(output) => {
                store.operations.insert(
                    operation.id.clone(),
                    StoredOperation::Terminal {
                        operation: operation.clone(),
                        status: OperationTerminalStatus::Completed,
                        output: Some(output.clone()),
                        error: None,
                        expires_at: Instant::now() + COMPLETED_TTL,
                    },
                );
                drop(store);
                self.audit(
                    "agent.operation.confirmed",
                    Some(operation.id.clone()),
                    serde_json::json!({ "tool": operation.tool }),
                )
                .await;
                Ok(serde_json::json!({
                    "status": "completed",
                    "operation_id": operation.id,
                    "output": output,
                    "replayed": false,
                }))
            }
            Err(error) => {
                let message = error.to_string();
                store.operations.insert(
                    operation.id.clone(),
                    terminal_operation(
                        &operation,
                        OperationTerminalStatus::Failed,
                        None,
                        Some(message.clone()),
                    ),
                );
                drop(store);
                self.audit(
                    "agent.operation.failed",
                    Some(operation.id.clone()),
                    serde_json::json!({ "tool": operation.tool, "error": message }),
                )
                .await;
                Ok(serde_json::json!({
                    "status": "failed",
                    "operation_id": operation.id,
                    "error": message,
                    "replayed": false,
                }))
            }
        }
    }

    async fn cancel_operation(&self, input: &Value) -> Result<Value> {
        let operation_id = operation_id(input)?;
        let mut store = confirmation_store().lock().await;
        store.cleanup();
        match store.operations.get(operation_id) {
            Some(StoredOperation::Pending(operation))
                if operation.actor_scope == self.actor_scope => {}
            Some(StoredOperation::Executing(_)) => {
                return Err(AppError::ValidationError(
                    "Operation is already executing".to_string(),
                ));
            }
            Some(StoredOperation::Terminal {
                operation, status, ..
            }) if operation.actor_scope == self.actor_scope
                && operation.plugin_id == self.plugin_id =>
            {
                return Err(AppError::ValidationError(format!(
                    "Operation is already {}",
                    status.as_str()
                )));
            }
            Some(_) => {
                return Err(AppError::Forbidden(
                    "Confirmation belongs to a different Agent conversation".to_string(),
                ));
            }
            None => {
                return Err(AppError::NotFound(
                    "Confirmation is unknown or expired".to_string(),
                ));
            }
        }
        let operation = match store.operations.remove(operation_id) {
            Some(StoredOperation::Pending(operation)) => operation,
            _ => unreachable!("pending operation was checked above"),
        };
        store.operations.insert(
            operation_id.to_string(),
            terminal_operation(&operation, OperationTerminalStatus::Cancelled, None, None),
        );
        drop(store);
        self.audit(
            "agent.operation.cancelled",
            Some(operation_id.to_string()),
            Value::Null,
        )
        .await;
        Ok(serde_json::json!({
            "status": "cancelled",
            "operation_id": operation_id,
        }))
    }

    async fn pending_operations(&self, input: &Value) -> Result<Value> {
        let requested_operation_id = input
            .get("operation_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if requested_operation_id.is_some_and(|value| value.len() > 64) {
            return Err(AppError::ValidationError(
                "operation_id is invalid".to_string(),
            ));
        }
        let include_terminal = input
            .get("include_terminal")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || requested_operation_id.is_some();
        let mut store = confirmation_store().lock().await;
        store.cleanup();
        let mut operations = store
            .operations
            .values()
            .filter_map(|operation| {
                stored_operation_view(
                    operation,
                    &self.actor_scope,
                    &self.plugin_id,
                    requested_operation_id,
                    include_terminal,
                )
            })
            .collect::<Vec<_>>();
        operations.sort_by(|left, right| {
            left["operation_id"]
                .as_str()
                .cmp(&right["operation_id"].as_str())
        });
        Ok(serde_json::json!({ "operations": operations }))
    }

    async fn execute_tool(&self, tool: &str, input: Value) -> Result<Value> {
        self.authorized_tool(tool)?;
        let router = PanelMcpServer::agent_tool_router();
        let arguments = match input {
            Value::Null => None,
            Value::Object(arguments) => Some(arguments),
            _ => {
                return Err(AppError::ValidationError(
                    "Agent tool input must be a JSON object".to_string(),
                ));
            }
        };
        let (server_transport, _client_transport) = tokio::io::duplex(8 * 1024);
        let mut running = serve_directly::<RoleServer, _, _, _, _>(
            PanelMcpServer::new(self.state.clone()),
            server_transport,
            None,
        );
        let mut request_context = rmcp::service::RequestContext::new(
            rmcp::model::NumberOrString::Number(1),
            running.peer().clone(),
        );
        let request = axum::http::Request::builder()
            .uri("/mcp")
            .body(())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let (mut parts, _) = request.into_parts();
        parts.extensions.insert(self.user.clone());
        request_context.extensions.insert(parts);
        let params = if let Some(arguments) = arguments {
            CallToolRequestParams::new(tool.to_string()).with_arguments(arguments)
        } else {
            CallToolRequestParams::new(tool.to_string())
        };
        let context = ToolCallContext::new(running.service(), params, request_context);
        let response = router.call(context).await.map_err(|error| {
            AppError::ValidationError(format!("{:?}: {}", error.code, error.message))
        })?;
        let _ = running.close().await;
        let CallToolResponse::Complete(result) = response else {
            return Err(AppError::ValidationError(
                "Agent tools do not support deferred MCP responses".to_string(),
            ));
        };
        if result.is_error == Some(true) {
            let details = serde_json::to_string(&result.content)
                .unwrap_or_else(|_| "MCP tool failed".to_string());
            return Err(AppError::ValidationError(details));
        }
        Ok(result
            .structured_content
            .unwrap_or_else(|| serde_json::to_value(result.content).unwrap_or(Value::Null)))
    }

    async fn audit(&self, action: &str, operation_id: Option<String>, details: Value) {
        AuditService::log_user(
            &self.state.db,
            &self.user,
            action,
            "agent_operation",
            operation_id,
            Some(details.to_string()),
        )
        .await;
    }
}

fn requested_tool_name(value: &Value) -> Option<String> {
    value
        .as_str()
        .or_else(|| value.get("name").and_then(Value::as_str))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
}

fn operation_actor_scope(
    plugin_id: &str,
    principal: &str,
    channel: &str,
    session_id: Option<&str>,
) -> String {
    format!(
        "{plugin_id}:{principal}:{channel}:{}",
        session_id.unwrap_or("unscoped")
    )
}

fn tool_catalog() -> &'static HashMap<&'static str, McpToolInfo> {
    static CATALOG: OnceLock<HashMap<&'static str, McpToolInfo>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        McpInfoResponse::current()
            .tools
            .into_iter()
            .map(|tool| (tool.name, tool))
            .collect()
    })
}

fn user_has_catalog_permission(user: &AuthenticatedUser, info: &McpToolInfo) -> bool {
    info.permission
        .parse::<Permission>()
        .is_ok_and(|permission| user.has_permission(permission))
}

fn operation_id(input: &Value) -> Result<&str> {
    input
        .get("operation_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 64)
        .ok_or_else(|| AppError::ValidationError("operation_id is required".to_string()))
}

fn validate_plan_step_id(step_id: &str) -> Result<()> {
    let step_id = step_id.trim();
    if step_id.is_empty()
        || step_id.len() > 32
        || !step_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(AppError::ValidationError(format!(
            "Invalid Agent operation plan step id '{step_id}'"
        )));
    }
    Ok(())
}

fn validate_plan_references(
    value: &Value,
    prior_steps: &HashSet<String>,
    current_step: &str,
) -> Result<()> {
    match value {
        Value::Object(object) if object.contains_key("$from_step") => {
            if object.len() != 2 || !object.contains_key("path") {
                return Err(AppError::ValidationError(format!(
                    "Step '{current_step}' contains an invalid result reference"
                )));
            }
            let source = object
                .get("$from_step")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|source| prior_steps.contains(*source))
                .ok_or_else(|| {
                    AppError::ValidationError(format!(
                        "Step '{current_step}' can only reference an earlier step"
                    ))
                })?;
            let path = object
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|path| path.starts_with('/') && path.len() <= 256)
                .ok_or_else(|| {
                    AppError::ValidationError(format!(
                        "Step '{current_step}' reference to '{source}' has an invalid JSON Pointer"
                    ))
                })?;
            if path.contains("//") {
                return Err(AppError::ValidationError(format!(
                    "Step '{current_step}' reference to '{source}' has an invalid JSON Pointer"
                )));
            }
            Ok(())
        }
        Value::Object(object) => {
            for child in object.values() {
                validate_plan_references(child, prior_steps, current_step)?;
            }
            Ok(())
        }
        Value::Array(values) => {
            for child in values {
                validate_plan_references(child, prior_steps, current_step)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn resolve_plan_references(
    value: &Value,
    outputs: &HashMap<String, Value>,
    current_step: &str,
) -> Result<Value> {
    match value {
        Value::Object(object) if object.contains_key("$from_step") => {
            let source = object
                .get("$from_step")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    AppError::ValidationError(format!(
                        "Step '{current_step}' contains an invalid result reference"
                    ))
                })?;
            let path = object.get("path").and_then(Value::as_str).ok_or_else(|| {
                AppError::ValidationError(format!(
                    "Step '{current_step}' contains an invalid result reference"
                ))
            })?;
            outputs
                .get(source)
                .and_then(|output| output.pointer(path))
                .cloned()
                .ok_or_else(|| {
                    AppError::ValidationError(format!(
                        "Step '{current_step}' could not resolve '{source}{path}'"
                    ))
                })
        }
        Value::Object(object) => object
            .iter()
            .map(|(key, child)| {
                resolve_plan_references(child, outputs, current_step)
                    .map(|resolved| (key.clone(), resolved))
            })
            .collect::<Result<Map<String, Value>>>()
            .map(Value::Object),
        Value::Array(values) => values
            .iter()
            .map(|child| resolve_plan_references(child, outputs, current_step))
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        value => Ok(value.clone()),
    }
}

fn max_risk_level(left: McpToolRiskLevel, right: McpToolRiskLevel) -> McpToolRiskLevel {
    fn rank(level: McpToolRiskLevel) -> u8 {
        match level {
            McpToolRiskLevel::Read => 0,
            McpToolRiskLevel::Low => 1,
            McpToolRiskLevel::Medium => 2,
            McpToolRiskLevel::High => 3,
        }
    }
    if rank(left) >= rank(right) {
        left
    } else {
        right
    }
}

fn failed_plan_output(
    summary: &str,
    mut completed_steps: Vec<Value>,
    failed_step: &OperationPlanStep,
    error: String,
) -> Value {
    completed_steps.push(serde_json::json!({
        "id": failed_step.id,
        "tool": failed_step.tool,
        "status": "failed",
        "error": error,
    }));
    serde_json::json!({
        "status": "failed",
        "summary": summary.trim(),
        "failed_step": failed_step.id,
        "error": error,
        "steps": completed_steps,
    })
}

fn staged_response(operation: &PendingOperation) -> Value {
    serde_json::json!({
        "status": "confirmation_required",
        "operation_id": operation.id,
        "tool": operation.tool,
        "risk_level": operation.risk_level,
        "target": operation.target_summary,
        "input": operation.redacted_input,
        "created_seconds_ago": operation.created_at.elapsed().as_secs(),
        "expires_in_seconds": operation.expires_at.saturating_duration_since(Instant::now()).as_secs(),
        "instruction": "Ask the user to confirm this operation. After explicit confirmation, call niupanel_confirm_operation with operation_id.",
    })
}

fn stored_operation_view(
    stored: &StoredOperation,
    actor_scope: &str,
    plugin_id: &str,
    requested_operation_id: Option<&str>,
    include_terminal: bool,
) -> Option<Value> {
    let operation = match stored {
        StoredOperation::Pending(operation)
        | StoredOperation::Executing(operation)
        | StoredOperation::Terminal { operation, .. } => operation,
    };
    if operation.actor_scope != actor_scope
        || operation.plugin_id != plugin_id
        || requested_operation_id.is_some_and(|operation_id| operation.id != operation_id)
    {
        return None;
    }
    match stored {
        StoredOperation::Pending(_) => Some(serde_json::json!({
            "operation_id": operation.id,
            "tool": operation.tool,
            "status": "pending_confirmation",
            "risk_level": operation.risk_level,
            "target": operation.target_summary,
            "input": operation.redacted_input,
            "expires_in_seconds": operation.expires_at.saturating_duration_since(Instant::now()).as_secs(),
        })),
        StoredOperation::Executing(_) => Some(serde_json::json!({
            "operation_id": operation.id,
            "tool": operation.tool,
            "status": "executing",
            "risk_level": operation.risk_level,
            "target": operation.target_summary,
        })),
        StoredOperation::Terminal {
            status,
            output,
            error,
            expires_at,
            ..
        } if include_terminal => Some(serde_json::json!({
            "operation_id": operation.id,
            "tool": operation.tool,
            "status": status.as_str(),
            "risk_level": operation.risk_level,
            "target": operation.target_summary,
            "input": operation.redacted_input,
            "output": output.as_ref().map(redact_sensitive_value),
            "error": error,
            "expires_in_seconds": expires_at.saturating_duration_since(Instant::now()).as_secs(),
        })),
        StoredOperation::Terminal { .. } => None,
    }
}

fn terminal_operation(
    operation: &PendingOperation,
    status: OperationTerminalStatus,
    output: Option<Value>,
    error: Option<String>,
) -> StoredOperation {
    StoredOperation::Terminal {
        operation: operation.clone(),
        status,
        output,
        error,
        expires_at: Instant::now() + COMPLETED_TTL,
    }
}

fn operation_target_summary(tool: &str, input: &Value) -> String {
    let Some(object) = input.as_object() else {
        return tool.to_string();
    };
    for key in [
        "task_id",
        "job_id",
        "environment_id",
        "variable_id",
        "path",
        "name",
        "key",
        "url",
    ] {
        if let Some(value) = object.get(key).filter(|value| !value.is_null()) {
            let rendered = match value {
                Value::String(value) => value.clone(),
                value => value.to_string(),
            };
            return format!("{key}={}", rendered.chars().take(160).collect::<String>());
        }
    }
    tool.to_string()
}

fn redact_sensitive_value(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    let normalized = key.to_ascii_lowercase();
                    let sensitive = [
                        "token",
                        "secret",
                        "password",
                        "api_key",
                        "authorization",
                        "cookie",
                        "private_key",
                        "value",
                    ]
                    .iter()
                    .any(|candidate| normalized.contains(candidate));
                    (
                        key.clone(),
                        if sensitive {
                            Value::String("[REDACTED]".to_string())
                        } else {
                            redact_sensitive_value(value)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_sensitive_value).collect()),
        value => value.clone(),
    }
}

fn control_tool_definitions() -> Vec<Value> {
    let operation_schema = Value::Object(Map::from_iter([
        ("type".to_string(), Value::String("object".to_string())),
        (
            "properties".to_string(),
            serde_json::json!({
                "operation_id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 64
                }
            }),
        ),
        ("required".to_string(), serde_json::json!(["operation_id"])),
        ("additionalProperties".to_string(), Value::Bool(false)),
    ]));
    vec![
        serde_json::json!({
            "name": EXECUTE_PLAN_TOOL,
            "description": "Stage or execute one ordered NiuPanel operation plan. Use this when a user goal needs multiple host tool calls so the complete, exact plan is authorized once. Each step is still permission checked. A later step may consume a prior result by using an exact reference object such as {\"$from_step\":\"create_task\",\"path\":\"/id\"} as an input value. Steps stop on the first failure.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 240,
                        "description": "Concise user-facing description of the complete effect"
                    },
                    "steps": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": MAX_PLAN_STEPS,
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "minLength": 1,
                                    "maxLength": 32,
                                    "pattern": "^[A-Za-z0-9_-]+$"
                                },
                                "tool": {
                                    "type": "string",
                                    "minLength": 1,
                                    "maxLength": 128
                                },
                                "input": {
                                    "type": "object",
                                    "description": "Exact host tool input. To inject an earlier result, replace a value with {\"$from_step\":\"step_id\",\"path\":\"/json/pointer\"}."
                                }
                            },
                            "required": ["id", "tool", "input"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["summary", "steps"],
                "additionalProperties": false
            },
            "access": "control",
            "requires_confirmation": false,
            "destructive": false,
            "category": "operations",
        }),
        serde_json::json!({
            "name": CONFIRM_TOOL,
            "description": "Execute one previously staged NiuPanel operation after the user explicitly confirms it.",
            "input_schema": operation_schema.clone(),
            "access": "control",
            "requires_confirmation": false,
            "destructive": false,
            "category": "operations",
        }),
        serde_json::json!({
            "name": CANCEL_TOOL,
            "description": "Cancel one pending NiuPanel operation.",
            "input_schema": operation_schema,
            "access": "control",
            "requires_confirmation": false,
            "destructive": false,
            "category": "operations",
        }),
        serde_json::json!({
            "name": PENDING_TOOL,
            "description": "List pending NiuPanel operations, or query one recent terminal operation by operation_id so the Agent can continue after user confirmation.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "operation_id": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": 64
                    },
                    "include_terminal": {
                        "type": "boolean"
                    }
                },
                "additionalProperties": false
            },
            "access": "read",
            "requires_confirmation": false,
            "destructive": false,
            "category": "operations",
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_permissions_do_not_require_confirmation() {
        assert_eq!(McpToolRiskLevel::Read, McpToolRiskLevel::Read);
    }

    #[test]
    fn operation_scope_isolated_by_session() {
        let first = operation_actor_scope("agent", "user:1", "ui", Some("session-a"));
        let second = operation_actor_scope("agent", "user:1", "ui", Some("session-b"));
        assert_ne!(first, second);
        assert_ne!(first, operation_actor_scope("agent", "user:1", "ui", None));
    }

    #[test]
    fn completed_operation_is_queryable_only_in_its_session_and_is_redacted() {
        let operation = PendingOperation {
            id: "operation-1".to_string(),
            actor_scope: operation_actor_scope("agent", "user:1", "ui", Some("session-a")),
            plugin_id: "agent".to_string(),
            tool: "tasks_run".to_string(),
            risk_level: McpToolRiskLevel::High,
            input: serde_json::json!({ "task_id": 7 }),
            redacted_input: serde_json::json!({ "task_id": 7 }),
            target_summary: "task_id=7".to_string(),
            created_at: Instant::now(),
            expires_at: Instant::now() + Duration::from_secs(60),
        };
        let terminal = terminal_operation(
            &operation,
            OperationTerminalStatus::Completed,
            Some(serde_json::json!({ "api_key": "secret", "log": "done" })),
            None,
        );
        assert!(
            stored_operation_view(
                &terminal,
                &operation.actor_scope,
                "agent",
                Some("operation-1"),
                false,
            )
            .is_none()
        );
        let view = stored_operation_view(
            &terminal,
            &operation.actor_scope,
            "agent",
            Some("operation-1"),
            true,
        )
        .expect("terminal operation view");
        assert_eq!(view["status"], "completed");
        assert_eq!(view["output"]["api_key"], "[REDACTED]");
        assert_eq!(view["output"]["log"], "done");
        assert!(
            stored_operation_view(
                &terminal,
                &operation_actor_scope("agent", "user:1", "ui", Some("session-b")),
                "agent",
                Some("operation-1"),
                true,
            )
            .is_none()
        );
    }

    #[test]
    fn requested_tool_names_accept_objects_and_strings() {
        assert_eq!(
            requested_tool_name(&serde_json::json!({ "name": "tasks_list" })).as_deref(),
            Some("tasks_list")
        );
        assert_eq!(
            requested_tool_name(&serde_json::json!("tasks_run")).as_deref(),
            Some("tasks_run")
        );
    }

    #[test]
    fn operation_plan_references_are_ordered_and_resolved_structurally() {
        let input = serde_json::json!({
            "task_id": {"$from_step": "create_task", "path": "/id"},
            "metadata": [{"$from_step": "create_task", "path": "/name"}]
        });
        let prior = HashSet::from(["create_task".to_string()]);
        validate_plan_references(&input, &prior, "run_task").expect("valid prior reference");

        let outputs = HashMap::from([(
            "create_task".to_string(),
            serde_json::json!({"id": 17, "name": "Demo"}),
        )]);
        let resolved =
            resolve_plan_references(&input, &outputs, "run_task").expect("resolved input");
        assert_eq!(resolved["task_id"], 17);
        assert_eq!(resolved["metadata"][0], "Demo");

        assert!(
            validate_plan_references(&input, &HashSet::new(), "run_task").is_err(),
            "forward and unknown references must be rejected"
        );
    }

    #[test]
    fn operation_plan_control_definition_is_bounded() {
        let definition = control_tool_definitions()
            .into_iter()
            .find(|definition| definition["name"] == EXECUTE_PLAN_TOOL)
            .expect("plan definition");
        assert_eq!(
            definition["input_schema"]["properties"]["steps"]["maxItems"],
            MAX_PLAN_STEPS
        );
        assert_eq!(
            max_risk_level(McpToolRiskLevel::Low, McpToolRiskLevel::High),
            McpToolRiskLevel::High
        );

        let failed = failed_plan_output(
            "write and run",
            vec![serde_json::json!({"id": "write", "status": "completed"})],
            &OperationPlanStep {
                id: "run".to_string(),
                tool: "tasks_run".to_string(),
                input: serde_json::json!({"task_id": 1}),
            },
            "start failed".to_string(),
        );
        assert_eq!(failed["status"], "failed");
        assert_eq!(failed["failed_step"], "run");
        assert_eq!(failed["steps"][0]["status"], "completed");
        assert_eq!(failed["steps"][1]["status"], "failed");
    }

    #[test]
    fn session_reset_removes_only_matching_pending_operations() {
        fn operation(id: &str, actor_scope: &str, plugin_id: &str) -> PendingOperation {
            PendingOperation {
                id: id.to_string(),
                actor_scope: actor_scope.to_string(),
                plugin_id: plugin_id.to_string(),
                tool: "tasks_run".to_string(),
                risk_level: McpToolRiskLevel::High,
                input: serde_json::json!({ "task_id": 1 }),
                redacted_input: serde_json::json!({ "task_id": 1 }),
                target_summary: "task_id=1".to_string(),
                created_at: Instant::now(),
                expires_at: Instant::now() + Duration::from_secs(60),
            }
        }

        let mut store = ConfirmationStore::default();
        store.operations.insert(
            "matching".to_string(),
            StoredOperation::Pending(operation("matching", "chat-a", "agent")),
        );
        store.operations.insert(
            "other-chat".to_string(),
            StoredOperation::Pending(operation("other-chat", "chat-b", "agent")),
        );
        store.operations.insert(
            "executing".to_string(),
            StoredOperation::Executing(operation("executing", "chat-a", "agent")),
        );

        assert_eq!(
            store.take_pending_operations("chat-a", "agent"),
            vec![("matching".to_string(), "tasks_run".to_string())]
        );
        assert!(!store.operations.contains_key("matching"));
        assert!(store.operations.contains_key("other-chat"));
        assert!(store.operations.contains_key("executing"));
    }
}
