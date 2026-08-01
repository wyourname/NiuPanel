use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpdateState {
    Idle,
    Downloading,
    Installing,
    Restarting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub state: UpdateState,
    pub progress: u8,
    pub message: String,
    pub error: Option<String>,
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            progress: 0,
            message: "".to_string(),
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub html_url: String,
    pub body: String,
    pub current_version: String,
    pub channel: String,
    pub prerelease: bool,
    pub update_available: bool,
    pub size: u64,
    pub launcher_managed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebReleaseInfo {
    pub version: String,
    pub current_version: String,
    pub release_tag: String,
    pub html_url: String,
    pub body: String,
    pub channel: String,
    pub prerelease: bool,
    pub update_available: bool,
    pub size: u64,
}
