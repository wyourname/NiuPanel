use super::*;

pub(super) fn installed_plugin_apps(
    user: Option<&AuthenticatedUser>,
) -> Result<Vec<PluginAppRecord>> {
    let mut apps = Vec::new();

    for (_, service) in plugin_services() {
        for plugin in service.list_plugins()? {
            if plugin.enabled && matches!(&plugin.status, PluginStatus::Enabled) {
                if let Some(app) = generic_plugin_app(plugin) {
                    if user.is_none_or(|user| user_can_access_plugin_ui(user, &app.ui.permissions))
                    {
                        apps.push(app);
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.cmp(&b.name).then(a.plugin_id.cmp(&b.plugin_id)));
    Ok(apps)
}

pub(super) fn installed_plugin_themes() -> Result<Vec<PluginThemeRecord>> {
    let mut themes = Vec::new();

    for (_, service) in plugin_services() {
        for plugin in service.list_plugins()? {
            if !plugin.enabled || !matches!(&plugin.status, PluginStatus::Enabled) {
                continue;
            }
            let Some(theme) = plugin.manifest.theme.clone().filter(|theme| theme.enabled) else {
                continue;
            };
            themes.push(PluginThemeRecord {
                plugin_id: plugin.manifest.id,
                name: plugin.manifest.name,
                version: plugin.manifest.version,
                description: plugin.manifest.description,
                theme,
            });
        }
    }

    themes.sort_by(|a, b| a.name.cmp(&b.name).then(a.plugin_id.cmp(&b.plugin_id)));
    Ok(themes)
}

pub(super) fn ensure_plugin_ui_access(
    plugin_id: &str,
    user: &AuthenticatedUser,
) -> Result<PluginAppRecord> {
    let app = installed_plugin_apps(None)?
        .into_iter()
        .find(|app| app.plugin_id == plugin_id)
        .ok_or_else(|| AppError::NotFound("Plugin app not found".to_string()))?;
    if !user_can_access_plugin_ui(user, &app.ui.permissions) {
        return Err(AppError::Forbidden(
            "Current user cannot access this plugin app".to_string(),
        ));
    }
    Ok(app)
}

pub(super) fn user_can_access_plugin_ui(user: &AuthenticatedUser, permissions: &[String]) -> bool {
    permissions
        .iter()
        .all(|permission| user_has_permission_string(user, permission))
}

pub(super) fn user_has_permission_string(user: &AuthenticatedUser, permission: &str) -> bool {
    Permission::from_str(permission)
        .map(|permission| user.has_permission(permission))
        .unwrap_or(false)
}

pub(super) fn parse_proxy_method(method: Option<&str>) -> Result<Method> {
    let method = method.unwrap_or("GET").to_ascii_uppercase();
    match method.as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "PATCH" => Ok(Method::PATCH),
        "DELETE" => Ok(Method::DELETE),
        _ => Err(AppError::ValidationError(
            "Plugin API method is not allowed".to_string(),
        )),
    }
}

pub(super) fn canonical_api_path(path: &str) -> Result<String> {
    let path = path.trim();
    if path.is_empty()
        || path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("//")
    {
        return Err(AppError::ValidationError(
            "Plugin API path is invalid".to_string(),
        ));
    }

    let normalized = path.strip_prefix("/api/v1").unwrap_or(path);
    let normalized = if normalized.starts_with('/') {
        normalized.to_string()
    } else {
        format!("/{normalized}")
    };
    let [pathname, ..] = normalized.split('?').collect::<Vec<_>>()[..] else {
        return Ok(normalized);
    };
    if pathname
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return Err(AppError::ValidationError(
            "Plugin API path is invalid".to_string(),
        ));
    }
    Ok(normalized)
}

pub(super) fn ensure_plugin_api_request_allowed(
    app: &PluginAppRecord,
    user: &AuthenticatedUser,
    method: &Method,
    path: &str,
) -> Result<()> {
    if !plugin_ui_api_allows(app, method, path) {
        return Err(AppError::Forbidden(format!(
            "Plugin manifest api.allow did not allow {method} {path}"
        )));
    }
    let permission = permission_for_plugin_api_request(method, path)?;
    if let Some(permission) = permission {
        let permission_text = permission.to_string();
        if !manifest_declares_permission(&app.ui.permissions, permission) {
            return Err(AppError::Forbidden(format!(
                "Plugin manifest did not declare required permission: {permission_text}"
            )));
        }
        if !user.has_permission(permission) {
            return Err(AppError::Forbidden(format!(
                "Current user lacks required permission: {permission_text}"
            )));
        }
    }
    Ok(())
}

pub(super) fn plugin_ui_api_allows(app: &PluginAppRecord, method: &Method, path: &str) -> bool {
    let allow = &app.ui.api.allow;
    if allow.is_empty() {
        return false;
    }
    allow
        .iter()
        .any(|rule| api_rule_matches(&rule.methods, &rule.path, method, path))
}

pub(super) fn api_rule_matches(
    methods: &[String],
    path_rule: &str,
    method: &Method,
    path: &str,
) -> bool {
    if !api_method_matches(methods, method) {
        return false;
    }
    api_path_matches(path_rule.trim(), path)
}

pub(super) fn api_method_matches(methods: &[String], method: &Method) -> bool {
    !methods.is_empty()
        && methods
            .iter()
            .any(|candidate| candidate.trim().eq_ignore_ascii_case(method.as_str()))
}

pub(super) fn api_path_matches(rule: &str, path: &str) -> bool {
    if let Some(prefix) = rule.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(prefix) = rule.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    path == rule
}

pub(super) fn manifest_declares_permission(
    declared_permissions: &[String],
    required: Permission,
) -> bool {
    declared_permissions
        .iter()
        .any(|value| Permission::from_str(value).is_ok_and(|declared| declared == required))
}

pub(super) fn permission_for_plugin_api_request(
    method: &Method,
    path: &str,
) -> Result<Option<Permission>> {
    let pathname = path.split('?').next().unwrap_or(path);
    let segments: Vec<&str> = pathname
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let Some(first) = segments.first().copied() else {
        return Err(AppError::ValidationError(
            "Plugin API path is invalid".to_string(),
        ));
    };

    let permission = match first {
        "plugins" => {
            if pathname == "/plugins/apps" && method == Method::GET {
                return Ok(None);
            }
            return Err(AppError::Forbidden(
                "Plugin API cannot access plugin management routes".to_string(),
            ));
        }
        "compiler" => {
            if method == Method::GET {
                Permission::CompilerRead
            } else {
                Permission::CompilerRun
            }
        }
        "tasks" => match method {
            &Method::GET => {
                if segments.len() <= 1 {
                    Permission::TaskList
                } else {
                    Permission::TaskRead
                }
            }
            &Method::POST => match segments.get(1).copied() {
                Some("run") => Permission::TaskRun,
                Some("stop") | Some("pause") | Some("resume") => Permission::TaskStop,
                Some("enable") | Some("disable") | Some("pin") | Some("unpin")
                | Some("settings") => Permission::TaskUpdate,
                _ => Permission::TaskCreate,
            },
            &Method::PATCH | &Method::PUT => Permission::TaskUpdate,
            &Method::DELETE => Permission::TaskDelete,
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin API method is not allowed".to_string(),
                ));
            }
        },
        "variables" => match method {
            &Method::GET => Permission::VarList,
            &Method::POST => match segments.get(1).copied() {
                Some("toggle") | Some("reorder") => Permission::VarUpdate,
                _ => Permission::VarCreate,
            },
            &Method::PATCH | &Method::PUT => Permission::VarUpdate,
            &Method::DELETE => Permission::VarDelete,
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin API method is not allowed".to_string(),
                ));
            }
        },
        "environments" => match method {
            &Method::GET => Permission::EnvRead,
            &Method::POST => match segments.get(1).copied() {
                Some("python") | Some("node") if segments.len() == 2 => Permission::EnvCreate,
                _ => Permission::EnvUpdate,
            },
            &Method::PUT | &Method::PATCH => Permission::EnvUpdate,
            &Method::DELETE => Permission::EnvDelete,
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin API method is not allowed".to_string(),
                ));
            }
        },
        "share" => match method {
            &Method::GET => Permission::ShareList,
            &Method::POST => Permission::ShareCreate,
            &Method::PATCH | &Method::PUT => Permission::ShareUpdate,
            &Method::DELETE => Permission::ShareDelete,
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin API method is not allowed".to_string(),
                ));
            }
        },
        "jobs" => match method {
            &Method::GET => {
                if segments.len() <= 1 {
                    Permission::JobList
                } else {
                    Permission::JobRead
                }
            }
            &Method::POST | &Method::DELETE => Permission::JobAll,
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin API method is not allowed".to_string(),
                ));
            }
        },
        "overview" if method == Method::GET => Permission::OverviewRead,
        _ => {
            return Err(AppError::Forbidden(format!(
                "Plugin API cannot access path: {pathname}"
            )));
        }
    };

    Ok(Some(permission))
}

pub(super) fn build_internal_api_url(
    path: &str,
    params: Option<&std::collections::BTreeMap<String, Value>>,
) -> Result<reqwest::Url> {
    let base = internal_api_base_url();
    let mut url = reqwest::Url::parse(&format!("{base}{path}"))
        .map_err(|e| AppError::ValidationError(format!("Plugin API path is invalid: {e}")))?;
    if let Some(params) = params {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in params {
            pairs.append_pair(key, &query_value_to_string(value));
        }
    }
    Ok(url)
}

pub(super) fn query_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

pub(super) fn internal_api_base_url() -> String {
    let server_addr = Config::global().server_addr.trim().trim_end_matches('/');
    if server_addr.starts_with("http://") || server_addr.starts_with("https://") {
        return format!("{server_addr}/api/v1");
    }

    let host_port = server_addr
        .parse::<SocketAddr>()
        .map(|addr| {
            let host = match addr.ip() {
                IpAddr::V4(ip) if ip.is_unspecified() => "127.0.0.1".to_string(),
                IpAddr::V6(ip) if ip.is_unspecified() => "[::1]".to_string(),
                IpAddr::V6(ip) => format!("[{ip}]"),
                IpAddr::V4(ip) => ip.to_string(),
            };
            format!("{host}:{}", addr.port())
        })
        .unwrap_or_else(|_| {
            server_addr
                .rsplit_once(':')
                .map(|(_, port)| format!("127.0.0.1:{port}"))
                .unwrap_or_else(|| format!("127.0.0.1:{server_addr}"))
        });

    format!("http://{host_port}/api/v1")
}
