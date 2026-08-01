use chrono::Utc;
use niupanel_common::auth::permissions::Permission;
use niupanel_common::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::{Duration, timeout};
use utoipa::ToSchema;

pub mod host;

pub const PROCESS_PROTOCOL_VERSION: u32 = 1;
const MANIFEST_FILE: &str = "plugin.json";
const STATE_FILE: &str = ".state.json";
const DEFAULT_PROCESS_TIMEOUT_SEC: u64 = 30;
const MAX_PROCESS_TIMEOUT_SEC: u64 = 180;
const MAX_PROCESS_OUTPUT_BYTES: usize = 1024 * 1024;
#[cfg(target_os = "linux")]
const PLUGIN_SANDBOX_UID: u32 = 65534;
#[cfg(target_os = "linux")]
const PLUGIN_SANDBOX_GID: u32 = 65534;
#[cfg(target_os = "linux")]
const PLUGIN_WEB_PORTS: [u16; 2] = [80, 443];

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginSandboxMode {
    Full,
    Compatible,
    Degraded,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginSandboxCapability {
    pub mode: PluginSandboxMode,
    pub landlock_abi: Option<i32>,
    pub uid_isolation: bool,
    pub seccomp: bool,
    pub no_new_privs: bool,
}

#[cfg(target_os = "linux")]
pub fn plugin_sandbox_capability() -> PluginSandboxCapability {
    const LANDLOCK_CREATE_RULESET_VERSION: libc::c_uint = 1;
    let abi = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<libc::c_void>(),
            0,
            LANDLOCK_CREATE_RULESET_VERSION,
        )
    } as i32;
    let landlock_abi = (abi > 0).then_some(abi);
    let uid_isolation = unsafe { libc::geteuid() } == 0;
    let mode = match (landlock_abi.is_some(), uid_isolation) {
        (true, true) => PluginSandboxMode::Full,
        (true, false) => PluginSandboxMode::Compatible,
        (false, _) => PluginSandboxMode::Degraded,
    };
    PluginSandboxCapability {
        mode,
        landlock_abi,
        uid_isolation,
        seccomp: true,
        no_new_privs: true,
    }
}

#[cfg(not(target_os = "linux"))]
pub fn plugin_sandbox_capability() -> PluginSandboxCapability {
    PluginSandboxCapability {
        mode: PluginSandboxMode::Unsupported,
        landlock_abi: None,
        uid_isolation: false,
        seccomp: false,
        no_new_privs: false,
    }
}

mod manifest;
mod process_invoke;
mod process_pool;
mod sandbox;
mod service;
mod service_storage;
mod support;
mod validation;

pub use manifest::*;
pub use process_pool::PluginProcessRuntime;
pub use service::PluginService;
pub use validation::*;

use process_invoke::*;
use process_pool::*;
use sandbox::*;
use support::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn landlock_rules_are_compatible_with_file_and_directory_targets() {
        if plugin_sandbox_capability().landlock_abi.is_none() {
            return;
        }
        let plugin = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        create_landlock_ruleset(plugin.path(), data.path(), true).unwrap();
    }

    #[test]
    fn ui_manifest_accepts_structured_display_and_api_allow() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entry = dir.path().join("ui.js");
        fs::write(&entry, "export default {}").expect("write ui");

        let ui = PluginUiManifest {
            enabled: true,
            mode: PluginUiMode::VueApp,
            entry: "ui.js".to_string(),
            sdk_version: Some("^0.1.0".to_string()),
            display: PluginUiDisplayManifest {
                sidebar: true,
                workspace: true,
                mobile: true,
                category: Some("plugins".to_string()),
                order: 10,
                layout: PluginUiLayout::Panel,
            },
            routes: vec![PluginUiRoute {
                id: Some("home".to_string()),
                path: "/plugins/example".to_string(),
                title: "Example".to_string(),
                description: Some("Example route".to_string()),
                icon: Some("i-ep-box".to_string()),
                menu: Some("main".to_string()),
                order: 0,
                hidden: false,
                exact: false,
            }],
            permissions: vec!["task:read".to_string()],
            api: PluginUiApiManifest {
                allow: vec![PluginUiApiRule {
                    methods: vec!["GET".to_string()],
                    path: "/tasks/**".to_string(),
                }],
            },
        };

        validate_plugin_ui_manifest(&ui, dir.path()).expect("valid ui manifest");
    }

    #[test]
    fn ui_manifest_rejects_invalid_api_allow_method() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entry = dir.path().join("ui.js");
        fs::write(&entry, "export default {}").expect("write ui");

        let ui = PluginUiManifest {
            enabled: true,
            mode: PluginUiMode::VueApp,
            entry: "ui.js".to_string(),
            sdk_version: None,
            display: PluginUiDisplayManifest::default(),
            routes: vec![PluginUiRoute {
                id: None,
                path: "/plugins/example".to_string(),
                title: "Example".to_string(),
                description: None,
                icon: None,
                menu: None,
                order: 0,
                hidden: false,
                exact: false,
            }],
            permissions: vec![],
            api: PluginUiApiManifest {
                allow: vec![PluginUiApiRule {
                    methods: vec!["TRACE".to_string()],
                    path: "/tasks/**".to_string(),
                }],
            },
        };

        assert!(validate_plugin_ui_manifest(&ui, dir.path()).is_err());
    }

    #[test]
    fn ui_manifest_rejects_wildcard_permissions_and_implicit_methods() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("ui.js"), "export default {}").expect("write ui");
        let mut ui = PluginUiManifest {
            enabled: true,
            mode: PluginUiMode::VueApp,
            entry: "ui.js".to_string(),
            sdk_version: None,
            display: PluginUiDisplayManifest::default(),
            routes: vec![PluginUiRoute {
                id: None,
                path: "/plugins/example".to_string(),
                title: "Example".to_string(),
                description: None,
                icon: None,
                menu: None,
                order: 0,
                hidden: false,
                exact: false,
            }],
            permissions: vec!["task:*".to_string()],
            api: PluginUiApiManifest::default(),
        };

        assert!(validate_plugin_ui_manifest(&ui, dir.path()).is_err());
        ui.permissions = vec!["task:read".to_string()];
        ui.api.allow = vec![PluginUiApiRule {
            methods: vec![],
            path: "/tasks/**".to_string(),
        }];
        assert!(validate_plugin_ui_manifest(&ui, dir.path()).is_err());
    }

    #[test]
    fn declarative_theme_accepts_hex_tokens_and_rejects_css_payloads() {
        let mut theme = PluginThemeManifest {
            enabled: true,
            light: PluginThemePalette {
                primary: Some("#0F766E".to_string()),
                ..PluginThemePalette::default()
            },
            dark: PluginThemePalette::default(),
        };
        validate_plugin_theme_manifest(&theme).expect("valid theme");

        theme.light.primary = Some("url(https://example.com/track)".to_string());
        assert!(validate_plugin_theme_manifest(&theme).is_err());
    }

    #[test]
    fn capabilities_validate_and_match_wildcards() {
        validate_plugin_capabilities(&[
            "compiler.versions".to_string(),
            "compiler.encrypt".to_string(),
            "task.read-only".to_string(),
        ])
        .expect("valid capabilities");
        assert!(plugin_has_capability(
            &["compiler.*".to_string()],
            "compiler.encrypt"
        ));
        assert!(!plugin_has_capability(
            &["compiler.versions".to_string()],
            "compiler.encrypt"
        ));
    }

    #[test]
    fn capabilities_reject_invalid_names() {
        assert!(validate_plugin_capabilities(&["Compiler.encrypt".to_string()]).is_err());
        assert!(validate_plugin_capabilities(&["compiler".to_string()]).is_err());
        assert!(validate_plugin_capabilities(&["compiler.*.encrypt".to_string()]).is_err());
        assert!(
            validate_plugin_capabilities(&[
                "compiler.encrypt".to_string(),
                "compiler.encrypt".to_string()
            ])
            .is_err()
        );
    }
}
