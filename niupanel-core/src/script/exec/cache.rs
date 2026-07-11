use niupanel_common::error::{AppError, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct DependencyCache;

impl DependencyCache {
    pub fn python_fingerprint_path(venv_path: &Path) -> PathBuf {
        venv_path.join(".niu_requirements_fingerprint")
    }

    pub fn python_fingerprint_marker_path(venv_path: &Path, fingerprint: &str) -> PathBuf {
        Self::fingerprint_marker_path(venv_path, fingerprint)
    }

    pub fn node_fingerprint_path(node_modules_dir: &Path) -> PathBuf {
        node_modules_dir.join(".niu_requirements_fingerprint")
    }

    pub fn node_fingerprint_marker_path(node_modules_dir: &Path, fingerprint: &str) -> PathBuf {
        Self::fingerprint_marker_path(node_modules_dir, fingerprint)
    }

    fn fingerprint_marker_path(base_dir: &Path, fingerprint: &str) -> PathBuf {
        let digest = Sha256::digest(fingerprint.as_bytes());
        let short_hash = hex::encode(&digest[..8]);
        base_dir
            .join(".niu_requirements_cache")
            .join(format!("{}.ok", short_hash))
    }

    pub fn python_fingerprint(
        venv_path: &Path,
        python_version: Option<&str>,
        requirements: &str,
        mirrors: Option<&HashMap<String, String>>,
    ) -> Result<String> {
        Self::build_fingerprint("python", venv_path, python_version, requirements, mirrors)
    }

    pub fn node_fingerprint(
        node_modules_dir: &Path,
        version: Option<&str>,
        requirements: &str,
        mirrors: Option<&HashMap<String, String>>,
    ) -> Result<String> {
        Self::build_fingerprint("node", node_modules_dir, version, requirements, mirrors)
    }

    fn build_fingerprint(
        kind: &str,
        base_path: &Path,
        version: Option<&str>,
        requirements: &str,
        mirrors: Option<&HashMap<String, String>>,
    ) -> Result<String> {
        let mut mirrors_vec: Vec<(String, String)> =
            mirrors.cloned().unwrap_or_default().into_iter().collect();
        mirrors_vec.sort_by(|a, b| a.0.cmp(&b.0));

        serde_json::to_string(&json!({
            "kind": kind,
            "base_path": base_path.to_string_lossy(),
            "version": version.unwrap_or_default(),
            "requirements": requirements
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>(),
            "mirrors": mirrors_vec,
        }))
        .map_err(AppError::Json)
    }

    pub async fn is_match(path: &Path, expected: &str) -> bool {
        match tokio::fs::read_to_string(path).await {
            Ok(existing) => existing == expected,
            Err(_) => false,
        }
    }

    pub async fn write(path: &Path, fingerprint: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(AppError::Io)?;
        }
        tokio::fs::write(path, fingerprint)
            .await
            .map_err(AppError::Io)
    }
}
