use super::*;

pub(super) fn generic_plugin_app(plugin: PluginRecord) -> Option<PluginAppRecord> {
    let ui = plugin.manifest.ui.clone()?;
    if !ui.enabled {
        return None;
    }
    let entry_cache_key = plugin_ui_entry_cache_key(&plugin, &ui);
    Some(PluginAppRecord {
        plugin_id: plugin.manifest.id.clone(),
        name: plugin.manifest.name.clone(),
        version: plugin.manifest.version.clone(),
        description: plugin.manifest.description.clone(),
        capabilities: plugin.manifest.capabilities.clone(),
        ui: app_ui(&plugin.manifest.id, ui, &entry_cache_key),
    })
}

pub(super) fn app_ui(plugin_id: &str, ui: PluginUiManifest, entry_cache_key: &str) -> PluginAppUi {
    let entry_name = Path::new(&ui.entry)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| ui.entry.clone());
    PluginAppUi {
        mode: ui.mode,
        entry_url: format!(
            "/api/v1/plugins/{plugin_id}/ui/{entry_name}?v={}",
            url_component(entry_cache_key)
        ),
        sdk_version: ui.sdk_version,
        display: ui.display,
        routes: ui.routes,
        permissions: ui.permissions,
        api: ui.api,
    }
}

pub(super) fn plugin_ui_entry_cache_key(plugin: &PluginRecord, ui: &PluginUiManifest) -> String {
    let version = plugin
        .active_version
        .as_deref()
        .unwrap_or(&plugin.manifest.version);
    let Some(plugin_path) = plugin.path.as_deref() else {
        return version.to_string();
    };
    let entry_path = Path::new(plugin_path).join(&ui.entry);
    let Ok(metadata) = fs::metadata(entry_path) else {
        return version.to_string();
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("{version}-{modified}-{}", metadata.len())
}

pub(super) fn url_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn resolve_plugin_ui_root(plugin_id: &str) -> Result<(PathBuf, PluginUiManifest)> {
    for (_, service) in plugin_services() {
        for plugin in service.list_plugins()? {
            if plugin.manifest.id == plugin_id
                && plugin.enabled
                && matches!(&plugin.status, PluginStatus::Enabled)
            {
                return plugin_ui_root(
                    plugin.path.as_deref(),
                    plugin.manifest.ui.as_ref(),
                    plugin_id,
                );
            }
        }
    }

    Err(AppError::NotFound("Plugin app not found".to_string()))
}

pub(super) fn plugin_ui_root(
    plugin_path: Option<&str>,
    ui: Option<&PluginUiManifest>,
    plugin_id: &str,
) -> Result<(PathBuf, PluginUiManifest)> {
    let plugin_path = plugin_path.ok_or_else(|| {
        AppError::ValidationError(format!("Plugin '{plugin_id}' does not have a local path"))
    })?;
    let ui = ui
        .filter(|ui| ui.enabled)
        .cloned()
        .ok_or_else(|| AppError::NotFound("Plugin UI is not enabled".to_string()))?;
    let entry_path = Path::new(&ui.entry);
    let ui_dir = entry_path.parent().unwrap_or_else(|| Path::new(""));
    Ok((PathBuf::from(plugin_path).join(ui_dir), ui))
}

pub(super) fn sanitize_ui_asset_path(path: &str) -> Result<PathBuf> {
    let mut clean = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin UI asset path is invalid".to_string(),
                ));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(AppError::ValidationError(
            "Plugin UI asset path is empty".to_string(),
        ));
    }
    Ok(clean)
}

pub(super) fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
