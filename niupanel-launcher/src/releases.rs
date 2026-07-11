use super::*;

pub(crate) fn fallback_manifest(config: &LauncherConfig) -> Result<CoreReleaseManifest> {
    Ok(CoreReleaseManifest {
        component: "core".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        launcher_protocol: RELEASE_PROTOCOL_VERSION,
        api_contract: API_CONTRACT_VERSION,
        schema_epoch: SCHEMA_EPOCH,
        schema_revision: SCHEMA_REVISION,
        target: format!("{}-{}", env::consts::OS, env::consts::ARCH),
        binary_sha256: hash_file(&config.bundled_binary)?,
        built_at: None,
    })
}

pub(crate) fn register_bundled_release(config: &LauncherConfig) -> Result<CoreReleaseDescriptor> {
    if !config.bundled_binary.is_file() {
        bail!(
            "Bundled Core binary does not exist: {}",
            config.bundled_binary.display()
        );
    }
    let manifest = read_core_release_manifest(&config.bundled_manifest_root)
        .map(Ok)
        .unwrap_or_else(|| fallback_manifest(config))?;
    validate_core_manifest(&manifest, &config.bundled_binary)?;

    let release_root = config
        .system_root
        .join("releases/core")
        .join(&manifest.version);
    fs::create_dir_all(&release_root)?;
    let installed_binary = release_root.join("niupanel");
    if installed_binary.exists() {
        let installed_hash = hash_file(&installed_binary)?;
        if !installed_hash.eq_ignore_ascii_case(&manifest.binary_sha256) {
            bail!(
                "Installed Core release {} does not match the bundled manifest",
                manifest.version
            );
        }
    } else {
        fs::copy(&config.bundled_binary, &installed_binary)?;
        copy_permissions(&config.bundled_binary, &installed_binary)?;
    }
    fs::write(
        release_root.join(CORE_RELEASE_MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    Ok(CoreReleaseDescriptor {
        version: manifest.version,
        path: installed_binary.to_string_lossy().into_owned(),
        launcher_protocol: manifest.launcher_protocol,
        api_contract: manifest.api_contract,
        schema_epoch: manifest.schema_epoch,
        schema_revision: manifest.schema_revision,
        target: manifest.target,
        installed_at: manifest.built_at.unwrap_or_else(|| Utc::now().to_rfc3339()),
    })
}

pub(crate) fn validate_core_manifest(manifest: &CoreReleaseManifest, binary: &Path) -> Result<()> {
    if manifest.component != "core" {
        bail!("Core manifest component must be 'core'");
    }
    Version::parse(&manifest.version).context("Core manifest version is invalid")?;
    if manifest.launcher_protocol != RELEASE_PROTOCOL_VERSION {
        bail!(
            "Core launcher protocol {} is not supported by launcher protocol {}",
            manifest.launcher_protocol,
            RELEASE_PROTOCOL_VERSION
        );
    }
    let actual_hash = hash_file(binary)?;
    if !actual_hash.eq_ignore_ascii_case(&manifest.binary_sha256) {
        bail!("Bundled Core binary checksum does not match core-release.json");
    }
    Ok(())
}

pub(crate) fn recover_missing_active(
    state: &mut CoreRuntimeState,
    bundled: &CoreReleaseDescriptor,
) {
    if !Path::new(&state.active.path).is_file() {
        state.active = bundled.clone();
        state.previous = None;
        state.pending = None;
        state.last_failure = Some(CoreActivationFailure {
            transaction_id: "startup-recovery".into(),
            version: bundled.version.clone(),
            failed_at: Utc::now().to_rfc3339(),
            message: "The configured active Core binary was missing; bundled Core restored".into(),
        });
    }
}

pub(crate) fn schedule_bundled_release(
    config: &LauncherConfig,
    state: &mut CoreRuntimeState,
    bundled: &CoreReleaseDescriptor,
) -> Result<()> {
    if state.pending.is_some() || state.active.version == bundled.version {
        return Ok(());
    }
    let policy = env::var("NIUPANEL_BOOT_CORE").unwrap_or_else(|_| "newer".into());
    let active_version = Version::parse(&state.active.version).ok();
    let bundled_version = Version::parse(&bundled.version).ok();
    let should_activate = policy == "bundled"
        || (policy == "newer"
            && bundled_version
                .zip(active_version)
                .map(|(bundled, active)| bundled > active)
                .unwrap_or(false));
    if !should_activate {
        return Ok(());
    }
    let restore_before_start = if bundled.schema_epoch != state.active.schema_epoch {
        Some(
            find_rollback_snapshot(&config.system_root, &bundled.version)?.ok_or_else(|| {
                anyhow!(
                    "Cannot boot bundled Core {} across schema epochs because no rollback snapshot is available",
                    bundled.version
                )
            })?,
        )
    } else {
        None
    };
    state.pending = Some(PendingCoreActivation {
        transaction_id: nanoid::nanoid!(16),
        candidate: bundled.clone(),
        reason: if bundled.schema_epoch < state.active.schema_epoch {
            CoreActivationReason::Rollback
        } else {
            CoreActivationReason::Update
        },
        requested_at: Utc::now().to_rfc3339(),
        restore_before_start: restore_before_start.map(|path| path.to_string_lossy().into_owned()),
    });
    write_core_runtime_state(&config.system_root, state).map_err(|error| anyhow!(error))
}

pub(crate) fn find_rollback_snapshot(
    system_root: &Path,
    target_version: &str,
) -> Result<Option<PathBuf>> {
    let transactions = system_root.join(niupanel_common::version::CORE_TRANSACTIONS_DIR);
    if !transactions.is_dir() {
        return Ok(None);
    }
    let mut matches = Vec::new();
    for entry in fs::read_dir(transactions)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let transaction =
            serde_json::from_slice::<CoreActivationTransaction>(&fs::read(entry.path())?);
        let Ok(transaction) = transaction else {
            continue;
        };
        let snapshot = PathBuf::from(&transaction.snapshot_path);
        if transaction.successful && transaction.from.version == target_version && snapshot.is_dir()
        {
            matches.push((transaction.completed_at, snapshot));
        }
    }
    matches.sort_by(|left, right| right.0.cmp(&left.0));
    Ok(matches.into_iter().next().map(|(_, snapshot)| snapshot))
}
