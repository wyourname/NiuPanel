use axum::extract::Multipart;
use axum::extract::multipart::Field;
use flate2::read::GzDecoder;
use niupanel_common::config::Config;
use niupanel_common::error::{AppError, Result};
use niupanel_common::upload::{TempUploadOptions, UploadedTempFile, stream_field_to_temp_file};
use openssl::base64;
use openssl::pkey::{Id, PKey, Public};
use openssl::sign::Verifier;
use sha2::{Digest, Sha256};
use std::fs;
use std::future::Future;
use std::io::{Cursor, Read, Seek};
use std::path::{Component, Path, PathBuf};
use tempfile::TempDir;
use zip::ZipArchive;

pub const MAX_PLUGIN_PACKAGE_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_PLUGIN_EXTRACTED_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_PLUGIN_ARCHIVE_ENTRIES: usize = 20_000;
const MAX_PLUGIN_UPLOAD_METADATA_BYTES: usize = 16 * 1024;

mod package;
mod upload;

use package::*;
pub use upload::*;

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::pkey::PKey;
    use openssl::sign::Signer;

    #[test]
    fn verifies_trusted_ed25519_signature() {
        let key = PKey::generate_ed25519().expect("generate ed25519 key");
        let public_key = key.raw_public_key().expect("raw public key");
        let data = b"plugin package bytes";
        let mut signer = Signer::new_without_digest(&key).expect("signer");
        let signature = signer.sign_oneshot_to_vec(data).expect("sign data");

        let trusted = vec![format!(
            "sha256:{}",
            hex::encode(Sha256::digest(&public_key))
        )];
        verify_package_signature_with_trust(
            data,
            &base64::encode_block(&signature),
            &base64::encode_block(&public_key),
            &trusted,
        )
        .expect("verified signature");
    }

    #[test]
    fn rejects_untrusted_ed25519_signature() {
        let key = PKey::generate_ed25519().expect("generate ed25519 key");
        let public_key = key.raw_public_key().expect("raw public key");
        let data = b"plugin package bytes";
        let mut signer = Signer::new_without_digest(&key).expect("signer");
        let signature = signer.sign_oneshot_to_vec(data).expect("sign data");

        let err = verify_package_signature_with_trust(
            data,
            &base64::encode_block(&signature),
            &base64::encode_block(&public_key),
            &[
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            ],
        )
        .expect_err("untrusted key rejected");

        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[test]
    fn accepts_unsigned_authenticated_admin_upload() {
        validate_admin_upload_integrity(b"plugin package bytes", None, None, None)
            .expect("unsigned uploads are accepted when signatures are not required");
    }

    #[test]
    fn rejects_archive_size_and_entry_limit_overflow() {
        let mut size_stats = ExtractionStats {
            entries: 0,
            bytes: 0,
        };
        let size_error = account_archive_entry(&mut size_stats, MAX_PLUGIN_EXTRACTED_BYTES + 1)
            .expect_err("oversized extraction rejected");
        assert!(matches!(size_error, AppError::FileSizeLimitExceeded(_)));

        let mut entry_stats = ExtractionStats {
            entries: MAX_PLUGIN_ARCHIVE_ENTRIES,
            bytes: 0,
        };
        let entry_error = account_archive_entry(&mut entry_stats, 0)
            .expect_err("too many archive entries rejected");
        assert!(matches!(entry_error, AppError::FileSizeLimitExceeded(_)));
    }
}
