use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};

fn is_usable_file(path: &Path) -> bool {
    path.is_file()
}

fn env_tool_path(tool_name: &str) -> Option<PathBuf> {
    let key = format!("NIUPANEL_TOOL_{}_PATH", tool_name.to_ascii_uppercase());
    std::env::var_os(key)
        .map(PathBuf::from)
        .filter(|p| is_usable_file(p))
}

fn env_tools_dir() -> Option<PathBuf> {
    std::env::var_os("NIUPANEL_TOOLS_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

fn executable_tools_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("tools")))
        .filter(|p| p.is_dir())
}

fn cwd_tools_dir() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|dir| dir.join("tools"))
        .filter(|p| p.is_dir())
}

fn data_tools_dir() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|dir| dir.join("data").join("tools"))
        .filter(|p| p.is_dir())
}

fn fnm_fallback_path() -> Option<PathBuf> {
    if let Ok(fnm_dir) = std::env::var("FNM_DIR") {
        let path = PathBuf::from(fnm_dir).join("fnm");
        if is_usable_file(&path) {
            return Some(path);
        }
    }

    home::home_dir()
        .map(|home| home.join(".local").join("share").join("fnm").join("fnm"))
        .filter(|p| is_usable_file(p))
}

pub fn resolve_tool_path(tool_name: &str) -> Option<PathBuf> {
    if let Some(path) = env_tool_path(tool_name) {
        return Some(path);
    }

    for dir in [
        env_tools_dir(),
        executable_tools_dir(),
        cwd_tools_dir(),
        data_tools_dir(),
    ]
    .into_iter()
    .flatten()
    {
        let path = dir.join(tool_name);
        if is_usable_file(&path) {
            return Some(path);
        }
    }

    if tool_name == "fnm" {
        if let Some(path) = fnm_fallback_path() {
            return Some(path);
        }
    }

    which::which(tool_name).ok()
}

pub fn require_tool_path(tool_name: &str, purpose: &str) -> Result<PathBuf> {
    resolve_tool_path(tool_name).ok_or_else(|| {
        AppError::Environment(format!(
            "依赖 '{}' 不存在，无法用于{}。请确认 NiuPanel 发布包中的 tools/ 目录完整，或通过 PATH / NIUPANEL_TOOLS_DIR 提供该工具。",
            tool_name, purpose
        ))
    })
}
