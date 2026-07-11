use super::*;

impl PluginService {
    pub(super) fn process_spec_for(&self, plugin: &PluginRecord) -> Result<ProcessPluginSpec> {
        if !matches!(plugin.manifest.runtime, PluginRuntime::Process) {
            return Err(AppError::ValidationError(
                "Only process plugins are supported in this context".to_string(),
            ));
        }
        let plugin_path = plugin
            .path
            .as_deref()
            .ok_or_else(|| AppError::ValidationError("Plugin path is missing".to_string()))?;
        let entry = plugin.manifest.entry.clone().ok_or_else(|| {
            AppError::ValidationError("Process plugin entry is missing".to_string())
        })?;

        Ok(ProcessPluginSpec {
            plugin_id: plugin.manifest.id.clone(),
            version: plugin.manifest.version.clone(),
            extension_point: self.extension_point.clone(),
            plugin_dir: PathBuf::from(plugin_path),
            entry,
            args: plugin.manifest.args.clone(),
            env: plugin.manifest.env.clone(),
            runtime_permissions: plugin.manifest.runtime_permissions.clone(),
            timeout_sec: plugin.manifest.timeout_sec,
            worker: plugin.manifest.worker.clone(),
            capabilities: plugin.manifest.capabilities.clone(),
            tools: plugin.manifest.tools.clone(),
        })
    }

    pub(super) fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root_dir)?;
        Ok(())
    }

    pub(super) fn plugins_dir(&self) -> PathBuf {
        self.root_dir.clone()
    }

    pub(super) fn plugin_dir(&self, id: &str) -> PathBuf {
        self.plugins_dir().join(id)
    }

    pub(super) fn history_dir(&self, id: &str) -> PathBuf {
        self.plugins_dir().join(".history").join(id)
    }

    pub(super) fn next_history_dir(&self, id: &str, version: &str) -> PathBuf {
        self.history_dir(id).join(format!(
            "{}-{}",
            Utc::now().timestamp_millis(),
            sanitize_version_for_path(version)
        ))
    }

    pub(super) fn read_local_plugin(&self, path: &Path) -> Result<PluginRecord> {
        let manifest = read_plugin_manifest(path)?;
        self.validate_manifest(&manifest, path)?;
        let state = self
            .read_state(&manifest.id)
            .unwrap_or_else(|_| PluginState {
                enabled: false,
                installed_at: now_rfc3339(),
                updated_at: now_rfc3339(),
                active_version: Some(manifest.version.clone()),
                package_sha256: None,
                history: vec![],
            });

        Ok(PluginRecord {
            enabled: state.enabled,
            active_version: state
                .active_version
                .or_else(|| Some(manifest.version.clone())),
            status: if state.enabled {
                PluginStatus::Enabled
            } else {
                PluginStatus::Disabled
            },
            source: PluginSource::Local,
            path: Some(path.display().to_string()),
            manifest,
        })
    }

    pub(super) fn read_state(&self, id: &str) -> Result<PluginState> {
        let state_path = self.plugin_dir(id).join(STATE_FILE);
        let content = fs::read_to_string(state_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub(super) fn write_state(&self, id: &str, state: PluginState) -> Result<()> {
        let state_path = self.plugin_dir(id).join(STATE_FILE);
        fs::write(state_path, serde_json::to_string_pretty(&state)?)?;
        Ok(())
    }

    pub(super) fn replace_plugin_dir(
        &self,
        source_dir: &Path,
        manifest: &PluginManifest,
    ) -> Result<()> {
        let target = self.plugin_dir(&manifest.id);
        let staging = self.plugins_dir().join(format!(
            ".{}.staging.{}",
            manifest.id,
            Utc::now().timestamp_millis()
        ));
        let backup = self.plugins_dir().join(format!(
            ".{}.backup.{}",
            manifest.id,
            Utc::now().timestamp_millis()
        ));

        copy_dir(source_dir, &staging)?;
        if target.exists() {
            fs::rename(&target, &backup)?;
        }

        if let Err(err) = fs::rename(&staging, &target) {
            if backup.exists() {
                let _ = fs::rename(&backup, &target);
            }
            return Err(AppError::Io(err));
        }

        if backup.exists() {
            fs::remove_dir_all(backup)?;
        }
        Ok(())
    }

    pub(super) fn replace_plugin_dir_with_history(
        &self,
        source_dir: &Path,
        manifest: &PluginManifest,
        previous_state: &PluginState,
    ) -> Result<Option<PluginVersionRecord>> {
        let target = self.plugin_dir(&manifest.id);
        let staging = self.plugins_dir().join(format!(
            ".{}.staging.{}",
            manifest.id,
            Utc::now().timestamp_millis()
        ));

        copy_dir(source_dir, &staging)?;

        let archived = if target.exists() {
            let previous_manifest = read_plugin_manifest(&target)?;
            let archive_path = self.next_history_dir(&manifest.id, &previous_manifest.version);
            if let Some(parent) = archive_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let record = archive_record_for(
                &previous_manifest.version,
                previous_state.package_sha256.clone(),
                &archive_path,
            );
            fs::rename(&target, &archive_path)?;
            Some(record)
        } else {
            None
        };

        if let Err(err) = fs::rename(&staging, &target) {
            if let Some(record) = &archived {
                let _ = fs::rename(PathBuf::from(&record.path), &target);
            }
            return Err(AppError::Io(err));
        }

        Ok(archived)
    }

    pub(super) fn validate_manifest(
        &self,
        manifest: &PluginManifest,
        source_dir: &Path,
    ) -> Result<()> {
        validate_plugin_id(&manifest.id)?;
        if manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
            return Err(AppError::ValidationError(
                "Plugin name and version are required".to_string(),
            ));
        }
        if !matches!(
            manifest.runtime,
            PluginRuntime::Builtin | PluginRuntime::Declarative
        ) {
            let entry = manifest
                .entry
                .as_deref()
                .ok_or_else(|| AppError::ValidationError("Plugin entry is required".to_string()))?;
            let entry_path = safe_entry_path(source_dir, entry)?;
            if !entry_path.exists() {
                return Err(AppError::ValidationError(format!(
                    "Plugin entry '{}' does not exist",
                    entry
                )));
            }
        }
        if let Some(ui) = &manifest.ui {
            validate_plugin_ui_manifest(ui, source_dir)?;
        }
        if let Some(theme) = &manifest.theme {
            validate_plugin_theme_manifest(theme)?;
        }
        validate_plugin_environment(&manifest.env)?;
        validate_plugin_runtime_permissions(&manifest.runtime_permissions)?;
        validate_plugin_capabilities(&manifest.capabilities)?;
        validate_plugin_compatibility_manifest(&manifest.compatibility)?;
        Ok(())
    }

    pub(super) fn error_plugin_record(&self, path: &Path, detail: String) -> PluginRecord {
        PluginRecord {
            manifest: PluginManifest {
                schema_version: default_schema_version(),
                id: path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                name: "Invalid Plugin".to_string(),
                version: "0.0.0".to_string(),
                description: detail,
                license: None,
                runtime: PluginRuntime::Process,
                protocol: PluginProcessProtocol::SingleShot,
                entry: None,
                args: vec![],
                env: HashMap::new(),
                runtime_permissions: vec![],
                timeout_sec: None,
                worker: PluginWorkerConfig::default(),
                capabilities: vec![],
                tools: vec![],
                compatibility: PluginCompatibilityManifest::default(),
                ui: None,
                theme: None,
            },
            enabled: false,
            active_version: None,
            source: PluginSource::Local,
            status: PluginStatus::Error,
            path: Some(path.display().to_string()),
        }
    }
}
