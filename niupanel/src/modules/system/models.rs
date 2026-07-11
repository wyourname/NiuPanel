use niupanel_common::version::WebReleaseManifest;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemVersionInfo {
    pub core_version: String,
    pub web_version: String,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub web_compatible: bool,
    pub web_compatibility_error: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub web_manifest: Option<WebReleaseManifest>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemHealth {
    pub status: &'static str,
    pub core_version: String,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
}
