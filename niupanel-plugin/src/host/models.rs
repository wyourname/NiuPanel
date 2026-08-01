use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;

use crate::{
    PluginThemeManifest, PluginUiApiManifest, PluginUiDisplayManifest, PluginUiMode, PluginUiRoute,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginInstallRequest {
    pub source_path: String,
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginUpdateRequest {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginImpactPreview {
    pub operation: String,
    pub plugin_id: String,
    pub name: String,
    pub current_version: Option<String>,
    pub target_version: String,
    pub ui_enabled: bool,
    pub theme_enabled: bool,
    pub routes: Vec<PluginImpactRoute>,
    pub route_conflicts: Vec<PluginRouteConflict>,
    pub permissions_added: Vec<String>,
    pub permissions_removed: Vec<String>,
    pub api_allow_added: Vec<String>,
    pub api_allow_removed: Vec<String>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
    pub install_allowed: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginImpactRoute {
    pub path: String,
    pub title: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginRouteConflict {
    pub path: String,
    pub plugin_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginHealthReport {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub healthy: bool,
    pub summary: String,
    pub checks: Vec<PluginHealthCheck>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginHealthCheck {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginMarketQuery {
    pub index_url: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginMarketInstallRequest {
    pub index_url: String,
    pub plugin_id: String,
    pub enable: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginMarketSourcesUpdateRequest {
    pub sources: Vec<PluginMarketSource>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PluginApiProxyRequest {
    pub method: Option<String>,
    pub path: String,
    pub data: Option<Value>,
    pub params: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PluginMarketSource {
    pub name: String,
    pub url: String,
    #[serde(default = "default_market_source_enabled")]
    pub enabled: bool,
}

fn default_market_source_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginMarketUpdateRecord {
    pub source_name: String,
    pub source_url: String,
    pub plugin_id: String,
    pub installed_version: String,
    pub available_version: String,
    pub entry: PluginMarketEntry,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PluginMarketIndex {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub signing: Option<PluginMarketSigning>,
    #[serde(default)]
    pub plugins: Vec<PluginMarketEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PluginMarketSigning {
    #[serde(default = "default_plugin_market_signing_algorithm")]
    pub algorithm: String,
    #[serde(default)]
    pub trusted_key: Option<String>,
    #[serde(default)]
    pub public_key_ed25519: Option<String>,
}

fn default_plugin_market_signing_algorithm() -> String {
    "ed25519".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PluginMarketEntry {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub assets: Vec<PluginMarketAsset>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PluginMarketAsset {
    pub platform: String,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub libc: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    pub file: String,
    #[serde(default)]
    pub checksum_sha256: Option<String>,
    #[serde(default)]
    pub signature_ed25519: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginAppRecord {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub ui: PluginAppUi,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginAppUi {
    pub mode: PluginUiMode,
    pub entry_url: String,
    pub sdk_version: Option<String>,
    pub display: PluginUiDisplayManifest,
    pub routes: Vec<PluginUiRoute>,
    pub permissions: Vec<String>,
    pub api: PluginUiApiManifest,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PluginThemeRecord {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub theme: PluginThemeManifest,
}
