use super::*;
use crate::modules::auth::middleware::ApiKeyAuthContext;
use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub(super) enum PluginInvocationSseMessage {
    Event(niupanel_plugin::ProcessPluginStreamEvent),
    Result(Value),
    Error { code: i32, message: String },
}

struct InvocationCancellationGuard(CancellationToken);

impl Drop for InvocationCancellationGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub(super) fn ensure_action_supports_streaming(
    plugin_id: &str,
    caller: PluginActionCaller,
    action_name: &str,
) -> Result<()> {
    let actions = unified_plugin_service().list_plugin_actions(plugin_id, caller)?;
    let action = actions
        .iter()
        .find(|action| action.name == action_name)
        .ok_or_else(|| AppError::NotFound("Plugin action not found".to_string()))?;
    if !action.streaming {
        return Err(AppError::ValidationError(format!(
            "Plugin action '{}' does not support streaming",
            action.name
        )));
    }
    Ok(())
}

pub(super) fn plugin_invocation_sse_response(
    mut receiver: mpsc::Receiver<PluginInvocationSseMessage>,
    cancellation: CancellationToken,
) -> Response {
    let stream = async_stream::stream! {
        let _guard = InvocationCancellationGuard(cancellation);
        while let Some(message) = receiver.recv().await {
            let (event, terminal) = match message {
                PluginInvocationSseMessage::Event(frame) => {
                    let data = serde_json::json!({
                        "sequence": frame.sequence,
                        "data": frame.data,
                    });
                    (
                        Event::default()
                            .event(frame.event)
                            .id(frame.sequence.to_string())
                            .data(data.to_string()),
                        false,
                    )
                }
                PluginInvocationSseMessage::Result(data) => (
                    Event::default().event("result").data(data.to_string()),
                    true,
                ),
                PluginInvocationSseMessage::Error { code, message } => (
                    Event::default()
                        .event("error")
                        .data(serde_json::json!({ "code": code, "message": message }).to_string()),
                    true,
                ),
            };
            yield Ok::<Event, Infallible>(event);
            if terminal {
                break;
            }
        }
    };
    let mut response = Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );
    response
}

pub(super) fn completed_plugin_invocation_sse_response(output: Value) -> Response {
    let (sender, receiver) = mpsc::channel(1);
    let _ = sender.try_send(PluginInvocationSseMessage::Result(output));
    drop(sender);
    plugin_invocation_sse_response(receiver, CancellationToken::new())
}

#[utoipa::path(
    get,
    path = "/open/api/plugins",
    responses((status = 200, description = "List plugins callable by the current token")),
    tag = "Plugins",
    security(("api_key" = []))
)]
pub async fn list_open_plugin_actions(
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<Vec<niupanel_plugin::PluginActionPlugin>>> {
    let caller = caller_for_open_api(&user);
    Ok(ApiResponse::success(
        unified_plugin_service().list_action_plugins(caller)?,
    ))
}

#[utoipa::path(
    get,
    path = "/open/api/plugins/{plugin_id}/actions",
    params(("plugin_id" = String, Path, description = "Plugin id")),
    responses((status = 200, description = "List actions callable by the current token")),
    tag = "Plugins",
    security(("api_key" = []))
)]
pub async fn list_open_plugin_actions_for_plugin(
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
) -> Result<ApiResponse<Vec<niupanel_plugin::PluginActionManifest>>> {
    let caller = caller_for_open_api(&user);
    Ok(ApiResponse::success(
        unified_plugin_service().list_plugin_actions(&plugin_id, caller)?,
    ))
}

#[utoipa::path(
    post,
    path = "/open/api/plugins/{plugin_id}/invoke",
    params(("plugin_id" = String, Path, description = "Plugin id")),
    request_body = PluginActionInvokeRequest,
    responses(
        (status = 200, description = "Invoke a plugin action"),
        (status = 400, description = "Invalid action input"),
        (status = 403, description = "Action is not available to this caller"),
        (status = 404, description = "Plugin or action not found"),
        (status = 429, description = "Plugin concurrency limit reached")
    ),
    tag = "Plugins",
    security(("api_key" = []))
)]
pub async fn invoke_open_plugin_action(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    api_key: Option<Extension<ApiKeyAuthContext>>,
    task_claims: Option<Extension<niupanel_common::auth::sdk_token::InternalSdkClaims>>,
    AxumPath(plugin_id): AxumPath<String>,
    headers: HeaderMap,
    Json(payload): Json<niupanel_plugin::PluginActionInvokeRequest>,
) -> Result<ApiResponse<Value>> {
    let caller = caller_for_open_api(&user);
    let action = payload.action.clone();
    let identity = crate::modules::agent_tools::AgentInvocationIdentity {
        principal: api_key
            .as_ref()
            .map(|context| format!("api_key:{}", context.0.key_id))
            .or_else(|| {
                task_claims
                    .as_ref()
                    .map(|claims| format!("task:{}", claims.0.task_id))
            })
            .unwrap_or_else(|| format!("user:{}", user.id)),
        channel: if api_key.is_some() { "api_key" } else { "task" }.to_string(),
        session_id: invocation_session_id(&payload),
        presented_grant: approval_grant_header(&headers),
        allow_implicit_grant: false,
    };
    let result: Result<Value> = async {
        let gateway = crate::modules::agent_tools::AgentToolGateway::for_plugin(
            state.clone(),
            user.clone(),
            &plugin_id,
            identity,
        )
        .await?;
        let coordination_scope = gateway.coordination_scope();
        let _session_guard =
            crate::modules::agent_invocations::acquire_session(&coordination_scope).await;
        let idempotency = crate::modules::agent_invocations::begin(
            &coordination_scope,
            &action,
            payload.client_request_id.as_deref(),
            &payload.input,
        )
        .await?;
        let token = match idempotency {
            crate::modules::agent_invocations::IdempotencyDecision::Execute(token) => token,
            crate::modules::agent_invocations::IdempotencyDecision::Replay(output) => {
                return Ok(output);
            }
        };
        let invocation_context =
            gateway.invocation_context(caller, payload.client_request_id.as_deref());
        let result = unified_plugin_service()
            .invoke_action_with_tools_context(
                &plugin_id,
                caller,
                payload,
                Some(invocation_context),
                gateway.definitions(),
                gateway.handler(),
            )
            .await
            .map(|response| response.output);
        match &result {
            Ok(output) => crate::modules::agent_invocations::complete(token, output).await,
            Err(_) => crate::modules::agent_invocations::fail(token).await,
        }
        result
    }
    .await;
    match result {
        Ok(output) => {
            audit_plugin_action(
                &state,
                &user,
                "plugin.invoke.completed",
                &plugin_id,
                &action,
                None,
                api_key.as_ref().map(|context| context.0.key_id),
                task_claims.as_ref().map(|claims| claims.0.task_id),
                task_claims.as_ref().and_then(|claims| claims.0.run_id),
            )
            .await;
            Ok(ApiResponse::success(output))
        }
        Err(error) => {
            audit_plugin_action(
                &state,
                &user,
                "plugin.invoke.denied_or_failed",
                &plugin_id,
                &action,
                Some(error.to_string()),
                api_key.as_ref().map(|context| context.0.key_id),
                task_claims.as_ref().map(|claims| claims.0.task_id),
                task_claims.as_ref().and_then(|claims| claims.0.run_id),
            )
            .await;
            Err(error)
        }
    }
}

#[utoipa::path(
    post,
    path = "/open/api/plugins/{plugin_id}/invoke/stream",
    params(("plugin_id" = String, Path, description = "Plugin id")),
    request_body = PluginActionInvokeRequest,
    responses(
        (status = 200, description = "Stream a plugin action as server-sent events"),
        (status = 400, description = "Action does not support streaming"),
        (status = 403, description = "Action is not available to this caller"),
        (status = 404, description = "Plugin or action not found")
    ),
    tag = "Plugins",
    security(("api_key" = []))
)]
pub async fn stream_open_plugin_action(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    api_key: Option<Extension<ApiKeyAuthContext>>,
    task_claims: Option<Extension<niupanel_common::auth::sdk_token::InternalSdkClaims>>,
    AxumPath(plugin_id): AxumPath<String>,
    headers: HeaderMap,
    Json(payload): Json<niupanel_plugin::PluginActionInvokeRequest>,
) -> Result<Response> {
    let caller = caller_for_open_api(&user);
    ensure_action_supports_streaming(&plugin_id, caller, &payload.action)?;
    let identity = crate::modules::agent_tools::AgentInvocationIdentity {
        principal: api_key
            .as_ref()
            .map(|context| format!("api_key:{}", context.0.key_id))
            .or_else(|| {
                task_claims
                    .as_ref()
                    .map(|claims| format!("task:{}", claims.0.task_id))
            })
            .unwrap_or_else(|| format!("user:{}", user.id)),
        channel: if api_key.is_some() { "api_key" } else { "task" }.to_string(),
        session_id: invocation_session_id(&payload),
        presented_grant: approval_grant_header(&headers),
        allow_implicit_grant: false,
    };
    let gateway = crate::modules::agent_tools::AgentToolGateway::for_plugin(
        state.clone(),
        user.clone(),
        &plugin_id,
        identity,
    )
    .await?;
    let coordination_scope = gateway.coordination_scope();
    let session_guard =
        crate::modules::agent_invocations::acquire_session(&coordination_scope).await;
    let idempotency = crate::modules::agent_invocations::begin(
        &coordination_scope,
        &payload.action,
        payload.client_request_id.as_deref(),
        &payload.input,
    )
    .await?;
    let token = match idempotency {
        crate::modules::agent_invocations::IdempotencyDecision::Execute(token) => token,
        crate::modules::agent_invocations::IdempotencyDecision::Replay(output) => {
            return Ok(completed_plugin_invocation_sse_response(output));
        }
    };
    let tools = gateway.definitions();
    let invocation_context =
        gateway.invocation_context(caller, payload.client_request_id.as_deref());
    let tool_handler = gateway.handler();
    let (sender, receiver) = mpsc::channel(32);
    let cancellation = CancellationToken::new();
    let invocation_cancellation = cancellation.clone();
    let action = payload.action.clone();
    let audit_state = state.clone();
    let audit_user = user.clone();
    let audit_plugin_id = plugin_id.clone();
    let api_key_id = api_key.as_ref().map(|context| context.0.key_id);
    let task_id = task_claims.as_ref().map(|claims| claims.0.task_id);
    let run_id = task_claims.as_ref().and_then(|claims| claims.0.run_id);
    tokio::spawn(async move {
        let _session_guard = session_guard;
        let event_sender = sender.clone();
        let result = unified_plugin_service()
            .invoke_action_stream_with_tools_context_cancellable(
                &plugin_id,
                caller,
                payload,
                Some(invocation_context),
                tools,
                niupanel_plugin::PluginStreamHandlers::new(tool_handler, move |event| {
                    let event_sender = event_sender.clone();
                    Box::pin(async move {
                        event_sender
                            .send(PluginInvocationSseMessage::Event(event))
                            .await
                            .map_err(|_| AppError::Cancelled)
                    }) as niupanel_plugin::PluginStreamFuture
                }),
                invocation_cancellation,
            )
            .await;
        match result {
            Ok(response) => {
                crate::modules::agent_invocations::complete(token, &response.output).await;
                audit_plugin_action(
                    &audit_state,
                    &audit_user,
                    "plugin.invoke.stream.completed",
                    &audit_plugin_id,
                    &action,
                    None,
                    api_key_id,
                    task_id,
                    run_id,
                )
                .await;
                let _ = sender
                    .send(PluginInvocationSseMessage::Result(response.output))
                    .await;
            }
            Err(AppError::Cancelled) if sender.is_closed() => {
                crate::modules::agent_invocations::fail(token).await;
            }
            Err(error) => {
                crate::modules::agent_invocations::fail(token).await;
                let (_, code, message) = error.get_meta();
                audit_plugin_action(
                    &audit_state,
                    &audit_user,
                    "plugin.invoke.stream.denied_or_failed",
                    &audit_plugin_id,
                    &action,
                    Some(message.clone()),
                    api_key_id,
                    task_id,
                    run_id,
                )
                .await;
                let _ = sender
                    .send(PluginInvocationSseMessage::Error { code, message })
                    .await;
            }
        }
    });

    Ok(plugin_invocation_sse_response(receiver, cancellation))
}

pub async fn list_ui_plugin_actions(
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
) -> Result<ApiResponse<Vec<niupanel_plugin::PluginActionManifest>>> {
    ensure_plugin_ui_access(&plugin_id, &user)?;
    Ok(ApiResponse::success(
        unified_plugin_service().list_plugin_actions(&plugin_id, PluginActionCaller::Ui)?,
    ))
}

pub(super) fn caller_for_open_api(user: &AuthenticatedUser) -> PluginActionCaller {
    match user.role {
        niupanel_common::auth::permissions::UserRole::ApiClient => PluginActionCaller::ApiKey,
        _ => PluginActionCaller::Task,
    }
}

pub(super) async fn audit_plugin_action(
    state: &AppState,
    user: &AuthenticatedUser,
    action: &str,
    plugin_id: &str,
    plugin_action: &str,
    error: Option<String>,
    api_key_id: Option<i32>,
    task_id: Option<i32>,
    run_id: Option<i32>,
) {
    let details = serde_json::json!({
        "plugin_action": plugin_action,
        "error": error,
        "api_key_id": api_key_id,
        "task_id": task_id,
        "run_id": run_id,
    });
    AuditService::log_user(
        &state.db,
        user,
        action,
        "plugin",
        Some(plugin_id.to_string()),
        Some(details.to_string()),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invocation_sse_serializes_events_and_terminal_result() {
        let (sender, receiver) = mpsc::channel(4);
        sender
            .send(PluginInvocationSseMessage::Event(
                niupanel_plugin::ProcessPluginStreamEvent {
                    kind: "stream_event".to_string(),
                    request_id: "request-1".to_string(),
                    sequence: 1,
                    event: "delta".to_string(),
                    data: serde_json::json!({"text": "hello"}),
                },
            ))
            .await
            .expect("stream event");
        sender
            .send(PluginInvocationSseMessage::Result(
                serde_json::json!({"message": "hello"}),
            ))
            .await
            .expect("result event");
        drop(sender);

        let response = plugin_invocation_sse_response(receiver, CancellationToken::new());
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/event-stream")
        );
        assert_eq!(
            response
                .headers()
                .get("x-accel-buffering")
                .and_then(|value| value.to_str().ok()),
            Some("no")
        );
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .expect("SSE body");
        let body = String::from_utf8(body.to_vec()).expect("UTF-8 SSE body");
        assert!(body.contains("event: delta\n"));
        assert!(body.contains("id: 1\n"));
        assert!(body.contains("data: {\"data\":{\"text\":\"hello\"},\"sequence\":1}\n"));
        assert!(body.contains("event: result\n"));
        assert!(body.contains("data: {\"message\":\"hello\"}\n"));
    }
}
