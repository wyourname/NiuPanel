use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::version::{
    RELEASE_PROTOCOL_VERSION, WEB_RELEASE_MANIFEST_FILE, WebReleaseManifest,
    read_web_release_manifest, validate_web_compatibility,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use utoipa::ToSchema;
use walkdir::WalkDir;

const WEB_STATE_FILE: &str = "web/state.json";
const WEB_CURRENT_LINK: &str = "web/current";
const WEB_RELEASES_DIR: &str = "releases/web";
const WEB_STAGING_DIR: &str = "web/staging";
const RETAINED_MANAGED_RELEASES: usize = 3;

mod archive;
mod releases;
mod runtime;

use archive::*;
pub use releases::*;
pub use runtime::*;

#[cfg(test)]
mod tests {
    use super::{validate_archive_path, verify_release_files};
    use niupanel_common::version::{ComponentCompatibility, WebReleaseManifest};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    #[test]
    fn rejects_archive_path_traversal() {
        assert!(validate_archive_path(Path::new("assets/app.js")).is_ok());
        assert!(validate_archive_path(Path::new("./assets/app.js")).is_ok());
        assert!(validate_archive_path(Path::new("./")).is_ok());
        assert!(validate_archive_path(Path::new("../index.html")).is_err());
        assert!(validate_archive_path(Path::new("/index.html")).is_err());
    }

    #[test]
    fn verifies_release_file_manifest() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("index.html"), b"<main>NiuPanel</main>").unwrap();
        let hash = hex::encode(Sha256::digest(b"<main>NiuPanel</main>"));
        let manifest = WebReleaseManifest {
            component: "web".into(),
            version: "2.0.0".into(),
            api_contract: 1,
            core: ComponentCompatibility {
                min: "0.8.0".into(),
                max: None,
            },
            built_at: None,
            files: BTreeMap::from([("index.html".into(), hash)]),
        };
        assert!(verify_release_files(temp.path(), &manifest).is_ok());
        fs::write(temp.path().join("index.html"), b"tampered").unwrap();
        assert!(verify_release_files(temp.path(), &manifest).is_err());
    }
}
