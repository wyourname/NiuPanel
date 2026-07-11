use super::*;

#[derive(Debug, Clone)]
pub(super) struct PluginHealthSubject {
    extension: String,
    plugin_id: String,
    name: String,
    version: String,
    enabled: bool,
    status: String,
    description: String,
    path: Option<String>,
    entry: Option<String>,
    capabilities: Vec<String>,
    compatibility: PluginCompatibilityManifest,
    ui: Option<PluginUiManifest>,
}

pub(super) fn plugin_health_reports() -> Result<Vec<PluginHealthReport>> {
    let subjects = installed_plugin_health_subjects()?;
    let mut reports = subjects
        .iter()
        .map(|subject| plugin_health_report(subject, &subjects))
        .collect::<Vec<_>>();
    reports.sort_by(|a, b| a.plugin_id.cmp(&b.plugin_id));
    Ok(reports)
}

pub(super) fn ensure_plugin_enable_allowed(plugin_id: &str) -> Result<()> {
    let reports = plugin_health_reports()?;
    let Some(report) = reports
        .into_iter()
        .find(|report| report.plugin_id == plugin_id)
    else {
        return Err(AppError::NotFound("Plugin not found".to_string()));
    };
    let errors = report
        .checks
        .into_iter()
        .filter(|check| check.severity == "error")
        .map(|check| check.message)
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "Plugin health check failed: {}",
            errors.join("; ")
        )))
    }
}

pub(super) fn installed_plugin_health_subjects() -> Result<Vec<PluginHealthSubject>> {
    let mut subjects = Vec::new();
    for (_, service) in plugin_services() {
        subjects.extend(service.list_plugins()?.into_iter().map(|plugin| {
            PluginHealthSubject {
                extension: PLUGIN_CONTEXT.to_string(),
                plugin_id: plugin.manifest.id,
                name: plugin.manifest.name,
                version: plugin.manifest.version,
                enabled: plugin.enabled,
                status: match plugin.status {
                    PluginStatus::Enabled => "enabled",
                    PluginStatus::Disabled => "disabled",
                    PluginStatus::Error => "error",
                }
                .to_string(),
                description: plugin.manifest.description,
                path: plugin.path,
                entry: plugin.manifest.entry,
                capabilities: plugin.manifest.capabilities,
                compatibility: plugin.manifest.compatibility,
                ui: plugin.manifest.ui,
            }
        }));
    }
    Ok(subjects)
}

pub(super) fn plugin_health_report(
    subject: &PluginHealthSubject,
    all_subjects: &[PluginHealthSubject],
) -> PluginHealthReport {
    let mut checks = Vec::new();

    if subject.status == "error" {
        checks.push(health_check(
            "manifest",
            "error",
            format!("插件清单无效: {}", subject.description),
        ));
    } else {
        checks.push(health_check("manifest", "ok", "插件清单可读取"));
    }

    let plugin_root = subject.path.as_ref().map(PathBuf::from);
    if let Some(root) = &plugin_root {
        if root.is_dir() {
            checks.push(health_check("package_path", "ok", "插件目录存在"));
        } else {
            checks.push(health_check("package_path", "error", "插件目录不存在"));
        }
    } else {
        checks.push(health_check("package_path", "warning", "插件没有本地目录"));
    }

    if let (Some(root), Some(entry)) = (&plugin_root, subject.entry.as_deref()) {
        if root.join(entry).exists() {
            checks.push(health_check("backend_entry", "ok", "后端入口存在"));
        } else {
            checks.push(health_check(
                "backend_entry",
                "error",
                format!("后端入口不存在: {entry}"),
            ));
        }
    }

    if let Some(ui) = subject.ui.as_ref().filter(|ui| ui.enabled) {
        if let Some(root) = &plugin_root {
            if root.join(&ui.entry).is_file() {
                checks.push(health_check("ui_entry", "ok", "UI 入口存在"));
            } else {
                checks.push(health_check(
                    "ui_entry",
                    "error",
                    format!("UI 入口不存在: {}", ui.entry),
                ));
            }
        }
        if ui.routes.iter().filter(|route| !route.hidden).count() == 0 {
            checks.push(health_check("ui_routes", "warning", "UI 没有可见路由"));
        }
        let conflicts = enabled_route_conflicts(subject, all_subjects);
        if conflicts.is_empty() {
            checks.push(health_check("ui_routes", "ok", "UI 路由未冲突"));
        } else {
            checks.push(health_check(
                "ui_routes",
                "error",
                format!("UI 路由冲突: {}", conflicts.join(", ")),
            ));
        }
    }

    for message in compatibility_blockers(&subject.compatibility).unwrap_or_default() {
        checks.push(health_check("compatibility", "error", message));
    }
    for message in compatibility_warnings(&subject.compatibility).unwrap_or_default() {
        checks.push(health_check("compatibility", "warning", message));
    }
    for message in capability_blockers(&subject.capabilities).unwrap_or_default() {
        checks.push(health_check("capabilities", "error", message));
    }
    for message in capability_warnings(&subject.extension, &subject.capabilities) {
        checks.push(health_check("capabilities", "warning", message));
    }

    let healthy = checks.iter().all(|check| check.severity != "error");
    let summary = if healthy {
        if checks.iter().any(|check| check.severity == "warning") {
            "存在警告".to_string()
        } else {
            "健康".to_string()
        }
    } else {
        "异常".to_string()
    };

    PluginHealthReport {
        plugin_id: subject.plugin_id.clone(),
        name: subject.name.clone(),
        version: subject.version.clone(),
        enabled: subject.enabled,
        healthy,
        summary,
        checks,
    }
}

pub(super) fn health_check(
    code: impl Into<String>,
    severity: impl Into<String>,
    message: impl Into<String>,
) -> PluginHealthCheck {
    PluginHealthCheck {
        code: code.into(),
        severity: severity.into(),
        message: message.into(),
    }
}

pub(super) fn enabled_route_conflicts(
    subject: &PluginHealthSubject,
    all_subjects: &[PluginHealthSubject],
) -> Vec<String> {
    if !subject.enabled {
        return Vec::new();
    }
    let Some(ui) = subject.ui.as_ref().filter(|ui| ui.enabled) else {
        return Vec::new();
    };
    let mut conflicts = Vec::new();
    for route in &ui.routes {
        for other in all_subjects {
            if other.plugin_id == subject.plugin_id || !other.enabled {
                continue;
            }
            let Some(other_ui) = other.ui.as_ref().filter(|ui| ui.enabled) else {
                continue;
            };
            if other_ui
                .routes
                .iter()
                .any(|other_route| other_route.path == route.path)
            {
                conflicts.push(format!("{} -> {}", route.path, other.plugin_id));
            }
        }
    }
    conflicts.sort();
    conflicts.dedup();
    conflicts
}
