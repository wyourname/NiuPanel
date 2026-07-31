use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Debug, ToSchema)]
pub struct EnvironmentInfo {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub env_type: String,
    pub is_installed: bool,
    pub recorded_packages: Option<usize>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateEnvRequest {
    pub version: String,
}

#[derive(Deserialize, ToSchema)]
pub struct InstallPackageRequest {
    pub packages: Vec<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct SetMirrorSourceRequest {
    pub mirror_url: String,
}
