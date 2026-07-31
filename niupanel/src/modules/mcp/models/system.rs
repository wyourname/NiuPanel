use super::*;

#[derive(Debug, Serialize, JsonSchema)]
pub struct CoreReleaseOutput {
    pub version: String,
    pub active: bool,
    pub previous: bool,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub target: String,
    pub installed_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CoreReleaseListOutput {
    pub launcher_managed: bool,
    pub active_version: String,
    pub previous_version: Option<String>,
    pub pending_version: Option<String>,
    pub items: Vec<CoreReleaseOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct WebReleaseOutput {
    pub version: String,
    pub active: bool,
    pub previous: bool,
    pub managed: bool,
    pub compatible: bool,
    pub compatibility_error: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct WebReleaseListOutput {
    pub active_version: String,
    pub previous_version: Option<String>,
    pub items: Vec<WebReleaseOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateCheckOutput {
    pub component: String,
    pub current_version: String,
    pub target_version: Option<String>,
    pub channel: String,
    pub prerelease: bool,
    pub update_available: bool,
    pub size: u64,
    pub release_url: Option<String>,
    pub summary: Option<String>,
}
