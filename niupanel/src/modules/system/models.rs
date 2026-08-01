use niupanel_plugin::PluginSandboxCapability;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemVersionInfo {
    pub panel_version: String,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
    pub plugin_sandbox: PluginSandboxCapability,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SystemHealth {
    pub status: &'static str,
    pub panel_version: String,
    pub core_version: String,
    pub api_contract: u32,
    pub schema_epoch: u32,
    pub schema_revision: u32,
}
