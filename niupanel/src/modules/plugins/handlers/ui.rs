use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/plugins/apps",
    responses(
        (status = 200, description = "List installed plugin apps")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn list_plugin_apps(
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<ApiResponse<Vec<PluginAppRecord>>> {
    Ok(ApiResponse::success(installed_plugin_apps(Some(&user))?))
}

#[utoipa::path(
    get,
    path = "/api/v1/plugins/themes",
    responses(
        (status = 200, description = "List enabled declarative plugin themes")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn list_plugin_themes() -> Result<ApiResponse<Vec<PluginThemeRecord>>> {
    Ok(ApiResponse::success(installed_plugin_themes()?))
}

#[utoipa::path(
    get,
    path = "/api/v1/plugins/health",
    responses(
        (status = 200, description = "List installed plugin health reports")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn list_plugin_health() -> Result<ApiResponse<Vec<PluginHealthReport>>> {
    Ok(ApiResponse::success(plugin_health_reports()?))
}

pub async fn serve_plugin_ui_asset(
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath((plugin_id, asset_path)): AxumPath<(String, String)>,
) -> Result<Response> {
    let (ui_root, _) = resolve_plugin_ui_root(&plugin_id)?;
    ensure_plugin_ui_access(&plugin_id, &user)?;
    let safe_path = sanitize_ui_asset_path(&asset_path)?;
    let target = ui_root.join(safe_path);
    if !target.starts_with(&ui_root) || !target.is_file() {
        return Err(AppError::NotFound("Plugin UI asset not found".to_string()));
    }

    let bytes = tokio::fs::read(&target).await?;
    let mut response = (StatusCode::OK, bytes).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type_for_path(&target)),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/api",
    request_body = PluginApiProxyRequest,
    responses(
        (status = 200, description = "Proxy an API request from a plugin UI after manifest and user permission checks")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn proxy_plugin_api_request(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
    Json(payload): Json<PluginApiProxyRequest>,
) -> Result<Response> {
    let method = parse_proxy_method(payload.method.as_deref())?;
    let api_path = canonical_api_path(&payload.path)?;
    let access = ensure_plugin_ui_access(&plugin_id, &user).and_then(|app| {
        ensure_plugin_api_request_allowed(&app, &user, &method, &api_path)?;
        Ok(app)
    });
    if let Err(error) = access {
        audit_plugin_request(
            &state,
            &user,
            "plugin.api.denied",
            &plugin_id,
            &method,
            &api_path,
            None,
            Some(error.to_string()),
        )
        .await;
        return Err(error);
    }

    let target_url = build_internal_api_url(&api_path, payload.params.as_ref())?;
    let internal_token =
        sdk_token::issue_internal_user_token(&Config::global().session_key, user.id, 30);
    let mut request = state.http_client.request(method.clone(), target_url);
    request = request.header(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {internal_token}"),
    );
    request = request.header(reqwest::header::ACCEPT, "application/json");
    if method != Method::GET {
        if let Some(data) = payload.data {
            request = request.json(&data);
        }
    }

    let upstream = match request.send().await {
        Ok(upstream) => upstream,
        Err(error) => {
            audit_plugin_request(
                &state,
                &user,
                "plugin.api.failed",
                &plugin_id,
                &method,
                &api_path,
                None,
                Some(error.to_string()),
            )
            .await;
            return Err(error.into());
        }
    };
    let status = StatusCode::from_u16(upstream.status().as_u16())
        .map_err(|_| AppError::Internal("Invalid upstream status code".to_string()))?;
    let content_type = upstream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| HeaderValue::from_str(value).ok());
    let body = upstream.bytes().await?;

    let mut response = (status, body).into_response();
    if let Some(content_type) = content_type {
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type);
    }
    audit_plugin_request(
        &state,
        &user,
        "plugin.api.completed",
        &plugin_id,
        &method,
        &api_path,
        Some(status.as_u16()),
        None,
    )
    .await;
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/invoke",
    request_body = PluginActionInvokeRequest,
    responses(
        (status = 200, description = "Invoke an enabled native plugin action")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn invoke_plugin_action(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
    headers: HeaderMap,
    Json(payload): Json<PluginActionInvokeRequest>,
) -> Result<ApiResponse<Value>> {
    let action = payload.action.clone();
    let result: Result<Value> = async {
        let service = unified_plugin_service();
        ensure_plugin_ui_access(&plugin_id, &user)?;
        let identity = ui_invocation_identity(&user, &payload, &headers);
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
        let crate::modules::agent_invocations::IdempotencyDecision::Execute(token) = idempotency
        else {
            let crate::modules::agent_invocations::IdempotencyDecision::Replay(output) =
                idempotency
            else {
                unreachable!()
            };
            return Ok(output);
        };
        let invocation_context = gateway
            .invocation_context(PluginActionCaller::Ui, payload.client_request_id.as_deref());
        let result = service
            .invoke_action_with_tools_context(
                &plugin_id,
                PluginActionCaller::Ui,
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
            audit_plugin_invoke(
                &state,
                &user,
                "plugin.invoke.completed",
                &plugin_id,
                &action,
                None,
            )
            .await;
            Ok(ApiResponse::success(output))
        }
        Err(error) => {
            audit_plugin_invoke(
                &state,
                &user,
                "plugin.invoke.denied_or_failed",
                &plugin_id,
                &action,
                Some(error.to_string()),
            )
            .await;
            Err(error)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/{id}/invoke/stream",
    request_body = PluginActionInvokeRequest,
    responses(
        (status = 200, description = "Stream an enabled native plugin action as server-sent events")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn stream_plugin_action(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    AxumPath(plugin_id): AxumPath<String>,
    headers: HeaderMap,
    Json(payload): Json<PluginActionInvokeRequest>,
) -> Result<Response> {
    ensure_plugin_ui_access(&plugin_id, &user)?;
    ensure_action_supports_streaming(&plugin_id, PluginActionCaller::Ui, &payload.action)?;
    let identity = ui_invocation_identity(&user, &payload, &headers);
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
        gateway.invocation_context(PluginActionCaller::Ui, payload.client_request_id.as_deref());
    let tool_handler = gateway.handler();
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    let cancellation = tokio_util::sync::CancellationToken::new();
    let invocation_cancellation = cancellation.clone();
    let action = payload.action.clone();
    let audit_state = state.clone();
    let audit_user = user.clone();
    let audit_plugin_id = plugin_id.clone();
    tokio::spawn(async move {
        let _session_guard = session_guard;
        let event_sender = sender.clone();
        let result = unified_plugin_service()
            .invoke_action_stream_with_tools_context_cancellable(
                &plugin_id,
                PluginActionCaller::Ui,
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
                audit_plugin_invoke(
                    &audit_state,
                    &audit_user,
                    "plugin.invoke.stream.completed",
                    &audit_plugin_id,
                    &action,
                    None,
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
                audit_plugin_invoke(
                    &audit_state,
                    &audit_user,
                    "plugin.invoke.stream.denied_or_failed",
                    &audit_plugin_id,
                    &action,
                    Some(message.clone()),
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

async fn audit_plugin_request(
    state: &AppState,
    user: &AuthenticatedUser,
    action: &str,
    plugin_id: &str,
    method: &Method,
    path: &str,
    status: Option<u16>,
    error: Option<String>,
) {
    let details = serde_json::json!({
        "method": method.as_str(),
        "path": path,
        "status": status,
        "error": error,
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

async fn audit_plugin_invoke(
    state: &AppState,
    user: &AuthenticatedUser,
    action: &str,
    plugin_id: &str,
    plugin_action: &str,
    error: Option<String>,
) {
    let details = serde_json::json!({
        "plugin_action": plugin_action,
        "error": error,
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
