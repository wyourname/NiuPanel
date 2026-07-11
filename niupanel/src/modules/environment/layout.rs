use niupanel_common::config::Config;
use std::path::PathBuf;

const PYTHON_ENV_PREFIX: &str = "venv_";
const NODE_ACTIVE_DEFAULT_PATH_LABEL: &str = "System Default";
const NODE_INSTALLED_PATH_LABEL: &str = "Installed";
const PYTHON_NOT_INSTALLED_PATH_LABEL: &str = "(Not Installed)";
const SHELL_ENV_NAME: &str = "System";
const SHELL_ENV_PATH_LABEL: &str = "system (sh)";
const SHELL_ENV_VERSION: &str = "system";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeKind {
    Python,
    Node,
    Shell,
}

impl RuntimeKind {
    pub(super) fn from_env_type(env_type: &str) -> Option<Self> {
        match env_type {
            "python" => Some(RuntimeKind::Python),
            "node" => Some(RuntimeKind::Node),
            "sh" | "shell" => Some(RuntimeKind::Shell),
            _ => None,
        }
    }

    pub(super) fn env_type(self) -> &'static str {
        match self {
            RuntimeKind::Python => "python",
            RuntimeKind::Node => "node",
            RuntimeKind::Shell => "sh",
        }
    }

    pub(super) fn runtime_dir(self) -> PathBuf {
        PathBuf::from(&Config::global().runtimes_dir).join(self.env_type())
    }

    pub(super) fn env_name(self, version: &str) -> String {
        match self {
            RuntimeKind::Python => format!("{PYTHON_ENV_PREFIX}{version}"),
            RuntimeKind::Node => version.to_string(),
            RuntimeKind::Shell => SHELL_ENV_NAME.to_string(),
        }
    }

    pub(super) fn env_path(self, name: &str) -> PathBuf {
        self.runtime_dir().join(name)
    }

    pub(super) fn is_managed_name(self, name: &str) -> bool {
        match self {
            RuntimeKind::Python => name.starts_with(PYTHON_ENV_PREFIX),
            RuntimeKind::Node => !name.trim().is_empty(),
            RuntimeKind::Shell => name == SHELL_ENV_NAME,
        }
    }

    pub(super) fn version_from_name(self, name: &str) -> Option<String> {
        match self {
            RuntimeKind::Python => name.strip_prefix(PYTHON_ENV_PREFIX).map(str::to_string),
            RuntimeKind::Node => Some(
                name.strip_prefix(PYTHON_ENV_PREFIX)
                    .unwrap_or(name)
                    .split(' ')
                    .next()
                    .unwrap_or(name)
                    .to_string(),
            ),
            RuntimeKind::Shell => Some(SHELL_ENV_VERSION.to_string()),
        }
    }

    pub(super) fn record_version(self, name: &str) -> String {
        self.version_from_name(name)
            .unwrap_or_else(|| name.to_string())
    }

    pub(super) fn missing_path_label(self) -> &'static str {
        match self {
            RuntimeKind::Python => PYTHON_NOT_INSTALLED_PATH_LABEL,
            RuntimeKind::Node => NODE_INSTALLED_PATH_LABEL,
            RuntimeKind::Shell => SHELL_ENV_PATH_LABEL,
        }
    }

    pub(super) fn node_path_label(is_default: bool) -> &'static str {
        if is_default {
            NODE_ACTIVE_DEFAULT_PATH_LABEL
        } else {
            NODE_INSTALLED_PATH_LABEL
        }
    }

    pub(super) fn shell_name() -> &'static str {
        SHELL_ENV_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_env_name_uses_runtime_prefix() {
        assert_eq!(RuntimeKind::Python.env_name("3.12.2"), "venv_3.12.2");
    }

    #[test]
    fn python_record_version_falls_back_to_name_without_prefix() {
        assert_eq!(
            RuntimeKind::Python.record_version("custom-python"),
            "custom-python"
        );
    }

    #[test]
    fn node_version_from_name_removes_legacy_prefix_and_status_suffix() {
        assert_eq!(
            RuntimeKind::Node.record_version("venv_20.11.1 default"),
            "20.11.1"
        );
        assert_eq!(
            RuntimeKind::Node.record_version("20.11.1 default"),
            "20.11.1"
        );
    }

    #[test]
    fn shell_kind_owns_system_identity() {
        assert_eq!(RuntimeKind::Shell.env_name("ignored"), "System");
        assert_eq!(RuntimeKind::Shell.env_type(), "sh");
        assert_eq!(RuntimeKind::Shell.record_version("ignored"), "system");
    }

    #[test]
    fn runtime_kind_parses_external_env_type() {
        assert_eq!(
            RuntimeKind::from_env_type("python"),
            Some(RuntimeKind::Python)
        );
        assert_eq!(RuntimeKind::from_env_type("node"), Some(RuntimeKind::Node));
        assert_eq!(
            RuntimeKind::from_env_type("shell"),
            Some(RuntimeKind::Shell)
        );
        assert_eq!(RuntimeKind::from_env_type("unknown"), None);
    }
}
