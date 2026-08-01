use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use std::fs;
use std::path::{Path, PathBuf};

const WEB_CURRENT_LINK: &str = "web/current";

fn system_path(relative: &str) -> PathBuf {
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

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir().map_err(AppError::Io)?.join(path))
    }
}

fn development_fallback_root() -> Result<PathBuf> {
    let root = system_path("web/development-fallback");
    fs::create_dir_all(&root).map_err(AppError::Io)?;
    fs::write(
        root.join("index.html"),
        b"<!doctype html><meta charset=\"utf-8\"><title>NiuPanel Web</title><p>The bundled Web UI is not available.</p><p><a href=\"/recovery\">Open recovery console</a></p>",
    )
    .map_err(AppError::Io)?;
    Ok(root)
}

#[cfg(unix)]
fn switch_current_link(target: &Path) -> Result<()> {
    use std::os::unix::fs::symlink;

    let current = web_root();
    let parent = current
        .parent()
        .ok_or_else(|| AppError::Internal("Invalid Web current path".to_string()))?;
    fs::create_dir_all(parent).map_err(AppError::Io)?;
    if fs::read_link(&current).ok().as_deref() == Some(target) {
        return Ok(());
    }
    let temporary = parent.join(format!(".current-{}", nanoid::nanoid!(10)));
    symlink(target, &temporary).map_err(AppError::Io)?;
    if let Ok(metadata) = current.symlink_metadata() {
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&current).map_err(AppError::Io)?;
        }
    }
    fs::rename(&temporary, &current).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        AppError::Io(error)
    })
}

#[cfg(not(unix))]
fn switch_current_link(_target: &Path) -> Result<()> {
    Err(AppError::Internal(
        "Atomic Web activation requires Unix symlinks".to_string(),
    ))
}

pub fn ensure_web_runtime() -> Result<()> {
    let configured = std::env::var_os("NIUPANEL_ACTIVE_WEB_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| Config::global().bundled_web_dir.clone());
    let target = absolute_path(&configured)?;
    let target = if target.join("index.html").is_file() {
        target
    } else if Config::global().allow_missing_bundled_web {
        development_fallback_root()?
    } else {
        return Err(AppError::NotFound(format!(
            "Web directory does not contain index.html: {}",
            target.display()
        )));
    };
    switch_current_link(&target)
}
