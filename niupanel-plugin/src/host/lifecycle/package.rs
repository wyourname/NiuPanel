use super::*;

pub(super) struct UploadedPluginPackage {
    pub(super) file_name: String,
    pub(super) bytes: Vec<u8>,
    pub(super) enable: bool,
}

pub(super) async fn read_plugin_package_upload(
    mut multipart: Multipart,
) -> Result<UploadedPluginPackage> {
    let mut file_name = None;
    let mut bytes = None;
    let mut enable = true;
    let mut checksum_sha256 = None;
    let mut signature_ed25519 = None;
    let mut public_key_ed25519 = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::ValidationError(err.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "enable" {
            let value = field
                .text()
                .await
                .map_err(|err| AppError::ValidationError(err.to_string()))?;
            enable = matches!(value.as_str(), "true" | "1" | "on");
            continue;
        }
        if name == "checksum_sha256" {
            let value = field
                .text()
                .await
                .map_err(|err| AppError::ValidationError(err.to_string()))?;
            let value = value.trim().to_ascii_lowercase();
            if !value.is_empty() {
                validate_sha256_hex(&value)?;
                checksum_sha256 = Some(value);
            }
            continue;
        }
        if name == "signature_ed25519" {
            let value = field
                .text()
                .await
                .map_err(|err| AppError::ValidationError(err.to_string()))?;
            let value = value.trim().to_string();
            if !value.is_empty() {
                signature_ed25519 = Some(value);
            }
            continue;
        }
        if name == "public_key_ed25519" {
            let value = field
                .text()
                .await
                .map_err(|err| AppError::ValidationError(err.to_string()))?;
            let value = value.trim().to_string();
            if !value.is_empty() {
                public_key_ed25519 = Some(value);
            }
            continue;
        }

        if name == "file" || field.file_name().is_some() {
            let current_file_name = field
                .file_name()
                .map(str::to_string)
                .unwrap_or_else(|| "plugin-package.tar.gz".to_string());
            let data = field
                .bytes()
                .await
                .map_err(|err| AppError::ValidationError(err.to_string()))?;
            if data.len() > MAX_PLUGIN_PACKAGE_BYTES {
                return Err(AppError::FileSizeLimitExceeded(
                    "Plugin package exceeds 100MB".to_string(),
                ));
            }
            file_name = Some(current_file_name);
            bytes = Some(data.to_vec());
        }
    }

    let bytes = bytes
        .ok_or_else(|| AppError::ValidationError("Missing plugin package file".to_string()))?;
    validate_package_integrity(
        &bytes,
        checksum_sha256.as_deref(),
        signature_ed25519.as_deref(),
        public_key_ed25519.as_deref(),
    )?;

    Ok(UploadedPluginPackage {
        file_name: file_name
            .ok_or_else(|| AppError::ValidationError("Missing plugin package file".to_string()))?,
        bytes,
        enable,
    })
}

pub(super) fn validate_package_integrity(
    bytes: &[u8],
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
) -> Result<()> {
    if bytes.len() > MAX_PLUGIN_PACKAGE_BYTES {
        return Err(AppError::FileSizeLimitExceeded(
            "Plugin package exceeds 100MB".to_string(),
        ));
    }
    if let Some(expected) = checksum_sha256 {
        let expected = expected.trim().to_ascii_lowercase();
        validate_sha256_hex(&expected)?;
        let actual = hex::encode(Sha256::digest(bytes));
        if actual != expected {
            return Err(AppError::ValidationError(
                "Plugin package checksum mismatch".to_string(),
            ));
        }
    }
    verify_uploaded_package_signature(bytes, signature_ed25519, public_key_ed25519)
}

pub(super) fn verify_uploaded_package_signature(
    bytes: &[u8],
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
) -> Result<()> {
    let config = Config::global();
    let signature_present = signature_ed25519.is_some() || public_key_ed25519.is_some();
    if !config.plugin_signature_required && !signature_present {
        return Ok(());
    }
    if config.trusted_plugin_public_keys.is_empty() {
        return Err(AppError::ValidationError(
            "Plugin signature verification requires trusted_plugin_public_keys".to_string(),
        ));
    }
    let signature = signature_ed25519.ok_or_else(|| {
        AppError::ValidationError("Missing signature_ed25519 for plugin package".to_string())
    })?;
    let public_key = public_key_ed25519.ok_or_else(|| {
        AppError::ValidationError("Missing public_key_ed25519 for plugin package".to_string())
    })?;

    verify_package_signature_with_trust(
        bytes,
        signature,
        public_key,
        &config.trusted_plugin_public_keys,
    )
}

pub(super) fn verify_package_signature_with_trust(
    bytes: &[u8],
    signature_ed25519: &str,
    public_key_ed25519: &str,
    trusted_public_keys: &[String],
) -> Result<()> {
    let public_key = parse_ed25519_public_key(public_key_ed25519)?;
    let fingerprint = ed25519_public_key_fingerprint(&public_key)?;
    if !trusted_plugin_key_matches(trusted_public_keys, &fingerprint)? {
        return Err(AppError::Forbidden(
            "Plugin package public key is not trusted".to_string(),
        ));
    }

    let signature = decode_base64(signature_ed25519)
        .map_err(|_| AppError::ValidationError("Invalid signature_ed25519".to_string()))?;
    verify_ed25519_signature(bytes, &signature, &public_key)
}

pub(super) fn parse_ed25519_public_key(value: &str) -> Result<PKey<Public>> {
    let trimmed = value.trim();
    if trimmed.starts_with("-----BEGIN") {
        return PKey::public_key_from_pem(trimmed.as_bytes())
            .map_err(|_| AppError::ValidationError("Invalid public_key_ed25519 PEM".to_string()));
    }

    let key_bytes = if is_hex(trimmed) && trimmed.len().is_multiple_of(2) {
        hex::decode(trimmed)
            .map_err(|_| AppError::ValidationError("Invalid public_key_ed25519 hex".to_string()))?
    } else {
        decode_base64(trimmed).map_err(|_| {
            AppError::ValidationError("Invalid public_key_ed25519 base64".to_string())
        })?
    };

    if key_bytes.len() == 32 {
        return PKey::public_key_from_raw_bytes(&key_bytes, Id::ED25519)
            .map_err(|_| AppError::ValidationError("Invalid raw Ed25519 public key".to_string()));
    }

    PKey::public_key_from_der(&key_bytes)
        .map_err(|_| AppError::ValidationError("Invalid public_key_ed25519 DER".to_string()))
}

pub(super) fn verify_ed25519_signature(
    bytes: &[u8],
    signature: &[u8],
    public_key: &PKey<Public>,
) -> Result<()> {
    let mut verifier = Verifier::new_without_digest(public_key)
        .map_err(|_| AppError::ValidationError("Invalid Ed25519 verifier".to_string()))?;
    let verified = verifier
        .verify_oneshot(signature, bytes)
        .map_err(|_| AppError::ValidationError("Invalid plugin package signature".to_string()))?;
    if verified {
        Ok(())
    } else {
        Err(AppError::ValidationError(
            "Plugin package signature verification failed".to_string(),
        ))
    }
}

pub(super) fn ed25519_public_key_fingerprint(public_key: &PKey<Public>) -> Result<String> {
    let raw = public_key
        .raw_public_key()
        .map_err(|_| AppError::ValidationError("Invalid Ed25519 public key".to_string()))?;
    Ok(hex::encode(Sha256::digest(raw)))
}

pub(super) fn trusted_plugin_key_matches(
    trusted_keys: &[String],
    fingerprint: &str,
) -> Result<bool> {
    for trusted_key in trusted_keys {
        let trusted_key = trusted_key.trim();
        if trusted_key.is_empty() {
            continue;
        }
        if let Some(value) = trusted_key.strip_prefix("sha256:") {
            if value.eq_ignore_ascii_case(fingerprint) {
                return Ok(true);
            }
            continue;
        }

        let key = parse_ed25519_public_key(trusted_key)?;
        if ed25519_public_key_fingerprint(&key)?.eq_ignore_ascii_case(fingerprint) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn decode_base64(
    value: &str,
) -> std::result::Result<Vec<u8>, openssl::error::ErrorStack> {
    let clean: String = value.chars().filter(|ch| !ch.is_whitespace()).collect();
    base64::decode_block(&clean)
}

pub(super) fn is_hex(value: &str) -> bool {
    value.chars().all(|ch| ch.is_ascii_hexdigit())
}

pub(super) fn validate_sha256_hex(value: &str) -> Result<()> {
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(AppError::ValidationError(
            "checksum_sha256 must be a 64-character hex SHA-256 digest".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn extract_plugin_package(file_name: &str, bytes: &[u8]) -> Result<TempDir> {
    let temp_dir = tempfile::tempdir().map_err(AppError::Io)?;
    let lower = file_name.to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(bytes, temp_dir.path())?;
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        let decoder = GzDecoder::new(Cursor::new(bytes));
        extract_tar(decoder, temp_dir.path())?;
    } else if lower.ends_with(".tar") {
        extract_tar(Cursor::new(bytes), temp_dir.path())?;
    } else {
        return Err(AppError::ValidationError(
            "Unsupported plugin package format. Supported: .zip, .tar, .tar.gz, .tgz".to_string(),
        ));
    }
    Ok(temp_dir)
}

pub(super) fn resolve_plugin_package_root(
    extracted_root: &Path,
    compatibility: ManifestCompatibility,
    package_label: &str,
) -> Result<PathBuf> {
    if has_plugin_manifest(extracted_root, compatibility) {
        return Ok(extracted_root.to_path_buf());
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(extracted_root).map_err(AppError::Io)? {
        let entry = entry.map_err(AppError::Io)?;
        let path = entry.path();
        if path.is_dir() && has_plugin_manifest(&path, compatibility) {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(AppError::ValidationError(format!(
            "{package_label} package must contain plugin.json at root or inside a single top-level directory"
        ))),
        _ => Err(AppError::ValidationError(format!(
            "{package_label} package contains multiple top-level plugin.json files"
        ))),
    }
}

pub(super) fn has_plugin_manifest(path: &Path, compatibility: ManifestCompatibility) -> bool {
    match compatibility {
        ManifestCompatibility::PluginJsonOnly => path.join("plugin.json").is_file(),
    }
}

pub(super) fn extract_tar<R>(reader: R, destination: &Path) -> Result<()>
where
    R: Read,
{
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries().map_err(AppError::Io)? {
        let mut entry = entry.map_err(AppError::Io)?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(AppError::ValidationError(
                "Plugin packages cannot contain links".to_string(),
            ));
        }
        if !(entry_type.is_file() || entry_type.is_dir()) {
            continue;
        }

        let path = entry.path().map_err(AppError::Io)?;
        let safe_path = sanitize_archive_path(&path)?;
        let output_path = destination.join(safe_path);
        ensure_within(destination, &output_path)?;
        if entry_type.is_dir() {
            fs::create_dir_all(&output_path).map_err(AppError::Io)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            entry.unpack(&output_path).map_err(AppError::Io)?;
        }
    }
    Ok(())
}

pub(super) fn extract_zip(bytes: &[u8], destination: &Path) -> Result<()> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)
        .map_err(|err| AppError::ValidationError(format!("Invalid zip package: {err}")))?;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|err| AppError::ValidationError(format!("Invalid zip entry: {err}")))?;
        let Some(enclosed_name) = file.enclosed_name() else {
            return Err(AppError::ValidationError(
                "Plugin package contains an unsafe zip path".to_string(),
            ));
        };
        let safe_path = sanitize_archive_path(&enclosed_name)?;
        let output_path = destination.join(safe_path);
        ensure_within(destination, &output_path)?;
        if file.is_dir() {
            fs::create_dir_all(&output_path).map_err(AppError::Io)?;
        } else {
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent).map_err(AppError::Io)?;
            }
            let mut output = fs::File::create(&output_path).map_err(AppError::Io)?;
            std::io::copy(&mut file, &mut output).map_err(AppError::Io)?;
        }
    }
    Ok(())
}

pub(super) fn sanitize_archive_path(path: &Path) -> Result<PathBuf> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            _ => {
                return Err(AppError::ValidationError(
                    "Plugin package contains an unsafe path".to_string(),
                ));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(AppError::ValidationError(
            "Plugin package contains an empty path".to_string(),
        ));
    }
    Ok(clean)
}

pub(super) fn ensure_within(base: &Path, target: &Path) -> Result<()> {
    if !target.starts_with(base) {
        return Err(AppError::ValidationError(
            "Plugin package entry escapes extraction directory".to_string(),
        ));
    }
    Ok(())
}
