use super::*;

pub(crate) fn create_database_snapshot(
    config: &LauncherConfig,
    transaction_id: &str,
) -> Result<PathBuf> {
    let directory = config.system_root.join("snapshots").join(transaction_id);
    if directory.exists() {
        fs::remove_dir_all(&directory)?;
    }
    fs::create_dir_all(&directory)?;
    copy_database_files(&config.database_path, &directory)?;
    Ok(directory)
}

pub(crate) fn restore_database_snapshot(config: &LauncherConfig, snapshot: &Path) -> Result<()> {
    if !snapshot.is_dir() {
        bail!("Database snapshot does not exist: {}", snapshot.display());
    }
    if let Some(parent) = config.database_path.parent() {
        fs::create_dir_all(parent)?;
    }
    for path in database_paths(&config.database_path) {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    for entry in fs::read_dir(snapshot)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::copy(
                entry.path(),
                config.database_path.with_file_name(entry.file_name()),
            )?;
        }
    }
    Ok(())
}

pub(crate) fn copy_database_files(database: &Path, destination: &Path) -> Result<()> {
    for path in database_paths(database) {
        if path.exists() {
            let file_name = path
                .file_name()
                .ok_or_else(|| anyhow!("Database path has no file name"))?;
            fs::copy(&path, destination.join(file_name))?;
        }
    }
    Ok(())
}

pub(crate) fn database_paths(database: &Path) -> Vec<PathBuf> {
    let raw = database.to_string_lossy();
    vec![
        database.to_path_buf(),
        PathBuf::from(format!("{raw}-wal")),
        PathBuf::from(format!("{raw}-shm")),
    ]
}

#[cfg(unix)]
pub(crate) fn copy_permissions(source: &Path, target: &Path) -> Result<()> {
    fs::set_permissions(target, fs::metadata(source)?.permissions())?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn copy_permissions(_source: &Path, _target: &Path) -> Result<()> {
    Ok(())
}
