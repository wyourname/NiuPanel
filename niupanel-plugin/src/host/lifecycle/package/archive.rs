use super::*;

pub(in crate::host::lifecycle) fn extract_plugin_package(
    file_name: &str,
    bytes: &[u8],
) -> Result<TempDir> {
    let temp_dir = tempfile::tempdir().map_err(AppError::Io)?;
    let lower = file_name.to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(Cursor::new(bytes), temp_dir.path())?;
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        let decoder = GzDecoder::new(Cursor::new(bytes));
        extract_tar(decoder, temp_dir.path())?;
    } else if lower.ends_with(".tar") {
        extract_tar(Cursor::new(bytes), temp_dir.path())?;
    } else {
        return Err(AppError::ValidationError(
            "Unsupported plugin package format. Supported: .zip, .tar, .tar.gz, .tgz".to_string(),
        ));
    }
    Ok(temp_dir)
}

pub(in crate::host::lifecycle) fn extract_plugin_package_file_to(
    file_name: &str,
    path: &Path,
    destination: &Path,
) -> Result<ExtractionStats> {
    fs::create_dir_all(destination).map_err(AppError::Io)?;
    let lower = file_name.to_lowercase();
    let file = fs::File::open(path).map_err(AppError::Io)?;
    if lower.ends_with(".zip") {
        extract_zip(file, destination)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        extract_tar(GzDecoder::new(file), destination)
    } else if lower.ends_with(".tar") {
        extract_tar(file, destination)
    } else {
        Err(AppError::ValidationError(
            "Unsupported plugin package format. Supported: .zip, .tar, .tar.gz, .tgz".to_string(),
        ))
    }
}

pub(in crate::host::lifecycle) fn resolve_plugin_package_root(
    extracted_root: &Path,
    compatibility: ManifestCompatibility,
    package_label: &str,
) -> Result<PathBuf> {
    if has_plugin_manifest(extracted_root, compatibility) {
        return Ok(extracted_root.to_path_buf());
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(extracted_root).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        let path = entry.path();
        if path.is_dir() && has_plugin_manifest(&path, compatibility) {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(AppError::ValidationError(format!(
            "{package_label} package must contain plugin.json at root or inside a single top-level directory"
        ))),
        _ => Err(AppError::ValidationError(format!(
            "{package_label} package contains multiple top-level plugin.json files"
        ))),
    }
}

pub(in crate::host::lifecycle) fn has_plugin_manifest(
    path: &Path,
    compatibility: ManifestCompatibility,
) -> bool {
    match compatibility {
        ManifestCompatibility::PluginJsonOnly => path.join("plugin.json").is_file(),
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::host::lifecycle) struct ExtractionStats {
    pub(in crate::host::lifecycle) entries: usize,
    pub(in crate::host::lifecycle) bytes: u64,
}

pub(in crate::host::lifecycle) fn account_archive_entry(
    stats: &mut ExtractionStats,
    size: u64,
) -> Result<()> {
    stats.entries = stats.entries.saturating_add(1);
    if stats.entries > MAX_PLUGIN_ARCHIVE_ENTRIES {
        return Err(AppError::FileSizeLimitExceeded(format!(
            "Plugin package contains more than {MAX_PLUGIN_ARCHIVE_ENTRIES} entries"
        )));
    }
    stats.bytes = stats.bytes.checked_add(size).ok_or_else(|| {
        AppError::FileSizeLimitExceeded("Plugin package extracted size overflow".to_string())
    })?;
    if stats.bytes > MAX_PLUGIN_EXTRACTED_BYTES {
        return Err(AppError::FileSizeLimitExceeded(
            "Plugin package extracts to more than 512MB".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::host::lifecycle) fn extract_tar<R>(
    reader: R,
    destination: &Path,
) -> Result<ExtractionStats>
where
    R: Read,
{
    let mut archive = tar::Archive::new(reader);
    let mut stats = ExtractionStats {
        entries: 0,
        bytes: 0,
    };
    for entry in archive.entries().map_err(AppError::Io)? {
        let mut entry = entry.map_err(AppError::Io)?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(AppError::ValidationError(
                "Plugin packages cannot contain links".to_string(),
            ));
        }
        let entry_size = if entry_type.is_file() {
            entry.header().size().map_err(AppError::Io)?
        } else {
            0
        };
        account_archive_entry(&mut stats, entry_size)?;
        if !(entry_type.is_file() || entry_type.is_dir()) {
            continue;
        }

        let path = entry.path().map_err(AppError::Io)?;
        let safe_path = sanitize_archive_path(&path)?;
        let output_path = destination.join(safe_path);
        ensure_within(destination, &output_path)?;
        if entry_type.is_dir() {
            fs::create_dir_all(&output_path).map_err(AppError::Io)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            entry.unpack(&output_path).map_err(AppError::Io)?;
        }
    }
    Ok(stats)
}

pub(in crate::host::lifecycle) fn extract_zip<R>(
    reader: R,
    destination: &Path,
) -> Result<ExtractionStats>
where
    R: Read + Seek,
{
    let mut archive = ZipArchive::new(reader)
        .map_err(|err| AppError::ValidationError(format!("Invalid zip package: {err}")))?;
    let mut stats = ExtractionStats {
        entries: 0,
        bytes: 0,
    };
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|err| AppError::ValidationError(format!("Invalid zip entry: {err}")))?;
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(AppError::ValidationError(
                "Plugin packages cannot contain links".to_string(),
            ));
        }
        account_archive_entry(&mut stats, file.size())?;
        let Some(enclosed_name) = file.enclosed_name() else {
            return Err(AppError::ValidationError(
                "Plugin package contains an unsafe zip path".to_string(),
            ));
        };
        let safe_path = sanitize_archive_path(&enclosed_name)?;
        let output_path = destination.join(safe_path);
        ensure_within(destination, &output_path)?;
        if file.is_dir() {
            fs::create_dir_all(&output_path).map_err(AppError::Io)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            let mut output = fs::File::create(&output_path).map_err(AppError::Io)?;
            let expected_size = file.size();
            let copied = std::io::copy(&mut file.take(expected_size + 1), &mut output)
                .map_err(AppError::Io)?;
            if copied != expected_size {
                return Err(AppError::ValidationError(
                    "Plugin package entry size changed during extraction".to_string(),
                ));
            }
        }
    }
    Ok(stats)
}

pub(in crate::host::lifecycle) fn sanitize_archive_path(path: &Path) -> Result<PathBuf> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin package contains an unsafe path".to_string(),
                ));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(AppError::ValidationError(
            "Plugin package contains an empty path".to_string(),
        ));
    }
    Ok(clean)
}

pub(in crate::host::lifecycle) fn ensure_within(base: &Path, target: &Path) -> Result<()> {
    if !target.starts_with(base) {
        return Err(AppError::ValidationError(
            "Plugin package entry escapes extraction directory".to_string(),
        ));
    }
    Ok(())
}
