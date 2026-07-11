use super::*;

pub fn validate_plugin_theme_manifest(theme: &PluginThemeManifest) -> Result<()> {
    if !theme.enabled {
        return Ok(());
    }

    validate_plugin_theme_palette(&theme.light, "theme.light")?;
    validate_plugin_theme_palette(&theme.dark, "theme.dark")?;
    if theme.light.is_empty() && theme.dark.is_empty() {
        return Err(AppError::ValidationError(
            "Enabled plugin theme must define at least one color token".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_plugin_theme_palette(
    palette: &PluginThemePalette,
    field: &str,
) -> Result<()> {
    for (name, value) in palette.values() {
        if let Some(value) = value {
            validate_theme_color(value).map_err(|_| {
                AppError::ValidationError(format!(
                    "Plugin {field}.{name} must be a 6 or 8 digit hex color"
                ))
            })?;
        }
    }
    Ok(())
}

pub(super) fn validate_theme_color(value: &str) -> Result<()> {
    let Some(hex) = value.trim().strip_prefix('#') else {
        return Err(AppError::ValidationError("Invalid theme color".to_string()));
    };
    if matches!(hex.len(), 6 | 8) && hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AppError::ValidationError("Invalid theme color".to_string()))
    }
}

impl PluginThemePalette {
    fn values(&self) -> [(&'static str, &Option<String>); 10] {
        [
            ("primary", &self.primary),
            ("bg_base", &self.bg_base),
            ("bg_card", &self.bg_card),
            ("bg_subtle", &self.bg_subtle),
            ("bg_soft", &self.bg_soft),
            ("text_default", &self.text_default),
            ("text_secondary", &self.text_secondary),
            ("text_muted", &self.text_muted),
            ("border_base", &self.border_base),
            ("border_light", &self.border_light),
        ]
    }

    fn is_empty(&self) -> bool {
        self.values().iter().all(|(_, value)| value.is_none())
    }
}

pub fn validate_plugin_ui_manifest(ui: &PluginUiManifest, source_dir: &Path) -> Result<()> {
    if !ui.enabled {
        return Ok(());
    }
    if ui.entry.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Plugin UI entry is required when ui.enabled is true".to_string(),
        ));
    }
    let entry_path = safe_entry_path(source_dir, &ui.entry)?;
    if !entry_path.is_file() {
        return Err(AppError::ValidationError(format!(
            "Plugin UI entry '{}' does not exist",
            ui.entry
        )));
    }
    if ui.routes.is_empty() {
        return Err(AppError::ValidationError(
            "Plugin UI routes are required when ui.enabled is true".to_string(),
        ));
    }
    for route in &ui.routes {
        if !route.path.starts_with("/plugins/") {
            return Err(AppError::ValidationError(format!(
                "Plugin UI route '{}' must start with /plugins/",
                route.path
            )));
        }
        if route.title.trim().is_empty() {
            return Err(AppError::ValidationError(
                "Plugin UI route title is required".to_string(),
            ));
        }
        if let Some(id) = &route.id {
            validate_ui_identifier(id, "Plugin UI route id")?;
        }
        if let Some(menu) = &route.menu {
            validate_ui_identifier(menu, "Plugin UI route menu")?;
        }
    }
    if let Some(category) = &ui.display.category {
        validate_ui_identifier(category, "Plugin UI display category")?;
    }
    validate_plugin_ui_permissions(&ui.permissions)?;
    validate_plugin_ui_api_manifest(&ui.api)?;
    Ok(())
}

pub(super) fn validate_plugin_ui_permissions(permissions: &[String]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for permission in permissions {
        let normalized = permission.trim();
        Permission::from_str(normalized).map_err(|_| {
            AppError::ValidationError(format!(
                "Plugin UI permission '{permission}' is not recognized"
            ))
        })?;
        if normalized.contains('*') {
            return Err(AppError::ValidationError(format!(
                "Plugin UI permission '{permission}' must be an exact permission"
            )));
        }
        if !seen.insert(normalized) {
            return Err(AppError::ValidationError(format!(
                "Plugin UI permission '{permission}' is duplicated"
            )));
        }
    }
    Ok(())
}

pub fn validate_plugin_compatibility_manifest(
    compatibility: &PluginCompatibilityManifest,
) -> Result<()> {
    if let Some(version) = &compatibility.panel.min_version {
        validate_version_text(version, "Plugin compatibility panel.min_version")?;
    }
    if let Some(version) = &compatibility.panel.max_version {
        validate_version_text(version, "Plugin compatibility panel.max_version")?;
    }
    for dependency in &compatibility.dependencies {
        validate_plugin_id(&dependency.id)?;
        if let Some(version) = &dependency.min_version {
            validate_version_text(version, "Plugin dependency min_version")?;
        }
    }
    Ok(())
}

pub fn validate_plugin_capabilities(capabilities: &[String]) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for capability in capabilities {
        validate_plugin_capability(capability)?;
        if !seen.insert(capability.trim().to_string()) {
            return Err(AppError::ValidationError(format!(
                "Plugin capability '{}' is duplicated",
                capability
            )));
        }
    }
    Ok(())
}

pub fn validate_plugin_capability(capability: &str) -> Result<()> {
    let value = capability.trim();
    let parts = value.split('.').collect::<Vec<_>>();
    let valid = !value.is_empty()
        && value.len() <= 96
        && parts.len() >= 2
        && parts.iter().enumerate().all(|(index, part)| {
            if part.is_empty() {
                return false;
            }
            if *part == "*" {
                return index == parts.len() - 1;
            }
            part.chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
        });
    if valid {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "Plugin capability '{}' is invalid",
            capability
        )))
    }
}

pub fn plugin_has_capability(capabilities: &[String], required: &str) -> bool {
    capabilities.iter().any(|capability| {
        let capability = capability.trim();
        capability == required
            || capability
                .strip_suffix(".*")
                .is_some_and(|prefix| required.starts_with(&format!("{prefix}.")))
    })
}

pub(super) fn validate_version_text(version: &str, field: &str) -> Result<()> {
    let value = version.trim();
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '+'));
    if valid {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!("{field} is invalid")))
    }
}

pub(super) fn validate_plugin_ui_api_manifest(api: &PluginUiApiManifest) -> Result<()> {
    for rule in &api.allow {
        let path = rule.path.trim();
        if path.is_empty() {
            return Err(AppError::ValidationError(
                "Plugin UI api.allow path is required".to_string(),
            ));
        }
        if !path.starts_with('/') {
            return Err(AppError::ValidationError(format!(
                "Plugin UI api.allow path '{}' must start with /",
                rule.path
            )));
        }
        if matches!(path, "*" | "/**") {
            return Err(AppError::ValidationError(
                "Plugin UI api.allow cannot grant all panel APIs".to_string(),
            ));
        }
        if path.starts_with("/plugins/") && (path.contains("/api") || path.contains("/ui")) {
            return Err(AppError::ValidationError(
                "Plugin UI api.allow cannot target plugin API proxy or UI asset routes".to_string(),
            ));
        }
        if rule.methods.is_empty() {
            return Err(AppError::ValidationError(
                "Plugin UI api.allow methods must be explicit".to_string(),
            ));
        }
        for method in &rule.methods {
            let method = method.trim().to_ascii_uppercase();
            if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
                return Err(AppError::ValidationError(format!(
                    "Plugin UI api.allow method '{}' is not allowed",
                    method
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_ui_identifier(value: &str, field: &str) -> Result<()> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_');
    if !valid {
        return Err(AppError::ValidationError(format!(
            "{field} must use lowercase letters, digits, '-' or '_'"
        )));
    }
    Ok(())
}

pub(super) fn default_schema_version() -> u32 {
    1
}

pub(super) fn default_invoke_action() -> String {
    "run".to_string()
}

pub(super) fn default_worker_min() -> usize {
    1
}

pub(super) fn default_worker_max() -> usize {
    2
}

pub(super) fn default_worker_idle_timeout_sec() -> u64 {
    300
}

pub(super) fn default_ui_sidebar() -> bool {
    true
}

pub(super) fn default_ui_workspace() -> bool {
    true
}

pub(super) fn default_ui_mobile() -> bool {
    true
}
