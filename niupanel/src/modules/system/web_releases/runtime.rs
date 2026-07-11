use super::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct WebReleasePointer {
    pub(super) version: String,
    pub(super) path: String,
    pub(super) managed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct WebRuntimeState {
    pub(super) protocol: u32,
    pub(super) active: WebReleasePointer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) previous: Option<WebReleasePointer>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebReleaseRecord {
    pub version: String,
    pub active: bool,
    pub previous: bool,
    pub managed: bool,
    pub compatible: bool,
    pub compatibility_error: Option<String>,
    pub manifest: Option<WebReleaseManifest>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebReleaseList {
    pub active_version: String,
    pub previous_version: Option<String>,
    pub releases: Vec<WebReleaseRecord>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WebReleaseMutation {
    pub version: String,
    pub active: bool,
    pub message: String,
}

pub(super) fn system_path(relative: &str) -> PathBuf {
    let root = &Config::global().system_dir;
    let root = if root.is_absolute() {
        root.clone()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    };
    root.join(relative)
}

pub fn web_root() -> PathBuf {
    system_path(WEB_CURRENT_LINK)
}

pub(super) fn state_path() -> PathBuf {
    system_path(WEB_STATE_FILE)
}

pub(super) fn releases_dir() -> PathBuf {
    system_path(WEB_RELEASES_DIR)
}

pub(super) fn staging_dir() -> PathBuf {
    system_path(WEB_STAGING_DIR)
}

pub(super) fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir().map_err(AppError::Io)?.join(path))
}

pub(super) fn read_state() -> Result<WebRuntimeState> {
    let content = fs::read_to_string(state_path()).map_err(AppError::Io)?;
    serde_json::from_str(&content).map_err(AppError::Json)
}

pub(super) fn write_state(state: &WebRuntimeState) -> Result<()> {
    let path = state_path();
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Internal("Invalid Web state path".to_string()))?;
    fs::create_dir_all(parent).map_err(AppError::Io)?;
    let temp = parent.join(format!(".state-{}.tmp", nanoid::nanoid!(10)));
    let content = serde_json::to_vec_pretty(state).map_err(AppError::Json)?;
    fs::write(&temp, content).map_err(AppError::Io)?;
    fs::rename(&temp, &path).map_err(AppError::Io)
}

#[cfg(unix)]
pub(super) fn switch_current_link(target: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    let current = web_root();
    let parent = current
        .parent()
        .ok_or_else(|| AppError::Internal("Invalid Web current path".to_string()))?;
    fs::create_dir_all(parent).map_err(AppError::Io)?;
    let temp = parent.join(format!(".current-{}", nanoid::nanoid!(10)));
    symlink(target, &temp).map_err(AppError::Io)?;
    fs::rename(&temp, &current).map_err(|error| {
        let _ = fs::remove_file(&temp);
        AppError::Io(error)
    })
}

#[cfg(not(unix))]
pub(super) fn switch_current_link(_target: &Path) -> Result<()> {
    Err(AppError::Internal(
        "Atomic Web release activation currently requires Unix symlinks".to_string(),
    ))
}

pub(super) fn pointer_from_path(path: PathBuf, managed: bool) -> WebReleasePointer {
    let manifest = read_web_release_manifest(&path);
    let version = manifest
        .map(|value| value.version)
        .or_else(|| std::env::var("NIUPANEL_WEB_VERSION").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "bundled".to_string());
    WebReleasePointer {
        version,
        path: path.to_string_lossy().into_owned(),
        managed,
    }
}

pub(super) fn development_fallback_root() -> Result<PathBuf> {
    let root = system_path("web/development-fallback");
    fs::create_dir_all(&root).map_err(AppError::Io)?;
    let index = root.join("index.html");
    let content = b"<!doctype html><meta charset=\"utf-8\"><title>NiuPanel Web</title><p>The bundled Web UI is not available in this development process.</p><p><a href=\"/recovery\">Open recovery console</a></p>";
    fs::write(&index, content).map_err(AppError::Io)?;
    let manifest = WebReleaseManifest {
        component: "web".into(),
        version: "development".into(),
        api_contract: niupanel_common::version::API_CONTRACT_VERSION,
        core: niupanel_common::version::ComponentCompatibility {
            min: env!("CARGO_PKG_VERSION").into(),
            max: None,
        },
        built_at: None,
        files: BTreeMap::from([("index.html".into(), hex::encode(Sha256::digest(content)))]),
    };
    fs::write(
        root.join(WEB_RELEASE_MANIFEST_FILE),
        serde_json::to_vec_pretty(&manifest).map_err(AppError::Json)?,
    )
    .map_err(AppError::Io)?;
    Ok(root)
}

pub(super) fn copy_release_directory(source: &Path, target: &Path) -> Result<()> {
    for entry in WalkDir::new(source).into_iter() {
        let entry = entry.map_err(|error| AppError::Io(error.into()))?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination).map_err(AppError::Io)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            fs::copy(entry.path(), destination).map_err(AppError::Io)?;
        } else {
            return Err(AppError::ValidationError(
                "Bundled Web release may not contain links".to_string(),
            ));
        }
    }
    Ok(())
}

pub(super) fn register_bundled_release(bundled: &Path) -> Result<WebReleasePointer> {
    let manifest = read_web_release_manifest(bundled).ok_or_else(|| {
        AppError::ValidationError("Bundled Web release manifest is missing".to_string())
    })?;
    validate_web_compatibility(env!("CARGO_PKG_VERSION"), &manifest)
        .map_err(AppError::ValidationError)?;
    verify_release_files(bundled, &manifest)?;

    let target = releases_dir().join(&manifest.version);
    if target.exists() {
        let installed_manifest = read_web_release_manifest(&target).ok_or_else(|| {
            AppError::ValidationError(format!(
                "Installed Web release {} has no manifest",
                manifest.version
            ))
        })?;
        if installed_manifest.files != manifest.files {
            if Config::global().allow_missing_bundled_web {
                tracing::warn!(
                    version = %manifest.version,
                    "Ignoring rebuilt bundled Web release with an unchanged version in development mode"
                );
                return Ok(pointer_from_path(target, true));
            }
            return Err(AppError::ValidationError(format!(
                "Web release {} already exists with different contents",
                manifest.version
            )));
        }
        verify_release_files(&target, &installed_manifest)?;
        return Ok(pointer_from_path(target, true));
    }

    let staging = staging_dir().join(format!("bundled-{}", nanoid::nanoid!(10)));
    fs::create_dir_all(&staging).map_err(AppError::Io)?;
    let result = (|| {
        copy_release_directory(bundled, &staging)?;
        verify_release_files(&staging, &manifest)?;
        fs::rename(&staging, &target).map_err(AppError::Io)?;
        Ok(pointer_from_path(target, true))
    })();
    let _ = fs::remove_dir_all(&staging);
    result
}

pub(super) fn bundled_release_pointer() -> Result<Option<WebReleasePointer>> {
    let bundled = absolute_path(&Config::global().bundled_web_dir)?;
    if bundled.is_dir() {
        return register_bundled_release(&bundled).map(Some);
    }
    if Config::global().allow_missing_bundled_web {
        return Ok(Some(pointer_from_path(development_fallback_root()?, false)));
    }
    Ok(None)
}

pub fn ensure_web_runtime() -> Result<()> {
    fs::create_dir_all(releases_dir()).map_err(AppError::Io)?;
    fs::create_dir_all(staging_dir()).map_err(AppError::Io)?;
    let bundled_pointer = bundled_release_pointer()?;

    let state = match read_state() {
        Ok(state) if Path::new(&state.active.path).is_dir() => state,
        _ => {
            let active = bundled_pointer.ok_or_else(|| {
                AppError::NotFound(format!(
                    "Bundled Web directory does not exist: {}",
                    Config::global().bundled_web_dir.display()
                ))
            })?;
            let state = WebRuntimeState {
                protocol: RELEASE_PROTOCOL_VERSION,
                active,
                previous: None,
            };
            write_state(&state)?;
            state
        }
    };

    let current = web_root();
    let points_to_active = fs::read_link(&current)
        .ok()
        .map(|target| target == PathBuf::from(&state.active.path))
        .unwrap_or(false);
    if !points_to_active {
        if let Ok(metadata) = current.symlink_metadata() {
            if metadata.file_type().is_symlink() || metadata.is_file() {
                fs::remove_file(&current).map_err(AppError::Io)?;
            } else if metadata.is_dir() {
                fs::remove_dir_all(&current).map_err(AppError::Io)?;
            }
        }
        switch_current_link(Path::new(&state.active.path))?;
    }
    Ok(())
}
