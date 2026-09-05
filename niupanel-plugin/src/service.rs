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

    pub fn validate_package_dir(&self, source_dir: impl AsRef<Path>) -> Result<PluginManifest> {
        self.ensure_dirs()?;
        let source_dir = source_dir.as_ref();
        let manifest = read_plugin_manifest(source_dir)?;
        self.validate_manifest(&manifest, source_dir)?;
        if matches!(manifest.runtime, PluginRuntime::Builtin) {
            return Err(AppError::ValidationError(
                "Builtin plugins cannot be installed from packages".to_string(),
            ));
        }
        Ok(manifest)
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
        let source_dir = source_dir.as_ref();
        let manifest = self.validate_package_dir(source_dir)?;

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

    pub fn install_prepared_dir(
        &self,
        source_dir: impl AsRef<Path>,
        enable: bool,
        package_sha256: String,
    ) -> Result<PluginRecord> {
        let source_dir = source_dir.as_ref();
        let manifest = self.validate_package_dir(source_dir)?;

        self.replace_plugin_dir_prepared(source_dir, &manifest)?;
        self.write_state(
            &manifest.id,
            PluginState {
                enabled: enable,
                installed_at: now_rfc3339(),
                updated_at: now_rfc3339(),
                active_version: Some(manifest.version.clone()),
                package_sha256: Some(package_sha256),
                history: vec![],
            },
        )?;
        self.get_plugin(&manifest.id)
    }

    pub fn update_from_dir(&self, id: &str, source_dir: impl AsRef<Path>) -> Result<PluginRecord> {
        validate_plugin_id(id)?;
        let source_dir = source_dir.as_ref();
        let manifest = self.validate_package_dir(source_dir)?;
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

    pub fn update_prepared_dir(
        &self,
        id: &str,
        source_dir: impl AsRef<Path>,
        package_sha256: String,
    ) -> Result<PluginRecord> {
        validate_plugin_id(id)?;
        let source_dir = source_dir.as_ref();
        let manifest = self.validate_package_dir(source_dir)?;
        if manifest.id != id {
            return Err(AppError::ValidationError(format!(
                "Plugin package id '{}' does not match target id '{}'",
                manifest.id, id
            )));
        }

        let mut previous_state = self.read_state(id)?;
        let archived =
            self.replace_plugin_dir_with_history_prepared(source_dir, &manifest, &previous_state)?;
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
                package_sha256: Some(package_sha256),
                history: previous_state.history,
            },
        )?;
        self.get_plugin(id)
    }

    pub async fn update_prepared_dir_async(
        &self,
        id: &str,
        source_dir: impl AsRef<Path>,
        package_sha256: String,
    ) -> Result<PluginRecord> {
        let record = self.update_prepared_dir(id, source_dir, package_sha256)?;
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

    pub fn list_action_plugins(
        &self,
        caller: PluginActionCaller,
    ) -> Result<Vec<PluginActionPlugin>> {
        let mut plugins = self
            .list_plugins()?
            .into_iter()
            .filter(|plugin| {
                plugin.enabled
                    && matches!(plugin.status, PluginStatus::Enabled)
                    && plugin
                        .manifest
                        .actions
                        .iter()
                        .any(|action| action.callers.contains(&caller))
            })
            .map(|plugin| PluginActionPlugin {
                id: plugin.manifest.id,
                name: plugin.manifest.name,
                version: plugin.manifest.version,
                description: plugin.manifest.description,
            })
            .collect::<Vec<_>>();
        plugins.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(plugins)
    }

    pub fn list_plugin_actions(
        &self,
        id: &str,
        caller: PluginActionCaller,
    ) -> Result<Vec<PluginActionManifest>> {
        let plugin = self.enabled_plugin(id)?;
        Ok(plugin
            .manifest
            .actions
            .into_iter()
            .filter(|action| action.callers.contains(&caller))
            .collect())
    }

    pub async fn invoke_action(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
    ) -> Result<PluginInvokeResponse> {
        let plugin = self.enabled_plugin(id)?;
        let action = plugin
            .manifest
            .actions
            .iter()
            .find(|action| action.name == request.action)
            .ok_or_else(|| AppError::NotFound("Plugin action not found".to_string()))?;
        if !action.callers.contains(&caller) {
            return Err(AppError::Forbidden(format!(
                "Plugin action '{}' is not available to caller '{}'",
                action.name,
                action_caller_name(caller)
            )));
        }
        validate_plugin_action_input(action, &request.input)?;
        let invocation = PluginInvokeRequest {
            action: request.action,
            input: request.input,
            timeout_sec: Some(action.timeout_sec),
        };
        self.invoke_plugin_record(plugin, invocation).await
    }

    pub async fn invoke_action_with_tools<F>(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
        tools: Vec<serde_json::Value>,
        tool_handler: F,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        self.invoke_action_with_tools_context(id, caller, request, None, tools, tool_handler)
            .await
    }

    pub async fn invoke_action_with_tools_context<F>(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
        invocation_context: Option<PluginInvocationContext>,
        tools: Vec<serde_json::Value>,
        tool_handler: F,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let plugin = self.enabled_plugin(id)?;
        let action = plugin
            .manifest
            .actions
            .iter()
            .find(|action| action.name == request.action)
            .ok_or_else(|| AppError::NotFound("Plugin action not found".to_string()))?;
        if !action.callers.contains(&caller) {
            return Err(AppError::Forbidden(format!(
                "Plugin action '{}' is not available to caller '{}'",
                action.name,
                action_caller_name(caller)
            )));
        }
        validate_plugin_action_input(action, &request.input)?;
        let invocation = PluginInvokeRequest {
            action: request.action,
            input: request.input,
            timeout_sec: Some(action.timeout_sec),
        };
        let mut spec = self.process_spec_for(&plugin)?;
        spec.tools = tools;
        spec.invocation_context = invocation_context;
        match plugin.manifest.protocol {
            PluginProcessProtocol::SingleShot => {
                self.process_runtime
                    .invoke_single_shot(spec, invocation)
                    .await
            }
            PluginProcessProtocol::JsonLines => {
                self.process_runtime
                    .invoke_json_lines(spec, invocation, tool_handler)
                    .await
            }
        }
    }

    pub async fn invoke_action_with_tools_cancellable<F>(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
        tools: Vec<serde_json::Value>,
        tool_handler: F,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        self.invoke_action_with_tools_context_cancellable(
            id,
            caller,
            request,
            None,
            tools,
            tool_handler,
            cancellation,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn invoke_action_with_tools_context_cancellable<F>(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
        invocation_context: Option<PluginInvocationContext>,
        tools: Vec<serde_json::Value>,
        tool_handler: F,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
    {
        let plugin = self.enabled_plugin(id)?;
        let action = plugin
            .manifest
            .actions
            .iter()
            .find(|action| action.name == request.action)
            .ok_or_else(|| AppError::NotFound("Plugin action not found".to_string()))?;
        if !action.callers.contains(&caller) {
            return Err(AppError::Forbidden(format!(
                "Plugin action '{}' is not available to caller '{}'",
                action.name,
                action_caller_name(caller)
            )));
        }
        validate_plugin_action_input(action, &request.input)?;
        let invocation = PluginInvokeRequest {
            action: request.action,
            input: request.input,
            timeout_sec: Some(action.timeout_sec),
        };
        let mut spec = self.process_spec_for(&plugin)?;
        spec.tools = tools;
        spec.invocation_context = invocation_context;
        match plugin.manifest.protocol {
            PluginProcessProtocol::SingleShot => {
                self.process_runtime
                    .invoke_single_shot_cancellable(spec, invocation, cancellation)
                    .await
            }
            PluginProcessProtocol::JsonLines => {
                self.process_runtime
                    .invoke_json_lines_cancellable(spec, invocation, tool_handler, cancellation)
                    .await
            }
        }
    }

    pub async fn invoke_action_stream_with_tools_cancellable<F, S>(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
        tools: Vec<serde_json::Value>,
        handlers: PluginStreamHandlers<F, S>,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
        S: Fn(ProcessPluginStreamEvent) -> PluginStreamFuture + Send + Sync,
    {
        self.invoke_action_stream_with_tools_context_cancellable(
            id,
            caller,
            request,
            None,
            tools,
            handlers,
            cancellation,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn invoke_action_stream_with_tools_context_cancellable<F, S>(
        &self,
        id: &str,
        caller: PluginActionCaller,
        request: PluginActionInvokeRequest,
        invocation_context: Option<PluginInvocationContext>,
        tools: Vec<serde_json::Value>,
        handlers: PluginStreamHandlers<F, S>,
        cancellation: CancellationToken,
    ) -> Result<PluginInvokeResponse>
    where
        F: Fn(ProcessPluginToolCall) -> PluginToolFuture + Send + Sync,
        S: Fn(ProcessPluginStreamEvent) -> PluginStreamFuture + Send + Sync,
    {
        let plugin = self.enabled_plugin(id)?;
        let action = plugin
            .manifest
            .actions
            .iter()
            .find(|action| action.name == request.action)
            .ok_or_else(|| AppError::NotFound("Plugin action not found".to_string()))?;
        if !action.callers.contains(&caller) {
            return Err(AppError::Forbidden(format!(
                "Plugin action '{}' is not available to caller '{}'",
                action.name,
                action_caller_name(caller)
            )));
        }
        if !action.streaming {
            return Err(AppError::ValidationError(format!(
                "Plugin action '{}' does not support streaming",
                action.name
            )));
        }
        if !matches!(plugin.manifest.protocol, PluginProcessProtocol::JsonLines) {
            return Err(AppError::ValidationError(
                "Streaming plugin actions require the json_lines protocol".to_string(),
            ));
        }
        validate_plugin_action_input(action, &request.input)?;
        let invocation = PluginInvokeRequest {
            action: request.action,
            input: request.input,
            timeout_sec: Some(action.timeout_sec),
        };
        let mut spec = self.process_spec_for(&plugin)?;
        spec.tools = tools;
        spec.invocation_context = invocation_context;
        let PluginStreamHandlers { tool, stream } = handlers;
        self.process_runtime
            .invoke_json_lines_streaming(spec, invocation, tool, stream, cancellation)
            .await
    }

    fn enabled_plugin(&self, id: &str) -> Result<PluginRecord> {
        let plugin = self.get_plugin(id)?;
        if !plugin.enabled || !matches!(plugin.status, PluginStatus::Enabled) {
            return Err(AppError::ValidationError(
                "Plugin is not enabled".to_string(),
            ));
        }
        Ok(plugin)
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

fn action_caller_name(caller: PluginActionCaller) -> &'static str {
    match caller {
        PluginActionCaller::Ui => "ui",
        PluginActionCaller::Task => "task",
        PluginActionCaller::ApiKey => "api_key",
        PluginActionCaller::Telegram => "telegram",
    }
}
