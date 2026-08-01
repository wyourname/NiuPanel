use super::*;

#[derive(Debug, Serialize, JsonSchema)]
pub struct PanelReleaseOutput {
    pub version: String,
    pub active: bool,
    pub previous: bool,
    pub rollback_available: bool,
    pub installed_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PanelReleaseListOutput {
    pub launcher_managed: bool,
    pub active_version: String,
    pub previous_version: Option<String>,
    pub pending_version: Option<String>,
    pub items: Vec<PanelReleaseOutput>,
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
