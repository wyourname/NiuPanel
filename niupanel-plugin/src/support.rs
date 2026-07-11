use super::*;

pub(super) fn read_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let content = fs::read_to_string(path.join(MANIFEST_FILE))?;
    Ok(serde_json::from_str(&content)?)
}

pub(super) fn validate_plugin_id(id: &str) -> Result<()> {
    let valid = !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_');
    if valid {
        Ok(())
    } else {
        Err(AppError::ValidationError(
            "Plugin id must use lowercase letters, digits, '-' or '_'".to_string(),
        ))
    }
}

pub(super) fn validate_version_id(id: &str) -> Result<()> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.');
    if valid {
        Ok(())
    } else {
        Err(AppError::ValidationError(
            "Plugin version id contains invalid characters".to_string(),
        ))
    }
}

pub(super) fn archive_record_for(
    version: &str,
    package_sha256: Option<String>,
    archive_path: &Path,
) -> PluginVersionRecord {
    PluginVersionRecord {
        id: archive_path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| Utc::now().timestamp_millis().to_string()),
        version: version.to_string(),
        archived_at: now_rfc3339(),
        path: archive_path.display().to_string(),
        package_sha256,
    }
}

pub(super) fn sanitize_version_for_path(version: &str) -> String {
    let value: String = version
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if value.is_empty() {
        "unknown".to_string()
    } else {
        value
    }
}

pub(super) fn safe_entry_path(base: &Path, entry: &str) -> Result<PathBuf> {
    let entry_path = Path::new(entry);
    if entry_path.is_absolute()
        || entry_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(AppError::ValidationError(
            "Plugin entry must be a relative path inside the plugin package".to_string(),
        ));
    }
    Ok(base.join(entry_path))
}

pub(super) fn copy_dir(source: &Path, target: &Path) -> Result<()> {
    if !source.is_dir() {
        return Err(AppError::ValidationError(
            "Plugin source path must be a directory".to_string(),
        ));
    }
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

pub(super) fn process_pool_key(spec: &ProcessPluginSpec) -> String {
    format!(
        "{}@{}:{}",
        spec.plugin_id, spec.version, spec.extension_point
    )
}

pub(super) fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(super) fn tool_error_code(err: &AppError) -> &'static str {
    match err {
        AppError::ValidationError(_) | AppError::Forbidden(_) => "tool_denied",
        _ => "tool_error",
    }
}

pub(super) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}
