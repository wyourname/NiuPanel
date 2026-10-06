use niupanel_common::warn;
use niupanel_core::audit::service::AuditService;
use std::sync::Arc;

pub(super) fn telegram_agent_handler(
    state: crate::common::state::AppState,
    configured_plugin_id: String,
) -> niupanel_common::telegram_protocol::TelegramAgentHandler {
    use niupanel_common::telegram_protocol::{TelegramAgentRequest, TelegramAgentResponse};
    use niupanel_plugin::{PluginActionCaller, PluginActionInvokeRequest};

    let plugin_id = if configured_plugin_id.trim().is_empty() {
        "niupanel-private-agents".to_string()
    } else {
        configured_plugin_id
    };
    Arc::new(move |request: TelegramAgentRequest, cancellation| {
        let plugin_id = plugin_id.clone();
        let state = state.clone();
        Box::pin(async move {
            use niupanel_common::error::AppError;
            use niupanel_common::telegram_protocol::{
                TelegramAgentAction, TelegramAgentError, TelegramAgentProgress,
            };

            let session_id = format!("telegram:{}", request.session_key());
            let user = crate::modules::auth::middleware::load_authenticated_user(
                &state,
                request.user_id,
                format!(
                    "telegram:{}:{}:{}",
                    request.chat_id,
                    request.thread_id.unwrap_or(0),
                    request.telegram_user_id
                ),
            )
            .await
            .map_err(|error| TelegramAgentError::Failed(error.to_string()))?;
            let grant_binding = crate::modules::agent_policy::ApprovalGrantBinding {
                plugin_id: plugin_id.clone(),
                principal: format!("user:{}", request.user_id),
                channel: "telegram".to_string(),
                session_id: session_id.clone(),
            };
            match request.action {
                TelegramAgentAction::EnableYolo => {
                    if user.role != niupanel_common::auth::permissions::UserRole::Admin {
                        return Err(TelegramAgentError::Failed(
                            "只有面板管理员可以启用 YOLO 模式".to_string(),
                        ));
                    }
                    let issued = crate::modules::agent_policy::issue_grant(
                        grant_binding.clone(),
                        crate::modules::agent_policy::EffectiveApprovalMode::Yolo,
                        None,
                    )
                    .await;
                    audit_telegram_grant_issued(&state, &user, &grant_binding, &issued).await;
                    return Ok(TelegramAgentResponse {
                        text: "当前 Telegram 会话已启用临时 YOLO。显式禁止的工具仍不会执行。使用 /mode normal 退出。".to_string(),
                    });
                }
                TelegramAgentAction::DisableYolo => {
                    if user.role != niupanel_common::auth::permissions::UserRole::Admin {
                        return Err(TelegramAgentError::Failed(
                            "只有面板管理员可以修改审批模式".to_string(),
                        ));
                    }
                    let revoked = crate::modules::agent_policy::revoke_grants(&grant_binding).await;
                    audit_telegram_grants_revoked(
                        &state,
                        &user,
                        &grant_binding,
                        revoked,
                        "mode_normal",
                    )
                    .await;
                    return Ok(TelegramAgentResponse {
                        text: "当前 Telegram 会话已恢复全局审批策略。".to_string(),
                    });
                }
                TelegramAgentAction::ResetSession => {
                    let revoked = crate::modules::agent_policy::revoke_grants(&grant_binding).await;
                    if revoked > 0 {
                        audit_telegram_grants_revoked(
                            &state,
                            &user,
                            &grant_binding,
                            revoked,
                            "session_reset",
                        )
                        .await;
                    }
                }
                TelegramAgentAction::Chat => {}
            }
            let (action, input) = if request.action == TelegramAgentAction::ResetSession {
                (
                    "session_delete",
                    serde_json::json!({ "session_id": session_id }),
                )
            } else {
                (
                    "chat",
                    serde_json::json!({
                        "message": request.text.clone(),
                        "attachments": request.attachments.clone(),
                        "session_id": session_id.clone(),
                        "panel_context": request.panel_context.clone(),
                        "route": "telegram",
                        "locale": "zh-CN",
                    }),
                )
            };
            let gateway = crate::modules::agent_tools::AgentToolGateway::for_plugin(
                state,
                user,
                &plugin_id,
                crate::modules::agent_tools::AgentInvocationIdentity {
                    principal: format!("user:{}", request.user_id),
                    channel: "telegram".to_string(),
                    session_id: Some(session_id.clone()),
                    presented_grant: None,
                    allow_implicit_grant: true,
                },
            )
            .await
            .map_err(|error| TelegramAgentError::Failed(error.to_string()))?;
            if request.action == TelegramAgentAction::ResetSession {
                gateway.cancel_pending_operations().await;
            }
            let coordination_scope = gateway.coordination_scope();
            let _session_guard =
                crate::modules::agent_invocations::acquire_session(&coordination_scope).await;
            let idempotency = crate::modules::agent_invocations::begin(
                &coordination_scope,
                action,
                Some(&request.client_request_id),
                &input,
            )
            .await
            .map_err(|error| TelegramAgentError::Failed(error.to_string()))?;
            let token = match idempotency {
                crate::modules::agent_invocations::IdempotencyDecision::Execute(token) => token,
                crate::modules::agent_invocations::IdempotencyDecision::Replay(output) => {
                    let text = if request.action == TelegramAgentAction::ResetSession {
                        "当前 Telegram Agent 会话已清空。".to_string()
                    } else {
                        output
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .filter(|message| !message.trim().is_empty())
                            .ok_or_else(|| {
                                TelegramAgentError::Failed(
                                    "Agent 重放结果缺少 message 字段".to_string(),
                                )
                            })?
                            .to_string()
                    };
                    return Ok(TelegramAgentResponse { text });
                }
            };
            let base_tool_handler = gateway.handler();
            let invocation_context = gateway.invocation_context(
                PluginActionCaller::Telegram,
                Some(&request.client_request_id),
            );
            let progress = request.progress.clone();
            let tool_handler = move |call: niupanel_plugin::ProcessPluginToolCall| {
                progress.send_replace(TelegramAgentProgress::CallingTool(call.tool.clone()));
                base_tool_handler(call)
            };
            let response = crate::modules::plugins::service::unified_plugin_service()
                .invoke_action_with_tools_context_cancellable(
                    &plugin_id,
                    PluginActionCaller::Telegram,
                    PluginActionInvokeRequest {
                        action: action.to_string(),
                        input,
                        client_request_id: Some(request.client_request_id.clone()),
                    },
                    Some(invocation_context),
                    gateway.definitions(),
                    tool_handler,
                    cancellation,
                )
                .await;

            let response = match response {
                Ok(response) => {
                    crate::modules::agent_invocations::complete(token, &response.output).await;
                    response
                }
                Err(AppError::Cancelled) => {
                    crate::modules::agent_invocations::fail(token).await;
                    if request.action == TelegramAgentAction::Chat
                        && let Err(cleanup_error) =
                            crate::modules::plugins::service::unified_plugin_service()
                                .invoke_action(
                                    &plugin_id,
                                    PluginActionCaller::Telegram,
                                    PluginActionInvokeRequest {
                                        action: "session_interrupt".to_string(),
                                        input: serde_json::json!({
                                            "session_id": session_id.clone(),
                                            "reason": "telegram_run_cancelled",
                                        }),
                                        client_request_id: Some(format!(
                                            "{}:interrupt",
                                            request.client_request_id
                                        )),
                                    },
                                )
                                .await
                    {
                        warn!(
                            "Failed to mark cancelled Telegram Agent session '{}' as interrupted: {}",
                            session_id, cleanup_error
                        );
                    }
                    return Err(TelegramAgentError::Cancelled);
                }
                Err(error) => {
                    crate::modules::agent_invocations::fail(token).await;
                    return Err(TelegramAgentError::Failed(error.to_string()));
                }
            };

            request.report_progress(TelegramAgentProgress::Finalizing);
            let text = if request.action == TelegramAgentAction::ResetSession {
                "当前 Telegram Agent 会话已清空。".to_string()
            } else {
                response
                    .output
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .filter(|message| !message.trim().is_empty())
                    .ok_or_else(|| {
                        TelegramAgentError::Failed("Agent 返回内容缺少 message 字段".to_string())
                    })?
                    .to_string()
            };
            Ok(TelegramAgentResponse { text })
        })
    })
}

async fn audit_telegram_grant_issued(
    state: &crate::common::state::AppState,
    user: &niupanel_common::auth::permissions::AuthenticatedUser,
    binding: &crate::modules::agent_policy::ApprovalGrantBinding,
    issued: &crate::modules::agent_policy::IssuedApprovalGrant,
) {
    AuditService::log_user(
        &state.db,
        user,
        "agent.approval_grant.issued",
        "plugin",
        Some(binding.plugin_id.clone()),
        Some(
            serde_json::json!({
                "grant_id": issued.grant_id,
                "principal": binding.principal,
                "channel": binding.channel,
                "session_id": binding.session_id,
                "expires_in_seconds": issued.expires_in_seconds,
            })
            .to_string(),
        ),
    )
    .await;
}

async fn audit_telegram_grants_revoked(
    state: &crate::common::state::AppState,
    user: &niupanel_common::auth::permissions::AuthenticatedUser,
    binding: &crate::modules::agent_policy::ApprovalGrantBinding,
    revoked: usize,
    reason: &'static str,
) {
    AuditService::log_user(
        &state.db,
        user,
        "agent.approval_grant.revoked",
        "plugin",
        Some(binding.plugin_id.clone()),
        Some(
            serde_json::json!({
                "principal": binding.principal,
                "channel": binding.channel,
                "session_id": binding.session_id,
                "revoked": revoked,
                "reason": reason,
            })
            .to_string(),
        ),
    )
    .await;
}
