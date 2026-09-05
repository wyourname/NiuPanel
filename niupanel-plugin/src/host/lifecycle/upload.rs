use super::*;

#[derive(Clone, Copy)]
pub enum ManifestCompatibility {
    PluginJsonOnly,
}

pub struct PreparedPluginPackage {
    _storage: TempDir,
    package_root: PathBuf,
    pub file_name: String,
    pub compressed_size: u64,
    pub extracted_size: u64,
    pub sha256: String,
    pub enable: bool,
    pub operation: String,
    pub target_plugin_id: Option<String>,
}

impl PreparedPluginPackage {
    pub fn package_root(&self) -> &Path {
        &self.package_root
    }
}

pub async fn prepare_plugin_package_upload(
    multipart: Multipart,
    upload_root: PathBuf,
    compatibility: ManifestCompatibility,
    package_label: &str,
) -> Result<PreparedPluginPackage> {
    tokio::fs::create_dir_all(&upload_root)
        .await
        .map_err(AppError::Io)?;
    let storage = tempfile::Builder::new()
        .prefix("session-")
        .tempdir_in(&upload_root)
        .map_err(AppError::Io)?;
    let package = read_plugin_package_upload(multipart, storage.path()).await?;
    let UploadedPluginPackage {
        file_name,
        upload,
        enable,
        operation,
        target_plugin_id,
    } = package;
    match (operation.as_str(), target_plugin_id.as_deref()) {
        ("install", None) | ("update", Some(_)) => {}
        ("install", Some(_)) => {
            return Err(AppError::ValidationError(
                "target_plugin_id is only valid for update uploads".to_string(),
            ));
        }
        ("update", None) => {
            return Err(AppError::ValidationError(
                "target_plugin_id is required for update uploads".to_string(),
            ));
        }
        _ => {
            return Err(AppError::ValidationError(
                "operation must be 'install' or 'update'".to_string(),
            ));
        }
    }
    let extraction_file_name = file_name.clone();
    let compressed_size = upload.size;
    let sha256 = upload.sha256.clone();
    let package_label = package_label.to_string();

    let (storage, package_root, stats) = tokio::task::spawn_blocking(move || {
        let extracted_root = storage.path().join("extracted");
        let stats = extract_plugin_package_file_to(
            &extraction_file_name,
            upload.path.as_ref(),
            &extracted_root,
        )?;
        let package_root =
            resolve_plugin_package_root(&extracted_root, compatibility, &package_label)?;
        Ok::<_, AppError>((storage, package_root, stats))
    })
    .await
    .map_err(|err| AppError::Internal(format!("Plugin extraction task failed: {err}")))??;

    Ok(PreparedPluginPackage {
        _storage: storage,
        package_root,
        file_name,
        compressed_size,
        extracted_size: stats.bytes,
        sha256,
        enable,
        operation,
        target_plugin_id,
    })
}

pub fn preview_package_bytes<R, P>(
    file_name: &str,
    bytes: &[u8],
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
    compatibility: ManifestCompatibility,
    package_label: &str,
    preview: P,
) -> Result<R>
where
    P: FnOnce(&Path) -> Result<R>,
{
    validate_package_integrity(
        bytes,
        checksum_sha256,
        signature_ed25519,
        public_key_ed25519,
    )?;
    let extracted = extract_plugin_package(file_name, bytes)?;
    let package_root = resolve_plugin_package_root(extracted.path(), compatibility, package_label)?;
    preview(&package_root)
}

#[allow(dead_code)]
pub fn install_package_bytes<R, F>(
    file_name: &str,
    bytes: &[u8],
    enable: bool,
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
    compatibility: ManifestCompatibility,
    package_label: &str,
    install: F,
) -> Result<R>
where
    F: FnOnce(PathBuf, bool) -> Result<R>,
{
    install_package_bytes_with_preflight(
        file_name,
        bytes,
        enable,
        checksum_sha256,
        signature_ed25519,
        public_key_ed25519,
        compatibility,
        package_label,
        |_| Ok(()),
        install,
    )
}

pub fn install_package_bytes_with_preflight<R, F, P>(
    file_name: &str,
    bytes: &[u8],
    enable: bool,
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
    compatibility: ManifestCompatibility,
    package_label: &str,
    preflight: P,
    install: F,
) -> Result<R>
where
    F: FnOnce(PathBuf, bool) -> Result<R>,
    P: FnOnce(&Path) -> Result<()>,
{
    validate_package_integrity(
        bytes,
        checksum_sha256,
        signature_ed25519,
        public_key_ed25519,
    )?;
    let extracted = extract_plugin_package(file_name, bytes)?;
    let package_root = resolve_plugin_package_root(extracted.path(), compatibility, package_label)?;
    preflight(&package_root)?;
    install(package_root, enable)
}

#[allow(dead_code)]
pub async fn update_package_bytes<R, F, Fut>(
    id: String,
    file_name: &str,
    bytes: &[u8],
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
    compatibility: ManifestCompatibility,
    package_label: &str,
    update: F,
) -> Result<R>
where
    F: FnOnce(String, PathBuf) -> Fut,
    Fut: Future<Output = Result<R>>,
{
    update_package_bytes_with_preflight(
        id,
        file_name,
        bytes,
        checksum_sha256,
        signature_ed25519,
        public_key_ed25519,
        compatibility,
        package_label,
        |_| Ok(()),
        update,
    )
    .await
}

pub async fn update_package_bytes_with_preflight<R, F, Fut, P>(
    id: String,
    file_name: &str,
    bytes: &[u8],
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
    compatibility: ManifestCompatibility,
    package_label: &str,
    preflight: P,
    update: F,
) -> Result<R>
where
    F: FnOnce(String, PathBuf) -> Fut,
    Fut: Future<Output = Result<R>>,
    P: FnOnce(&Path) -> Result<()>,
{
    validate_package_integrity(
        bytes,
        checksum_sha256,
        signature_ed25519,
        public_key_ed25519,
    )?;
    let extracted = extract_plugin_package(file_name, bytes)?;
    let package_root = resolve_plugin_package_root(extracted.path(), compatibility, package_label)?;
    preflight(&package_root)?;
    update(id, package_root).await
}
