use super::models::SystemVersionInfo;
use niupanel_common::version::{
    API_CONTRACT_VERSION, SCHEMA_EPOCH, SCHEMA_REVISION, read_web_release_manifest,
    validate_web_compatibility,
};

pub fn version_info() -> SystemVersionInfo {
    let core_version = env!("CARGO_PKG_VERSION").to_string();
    let web_root = niupanel_common::config::CONFIG
        .get()
        .map(|config| config.system_dir.join("web/current"))
        .unwrap_or_else(|| "web".into());
    let web_manifest = read_web_release_manifest(web_root);
    let web_version = std::env::var("NIUPANEL_WEB_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            web_manifest
                .as_ref()
                .map(|manifest| manifest.version.clone())
        })
        .unwrap_or_else(|| "unknown".to_string());
    let compatibility = web_manifest
        .as_ref()
        .map(|manifest| validate_web_compatibility(&core_version, manifest))
        .unwrap_or(Ok(()));

    SystemVersionInfo {
        core_version,
        web_version,
        api_contract: API_CONTRACT_VERSION,
        schema_epoch: SCHEMA_EPOCH,
        schema_revision: SCHEMA_REVISION,
        web_compatible: compatibility.is_ok(),
        web_compatibility_error: compatibility.err(),
        web_manifest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use niupanel_common::version::{ComponentCompatibility, WebReleaseManifest};

    fn manifest(min: &str, max: Option<&str>, api_contract: u32) -> WebReleaseManifest {
        WebReleaseManifest {
            component: "web".to_string(),
            version: "2.0.0".to_string(),
            api_contract,
            core: ComponentCompatibility {
                min: min.to_string(),
                max: max.map(str::to_string),
            },
            built_at: None,
            files: Default::default(),
        }
    }

    #[test]
    fn accepts_compatible_web_manifest() {
        assert!(validate_web_compatibility("0.8.0", &manifest("0.8.0", Some("0.9.0"), 1)).is_ok());
    }

    #[test]
    fn rejects_api_contract_mismatch() {
        assert!(validate_web_compatibility("0.8.0", &manifest("0.8.0", None, 2)).is_err());
    }

    #[test]
    fn rejects_core_outside_supported_range() {
        assert!(validate_web_compatibility("0.7.9", &manifest("0.8.0", None, 1)).is_err());
        assert!(validate_web_compatibility("1.0.0", &manifest("0.8.0", Some("0.9.9"), 1)).is_err());
    }
}
