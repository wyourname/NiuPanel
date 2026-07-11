use super::*;

pub(super) fn validate_archive_path(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::ValidationError(format!(
            "Web package contains an unsafe path: {}",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn unpack_archive(package_path: &Path, destination: &Path) -> Result<()> {
    let file = fs::File::open(package_path).map_err(AppError::Io)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive
        .entries()
        .map_err(|error| AppError::ValidationError(error.to_string()))?
    {
        let mut entry = entry.map_err(|error| AppError::ValidationError(error.to_string()))?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(AppError::ValidationError(
                "Web package may only contain regular files and directories".to_string(),
            ));
        }
        let path = entry
            .path()
            .map_err(|error| AppError::ValidationError(error.to_string()))?;
        validate_archive_path(&path)?;
        entry
            .unpack_in(destination)
            .map_err(|error| AppError::ValidationError(error.to_string()))?;
    }
    Ok(())
}

pub(super) fn find_release_root(staging: &Path) -> Result<PathBuf> {
    let manifests = WalkDir::new(staging)
        .min_depth(1)
        .max_depth(3)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.file_name() == WEB_RELEASE_MANIFEST_FILE)
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    if manifests.len() != 1 {
        return Err(AppError::ValidationError(
            "Web package must contain exactly one release-manifest.json".to_string(),
        ));
    }
    Ok(manifests[0]
        .parent()
        .expect("manifest path has a parent")
        .to_path_buf())
}

pub(super) fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(AppError::Io)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(AppError::Io)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub(super) fn verify_release_files(root: &Path, manifest: &WebReleaseManifest) -> Result<()> {
    if manifest.files.is_empty() {
        return Err(AppError::ValidationError(
            "Web release manifest does not contain file checksums".to_string(),
        ));
    }
    if !root.join("index.html").is_file() {
        return Err(AppError::ValidationError(
            "Web release does not contain index.html".to_string(),
        ));
    }

    let mut actual_files = BTreeSet::new();
    for entry in WalkDir::new(root).into_iter() {
        let entry = entry.map_err(|error| AppError::Io(error.into()))?;
        if !entry.file_type().is_file() || entry.file_name() == WEB_RELEASE_MANIFEST_FILE {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|error| AppError::Internal(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        actual_files.insert(relative);
    }
    let declared_files = manifest.files.keys().cloned().collect::<BTreeSet<_>>();
    if actual_files != declared_files {
        return Err(AppError::ValidationError(
            "Web release file list does not match its manifest".to_string(),
        ));
    }

    for (relative, expected_hash) in &manifest.files {
        validate_archive_path(Path::new(relative))?;
        let actual_hash = hash_file(&root.join(relative))?;
        if !actual_hash.eq_ignore_ascii_case(expected_hash) {
            return Err(AppError::ValidationError(format!(
                "Web release checksum mismatch: {relative}"
            )));
        }
    }
    Ok(())
}
