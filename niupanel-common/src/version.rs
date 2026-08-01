use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use utoipa::ToSchema;

pub const API_CONTRACT_VERSION: u32 = 1;
pub const SCHEMA_EPOCH: u32 = 1;
pub const SCHEMA_REVISION: u32 = 30;
pub const WEB_RELEASE_MANIFEST_FILE: &str = "release-manifest.json";
pub const CORE_RELEASE_MANIFEST_FILE: &str = "core-release.json";
pub const UPDATE_CHANNEL_INDEX_SCHEMA_VERSION: u32 = 1;
pub const MINIMUM_UPDATE_CORE_VERSION: &str = "0.8.0";
pub const CORE_STATE_FILE: &str = "core/state.json";
pub const CORE_TRANSACTIONS_DIR: &str = "core/transactions";
pub const RELEASE_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ComponentCompatibility {
    pub min: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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

/// The sole remote update contract used by 0.8.0 and later. The index lives
/// independently of component releases so Core and Web can advance separately.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateChannelIndex {
    pub schema_version: u32,
    pub channel: String,
    pub updated_at: String,
    pub core: UpdateCoreComponent,
    pub web: UpdateWebComponent,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateAsset {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateCoreAsset {
    pub name: String,
    pub url: String,
    pub target: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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
pub enum CoreActivationReason {
    Update,
    Rollback,
    Recovery,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct PendingCoreActivation {
    pub transaction_id: String,
    pub candidate: CoreReleaseDescriptor,
    pub reason: CoreActivationReason,
    pub requested_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_before_start: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CoreActivationFailure {
    pub transaction_id: String,
    pub version: String,
    pub failed_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CoreRuntimeState {
    pub protocol: u32,
    pub active: CoreReleaseDescriptor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<CoreReleaseDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<PendingCoreActivation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<CoreActivationFailure>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CoreActivationTransaction {
    pub transaction_id: String,
    pub from: CoreReleaseDescriptor,
    pub to: CoreReleaseDescriptor,
    pub reason: CoreActivationReason,
    pub snapshot_path: String,
    pub requested_at: String,
    pub completed_at: String,
    pub successful: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

pub fn validate_web_compatibility(
    core_version: &str,
    manifest: &WebReleaseManifest,
) -> Result<(), String> {
    if manifest.component != "web" {
        return Err("Web manifest component must be 'web'".to_string());
    }
    if manifest.api_contract != API_CONTRACT_VERSION {
        return Err(format!(
            "API contract mismatch: core={}, web={}",
            API_CONTRACT_VERSION, manifest.api_contract
        ));
    }

    let core = Version::parse(core_version).map_err(|error| error.to_string())?;
    let min = Version::parse(&manifest.core.min).map_err(|error| error.to_string())?;
    if core < min {
        return Err(format!(
            "Core {core_version} is older than required {}",
            manifest.core.min
        ));
    }
    if let Some(max) = manifest.core.max.as_deref() {
        let max_version = Version::parse(max).map_err(|error| error.to_string())?;
        if core > max_version {
            return Err(format!("Core {core_version} is newer than supported {max}"));
        }
    }
    Ok(())
}

pub fn read_core_runtime_state(system_root: impl AsRef<Path>) -> Result<CoreRuntimeState, String> {
    let path = system_root.as_ref().join(CORE_STATE_FILE);
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

pub fn write_core_runtime_state(
    system_root: impl AsRef<Path>,
    state: &CoreRuntimeState,
) -> Result<(), String> {
    let path = system_root.as_ref().join(CORE_STATE_FILE);
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid Core state path".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    let temp = parent.join(format!(".state-{}.tmp", std::process::id()));
    let content = serde_json::to_vec_pretty(state)
        .map_err(|error| format!("Failed to serialize Core state: {error}"))?;
    std::fs::write(&temp, content)
        .map_err(|error| format!("Failed to write {}: {error}", temp.display()))?;
    std::fs::rename(&temp, &path)
        .map_err(|error| format!("Failed to activate {}: {error}", path.display()))
}

pub fn write_core_activation_transaction(
    system_root: impl AsRef<Path>,
    transaction: &CoreActivationTransaction,
) -> Result<(), String> {
    let directory = system_root.as_ref().join(CORE_TRANSACTIONS_DIR);
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create {}: {error}", directory.display()))?;
    let path = directory.join(format!("{}.json", transaction.transaction_id));
    let temp = directory.join(format!(".{}.tmp", transaction.transaction_id));
    let content = serde_json::to_vec_pretty(transaction)
        .map_err(|error| format!("Failed to serialize Core transaction: {error}"))?;
    std::fs::write(&temp, content)
        .map_err(|error| format!("Failed to write {}: {error}", temp.display()))?;
    std::fs::rename(&temp, &path)
        .map_err(|error| format!("Failed to activate {}: {error}", path.display()))
}
