use super::*;

#[derive(Clone)]
pub struct PluginService {
    pub(super) root_dir: PathBuf,
    pub(super) extension_point: String,
    pub(super) process_runtime: PluginProcessRuntime,
}

impl PluginService {
    pub fn new(root_dir: impl Into<PathBuf>, extension_point: impl Into<String>) -> Self {
        Self {
            root_dir: root_dir.into(),
            extension_point: extension_point.into(),
            process_runtime: PluginProcessRuntime::new(),
        }
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginRecord>> {
        self.ensure_dirs()?;
        let mut plugins = Vec::new();
        let plugins_dir = self.plugins_dir();

        for entry in fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }

            match self.read_local_plugin(&path) {
                Ok(record) => plugins.push(record),
                Err(err) => plugins.push(self.error_plugin_record(&path, err.to_string())),
            }
        }

        plugins.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
        Ok(plugins)
    }

    pub fn install_from_dir(
        &self,
        source_dir: impl AsRef<Path>,
        enable: bool,
    ) -> Result<PluginRecord> {
        self.ensure_dirs()?;
        let source_dir = source_dir.as_ref();
        let manifest = read_plugin_manifest(source_dir)?;
        self.validate_manifest(&manifest, source_dir)?;
        if matches!(manifest.runtime, PluginRuntime::Builtin) {
            return Err(AppError::ValidationError(
                "Builtin plugins cannot be installed from packages".to_string(),
            ));
        }

        self.replace_plugin_dir(source_dir, &manifest)?;
        self.write_state(
            &manifest.id,
            PluginState {
                enabled: enable,
                installed_at: now_rfc3339(),
                updated_at: now_rfc3339(),
                active_version: Some(manifest.version.clone()),
                package_sha256: None,
                history: vec![],
            },
        )?;
        self.get_plugin(&manifest.id)
    }

    pub fn update_from_dir(&self, id: &str, source_dir: impl AsRef<Path>) -> Result<PluginRecord> {
        self.ensure_dirs()?;
        validate_plugin_id(id)?;
        let source_dir = source_dir.as_ref();
        let manifest = read_plugin_manifest(source_dir)?;
        self.validate_manifest(&manifest, source_dir)?;
        if manifest.id != id {
            return Err(AppError::ValidationError(format!(
                "Plugin package id '{}' does not match target id '{}'",
                manifest.id, id
            )));
        }

        let mut previous_state = self.read_state(id)?;
        let archived =
            self.replace_plugin_dir_with_history(source_dir, &manifest, &previous_state)?;
        if let Some(archived) = archived {
            previous_state.history.push(archived);
        }
        self.write_state(
            id,
            PluginState {
                enabled: previous_state.enabled,
                installed_at: previous_state.installed_at,
                updated_at: now_rfc3339(),
                active_version: Some(manifest.version.clone()),
                package_sha256: previous_state.package_sha256,
                history: previous_state.history,
            },
        )?;
        self.get_plugin(id)
    }

    pub async fn update_from_dir_async(
        &self,
        id: &str,
        source_dir: impl AsRef<Path>,
    ) -> Result<PluginRecord> {
        let record = self.update_from_dir(id, source_dir)?;
        self.process_runtime.stop_plugin(id).await;
        Ok(record)
    }

    pub fn list_versions(&self, id: &str) -> Result<Vec<PluginVersionRecord>> {
        validate_plugin_id(id)?;
        let state = self.read_state(id)?;
        Ok(state
            .history
            .into_iter()
            .filter(|version| Path::new(&version.path).is_dir())
            .collect())
    }

    pub fn rollback(&self, id: &str, version_id: &str) -> Result<PluginRecord> {
        self.ensure_dirs()?;
        validate_plugin_id(id)?;
        validate_version_id(version_id)?;

        let mut state = self.read_state(id)?;
        let version_index = state
            .history
            .iter()
            .position(|version| version.id == version_id)
            .ok_or_else(|| AppError::NotFound("Plugin version not found".to_string()))?;
        let restore_version = state.history.remove(version_index);
        let restore_path = PathBuf::from(&restore_version.path);
        if !restore_path.is_dir() {
            return Err(AppError::NotFound(
                "Plugin version directory not found".to_string(),
            ));
        }

        let restored_manifest = read_plugin_manifest(&restore_path)?;
        self.validate_manifest(&restored_manifest, &restore_path)?;
        if restored_manifest.id != id {
            return Err(AppError::ValidationError(
                "Plugin version id does not match target plugin".to_string(),
            ));
        }

        let target = self.plugin_dir(id);
        let current_manifest = read_plugin_manifest(&target)?;
        let current_version = archive_record_for(
            &current_manifest.version,
            state.package_sha256.clone(),
            &self.next_history_dir(id, &current_manifest.version),
        );
        let current_archive = PathBuf::from(&current_version.path);
        if let Some(parent) = current_archive.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&target, &current_archive)?;

        if let Err(err) = fs::rename(&restore_path, &target) {
            let _ = fs::rename(&current_archive, &target);
            state.history.push(restore_version);
            return Err(AppError::Io(err));
        }

        state.history.push(current_version);
        self.write_state(
            id,
            PluginState {
                enabled: state.enabled,
                installed_at: state.installed_at,
                updated_at: now_rfc3339(),
                active_version: Some(restored_manifest.version.clone()),
                package_sha256: restore_version.package_sha256,
                history: state.history,
            },
        )?;
        self.get_plugin(id)
    }

    pub async fn rollback_async(&self, id: &str, version_id: &str) -> Result<PluginRecord> {
        let record = self.rollback(id, version_id)?;
        self.process_runtime.stop_plugin(id).await;
        Ok(record)
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<PluginRecord> {
        validate_plugin_id(id)?;
        let mut state = self.read_state(id)?;
        state.enabled = enabled;
        state.updated_at = now_rfc3339();
        self.write_state(id, state)?;
        self.get_plugin(id)
    }

    pub async fn set_enabled_async(&self, id: &str, enabled: bool) -> Result<PluginRecord> {
        let record = self.set_enabled(id, enabled)?;
        if !enabled {
            self.process_runtime.stop_plugin(id).await;
        }
        Ok(record)
    }

    pub fn uninstall(&self, id: &str) -> Result<()> {
        validate_plugin_id(id)?;
        let target = self.plugin_dir(id);
        if !target.exists() {
            return Err(AppError::NotFound("Plugin not found".to_string()));
        }
        fs::remove_dir_all(target)?;
        Ok(())
    }

    pub async fn uninstall_async(&self, id: &str) -> Result<()> {
        self.uninstall(id)?;
        self.process_runtime.stop_plugin(id).await;
        Ok(())
    }

    pub fn get_plugin(&self, id: &str) -> Result<PluginRecord> {
        validate_plugin_id(id)?;
        let path = self.plugin_dir(id);
        if !path.exists() {
            return Err(AppError::NotFound("Plugin not found".to_string()));
        }
        self.read_local_plugin(&path)
    }

    pub async fn invoke_first_enabled(
        &self,
        request: PluginInvokeRequest,
    ) -> Result<Option<PluginInvokeResponse>> {
        self.invoke_first_enabled_matching(request, |_| true).await
    }

    pub async fn invoke_first_enabled_with_capability(
        &self,
        required_capability: &str,
        request: PluginInvokeRequest,
    ) -> Result<Option<PluginInvokeResponse>> {
        validate_plugin_capability(required_capability)?;
        self.invoke_first_enabled_matching(request, |plugin| {
            plugin_has_capability(&plugin.manifest.capabilities, required_capability)
        })
        .await
    }

    async fn invoke_first_enabled_matching<F>(
        &self,
        request: PluginInvokeRequest,
        predicate: F,
    ) -> Result<Option<PluginInvokeResponse>>
    where
        F: Fn(&PluginRecord) -> bool,
    {
        let Some(plugin) = self.list_plugins()?.into_iter().find(|plugin| {
            plugin.enabled && matches!(plugin.status, PluginStatus::Enabled) && predicate(plugin)
        }) else {
            return Ok(None);
        };

        let response = self.invoke_plugin_record(plugin, request).await?;
        Ok(Some(response))
    }

    pub async fn invoke_plugin(
        &self,
        id: &str,
        request: PluginInvokeRequest,
    ) -> Result<PluginInvokeResponse> {
        let plugin = self.get_plugin(id)?;
        if !plugin.enabled {
            return Err(AppError::ValidationError("Plugin is disabled".to_string()));
        }
        self.invoke_plugin_record(plugin, request).await
    }

    pub async fn invoke_plugin_with_tools<F>(
        &self,
        id: &str,
        request: PluginInvokeRequest,
        extra_tools: Vec<serde_json::Value>,
        tool_handler: F,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let plugin = self.get_plugin(id)?;
        if !plugin.enabled {
            return Err(AppError::ValidationError("Plugin is disabled".to_string()));
        }
        let mut spec = self.process_spec_for(&plugin)?;
        spec.tools.extend(extra_tools);
        match plugin.manifest.protocol {
            PluginProcessProtocol::SingleShot => {
                self.process_runtime.invoke_single_shot(spec, request).await
            }
            PluginProcessProtocol::JsonLines => {
                self.process_runtime
                    .invoke_json_lines(spec, request, tool_handler)
                    .await
            }
        }
    }

    async fn invoke_plugin_record(
        &self,
        plugin: PluginRecord,
        request: PluginInvokeRequest,
    ) -> Result<PluginInvokeResponse> {
        let spec = self.process_spec_for(&plugin)?;
        match plugin.manifest.protocol {
            PluginProcessProtocol::SingleShot => {
                self.process_runtime.invoke_single_shot(spec, request).await
            }
            PluginProcessProtocol::JsonLines => {
                self.process_runtime
                    .invoke_json_lines(spec, request, |_| {
                        Box::pin(async {
                            Err(AppError::ValidationError(
                                "Plugin tool calls are not available in this context".to_string(),
                            ))
                        })
                    })
                    .await
            }
        }
    }
}
