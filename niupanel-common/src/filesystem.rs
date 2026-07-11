use crate::config::Config;
use crate::error::{AppError, Result};
use std::path::{PathBuf, StripPrefixError};

pub fn resolve_path(relative_path: &str) -> Result<PathBuf> {
    let base = PathBuf::from(&Config::global().scripts_dir);
    if !base.exists() {
        std::fs::create_dir_all(&base).map_err(AppError::Io)?;
    }

    let full_path = base.join(relative_path);
    let canonical_base_path = base.canonicalize().map_err(AppError::Io)?;

    let canonical_full_path = match full_path.canonicalize() {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = full_path.parent().ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid path",
                ))
            })?;

            let canonical_parent = parent.canonicalize().map_err(AppError::Io)?;
            if !canonical_parent.starts_with(&canonical_base_path) {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Access to path outside base directory is denied",
                )));
            }

            let file_name = full_path.file_name().ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Invalid filename",
                ))
            })?;

            return Ok(canonical_parent.join(file_name));
        }
        Err(e) => return Err(AppError::Io(e)),
    };

    if !canonical_full_path.starts_with(&canonical_base_path) {
        return Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Access to path outside base directory is denied",
        )));
    }

    Ok(canonical_full_path)
}

pub fn get_relative_path(full_path: &PathBuf) -> Result<String> {
    let base = PathBuf::from(&Config::global().scripts_dir);

    // Ensure base directory exists
    if !base.exists() {
        std::fs::create_dir_all(&base).map_err(AppError::Io)?;
    }

    // Try stripping the relative base first
    if let Ok(stripped) = full_path.strip_prefix(&base) {
        return stripped
            .to_str()
            .map(|s| s.to_string())
            .ok_or(AppError::Generic("Invalid path encoding".to_string()));
    }

    // If that fails, try stripping the canonical (absolute) base
    let canonical_base = base.canonicalize().map_err(AppError::Io)?;
    full_path
        .strip_prefix(&canonical_base)
        .map_err(|e: StripPrefixError| {
            AppError::Generic(format!("Path is not relative to base: {}", e))
        })?
        .to_str()
        .map(|s| s.to_string())
        .ok_or(AppError::Generic("Invalid path encoding".to_string()))
}
