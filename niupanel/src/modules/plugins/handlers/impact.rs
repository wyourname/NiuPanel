use super::*;

#[derive(Debug, Clone)]
pub(super) struct PluginImpactCandidate {
    id: String,
    name: String,
    version: String,
    capabilities: Vec<String>,
    compatibility: PluginCompatibilityManifest,
    ui: Option<PluginUiManifest>,
    theme_enabled: bool,
    runtime_permissions: Vec<PluginRuntimePermission>,
}

pub(super) fn installed_plugins() -> Result<Vec<PluginRecord>> {
    let mut plugins = unified_plugin_service().list_plugins()?;
    plugins.sort_by(|left, right| {
        left.manifest
            .name
            .cmp(&right.manifest.name)
            .then(left.manifest.id.cmp(&right.manifest.id))
    });
    Ok(plugins)
}

pub(super) fn ensure_plugin_impact_allowed(
    operation: &str,
    extension: &str,
    source_path: &Path,
    current_id: Option<&str>,
) -> Result<()> {
    let preview = preview_plugin_impact(operation, extension, source_path, current_id)?;
    if preview.install_allowed {
        return Ok(());
    }
    Err(AppError::ValidationError(format!(
        "Plugin impact preview blocked this operation: {}",
        preview.blockers.join("; ")
    )))
}

pub(super) fn preview_plugin_impact(
    operation: &str,
    extension: &str,
    source_path: &Path,
    current_id: Option<&str>,
) -> Result<PluginImpactPreview> {
    let candidate = read_plugin_impact_candidate(extension, source_path)?;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if let Some(current_id) = current_id {
        if candidate.id != current_id {
            blockers.push(format!(
                "插件包 id '{}' 与目标 id '{}' 不一致",
                candidate.id, current_id
            ));
        }
    } else if installed_plugin_candidate(extension, &candidate.id)?.is_some() {
        blockers.push(format!(
            "插件 '{}' 已安装，请使用更新操作以保留版本历史",
            candidate.id
        ));
    }

    blockers.extend(compatibility_blockers(&candidate.compatibility)?);
    warnings.extend(compatibility_warnings(&candidate.compatibility)?);
    blockers.extend(capability_blockers(&candidate.capabilities)?);
    warnings.extend(capability_warnings(extension, &candidate.capabilities));
    if candidate
        .runtime_permissions
        .contains(&PluginRuntimePermission::NetworkOutbound)
    {
        warnings.push(
            "插件后端申请 HTTP/HTTPS 出站权限，可向外部 Web 服务发送处理数据；旧内核可能无法限制目标端口"
                .to_string(),
        );
    }

    let current = if let Some(current_id) = current_id {
        installed_plugin_candidate(extension, current_id)?
    } else {
        installed_plugin_candidate(extension, &candidate.id)?
    };

    let route_conflicts = plugin_route_conflicts(&candidate, Some(&candidate.id))?;
    if !route_conflicts.is_empty() {
        blockers.push(format!("发现 {} 个插件 UI 路由冲突", route_conflicts.len()));
    }

    if candidate.ui.is_none() {
        if !candidate.theme_enabled {
            warnings.push("插件未声明原生 UI，不会作为应用入口显示".to_string());
        }
    } else if candidate
        .ui
        .as_ref()
        .is_some_and(|ui| ui.enabled && ui.routes.is_empty())
    {
        warnings.push("插件 UI 已启用但没有可用路由".to_string());
    }
    if candidate.ui.as_ref().is_some_and(|ui| ui.enabled) {
        warnings.push("原生 Vue UI 会在面板同源上下文运行，仅应安装可信签名包".to_string());
    }

    let current_permissions =
        permission_set(current.as_ref().and_then(|plugin| plugin.ui.as_ref()));
    let target_permissions = permission_set(candidate.ui.as_ref());
    let current_api_allow = api_allow_set(current.as_ref().and_then(|plugin| plugin.ui.as_ref()));
    let target_api_allow = api_allow_set(candidate.ui.as_ref());

    let routes = candidate
        .ui
        .as_ref()
        .map(|ui| {
            ui.routes
                .iter()
                .map(|route| PluginImpactRoute {
                    path: route.path.clone(),
                    title: route.title.clone(),
                    hidden: route.hidden,
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(PluginImpactPreview {
        operation: operation.to_string(),
        plugin_id: candidate.id,
        name: candidate.name,
        current_version: current.map(|plugin| plugin.version),
        target_version: candidate.version,
        ui_enabled: candidate.ui.as_ref().is_some_and(|ui| ui.enabled),
        theme_enabled: candidate.theme_enabled,
        routes,
        route_conflicts,
        permissions_added: set_difference(&target_permissions, &current_permissions),
        permissions_removed: set_difference(&current_permissions, &target_permissions),
        api_allow_added: set_difference(&target_api_allow, &current_api_allow),
        api_allow_removed: set_difference(&current_api_allow, &target_api_allow),
        warnings,
        install_allowed: blockers.is_empty(),
        blockers,
    })
}

pub(super) fn read_plugin_impact_candidate(
    extension: &str,
    source_path: &Path,
) -> Result<PluginImpactCandidate> {
    plugin_service(extension)?;
    let content = fs::read_to_string(source_path.join("plugin.json"))?;
    let manifest: PluginManifest = serde_json::from_str(&content)?;
    Ok(PluginImpactCandidate {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version,
        capabilities: manifest.capabilities,
        compatibility: manifest.compatibility,
        ui: manifest.ui,
        theme_enabled: manifest.theme.is_some_and(|theme| theme.enabled),
        runtime_permissions: manifest.runtime_permissions,
    })
}

pub(super) fn compatibility_blockers(
    compatibility: &PluginCompatibilityManifest,
) -> Result<Vec<String>> {
    let mut blockers = Vec::new();
    let panel_version = env!("CARGO_PKG_VERSION");
    if compatibility
        .panel
        .min_version
        .as_deref()
        .is_some_and(|min| version_is_newer(min, panel_version))
    {
        blockers.push(format!(
            "需要面板版本 >= {}，当前版本 {}",
            compatibility
                .panel
                .min_version
                .as_deref()
                .unwrap_or_default(),
            panel_version
        ));
    }
    if compatibility
        .panel
        .max_version
        .as_deref()
        .is_some_and(|max| version_is_newer(panel_version, max))
    {
        blockers.push(format!(
            "仅兼容面板版本 <= {}，当前版本 {}",
            compatibility
                .panel
                .max_version
                .as_deref()
                .unwrap_or_default(),
            panel_version
        ));
    }

    let installed = installed_plugin_ui_candidates()?;
    for dependency in &compatibility.dependencies {
        if dependency.optional {
            continue;
        }
        match dependency_satisfied(dependency, &installed) {
            DependencyState::Satisfied => {}
            DependencyState::Missing => blockers.push(format!("缺少依赖插件 '{}'", dependency.id)),
            DependencyState::VersionTooLow { installed_version } => blockers.push(format!(
                "依赖插件 '{}' 版本过低: 当前 v{}，需要 >= v{}",
                dependency.id,
                installed_version,
                dependency.min_version.as_deref().unwrap_or_default()
            )),
        }
    }
    Ok(blockers)
}

pub(super) fn capability_blockers(capabilities: &[String]) -> Result<Vec<String>> {
    validate_plugin_capabilities(capabilities)?;
    Ok(Vec::new())
}

pub(super) fn capability_warnings(_context: &str, capabilities: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();
    if capabilities
        .iter()
        .any(|capability| capability.starts_with("compiler."))
    {
        for required in ["compiler.versions", "compiler.encrypt"] {
            if !plugin_has_capability(capabilities, required) {
                warnings.push(format!(
                    "未声明能力 '{}'，对应 compiler 调用会跳过该插件",
                    required
                ));
            }
        }
    }
    warnings
}

pub(super) fn compatibility_warnings(
    compatibility: &PluginCompatibilityManifest,
) -> Result<Vec<String>> {
    let installed = installed_plugin_ui_candidates()?;
    let mut warnings = Vec::new();
    for dependency in &compatibility.dependencies {
        if !dependency.optional {
            continue;
        }
        match dependency_satisfied(dependency, &installed) {
            DependencyState::Satisfied => {}
            DependencyState::Missing => {
                warnings.push(format!("可选依赖插件 '{}' 未安装", dependency.id))
            }
            DependencyState::VersionTooLow { installed_version } => warnings.push(format!(
                "可选依赖插件 '{}' 版本较低: 当前 v{}，建议 >= v{}",
                dependency.id,
                installed_version,
                dependency.min_version.as_deref().unwrap_or_default()
            )),
        }
    }
    Ok(warnings)
}

pub(super) enum DependencyState {
    Satisfied,
    Missing,
    VersionTooLow { installed_version: String },
}

pub(super) fn dependency_satisfied(
    dependency: &PluginDependency,
    installed: &[PluginImpactCandidate],
) -> DependencyState {
    let Some(plugin) = installed.iter().find(|plugin| plugin.id == dependency.id) else {
        return DependencyState::Missing;
    };

    if dependency
        .min_version
        .as_deref()
        .is_some_and(|min| version_is_newer(min, &plugin.version))
    {
        return DependencyState::VersionTooLow {
            installed_version: plugin.version.clone(),
        };
    }

    DependencyState::Satisfied
}

pub(super) fn installed_plugin_candidate(
    extension: &str,
    plugin_id: &str,
) -> Result<Option<PluginImpactCandidate>> {
    Ok(plugin_service(extension)?
        .list_plugins()?
        .into_iter()
        .find(|plugin| plugin.manifest.id == plugin_id)
        .map(|plugin| PluginImpactCandidate {
            id: plugin.manifest.id,
            name: plugin.manifest.name,
            version: plugin.manifest.version,
            capabilities: plugin.manifest.capabilities,
            compatibility: plugin.manifest.compatibility,
            ui: plugin.manifest.ui,
            theme_enabled: plugin.manifest.theme.is_some_and(|theme| theme.enabled),
            runtime_permissions: plugin.manifest.runtime_permissions,
        }))
}

pub(super) fn installed_plugin_ui_candidates() -> Result<Vec<PluginImpactCandidate>> {
    let mut plugins = Vec::new();
    for (_, service) in plugin_services() {
        plugins.extend(
            service
                .list_plugins()?
                .into_iter()
                .map(|plugin| PluginImpactCandidate {
                    id: plugin.manifest.id,
                    name: plugin.manifest.name,
                    version: plugin.manifest.version,
                    capabilities: plugin.manifest.capabilities,
                    compatibility: plugin.manifest.compatibility,
                    ui: plugin.manifest.ui,
                    theme_enabled: plugin.manifest.theme.is_some_and(|theme| theme.enabled),
                    runtime_permissions: plugin.manifest.runtime_permissions,
                }),
        );
    }
    Ok(plugins)
}

pub(super) fn plugin_route_conflicts(
    candidate: &PluginImpactCandidate,
    ignore_plugin_id: Option<&str>,
) -> Result<Vec<PluginRouteConflict>> {
    let mut conflicts = Vec::new();
    let Some(ui) = candidate.ui.as_ref().filter(|ui| ui.enabled) else {
        return Ok(conflicts);
    };

    let mut seen = HashSet::new();
    for route in &ui.routes {
        if !seen.insert(route.path.clone()) {
            conflicts.push(PluginRouteConflict {
                path: route.path.clone(),
                plugin_id: candidate.id.clone(),
                name: candidate.name.clone(),
            });
        }
    }

    for installed in installed_plugin_ui_candidates()? {
        if ignore_plugin_id.is_some_and(|id| id == installed.id) {
            continue;
        }
        let Some(installed_ui) = installed.ui.as_ref().filter(|ui| ui.enabled) else {
            continue;
        };
        for route in &ui.routes {
            if installed_ui
                .routes
                .iter()
                .any(|installed_route| installed_route.path == route.path)
            {
                conflicts.push(PluginRouteConflict {
                    path: route.path.clone(),
                    plugin_id: installed.id.clone(),
                    name: installed.name.clone(),
                });
            }
        }
    }

    conflicts.sort_by(|a, b| a.path.cmp(&b.path).then(a.plugin_id.cmp(&b.plugin_id)));
    Ok(conflicts)
}

pub(super) fn permission_set(ui: Option<&PluginUiManifest>) -> BTreeSet<String> {
    ui.map(|ui| ui.permissions.iter().cloned().collect())
        .unwrap_or_default()
}

pub(super) fn api_allow_set(ui: Option<&PluginUiManifest>) -> BTreeSet<String> {
    ui.map(|ui| ui.api.allow.iter().map(api_rule_label).collect())
        .unwrap_or_default()
}

pub(super) fn api_rule_label(rule: &PluginUiApiRule) -> String {
    let methods = if rule.methods.is_empty() {
        "*".to_string()
    } else {
        rule.methods
            .iter()
            .map(|method| method.trim().to_ascii_uppercase())
            .collect::<Vec<_>>()
            .join(",")
    };
    format!("{methods} {}", rule.path)
}

pub(super) fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}
