use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use utoipa::ToSchema;

pub const API_CONTRACT_VERSION: u32 = 1;
pub const SCHEMA_EPOCH: u32 = 1;
pub const SCHEMA_REVISION: u32 = 30;
pub const WEB_RELEASE_MANIFEST_FILE: &str = "release-manifest.json";
pub const CORE_RELEASE_MANIFEST_FILE: &str = "core-release.json";
pub const UPDATE_CHANNEL_INDEX_SCHEMA_VERSION: u32 = 2;
pub const MINIMUM_UPDATE_CORE_VERSION: &str = "0.8.0";
pub const MINIMUM_WEB_CORE_VERSION: &str = "0.8.4";
pub const RELEASE_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ComponentCompatibility {
    pub min: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WebReleaseManifest {
    pub component: String,
    pub version: String,
    pub api_contract: u32,
    pub core: ComponentCompatibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CoreReleaseManifest {
    pub component: String,
    pub version: String,
    pub launcher_protocol: u32,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub target: String,
    pub binary_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<String>,
}

/// The remote release contract. Panel releases are versioned independently so
/// either component can advance without rebuilding the other one.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateChannelIndex {
    pub schema_version: u32,
    pub channel: String,
    pub updated_at: String,
    pub release: PanelUpdateRelease,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PanelUpdateRelease {
    pub version: String,
    pub tag: String,
    pub release_url: String,
    #[serde(default)]
    pub notes: String,
    pub launcher_protocol: u32,
    pub core: UpdateCoreComponent,
    pub web: UpdateWebComponent,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateAsset {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCoreAsset {
    pub name: String,
    pub url: String,
    pub target: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateCoreComponent {
    pub version: String,
    pub tag: String,
    pub release_url: String,
    #[serde(default)]
    pub notes: String,
    pub launcher_protocol: u32,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub assets: BTreeMap<String, UpdateCoreAsset>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateWebComponent {
    pub version: String,
    pub tag: String,
    pub release_url: String,
    #[serde(default)]
    pub notes: String,
    pub api_contract: u32,
    pub core: ComponentCompatibility,
    pub asset: UpdateAsset,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub struct CoreReleaseDescriptor {
    pub version: String,
    pub path: String,
    pub launcher_protocol: u32,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub target: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PanelActivationReason {
    Update,
    Rollback,
    Recovery,
}

pub fn read_web_release_manifest(web_root: impl AsRef<Path>) -> Option<WebReleaseManifest> {
    let path = web_root.as_ref().join(WEB_RELEASE_MANIFEST_FILE);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn read_core_release_manifest(core_root: impl AsRef<Path>) -> Option<CoreReleaseManifest> {
    let path = core_root.as_ref().join(CORE_RELEASE_MANIFEST_FILE);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
