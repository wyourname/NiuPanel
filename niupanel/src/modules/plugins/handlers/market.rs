use super::*;

struct MarketPluginPackage {
    entry: PluginMarketEntry,
    asset: PluginMarketAsset,
    public_key_ed25519: Option<String>,
    file_name: String,
    bytes: Vec<u8>,
}

#[utoipa::path(
    get,
    path = "/api/v1/plugins/market",
    params(
        ("index_url" = String, Query, description = "Plugin market index URL")
    ),
    responses(
        (status = 200, description = "Fetch plugin market index")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn get_plugin_market(
    State(state): State<AppState>,
    Query(query): Query<PluginMarketQuery>,
) -> Result<ApiResponse<PluginMarketIndex>> {
    Ok(ApiResponse::success(
        fetch_plugin_market_index(&state, &query.index_url).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/plugins/market/sources",
    responses(
        (status = 200, description = "List persisted plugin market sources")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn list_plugin_market_sources(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<PluginMarketSource>>> {
    Ok(ApiResponse::success(load_market_sources(&state).await?))
}

#[utoipa::path(
    put,
    path = "/api/v1/plugins/market/sources",
    request_body = PluginMarketSourcesUpdateRequest,
    responses(
        (status = 200, description = "Persist plugin market sources")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn update_plugin_market_sources(
    State(state): State<AppState>,
    Json(payload): Json<PluginMarketSourcesUpdateRequest>,
) -> Result<ApiResponse<Vec<PluginMarketSource>>> {
    let sources = normalize_market_sources(payload.sources)?;
    state
        .settings
        .set(
            PLUGIN_MARKET_SOURCES_KEY,
            &serde_json::to_string(&sources)?,
            Some("Plugins"),
        )
        .await?;
    Ok(ApiResponse::success(sources))
}

#[utoipa::path(
    get,
    path = "/api/v1/plugins/market/updates",
    responses(
        (status = 200, description = "Check installed plugins against persisted market sources")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn check_plugin_market_updates(
    State(state): State<AppState>,
) -> Result<ApiResponse<Vec<PluginMarketUpdateRecord>>> {
    Ok(ApiResponse::success(check_market_updates(&state).await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/market/preview",
    request_body = PluginMarketInstallRequest,
    responses(
        (status = 200, description = "Preview install or update impact for a plugin from a market index")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn preview_plugin_from_market(
    State(state): State<AppState>,
    Json(payload): Json<PluginMarketInstallRequest>,
) -> Result<ApiResponse<PluginImpactPreview>> {
    let package = load_market_plugin_package(&state, &payload).await?;
    let installed = unified_plugin_service()
        .get_plugin(&package.entry.id)
        .is_ok();
    let operation = if installed { "update" } else { "install" };
    let compatibility = ManifestCompatibility::PluginJsonOnly;
    let current_id = installed.then_some(package.entry.id.as_str());
    Ok(ApiResponse::success(lifecycle::preview_package_bytes(
        &package.file_name,
        &package.bytes,
        package.asset.checksum_sha256.as_deref(),
        package.asset.signature_ed25519.as_deref(),
        package.public_key_ed25519.as_deref(),
        compatibility,
        "Plugin",
        |source_path| preview_plugin_impact(operation, PLUGIN_CONTEXT, source_path, current_id),
    )?))
}

#[utoipa::path(
    post,
    path = "/api/v1/plugins/market/install",
    request_body = PluginMarketInstallRequest,
    responses(
        (status = 200, description = "Install or update a plugin from a market index")
    ),
    tag = "Plugins",
    security(("session_cookie" = []))
)]
pub async fn install_plugin_from_market(
    State(state): State<AppState>,
    Json(payload): Json<PluginMarketInstallRequest>,
) -> Result<ApiResponse<PluginRecord>> {
    let package = load_market_plugin_package(&state, &payload).await?;
    let enable = payload.enable.unwrap_or(true);
    let service = unified_plugin_service();
    let record = if service.get_plugin(&package.entry.id).is_ok() {
        let plugin_id = package.entry.id.clone();
        let update_service = service.clone();
        lifecycle::update_package_bytes_with_preflight(
            package.entry.id.clone(),
            &package.file_name,
            &package.bytes,
            package.asset.checksum_sha256.as_deref(),
            package.asset.signature_ed25519.as_deref(),
            package.public_key_ed25519.as_deref(),
            ManifestCompatibility::PluginJsonOnly,
            "Plugin",
            move |source_path| {
                ensure_plugin_impact_allowed(
                    "update",
                    PLUGIN_CONTEXT,
                    source_path,
                    Some(&plugin_id),
                )
            },
            move |id, source_path| async move {
                update_service.update_from_dir_async(&id, source_path).await
            },
        )
        .await?
    } else {
        let install_service = service.clone();
        lifecycle::install_package_bytes_with_preflight(
            &package.file_name,
            &package.bytes,
            enable,
            package.asset.checksum_sha256.as_deref(),
            package.asset.signature_ed25519.as_deref(),
            package.public_key_ed25519.as_deref(),
            ManifestCompatibility::PluginJsonOnly,
            "Plugin",
            move |source_path| {
                ensure_plugin_impact_allowed("install", PLUGIN_CONTEXT, source_path, None)
            },
            move |source_path, enable| install_service.install_from_dir(source_path, enable),
        )?
    };
    Ok(ApiResponse::success(record))
}

async fn load_market_plugin_package(
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

pub(super) async fn fetch_plugin_market_index(
    state: &AppState,
    index_url: &str,
) -> Result<PluginMarketIndex> {
    let index_url = reqwest::Url::parse(index_url)
        .map_err(|err| AppError::ValidationError(format!("Invalid plugin market URL: {err}")))?;
    if !matches!(index_url.scheme(), "http" | "https") {
        return Err(AppError::ValidationError(
            "Plugin market URL must use http or https".to_string(),
        ));
    }

    let response = state.http_client.get(index_url).send().await?;
    if !response.status().is_success() {
        return Err(AppError::ExternalApi(format!(
            "Plugin market request failed: {}",
            response.status()
        )));
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_PLUGIN_MARKET_INDEX_BYTES {
        return Err(AppError::FileSizeLimitExceeded(
            "Plugin market index exceeds 2MB".to_string(),
        ));
    }
    let mut index = decode_plugin_market_index(&bytes)?;
    validate_plugin_market_index(&mut index)?;
    Ok(index)
}

fn decode_plugin_market_index(bytes: &[u8]) -> Result<PluginMarketIndex> {
    serde_json::from_slice(bytes).map_err(AppError::Json)
}

fn current_plugin_market_platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some("linux-x64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        ("linux", "arm") => Some("linux-arm32"),
        _ => None,
    }
}

pub(super) async fn load_market_sources(state: &AppState) -> Result<Vec<PluginMarketSource>> {
    let value = state.settings.get(PLUGIN_MARKET_SOURCES_KEY).await?;
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    let sources: Vec<PluginMarketSource> = serde_json::from_str(&value)?;
    normalize_market_sources(sources)
}

pub(super) fn normalize_market_sources(
    sources: Vec<PluginMarketSource>,
) -> Result<Vec<PluginMarketSource>> {
    let mut normalized = Vec::new();
    let mut seen_urls = HashSet::new();

    for source in sources {
        let url_text = source.url.trim();
        if url_text.is_empty() {
            return Err(AppError::ValidationError(
                "Plugin market source URL is required".to_string(),
            ));
        }
        let url = reqwest::Url::parse(url_text).map_err(|err| {
            AppError::ValidationError(format!("Invalid plugin market URL: {err}"))
        })?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(AppError::ValidationError(
                "Plugin market source URL must use http or https".to_string(),
            ));
        }

        let normalized_url = url.to_string();
        if !seen_urls.insert(normalized_url.clone()) {
            continue;
        }

        let name = source.name.trim();
        normalized.push(PluginMarketSource {
            name: if name.is_empty() {
                default_market_source_name(&url)
            } else {
                name.to_string()
            },
            url: normalized_url,
            enabled: source.enabled,
        });
    }

    normalized.sort_by(|a, b| a.name.cmp(&b.name).then(a.url.cmp(&b.url)));
    Ok(normalized)
}

pub(super) fn default_market_source_name(url: &reqwest::Url) -> String {
    let host = url.host_str().unwrap_or("Plugin Market");
    let path = url.path().trim_matches('/');
    if path.is_empty() {
        host.to_string()
    } else {
        format!("{host}/{path}")
    }
}

pub(super) async fn check_market_updates(
    state: &AppState,
) -> Result<Vec<PluginMarketUpdateRecord>> {
    let installed = installed_plugins()?;
    let mut updates = Vec::new();

    for source in load_market_sources(state)
        .await?
        .into_iter()
        .filter(|source| source.enabled)
    {
        let index = fetch_plugin_market_index(state, &source.url).await?;
        for entry in index.plugins {
            let installed_version = installed
                .iter()
                .find(|plugin| plugin.manifest.id == entry.id)
                .map(|plugin| plugin.manifest.version.clone());

            let Some(installed_version) = installed_version else {
                continue;
            };
            if !version_is_newer(&entry.version, &installed_version) {
                continue;
            }

            updates.push(PluginMarketUpdateRecord {
                source_name: source.name.clone(),
                source_url: source.url.clone(),
                plugin_id: entry.id.clone(),
                installed_version,
                available_version: entry.version.clone(),
                entry,
            });
        }
    }

    updates.sort_by(|a, b| {
        a.plugin_id
            .cmp(&b.plugin_id)
            .then(a.source_name.cmp(&b.source_name))
    });
    Ok(updates)
}

pub(super) fn version_is_newer(available: &str, installed: &str) -> bool {
    let available = available.trim().trim_start_matches('v');
    let installed = installed.trim().trim_start_matches('v');
    if available == installed {
        return false;
    }

    match (
        parse_numeric_version(available),
        parse_numeric_version(installed),
    ) {
        (Some(available), Some(installed)) => {
            compare_numeric_versions(&available, &installed).is_gt()
        }
        _ => true,
    }
}

pub(super) fn parse_numeric_version(version: &str) -> Option<Vec<u64>> {
    if version.is_empty() {
        return None;
    }
    version
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

pub(super) fn compare_numeric_versions(left: &[u64], right: &[u64]) -> Ordering {
    let len = left.len().max(right.len());
    for index in 0..len {
        let left = left.get(index).copied().unwrap_or(0);
        let right = right.get(index).copied().unwrap_or(0);
        match left.cmp(&right) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

pub(super) fn validate_plugin_market_index(index: &mut PluginMarketIndex) -> Result<()> {
    if index.schema_version != 1 {
        return Err(AppError::ValidationError(
            "Plugin market schema_version must be 1".to_string(),
        ));
    }
    if index.name.trim().is_empty() {
        return Err(AppError::ValidationError(
            "Plugin market name is required".to_string(),
        ));
    }
    if let Some(signing) = &index.signing {
        if signing.algorithm != "ed25519" {
            return Err(AppError::ValidationError(
                "Plugin market signing algorithm must be ed25519".to_string(),
            ));
        }
    }
    for entry in &mut index.plugins {
        if entry.name.trim().is_empty() {
            entry.name = entry.id.clone();
        }
        validate_market_entry(entry)?;
    }
    index
        .plugins
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(())
}

pub(super) fn validate_market_entry(entry: &PluginMarketEntry) -> Result<()> {
    if entry.id.trim().is_empty() || entry.version.trim().is_empty() || entry.assets.is_empty() {
        return Err(AppError::ValidationError(
            "Plugin market entries require id, version and platform assets".to_string(),
        ));
    }
    for asset in &entry.assets {
        if asset.platform.trim().is_empty() || asset.file.trim().is_empty() {
            return Err(AppError::ValidationError(format!(
                "Plugin market entry '{}' has an invalid platform asset",
                entry.id
            )));
        }
    }
    Ok(())
}

pub(super) fn select_market_asset(entry: &PluginMarketEntry) -> Result<&PluginMarketAsset> {
    let platform = current_plugin_market_platform().ok_or_else(|| {
        AppError::ValidationError(format!(
            "Platform plugin market is unsupported on {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
        ))
    })?;
    entry
        .assets
        .iter()
        .find(|asset| asset.platform == platform)
        .ok_or_else(|| {
            AppError::ValidationError(format!(
                "Plugin market entry '{}' has no package for platform '{platform}'",
                entry.id
            ))
        })
}

pub(super) fn resolve_market_download_url(
    index_url: &str,
    download_url: &str,
) -> Result<reqwest::Url> {
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

pub(super) async fn download_plugin_market_package(state: &AppState, url: &str) -> Result<Vec<u8>> {
    let response = state.http_client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(AppError::ExternalApi(format!(
            "Plugin package download failed: {}",
            response.status()
        )));
    }
    Ok(response.bytes().await?.to_vec())
}

pub(super) fn market_package_file_name(url: &reqwest::Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|segment| !segment.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "plugin-package.tgz".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_asset_market_index_is_the_only_supported_schema() {
        let platform = current_plugin_market_platform().expect("test host is supported");
        let mut index = decode_plugin_market_index(
            format!(
                r#"{{
                    "schema_version": 1,
                    "name": "Private plugins",
                    "signing": {{ "public_key_ed25519": "test-public-key" }},
                    "plugins": [{{
                        "id": "private-loader",
                        "version": "0.1.0",
                        "assets": [{{
                            "platform": "{platform}",
                            "file": "./private-loader.tgz",
                            "checksum_sha256": "abc",
                            "signature_ed25519": "signature"
                        }}]
                    }}]
                }}"#,
            )
            .as_bytes(),
        )
        .expect("platform asset index deserializes");
        validate_plugin_market_index(&mut index).expect("platform asset index validates");

        assert_eq!(index.plugins.len(), 1);
        assert_eq!(index.plugins[0].name, "private-loader");
        assert_eq!(index.plugins[0].assets[0].file, "./private-loader.tgz");
        assert_eq!(
            index
                .signing
                .as_ref()
                .and_then(|signing| signing.public_key_ed25519.as_deref()),
            Some("test-public-key")
        );
    }

    #[test]
    fn legacy_download_url_market_entries_are_rejected() {
        let mut index = decode_plugin_market_index(
            br#"{
                "schema_version": 1,
                "name": "Legacy",
                "plugins": [{
                    "id": "legacy-plugin",
                    "name": "Legacy Plugin",
                    "version": "0.1.0",
                    "download_url": "./legacy-plugin.tgz"
                }]
            }"#,
        )
        .expect("legacy JSON still has valid JSON syntax");

        assert!(validate_plugin_market_index(&mut index).is_err());
    }

    #[test]
    fn checked_in_platform_market_index_is_accepted() {
        let mut index = decode_plugin_market_index(include_bytes!(
            "../../../../../plugins/niupanel-private-plugins.json"
        ))
        .expect("private plugin market index parses");
        validate_plugin_market_index(&mut index).expect("private plugin market index validates");

        let asset = select_market_asset(&index.plugins[0]).expect("host asset is present");
        assert_eq!(asset.platform, current_plugin_market_platform().unwrap());
        assert!(!asset.file.is_empty());
    }
}
