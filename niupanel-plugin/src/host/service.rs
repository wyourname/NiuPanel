use crate::PluginService;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use std::sync::OnceLock;

pub const PLUGIN_CONTEXT: &str = "plugin";

static PLUGIN_SERVICE: OnceLock<PluginService> = OnceLock::new();

pub fn unified_plugin_service() -> PluginService {
    PLUGIN_SERVICE
        .get_or_init(|| PluginService::new(Config::global().plugins_dir.clone(), PLUGIN_CONTEXT))
        .clone()
}

pub fn plugin_service(extension: &str) -> Result<PluginService> {
    match extension {
        PLUGIN_CONTEXT => Ok(unified_plugin_service()),
        _ => Err(AppError::ValidationError(format!(
            "Unsupported plugin context '{extension}'"
        ))),
    }
}

pub fn plugin_services() -> Vec<(&'static str, PluginService)> {
    vec![(PLUGIN_CONTEXT, unified_plugin_service())]
}
