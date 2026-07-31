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
    request_body = PluginInvokeRequest,
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
    Json(payload): Json<PluginInvokeRequest>,
) -> Result<ApiResponse<Value>> {
    let action = payload.action.clone();
    let result = async {
        let service = unified_plugin_service();
        let record = service.get_plugin(&plugin_id)?;
        ensure_plugin_ui_access(&plugin_id, &user)?;

        if !record.enabled || !matches!(record.status, PluginStatus::Enabled) {
            return Err(AppError::ValidationError(
                "Plugin is not enabled".to_string(),
            ));
        }
        if !plugin_ui_may_invoke(&record.manifest.capabilities) {
            return Err(AppError::ValidationError(
                "Plugin does not declare ui.invoke capability".to_string(),
            ));
        }

        service.invoke_plugin(&plugin_id, payload).await
    }
    .await;

    match result {
        Ok(response) => {
            audit_plugin_invoke(
                &state,
                &user,
                "plugin.invoke.completed",
                &plugin_id,
                &action,
                None,
            )
            .await;
            Ok(ApiResponse::success(response.output))
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

fn plugin_ui_may_invoke(capabilities: &[String]) -> bool {
    plugin_has_capability(capabilities, "ui.invoke")
        || plugin_has_capability(capabilities, "agents.invoke")
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

#[cfg(test)]
mod tests {
    use super::plugin_ui_may_invoke;

    #[test]
    fn ui_invoke_accepts_native_and_legacy_agent_capabilities() {
        assert!(plugin_ui_may_invoke(&["ui.invoke".to_string()]));
        assert!(plugin_ui_may_invoke(&["agents.invoke".to_string()]));
        assert!(plugin_ui_may_invoke(&["ui.*".to_string()]));
        assert!(!plugin_ui_may_invoke(&["yyb.accounts".to_string()]));
    }
}
