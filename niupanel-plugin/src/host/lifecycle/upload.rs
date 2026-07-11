use super::*;

#[derive(Clone, Copy)]
pub enum ManifestCompatibility {
    PluginJsonOnly,
}

#[allow(dead_code)]
pub async fn upload_install<R, F>(
    multipart: Multipart,
    compatibility: ManifestCompatibility,
    package_label: &str,
    install: F,
) -> Result<ApiResponse<R>>
where
    F: FnOnce(PathBuf, bool) -> Result<R>,
{
    upload_install_with_preflight(multipart, compatibility, package_label, |_| Ok(()), install)
        .await
}

pub async fn upload_install_with_preflight<R, F, P>(
    multipart: Multipart,
    compatibility: ManifestCompatibility,
    package_label: &str,
    preflight: P,
    install: F,
) -> Result<ApiResponse<R>>
where
    F: FnOnce(PathBuf, bool) -> Result<R>,
    P: FnOnce(&Path) -> Result<()>,
{
    let package = read_plugin_package_upload(multipart).await?;
    Ok(ApiResponse::success(install_package_bytes_with_preflight(
        &package.file_name,
        &package.bytes,
        package.enable,
        None,
        None,
        None,
        compatibility,
        package_label,
        preflight,
        install,
    )?))
}

#[allow(dead_code)]
pub async fn upload_update<R, F, Fut>(
    id: String,
    multipart: Multipart,
    compatibility: ManifestCompatibility,
    package_label: &str,
    update: F,
) -> Result<ApiResponse<R>>
where
    F: FnOnce(String, PathBuf) -> Fut,
    Fut: Future<Output = Result<R>>,
{
    upload_update_with_preflight(
        id,
        multipart,
        compatibility,
        package_label,
        |_| Ok(()),
        update,
    )
    .await
}

pub async fn upload_update_with_preflight<R, F, Fut, P>(
    id: String,
    multipart: Multipart,
    compatibility: ManifestCompatibility,
    package_label: &str,
    preflight: P,
    update: F,
) -> Result<ApiResponse<R>>
where
    F: FnOnce(String, PathBuf) -> Fut,
    Fut: Future<Output = Result<R>>,
    P: FnOnce(&Path) -> Result<()>,
{
    let package = read_plugin_package_upload(multipart).await?;
    Ok(ApiResponse::success(
        update_package_bytes_with_preflight(
            id,
            &package.file_name,
            &package.bytes,
            None,
            None,
            None,
            compatibility,
            package_label,
            preflight,
            update,
        )
        .await?,
    ))
}

pub async fn preview_upload<R, P>(
    multipart: Multipart,
    compatibility: ManifestCompatibility,
    package_label: &str,
    preview: P,
) -> Result<ApiResponse<R>>
where
    P: FnOnce(&Path) -> Result<R>,
{
    let package = read_plugin_package_upload(multipart).await?;
    let extracted = extract_plugin_package(&package.file_name, &package.bytes)?;
    let package_root = resolve_plugin_package_root(extracted.path(), compatibility, package_label)?;
    Ok(ApiResponse::success(preview(&package_root)?))
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
