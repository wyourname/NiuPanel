use super::*;

pub(super) fn record_for(pointer: &WebReleasePointer, state: &WebRuntimeState) -> WebReleaseRecord {
    let manifest = read_web_release_manifest(&pointer.path);
    let compatibility = manifest
        .as_ref()
        .map(|value| validate_web_compatibility(env!("CARGO_PKG_VERSION"), value))
        .unwrap_or_else(|| Err("Web release manifest is missing".to_string()));
    WebReleaseRecord {
        version: pointer.version.clone(),
        active: pointer.path == state.active.path,
        previous: state
            .previous
            .as_ref()
            .map(|value| value.path == pointer.path)
            .unwrap_or(false),
        managed: pointer.managed,
        compatible: compatibility.is_ok(),
        compatibility_error: compatibility.err(),
        manifest,
    }
}

pub fn list_releases() -> Result<WebReleaseList> {
    ensure_web_runtime()?;
    let state = read_state()?;
    let mut pointers = vec![state.active.clone()];
    if let Some(previous) = state.previous.clone() {
        if previous.path != state.active.path {
            pointers.push(previous);
        }
    }
    for entry in fs::read_dir(releases_dir()).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        if !entry.file_type().map_err(AppError::Io)?.is_dir() {
            continue;
        }
        let pointer = pointer_from_path(entry.path(), true);
        if !pointers.iter().any(|value| value.path == pointer.path) {
            pointers.push(pointer);
        }
    }

    let mut releases = pointers
        .iter()
        .map(|pointer| record_for(pointer, &state))
        .collect::<Vec<_>>();
    releases.sort_by(|left, right| {
        right
            .active
            .cmp(&left.active)
            .then_with(|| right.previous.cmp(&left.previous))
            .then_with(|| right.version.cmp(&left.version))
    });
    Ok(WebReleaseList {
        active_version: state.active.version,
        previous_version: state.previous.map(|value| value.version),
        releases,
    })
}

pub fn install_release(package_path: &Path, activate: bool) -> Result<WebReleaseMutation> {
    install_release_expected(package_path, activate, None)
}

pub fn install_release_expected(
    package_path: &Path,
    activate: bool,
    expected_version: Option<&str>,
) -> Result<WebReleaseMutation> {
    ensure_web_runtime()?;
    let staging = staging_dir().join(nanoid::nanoid!(12));
    fs::create_dir_all(&staging).map_err(AppError::Io)?;
    let result = (|| {
        unpack_archive(package_path, &staging)?;
        let root = find_release_root(&staging)?;
        let manifest = read_web_release_manifest(&root)
            .ok_or_else(|| AppError::ValidationError("Invalid Web release manifest".to_string()))?;
        if let Some(expected_version) = expected_version {
            if manifest.version != expected_version {
                return Err(AppError::ValidationError(format!(
                    "Web 包版本 {} 与发布索引版本 {} 不一致",
                    manifest.version, expected_version
                )));
            }
        }
        validate_web_compatibility(env!("CARGO_PKG_VERSION"), &manifest)
            .map_err(AppError::ValidationError)?;
        verify_release_files(&root, &manifest)?;

        let target = releases_dir().join(&manifest.version);
        if target.exists() {
            let installed = read_web_release_manifest(&target).ok_or_else(|| {
                AppError::ValidationError(format!(
                    "Web release {} is installed without a valid manifest",
                    manifest.version
                ))
            })?;
            if installed.files != manifest.files {
                return Err(AppError::ValidationError(format!(
                    "Web release {} already exists with different contents; versions are immutable",
                    manifest.version
                )));
            }
            verify_release_files(&target, &installed)?;
            return if activate {
                activate_pointer(pointer_from_path(target, true))
            } else {
                Ok(WebReleaseMutation {
                    version: manifest.version,
                    active: false,
                    message: "Web release is already installed".to_string(),
                })
            };
        }
        fs::rename(&root, &target).map_err(AppError::Io)?;
        let mutation = if activate {
            activate_pointer(pointer_from_path(target, true))?
        } else {
            WebReleaseMutation {
                version: manifest.version,
                active: false,
                message: "Web release installed".to_string(),
            }
        };
        prune_releases()?;
        Ok(mutation)
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}

pub(super) fn activate_pointer(candidate: WebReleasePointer) -> Result<WebReleaseMutation> {
    let manifest = read_web_release_manifest(&candidate.path)
        .ok_or_else(|| AppError::ValidationError("Web release manifest is missing".to_string()))?;
    validate_web_compatibility(env!("CARGO_PKG_VERSION"), &manifest)
        .map_err(AppError::ValidationError)?;

    let mut state = read_state()?;
    if state.active.path == candidate.path {
        return Ok(WebReleaseMutation {
            version: candidate.version,
            active: true,
            message: "Web release is already active".to_string(),
        });
    }
    switch_current_link(Path::new(&candidate.path))?;
    let previous = std::mem::replace(&mut state.active, candidate.clone());
    state.previous = Some(previous.clone());
    if let Err(error) = write_state(&state) {
        let _ = switch_current_link(Path::new(&previous.path));
        return Err(error);
    }
    Ok(WebReleaseMutation {
        version: candidate.version,
        active: true,
        message: "Web release activated".to_string(),
    })
}

pub fn activate_release(version: &str) -> Result<WebReleaseMutation> {
    ensure_web_runtime()?;
    let target = releases_dir().join(version);
    if !target.is_dir() {
        return Err(AppError::NotFound(format!(
            "Web release {version} is not installed"
        )));
    }
    activate_pointer(pointer_from_path(target, true))
}

pub fn rollback_release() -> Result<WebReleaseMutation> {
    ensure_web_runtime()?;
    let state = read_state()?;
    let previous = state
        .previous
        .ok_or_else(|| AppError::NotFound("No previous Web release is available".to_string()))?;
    activate_pointer(previous)
}

pub(super) fn prune_releases() -> Result<()> {
    let state = read_state()?;
    let protected = [
        Some(state.active.path),
        state.previous.map(|value| value.path),
    ]
    .into_iter()
    .flatten()
    .collect::<BTreeSet<_>>();
    let mut releases = fs::read_dir(releases_dir())
        .map_err(AppError::Io)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .collect::<Vec<_>>();
    releases.sort_by_key(|entry| entry.metadata().and_then(|value| value.modified()).ok());
    let excess = releases.len().saturating_sub(RETAINED_MANAGED_RELEASES);
    for entry in releases.into_iter().take(excess) {
        if !protected.contains(&entry.path().to_string_lossy().into_owned()) {
            fs::remove_dir_all(entry.path()).map_err(AppError::Io)?;
        }
    }
    Ok(())
}
