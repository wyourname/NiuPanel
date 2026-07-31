use anyhow::Result;
use niupanel_common::{error, info};
use std::fs;
use std::path::Path;

pub async fn init_sdk() -> Result<(String, String)> {
    info!("Provisioning NiuPanel SDK...");
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let abs_current_dir = fs::canonicalize(&current_dir).unwrap_or(current_dir);
    let sdk_path = abs_current_dir.join("data/niu-sdk");

    if let Err(e) = niupanel_sdk::provision(&sdk_path) {
        error!("Failed to provision SDK: {}", e);
        return Err(e.into());
    }

    info!("SDK provisioned at {}", sdk_path.display());

    let python_sdk_path = sdk_path.join("python");
    let node_sdk_path = sdk_path.join("node");

    Ok((
        normalize_sdk_path(&python_sdk_path),
        normalize_sdk_path(&node_sdk_path),
    ))
}

fn normalize_sdk_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
