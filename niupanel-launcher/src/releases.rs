use super::*;
const BOOTSTRAP_INSTALLED_AT: &str = "1970-01-01T00:00:00Z";

pub(crate) async fn ensure_panel_runtime(config: &LauncherConfig) -> Result<PanelRuntimeState> {
    if runtime_database_path(&config.system_root).exists() {
        return read_panel_runtime_state(&config.system_root)
            .await
            .map_err(|error| anyhow!(error));
    }
    let bootstrap = install_bootstrap_release(config)?;
    initialize_runtime(&config.system_root, &bootstrap)
        .await
        .map_err(|error| anyhow!(error))
}

/// Reconciles a newer Panel bundled by an installer or Docker image with the
/// persistent runtime. A bundled release is an update candidate, never an
/// implicit downgrade, and is activated by the same probation/rollback path as
/// an online update.
pub(crate) async fn reconcile_bundled_release(
    config: &LauncherConfig,
    state: &mut PanelRuntimeState,
) -> Result<()> {
    if state.pending.is_some() {
        return Ok(());
    }
    let bundled_version = bundled_panel_version(config)?;
    let bundled = semver::Version::parse(&bundled_version)
        .with_context(|| format!("Invalid bundled Panel version {bundled_version}"))?;
    let active = semver::Version::parse(&state.active.version)
        .with_context(|| format!("Invalid active Panel version {}", state.active.version))?;
    if bundled <= active {
        return Ok(());
    }

    let installed = list_panel_releases(&config.system_root)
        .await
        .map_err(|error| anyhow!(error))?
        .into_iter()
        .find(|release| release.version == bundled_version);
    let candidate = match installed {
        Some(release) if verify_panel_release(&release).is_ok() => release,
        Some(release) => {
            let mut repaired = install_bootstrap_release(config)?;
            repaired.installed_at = release.installed_at;
            repaired.core.installed_at = release.core.installed_at;
            repaired
        }
        None => install_bootstrap_release(config)?,
    };
    state.pending = Some(PendingPanelActivation {
        transaction_id: nanoid::nanoid!(16),
        candidate,
        reason: niupanel_common::version::PanelActivationReason::Update,
        requested_at: Utc::now().to_rfc3339(),
        activation_snapshot_path: None,
        restore_before_start: None,
    });
    write_panel_runtime_state(&config.system_root, state)
        .await
        .map_err(|error| anyhow!(error))
}

fn bundled_panel_version(config: &LauncherConfig) -> Result<String> {
    config
        .bootstrap_panel_version
        .clone()
        .or_else(|| {
            config.bootstrap_core_version.clone().or_else(|| {
                read_core_release_manifest(&config.bundled_manifest_root)
                    .map(|manifest| manifest.version)
            })
        })
        .ok_or_else(|| {
            anyhow!(
                "Bundled Panel version is unavailable; provide NIUPANEL_BOOTSTRAP_PANEL_VERSION or verified Core metadata"
            )
        })
}

fn install_bootstrap_release(config: &LauncherConfig) -> Result<PanelReleaseDescriptor> {
    if !config.bundled_binary.is_file() {
        bail!(
            "Bundled Core binary does not exist: {}",
            config.bundled_binary.display()
        );
    }
    if !config.bundled_web_dir.is_dir() {
        bail!(
            "Bundled Web directory does not exist: {}",
            config.bundled_web_dir.display()
        );
    }
    let manifest = read_core_release_manifest(&config.bundled_manifest_root);
    let core_version = config
        .bootstrap_core_version
        .clone()
        .or_else(|| manifest.as_ref().map(|value| value.version.clone()))
        .ok_or_else(|| {
            anyhow!(
                "Bundled Core version is unavailable; provide NIUPANEL_BOOTSTRAP_CORE_VERSION or core-release.json"
            )
        })?;
    let binary_sha256 = hash_file(&config.bundled_binary)?;
    if let Some(manifest) = manifest.as_ref() {
        validate_bundled_core(manifest, &core_version, &binary_sha256)?;
    }
    let panel_version = config
        .bootstrap_panel_version
        .clone()
        .unwrap_or_else(|| core_version.clone());
    let web_manifest = read_web_release_manifest(&config.bundled_web_dir);
    let web_version = config
        .bootstrap_web_version
        .clone()
        .or_else(|| web_manifest.as_ref().map(|value| value.version.clone()))
        .ok_or_else(|| {
            anyhow!(
                "Bundled Web version is unavailable; provide NIUPANEL_BOOTSTRAP_WEB_VERSION or release-manifest.json"
            )
        })?;
    semver::Version::parse(&panel_version)
        .with_context(|| format!("Invalid bundled Panel version {panel_version}"))?;
    semver::Version::parse(&core_version)
        .with_context(|| format!("Invalid bundled Core version {core_version}"))?;
    semver::Version::parse(&web_version)
        .with_context(|| format!("Invalid bundled Web version {web_version}"))?;
    if let Some(manifest) = web_manifest.as_ref() {
        validate_bundled_web(
            &config.bundled_web_dir,
            manifest,
            &web_version,
            &core_version,
        )?;
    }
    let release_root = config
        .system_root
        .join("releases/panel")
        .join(&panel_version);
    let core_manifest = manifest.unwrap_or(CoreReleaseManifest {
        component: "core".into(),
        version: core_version.clone(),
        launcher_protocol: RELEASE_PROTOCOL_VERSION,
        api_contract: API_CONTRACT_VERSION,
        schema_epoch: SCHEMA_EPOCH,
        schema_revision: SCHEMA_REVISION,
        target: bundled_core_target()?.to_string(),
        binary_sha256,
        built_at: None,
    });
    let installed_at = core_manifest
        .built_at
        .clone()
        .unwrap_or_else(|| BOOTSTRAP_INSTALLED_AT.to_string());
    let staging_root = config.system_root.join("staging").join(format!(
        "bootstrap-{}-{}",
        panel_version,
        nanoid::nanoid!(10)
    ));
    let staging_core = staging_root.join("core");
    let staging_web = staging_root.join("web");
    let result = (|| {
        fs::create_dir_all(&staging_core)?;
        let staging_binary = staging_core.join("niupanel");
        fs::copy(&config.bundled_binary, &staging_binary)?;
        copy_permissions(&config.bundled_binary, &staging_binary)?;
        let bundled_tools = config.bundled_manifest_root.join("tools");
        if bundled_tools.is_dir() {
            copy_tree(&bundled_tools, &staging_core.join("tools"), false)?;
        }
        copy_tree(&config.bundled_web_dir, &staging_web, true)?;

        let descriptor = PanelReleaseDescriptor {
            version: panel_version,
            core: CoreReleaseDescriptor {
                version: core_version,
                path: release_root
                    .join("core/niupanel")
                    .to_string_lossy()
                    .into_owned(),
                launcher_protocol: core_manifest.launcher_protocol,
                api_contract: core_manifest.api_contract,
                schema_epoch: core_manifest.schema_epoch,
                schema_revision: core_manifest.schema_revision,
                target: core_manifest.target,
                installed_at: installed_at.clone(),
            },
            core_digest: directory_digest(&staging_core).map_err(|error| anyhow!(error))?,
            web_version,
            web_path: release_root.join("web").to_string_lossy().into_owned(),
            web_digest: directory_digest(&staging_web).map_err(|error| anyhow!(error))?,
            installed_at,
        };
        if release_root.exists() {
            fs::remove_dir_all(&release_root)?;
        }
        if let Some(parent) = release_root.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staging_root, &release_root)?;
        Ok(descriptor)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging_root);
    }
    result
}

fn bundled_core_target() -> Result<&'static str> {
    match env::consts::ARCH {
        "x86_64" => Ok("x86_64-unknown-linux-musl"),
        "aarch64" => Ok("aarch64-unknown-linux-musl"),
        "arm" => Ok("armv7-unknown-linux-musleabihf"),
        architecture => bail!("Unsupported bundled Core architecture: {architecture}"),
    }
}

fn validate_bundled_core(
    manifest: &CoreReleaseManifest,
    expected_version: &str,
    binary_sha256: &str,
) -> Result<()> {
    if manifest.component != "core"
        || manifest.version != expected_version
        || manifest.launcher_protocol != RELEASE_PROTOCOL_VERSION
        || manifest.api_contract != API_CONTRACT_VERSION
        || manifest.schema_epoch != SCHEMA_EPOCH
        || manifest.schema_revision != SCHEMA_REVISION
        || manifest.target != bundled_core_target()?
        || !manifest.binary_sha256.eq_ignore_ascii_case(binary_sha256)
    {
        bail!("Bundled Core does not match core-release.json");
    }
    Ok(())
}

fn validate_bundled_web(
    root: &Path,
    manifest: &WebReleaseManifest,
    expected_version: &str,
    core_version: &str,
) -> Result<()> {
    let core = semver::Version::parse(core_version)
        .with_context(|| format!("Invalid bundled Core version {core_version}"))?;
    let minimum = semver::Version::parse(&manifest.core.min)
        .with_context(|| format!("Invalid Web minimum Core version {}", manifest.core.min))?;
    let maximum = manifest
        .core
        .max
        .as_deref()
        .map(semver::Version::parse)
        .transpose()
        .context("Invalid Web maximum Core version")?;
    if manifest.component != "web"
        || manifest.version != expected_version
        || manifest.api_contract != API_CONTRACT_VERSION
        || core < minimum
        || maximum.as_ref().is_some_and(|maximum| core > *maximum)
    {
        bail!("Bundled Web does not match release-manifest.json");
    }

    let mut actual_files = std::collections::BTreeSet::new();
    collect_bundled_web_files(root, root, &mut actual_files)?;
    let declared_files = manifest.files.keys().cloned().collect();
    if actual_files != declared_files {
        bail!("Bundled Web files do not match release-manifest.json");
    }
    for (relative, expected_hash) in &manifest.files {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            bail!("Bundled Web manifest contains an unsafe path: {relative}");
        }
        if !hash_file(&root.join(path))?.eq_ignore_ascii_case(expected_hash) {
            bail!("Bundled Web file digest does not match: {relative}");
        }
    }
    Ok(())
}

fn collect_bundled_web_files(
    root: &Path,
    current: &Path,
    files: &mut std::collections::BTreeSet<String>,
) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_bundled_web_files(root, &entry.path(), files)?;
        } else if file_type.is_file() && entry.file_name() != WEB_RELEASE_MANIFEST_FILE {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .ok_or_else(|| anyhow!("Bundled Web contains a path that is not valid UTF-8"))?;
            files.insert(relative.replace('\\', "/"));
        } else if !file_type.is_file() {
            bail!(
                "Bundled Web contains an unsupported entry: {}",
                entry.path().display()
            );
        }
    }
    Ok(())
}

pub(crate) async fn recover_missing_active(
    config: &LauncherConfig,
    state: &mut PanelRuntimeState,
) -> Result<()> {
    let active_error = match verify_panel_release(&state.active) {
        Ok(()) => return Ok(()),
        Err(error) => error,
    };
    if state
        .pending
        .as_ref()
        .is_some_and(|pending| verify_panel_release(&pending.candidate).is_ok())
    {
        return Ok(());
    }
    if let Some(invalid_pending) = state.pending.take() {
        state.last_failure = Some(PanelActivationFailure {
            transaction_id: invalid_pending.transaction_id,
            version: invalid_pending.candidate.version,
            failed_at: Utc::now().to_rfc3339(),
            message: "Pending Panel release failed integrity verification".into(),
        });
    }

    let failed_version = state
        .last_failure
        .as_ref()
        .map(|failure| failure.version.as_str());
    if let Some(previous) = state
        .previous
        .clone()
        .filter(|release| failed_version != Some(release.version.as_str()))
        .filter(|release| verify_panel_release(release).is_ok())
    {
        if let Some(snapshot) = latest_snapshot_for_release(&config.system_root, &previous.version)
            .await
            .map_err(|error| anyhow!(error))?
        {
            queue_recovery(state, previous, Some(snapshot), active_error);
            return write_panel_runtime_state(&config.system_root, state)
                .await
                .map_err(|error| anyhow!(error));
        }
    }

    let bootstrap = install_bootstrap_release(config)?;
    if bootstrap.version == state.active.version && verify_panel_release(&state.active).is_ok() {
        state.last_failure = Some(PanelActivationFailure {
            transaction_id: nanoid::nanoid!(16),
            version: state.active.version.clone(),
            failed_at: Utc::now().to_rfc3339(),
            message: "Active Panel files were repaired from the immutable bundled release".into(),
        });
        return write_panel_runtime_state(&config.system_root, state)
            .await
            .map_err(|error| anyhow!(error));
    }
    if bootstrap.version == state.active.version {
        bail!(
            "Bundled Panel {} does not match the immutable active release descriptor",
            bootstrap.version
        );
    }
    if failed_version == Some(bootstrap.version.as_str()) {
        bail!(
            "Active Panel {} is invalid and recovery candidate {} already failed",
            state.active.version,
            bootstrap.version
        );
    }
    let active_version = semver::Version::parse(&state.active.version)
        .with_context(|| format!("Invalid active Panel version {}", state.active.version))?;
    let bootstrap_version = semver::Version::parse(&bootstrap.version)
        .with_context(|| format!("Invalid bundled Panel version {}", bootstrap.version))?;
    let restore = if bootstrap_version < active_version {
        latest_snapshot_for_release(&config.system_root, &bootstrap.version)
            .await
            .map_err(|error| anyhow!(error))?
            .ok_or_else(|| {
                anyhow!(
                    "Active Panel {} is invalid and no database snapshot exists for safe recovery to bundled Panel {}",
                    state.active.version,
                    bootstrap.version
                )
            })?
            .into()
    } else {
        None
    };
    queue_recovery(state, bootstrap, restore, active_error);
    write_panel_runtime_state(&config.system_root, state)
        .await
        .map_err(|error| anyhow!(error))
}

fn queue_recovery(
    state: &mut PanelRuntimeState,
    candidate: PanelReleaseDescriptor,
    restore_before_start: Option<PathBuf>,
    active_error: String,
) {
    state.pending = Some(PendingPanelActivation {
        transaction_id: nanoid::nanoid!(16),
        candidate,
        reason: niupanel_common::version::PanelActivationReason::Recovery,
        requested_at: Utc::now().to_rfc3339(),
        activation_snapshot_path: None,
        restore_before_start: restore_before_start.map(|path| path.to_string_lossy().into_owned()),
    });
    state.last_failure = Some(PanelActivationFailure {
        transaction_id: "startup-integrity-check".into(),
        version: state.active.version.clone(),
        failed_at: Utc::now().to_rfc3339(),
        message: active_error,
    });
}

fn copy_tree(source: &Path, destination: &Path, strip_manifests: bool) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        if strip_manifests
            && (name == WEB_RELEASE_MANIFEST_FILE || name == CORE_RELEASE_MANIFEST_FILE)
        {
            continue;
        }
        let target = destination.join(&name);
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target, strip_manifests)?;
        } else if entry.file_type()?.is_file() {
            fs::copy(entry.path(), &target)?;
            copy_permissions(&entry.path(), &target)?;
        }
    }
    Ok(())
}
