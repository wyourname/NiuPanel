use niupanel_core::settings::{
    NPM_REGISTRY_MIRROR, PNPM_NODE_DIST_MIRROR, SettingsService, UV_PYPI_MIRROR, UV_PYTHON_MIRROR,
};
use std::collections::HashMap;

const PNPM_NODE_DIST_MIRROR_ENV: &str = "PNPM_NODE_DIST_MIRROR";
const NPM_CONFIG_REGISTRY_ENV: &str = "npm_config_registry";
const UV_INDEX_URL_ENV: &str = "UV_INDEX_URL";
const UV_PYTHON_INSTALL_MIRROR_ENV: &str = "UV_PYTHON_INSTALL_MIRROR";
const DEFAULT_NODE_DIST_MIRROR: &str = "https://mirrors.ustc.edu.cn/node/";

pub(super) async fn python_install(settings: &SettingsService) -> HashMap<String, String> {
    let mut mirrors = HashMap::new();
    insert_setting_mirror(
        settings,
        &mut mirrors,
        UV_PYTHON_MIRROR,
        UV_PYTHON_INSTALL_MIRROR_ENV,
    )
    .await;
    mirrors
}

pub(super) async fn python_index(settings: &SettingsService) -> HashMap<String, String> {
    let mut mirrors = HashMap::new();
    insert_setting_mirror(settings, &mut mirrors, UV_PYPI_MIRROR, UV_INDEX_URL_ENV).await;
    mirrors
}

pub(super) async fn python_install_and_index(
    settings: &SettingsService,
) -> HashMap<String, String> {
    let mut mirrors = HashMap::new();
    insert_setting_mirror(
        settings,
        &mut mirrors,
        UV_PYTHON_MIRROR,
        UV_PYTHON_INSTALL_MIRROR_ENV,
    )
    .await;
    insert_setting_mirror(settings, &mut mirrors, UV_PYPI_MIRROR, UV_INDEX_URL_ENV).await;
    mirrors
}

pub(super) async fn node_distribution(settings: &SettingsService) -> HashMap<String, String> {
    let node_mirror = settings
        .get(PNPM_NODE_DIST_MIRROR)
        .await
        .unwrap_or_else(|_| DEFAULT_NODE_DIST_MIRROR.to_string());
    let mut mirrors = HashMap::new();
    if !node_mirror.is_empty() {
        mirrors.insert(PNPM_NODE_DIST_MIRROR_ENV.to_string(), node_mirror);
    }
    mirrors
}

pub(super) async fn node_install(settings: &SettingsService) -> HashMap<String, String> {
    let mut mirrors = node_distribution(settings).await;
    insert_setting_mirror(
        settings,
        &mut mirrors,
        NPM_REGISTRY_MIRROR,
        NPM_CONFIG_REGISTRY_ENV,
    )
    .await;
    mirrors
}

async fn insert_setting_mirror(
    settings: &SettingsService,
    mirrors: &mut HashMap<String, String>,
    setting_key: &str,
    runtime_env_key: &str,
) {
    if let Ok(mirror) = settings.get(setting_key).await {
        if !mirror.is_empty() {
            mirrors.insert(runtime_env_key.to_string(), mirror);
        }
    }
}
