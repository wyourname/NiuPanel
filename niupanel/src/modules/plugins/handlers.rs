use super::lifecycle::{self, ManifestCompatibility};
use super::models::{
    PluginApiProxyRequest, PluginAppRecord, PluginAppUi, PluginHealthCheck, PluginHealthReport,
    PluginImpactPreview, PluginImpactRoute, PluginInstallRequest, PluginMarketAsset,
    PluginMarketEntry, PluginMarketIndex, PluginMarketInstallRequest, PluginMarketQuery,
    PluginMarketSource, PluginMarketSourcesUpdateRequest, PluginMarketUpdateRecord,
    PluginRouteConflict, PluginThemeRecord, PluginUpdateRequest,
};
use super::service::{PLUGIN_CONTEXT, plugin_service, plugin_services, unified_plugin_service};
use crate::common::state::AppState;
use crate::modules::auth::service::AuthenticatedUser;
use axum::{
    Extension, Json,
    extract::{Multipart, Path as AxumPath, Query, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use niupanel_common::auth::permissions::Permission;
use niupanel_common::auth::sdk_token;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::response::ApiResponse;
use niupanel_core::audit::service::AuditService;
use niupanel_plugin::{
    PluginCompatibilityManifest, PluginDependency, PluginInvokeRequest, PluginManifest,
    PluginRecord, PluginRuntimePermission, PluginStatus, PluginUiApiRule, PluginUiManifest,
    PluginVersionRecord, plugin_has_capability, validate_plugin_capabilities,
};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::time::UNIX_EPOCH;

const MAX_PLUGIN_MARKET_INDEX_BYTES: usize = 2 * 1024 * 1024;
const PLUGIN_MARKET_SOURCES_KEY: &str = "plugins.market.sources";

mod health;
mod impact;
mod market;
mod operations;
mod ui;
mod ui_access;
mod ui_manifest;

use health::*;
use impact::*;
pub use market::*;
pub use operations::*;
pub use ui::*;
use ui_access::*;
use ui_manifest::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_versions_are_compared_by_segment() {
        assert!(version_is_newer("1.10.0", "1.2.0"));
        assert!(version_is_newer("v2.0.0", "1.9.9"));
        assert!(!version_is_newer("1.0.0", "1.0"));
        assert!(!version_is_newer("1.2.0", "1.10.0"));
    }

    #[test]
    fn non_numeric_versions_fall_back_to_difference() {
        assert!(version_is_newer("1.0.0-beta.2", "1.0.0-beta.1"));
        assert!(!version_is_newer("1.0.0-beta.1", "1.0.0-beta.1"));
    }

    #[test]
    fn market_sources_are_normalized_and_deduplicated() {
        let sources = normalize_market_sources(vec![
            PluginMarketSource {
                name: " Internal ".to_string(),
                url: "https://example.com/plugins/index.json".to_string(),
                enabled: true,
            },
            PluginMarketSource {
                name: "Duplicate".to_string(),
                url: "https://example.com/plugins/index.json".to_string(),
                enabled: false,
            },
            PluginMarketSource {
                name: "".to_string(),
                url: "https://plugins.example.com/".to_string(),
                enabled: true,
            },
        ])
        .expect("sources normalize");

        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|source| source.name == "Internal"));
        assert!(
            sources
                .iter()
                .any(|source| source.name == "plugins.example.com")
        );
    }

    #[test]
    fn api_allow_rules_match_methods_and_prefixes() {
        assert!(api_rule_matches(
            &["GET".to_string()],
            "/tasks/**",
            &Method::GET,
            "/tasks/1/logs"
        ));
        assert!(api_rule_matches(
            &["POST".to_string()],
            "/compiler/*",
            &Method::POST,
            "/compiler/encrypt"
        ));
        assert!(!api_rule_matches(
            &Vec::<String>::new(),
            "/compiler/*",
            &Method::POST,
            "/compiler/encrypt"
        ));
        assert!(!api_rule_matches(
            &["POST".to_string()],
            "/tasks/**",
            &Method::GET,
            "/tasks/1"
        ));
        assert!(!api_rule_matches(
            &["GET".to_string()],
            "/tasks/**",
            &Method::GET,
            "/variables"
        ));
    }

    #[test]
    fn plugin_api_never_proxies_host_files_or_settings() {
        assert!(permission_for_plugin_api_request(&Method::GET, "/files/file").is_err());
        assert!(permission_for_plugin_api_request(&Method::GET, "/settings").is_err());
        assert!(permission_for_plugin_api_request(&Method::POST, "/plugins/install").is_err());
    }

    #[test]
    fn plugin_variable_value_routes_require_read_permission() {
        assert_eq!(
            permission_for_plugin_api_request(&Method::GET, "/variables")
                .expect("metadata route is allowed"),
            Some(Permission::VarList)
        );
        assert_eq!(
            permission_for_plugin_api_request(&Method::GET, "/variables/with-values")
                .expect("value list route is allowed"),
            Some(Permission::VarRead)
        );
        assert_eq!(
            permission_for_plugin_api_request(&Method::GET, "/variables/42/value")
                .expect("single value route is allowed"),
            Some(Permission::VarRead)
        );
        assert_eq!(
            permission_for_plugin_api_request(&Method::POST, "/variables/batch-save")
                .expect("batch save route is allowed"),
            Some(Permission::VarUpdate)
        );
    }

    #[test]
    fn plugin_app_entry_url_includes_cache_key() {
        let ui = PluginUiManifest {
            enabled: true,
            mode: niupanel_plugin::PluginUiMode::VueApp,
            entry: "ui/dist/niupanel-plugin.js".to_string(),
            sdk_version: None,
            display: niupanel_plugin::PluginUiDisplayManifest::default(),
            routes: Vec::new(),
            permissions: Vec::new(),
            api: niupanel_plugin::PluginUiApiManifest::default(),
        };

        let app_ui = app_ui("example-plugin", ui, "0.1.0+build/1");

        assert_eq!(
            app_ui.entry_url,
            "/api/v1/plugins/example-plugin/ui/niupanel-plugin.js?v=0.1.0_build_1"
        );
    }
}
