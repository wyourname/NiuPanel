use flate2::read::GzDecoder;
use niupanel_common::error::{AppError, Result};
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveKind {
    Gzip,
    Tar,
    TarGzip,
    Zip,
}

pub(super) fn extract_archive(source: &Path, destination: &Path) -> Result<usize> {
    if !source.exists() {
        return Err(AppError::NotFound("Archive file not found".to_string()));
    }
    if !source.is_file() {
        return Err(AppError::Generic("Archive path is not a file".to_string()));
    }

    fs::create_dir_all(destination).map_err(AppError::Io)?;
    let destination = destination.canonicalize().map_err(AppError::Io)?;

    match archive_kind(source)? {
        ArchiveKind::Gzip => extract_gzip(source, &destination),
        ArchiveKind::Tar => extract_tar(File::open(source).map_err(AppError::Io)?, &destination),
        ArchiveKind::TarGzip => {
            let file = File::open(source).map_err(AppError::Io)?;
            extract_tar(GzDecoder::new(file), &destination)
        }
        ArchiveKind::Zip => extract_zip(source, &destination),
    }
}

fn archive_kind(source: &Path) -> Result<ArchiveKind> {
    let name = source
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_lowercase();

    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Ok(ArchiveKind::TarGzip);
    }
    if name.ends_with(".tar") {
        return Ok(ArchiveKind::Tar);
    }
    if name.ends_with(".zip") {
        return Ok(ArchiveKind::Zip);
    }
    if name.ends_with(".gz") {
        return Ok(ArchiveKind::Gzip);
    }

    Err(AppError::Generic(
        "Unsupported archive format. Supported: .zip, .tar, .tar.gz, .tgz, .gz".to_string(),
    ))
}

fn extract_tar<R>(reader: R, destination: &Path) -> Result<usize>
where
    R: io::Read,
{
    let mut archive = tar::Archive::new(reader);
    let mut extracted = 0;

    for entry_result in archive
        .entries()
        .map_err(|error| AppError::Generic(format!("Failed to read archive entries: {}", error)))?
    {
        let mut entry = entry_result
            .map_err(|error| AppError::Generic(format!("Invalid archive entry: {}", error)))?;
        let entry_path = entry
            .path()
            .map_err(|error| AppError::Generic(format!("Invalid archive path: {}", error)))?;
        let entry_path = entry_path.into_owned();

        ensure_safe_tar_entry(entry.header().entry_type(), &entry_path)?;
        let output_path = safe_output_path(destination, &entry_path)?;

        prepare_output_target(destination, &output_path)?;

        entry.unpack(&output_path).map_err(|error| {
            AppError::Generic(format!("Failed to extract archive entry: {}", error))
        })?;
        extracted += 1;
    }

    Ok(extracted)
}

fn extract_zip(source: &Path, destination: &Path) -> Result<usize> {
    let file = File::open(source).map_err(AppError::Io)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| AppError::Generic(error.to_string()))?;
    let mut extracted = 0;

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| AppError::Generic(error.to_string()))?;
        let enclosed_name = file.enclosed_name().ok_or_else(|| {
            AppError::Generic(format!("Unsafe archive entry path: {}", file.name()))
        })?;
        if file.is_symlink() {
            return Err(AppError::Generic(format!(
                "Archive symlink entries are not supported: {}",
                file.name()
            )));
        }
        let output_path = safe_output_path(destination, &enclosed_name)?;

        if file.is_dir() {
            prepare_output_target(destination, &output_path)?;
            fs::create_dir_all(&output_path).map_err(AppError::Io)?;
        } else {
            prepare_output_target(destination, &output_path)?;
            let mut output_file = File::create(&output_path).map_err(AppError::Io)?;
            io::copy(&mut file, &mut output_file).map_err(AppError::Io)?;
        }
        extracted += 1;
    }

    Ok(extracted)
}

fn extract_gzip(source: &Path, destination: &Path) -> Result<usize> {
    let file = File::open(source).map_err(AppError::Io)?;
    let mut decoder = GzDecoder::new(file);
    let output_name = gzip_output_name(source)?;
    let output_path = safe_output_path(destination, Path::new(&output_name))?;

    prepare_output_target(destination, &output_path)?;
    let mut output_file = File::create(output_path).map_err(AppError::Io)?;
    io::copy(&mut decoder, &mut output_file).map_err(AppError::Io)?;
    Ok(1)
}

fn gzip_output_name(source: &Path) -> Result<String> {
    let file_name = source
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| AppError::Generic("Invalid gzip filename".to_string()))?;

    Ok(file_name
        .strip_suffix(".gz")
        .filter(|name| !name.is_empty())
        .unwrap_or("decompressed")
        .to_string())
}

fn ensure_safe_tar_entry(entry_type: tar::EntryType, entry_path: &Path) -> Result<()> {
    if entry_type.is_symlink() || entry_type.is_hard_link() {
        return Err(AppError::Generic(format!(
            "Archive link entries are not supported: {}",
            entry_path.display()
        )));
    }

    if entry_type.is_file() || entry_type.is_dir() || entry_type.is_contiguous() {
        return Ok(());
    }

    Err(AppError::Generic(format!(
        "Unsupported tar entry type for {}",
        entry_path.display()
    )))
}

fn safe_output_path(destination: &Path, entry_path: &Path) -> Result<PathBuf> {
    let mut clean_path = PathBuf::new();
    for component in entry_path.components() {
        match component {
            Component::Normal(part) => clean_path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::Generic(format!(
                    "Unsafe archive entry path: {}",
                    entry_path.display()
                )));
            }
        }
    }

    if clean_path.as_os_str().is_empty() {
        return Err(AppError::Generic("Archive entry path is empty".to_string()));
    }

    let output_path = destination.join(clean_path);
    if !output_path.starts_with(destination) {
        return Err(AppError::Generic(format!(
            "Archive entry escapes target directory: {}",
            entry_path.display()
        )));
    }

    Ok(output_path)
}

fn prepare_output_target(destination: &Path, output_path: &Path) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(output_path) {
        if metadata.file_type().is_symlink() {
            return Err(AppError::Generic(format!(
                "Refusing to overwrite symlink during extraction: {}",
                output_path.display()
            )));
        }
    }

    if let Some(parent) = output_path.parent() {
        prepare_parent_dir(destination, parent)?;
    }

    Ok(())
}

fn prepare_parent_dir(destination: &Path, parent: &Path) -> Result<()> {
    let relative_parent = parent.strip_prefix(destination).map_err(|_| {
        AppError::Generic(format!(
            "Archive entry parent escapes target directory: {}",
            parent.display()
        ))
    })?;

    let mut current = destination.to_path_buf();
    for component in relative_parent.components() {
        match component {
            Component::Normal(part) => {
                current.push(part);
                ensure_directory_component(destination, &current)?;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(AppError::Generic(format!(
                    "Unsafe archive entry parent path: {}",
                    parent.display()
                )));
            }
        }
    }

    Ok(())
}

fn ensure_directory_component(destination: &Path, path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(AppError::Generic(format!(
                    "Refusing to follow symlink during extraction: {}",
                    path.display()
                )));
            }
            if !metadata.is_dir() {
                return Err(AppError::Generic(format!(
                    "Archive entry parent is not a directory: {}",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(AppError::Io)?;
        }
        Err(error) => return Err(AppError::Io(error)),
    }

    let canonical = path.canonicalize().map_err(AppError::Io)?;
    if !canonical.starts_with(destination) {
        return Err(AppError::Generic(format!(
            "Archive entry parent escapes target directory: {}",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn archive_kind_detects_common_formats() {
        assert_eq!(archive_kind(Path::new("a.zip")).unwrap(), ArchiveKind::Zip);
        assert_eq!(archive_kind(Path::new("a.tar")).unwrap(), ArchiveKind::Tar);
        assert_eq!(
            archive_kind(Path::new("a.tar.gz")).unwrap(),
            ArchiveKind::TarGzip
        );
        assert_eq!(
            archive_kind(Path::new("a.tgz")).unwrap(),
            ArchiveKind::TarGzip
        );
        assert_eq!(archive_kind(Path::new("a.gz")).unwrap(), ArchiveKind::Gzip);
    }

    #[test]
    fn safe_output_path_rejects_traversal() {
        let destination = Path::new("/tmp/extract");

        assert!(safe_output_path(destination, Path::new("../x")).is_err());
        assert!(safe_output_path(destination, Path::new("/x")).is_err());
    }

    #[test]
    fn tar_link_entries_are_rejected() {
        assert!(ensure_safe_tar_entry(tar::EntryType::symlink(), Path::new("link")).is_err());
        assert!(ensure_safe_tar_entry(tar::EntryType::hard_link(), Path::new("link")).is_err());
        assert!(ensure_safe_tar_entry(tar::EntryType::file(), Path::new("file")).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn prepare_output_target_rejects_existing_symlink() {
        let dir = tempdir().unwrap();
        let destination = dir.path().canonicalize().unwrap();
        let target = destination.join("target");
        fs::write(&target, "x").unwrap();

        let link = destination.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(prepare_output_target(&destination, &link).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn prepare_output_target_rejects_parent_symlink_without_creating_child() {
        let dir = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let destination = dir.path().canonicalize().unwrap();

        let link = destination.join("link");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();

        let output_path = link.join("created").join("file.txt");

        assert!(prepare_output_target(&destination, &output_path).is_err());
        assert!(!outside.path().join("created").exists());
    }
}
