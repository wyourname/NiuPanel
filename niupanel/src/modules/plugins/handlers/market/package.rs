use super::*;

pub(super) struct MarketPluginPackage {
    pub(super) entry: PluginMarketEntry,
    pub(super) asset: PluginMarketAsset,
    pub(super) public_key_ed25519: Option<String>,
    pub(super) file_name: String,
    pub(super) bytes: Vec<u8>,
}

pub(super) async fn load_market_plugin_package(
    state: &AppState,
    payload: &PluginMarketInstallRequest,
) -> Result<MarketPluginPackage> {
    let index = fetch_plugin_market_index(state, &payload.index_url).await?;
    let entry = index
        .plugins
        .iter()
        .find(|entry| entry.id == payload.plugin_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("Plugin market entry not found".to_string()))?;
    let asset = select_market_asset(&entry)?.clone();
    let download_url = resolve_market_download_url(&payload.index_url, &asset.file)?;
    let bytes = download_plugin_market_package(state, download_url.as_str()).await?;
    let file_name = market_package_file_name(&download_url);
    let public_key_ed25519 = index.signing.and_then(|signing| signing.public_key_ed25519);
    Ok(MarketPluginPackage {
        entry,
        asset,
        public_key_ed25519,
        file_name,
        bytes,
    })
}

fn resolve_market_download_url(index_url: &str, download_url: &str) -> Result<reqwest::Url> {
    let base = reqwest::Url::parse(index_url)
        .map_err(|err| AppError::ValidationError(format!("Invalid plugin market URL: {err}")))?;
    let resolved = base
        .join(download_url)
        .map_err(|err| AppError::ValidationError(format!("Invalid plugin download URL: {err}")))?;
    if !matches!(resolved.scheme(), "http" | "https") {
        return Err(AppError::ValidationError(
            "Plugin download URL must use http or https".to_string(),
        ));
    }
    Ok(resolved)
}

async fn download_plugin_market_package(state: &AppState, url: &str) -> Result<Vec<u8>> {
    let response = state.http_client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(AppError::ExternalApi(format!(
            "Plugin package download failed: {}",
            response.status()
        )));
    }
    Ok(response.bytes().await?.to_vec())
}

fn market_package_file_name(url: &reqwest::Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|segment| !segment.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "plugin-package.tgz".to_string())
}
