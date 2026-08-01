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
        "../../../../../../plugins/niupanel-private-plugins.json"
    ))
    .expect("private plugin market index parses");
    validate_plugin_market_index(&mut index).expect("private plugin market index validates");

    let asset = select_market_asset(&index.plugins[0]).expect("host asset is present");
    assert_eq!(asset.platform, current_plugin_market_platform().unwrap());
    assert!(!asset.file.is_empty());
}
