use super::models::SystemVersionInfo;
use niupanel_common::version::{API_CONTRACT_VERSION, SCHEMA_EPOCH, SCHEMA_REVISION};

pub fn version_info() -> SystemVersionInfo {
    SystemVersionInfo {
        panel_version: runtime_version("NIUPANEL_ACTIVE_PANEL_VERSION")
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string()),
        api_contract: API_CONTRACT_VERSION,
        schema_epoch: SCHEMA_EPOCH,
        schema_revision: SCHEMA_REVISION,
        plugin_sandbox: niupanel_plugin::plugin_sandbox_capability(),
    }
}

fn runtime_version(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
