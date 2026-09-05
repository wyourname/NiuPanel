use super::*;

mod archive;
pub(super) use archive::*;

pub(super) struct UploadedPluginPackage {
    pub(super) file_name: String,
    pub(super) upload: UploadedTempFile,
    pub(super) enable: bool,
    pub(super) operation: String,
    pub(super) target_plugin_id: Option<String>,
}

async fn read_upload_text(field: &mut Field<'_>) -> Result<String> {
    let mut value = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|err| AppError::ValidationError(err.to_string()))?
    {
        if value.len().saturating_add(chunk.len()) > MAX_PLUGIN_UPLOAD_METADATA_BYTES {
            return Err(AppError::ValidationError(
                "Plugin upload metadata field is too large".to_string(),
            ));
        }
        value.extend_from_slice(&chunk);
    }
    String::from_utf8(value)
        .map_err(|_| AppError::ValidationError("Plugin upload metadata must be UTF-8".to_string()))
}

pub(super) async fn read_plugin_package_upload(
    mut multipart: Multipart,
    upload_directory: &Path,
) -> Result<UploadedPluginPackage> {
    let mut file_name = None;
    let mut upload = None;
    let mut enable = true;
    let mut checksum_sha256 = None;
    let mut signature_ed25519 = None;
    let mut public_key_ed25519 = None;
    let mut operation = None;
    let mut target_plugin_id = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::ValidationError(err.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "enable" {
            let value = read_upload_text(&mut field).await?;
            enable = matches!(value.as_str(), "true" | "1" | "on");
            continue;
        }
        if name == "operation" {
            let value = read_upload_text(&mut field).await?;
            operation = Some(value.trim().to_string());
            continue;
        }
        if name == "target_plugin_id" {
            let value = read_upload_text(&mut field).await?;
            let value = value.trim().to_string();
            if !value.is_empty() {
                target_plugin_id = Some(value);
            }
            continue;
        }
        if name == "checksum_sha256" {
            let value = read_upload_text(&mut field).await?;
            let value = value.trim().to_ascii_lowercase();
            if !value.is_empty() {
                validate_sha256_hex(&value)?;
                checksum_sha256 = Some(value);
            }
            continue;
        }
        if name == "signature_ed25519" {
            let value = read_upload_text(&mut field).await?;
            let value = value.trim().to_string();
            if !value.is_empty() {
                signature_ed25519 = Some(value);
            }
            continue;
        }
        if name == "public_key_ed25519" {
            let value = read_upload_text(&mut field).await?;
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
            let streamed = stream_field_to_temp_file(
                &mut field,
                TempUploadOptions::new(upload_directory, "package-")
                    .with_max_size(MAX_PLUGIN_PACKAGE_BYTES as u64),
            )
            .await?;
            file_name = Some(current_file_name);
            upload = Some(streamed);
            continue;
        }

        return Err(AppError::ValidationError(format!(
            "Unknown plugin upload field '{name}'"
        )));
    }

    let upload = upload
        .ok_or_else(|| AppError::ValidationError("Missing plugin package file".to_string()))?;
    validate_admin_uploaded_file(
        &upload,
        checksum_sha256.as_deref(),
        signature_ed25519.as_deref(),
        public_key_ed25519.as_deref(),
    )
    .await?;

    Ok(UploadedPluginPackage {
        file_name: file_name
            .ok_or_else(|| AppError::ValidationError("Missing plugin package file".to_string()))?,
        upload,
        enable,
        operation: operation
            .ok_or_else(|| AppError::ValidationError("Missing upload operation".to_string()))?,
        target_plugin_id,
    })
}

async fn validate_admin_uploaded_file(
    upload: &UploadedTempFile,
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
) -> Result<()> {
    if let Some(expected) = checksum_sha256 {
        let expected = expected.trim().to_ascii_lowercase();
        validate_sha256_hex(&expected)?;
        if upload.sha256 != expected {
            return Err(AppError::ValidationError(
                "Plugin package checksum mismatch".to_string(),
            ));
        }
    }
    if signature_ed25519.is_none() && public_key_ed25519.is_none() {
        return Ok(());
    }
    let upload_path: &Path = upload.path.as_ref();
    let bytes = tokio::fs::read(upload_path).await.map_err(AppError::Io)?;
    let signature = signature_ed25519.map(str::to_string);
    let public_key = public_key_ed25519.map(str::to_string);
    tokio::task::spawn_blocking(move || {
        verify_configured_package_signature(&bytes, signature.as_deref(), public_key.as_deref())
    })
    .await
    .map_err(|err| AppError::Internal(format!("Plugin signature task failed: {err}")))?
}

pub(super) fn validate_package_integrity(
    bytes: &[u8],
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
) -> Result<()> {
    validate_package_bytes(bytes, checksum_sha256)?;
    verify_configured_package_signature(bytes, signature_ed25519, public_key_ed25519)
}

/// A package selected by an authenticated administrator is an explicit local
/// trust decision. Verify a signature when the client supplies signing data.
#[cfg(test)]
pub(super) fn validate_admin_upload_integrity(
    bytes: &[u8],
    checksum_sha256: Option<&str>,
    signature_ed25519: Option<&str>,
    public_key_ed25519: Option<&str>,
) -> Result<()> {
    validate_package_bytes(bytes, checksum_sha256)?;
    if signature_ed25519.is_none() && public_key_ed25519.is_none() {
        return Ok(());
    }
    verify_configured_package_signature(bytes, signature_ed25519, public_key_ed25519)
}

fn validate_package_bytes(bytes: &[u8], checksum_sha256: Option<&str>) -> Result<()> {
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
    Ok(())
}

pub(super) fn verify_configured_package_signature(
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
