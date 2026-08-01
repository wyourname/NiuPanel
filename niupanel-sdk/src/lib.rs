use std::fs;
use std::io::Write;
use std::path::Path;

const PYTHON_SDK: &str = include_str!("python/niu/__init__.py");
const PYTHON_SITECUSTOMIZE: &str = include_str!("python/sitecustomize.py");
const NODE_SDK_INDEX: &str = include_str!("node/niu/index.js");
const NODE_SDK_PRELOAD: &str = include_str!("node/niu/preload.js");
const NODE_SHARED_DEPENDENCIES_LOADER: &str =
    include_str!("node/niu/shared-dependencies-loader.mjs");
const NODE_SHARED_DEPENDENCIES_REGISTER: &str =
    include_str!("node/niu/shared-dependencies-register.mjs");
const NODE_SDK_PACKAGE: &str = include_str!("node/niu/package.json");
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
const MANIFEST_FILE: &str = "manifest.json";
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

pub fn provision(base_path: &Path) -> std::io::Result<()> {
    let python_path = base_path.join("python/niu");
    let node_path = base_path.join("node/niu");
    let manifest_path = base_path.join(MANIFEST_FILE);

    if is_provisioned_current(&manifest_path, &python_path, &node_path) {
        return Ok(());
    }

    fs::create_dir_all(&python_path)?;
    fs::create_dir_all(&node_path)?;

    let mut py_file = fs::File::create(python_path.join("__init__.py"))?;
    py_file.write_all(PYTHON_SDK.as_bytes())?;

    let mut py_sitecustomize_file = fs::File::create(base_path.join("python/sitecustomize.py"))?;
    py_sitecustomize_file.write_all(PYTHON_SITECUSTOMIZE.as_bytes())?;

    let mut js_file = fs::File::create(node_path.join("index.js"))?;
    js_file.write_all(NODE_SDK_INDEX.as_bytes())?;

    let mut js_preload_file = fs::File::create(node_path.join("preload.js"))?;
    js_preload_file.write_all(NODE_SDK_PRELOAD.as_bytes())?;

    let mut js_loader_file = fs::File::create(node_path.join("shared-dependencies-loader.mjs"))?;
    js_loader_file.write_all(NODE_SHARED_DEPENDENCIES_LOADER.as_bytes())?;

    let mut js_loader_register_file =
        fs::File::create(node_path.join("shared-dependencies-register.mjs"))?;
    js_loader_register_file.write_all(NODE_SHARED_DEPENDENCIES_REGISTER.as_bytes())?;

    let mut json_file = fs::File::create(node_path.join("package.json"))?;
    json_file.write_all(NODE_SDK_PACKAGE.as_bytes())?;

    let mut manifest_file = fs::File::create(manifest_path)?;
    manifest_file.write_all(build_manifest().as_bytes())?;

    Ok(())
}

fn is_provisioned_current(manifest_path: &Path, python_path: &Path, node_path: &Path) -> bool {
    let expected_manifest = build_manifest();
    let Ok(existing_manifest) = fs::read_to_string(manifest_path) else {
        return false;
    };

    existing_manifest == expected_manifest
        && python_path.join("__init__.py").exists()
        && python_path
            .parent()
            .is_some_and(|path| path.join("sitecustomize.py").exists())
        && node_path.join("index.js").exists()
        && node_path.join("preload.js").exists()
        && node_path.join("shared-dependencies-loader.mjs").exists()
        && node_path.join("shared-dependencies-register.mjs").exists()
        && node_path.join("package.json").exists()
}

fn build_manifest() -> String {
    format!(
        concat!(
            "{{\n",
            "  \"sdk_version\": \"{}\",\n",
            "  \"content_hash\": \"{:016x}\",\n",
            "  \"python_entry\": \"python/niu/__init__.py\",\n",
            "  \"python_sitecustomize\": \"python/sitecustomize.py\",\n",
            "  \"node_entry\": \"node/niu/index.js\",\n",
            "  \"node_preload\": \"node/niu/preload.js\",\n",
            "  \"node_shared_dependencies_loader\": \"node/niu/shared-dependencies-loader.mjs\",\n",
            "  \"node_shared_dependencies_register\": \"node/niu/shared-dependencies-register.mjs\",\n",
            "  \"node_package\": \"node/niu/package.json\"\n",
            "}}\n"
        ),
        SDK_VERSION,
        sdk_content_hash()
    )
}

fn sdk_content_hash() -> u64 {
    [
        PYTHON_SDK,
        PYTHON_SITECUSTOMIZE,
        NODE_SDK_INDEX,
        NODE_SDK_PRELOAD,
        NODE_SHARED_DEPENDENCIES_LOADER,
        NODE_SHARED_DEPENDENCIES_REGISTER,
        NODE_SDK_PACKAGE,
    ]
    .into_iter()
    .fold(FNV_OFFSET_BASIS, |mut hash, content| {
        for byte in content.bytes().chain(std::iter::once(0xff)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_provision() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        provision(path).unwrap();

        assert!(path.join("python/niu/__init__.py").exists());
        assert!(path.join("python/sitecustomize.py").exists());
        assert!(path.join("node/niu/index.js").exists());
        assert!(path.join("node/niu/preload.js").exists());
        assert!(
            path.join("node/niu/shared-dependencies-loader.mjs")
                .exists()
        );
        assert!(
            path.join("node/niu/shared-dependencies-register.mjs")
                .exists()
        );
        assert!(path.join("node/niu/package.json").exists());

        let py_content = fs::read_to_string(path.join("python/niu/__init__.py")).unwrap();
        assert!(py_content.contains("class NiuPanelSDK"));
        assert!(py_content.contains("headers[\"X-API-Key\"] = self.api_key"));

        let node_content = fs::read_to_string(path.join("node/niu/index.js")).unwrap();
        assert!(node_content.contains("headers[\"X-API-Key\"] = this.apiKey"));

        let manifest = fs::read_to_string(path.join(MANIFEST_FILE)).unwrap();
        assert!(manifest.contains("\"content_hash\""));
    }

    #[test]
    fn test_provision_replaces_legacy_manifest() {
        let dir = tempdir().unwrap();
        let path = dir.path();

        fs::create_dir_all(path.join("python/niu")).unwrap();
        fs::create_dir_all(path.join("node/niu")).unwrap();
        fs::write(path.join("python/niu/__init__.py"), "stale").unwrap();
        fs::write(path.join("python/sitecustomize.py"), "stale").unwrap();
        fs::write(path.join("node/niu/index.js"), "stale").unwrap();
        fs::write(path.join("node/niu/preload.js"), "stale").unwrap();
        fs::write(path.join("node/niu/package.json"), "stale").unwrap();
        fs::write(
            path.join(MANIFEST_FILE),
            format!("{{\n  \"sdk_version\": \"{SDK_VERSION}\"\n}}\n"),
        )
        .unwrap();

        provision(path).unwrap();

        let py_content = fs::read_to_string(path.join("python/niu/__init__.py")).unwrap();
        assert!(py_content.contains("headers[\"X-API-Key\"] = self.api_key"));
        assert_eq!(
            fs::read_to_string(path.join(MANIFEST_FILE)).unwrap(),
            build_manifest()
        );
    }
}
