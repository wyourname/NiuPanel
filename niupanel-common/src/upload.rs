use crate::error::{AppError, Result};
use axum::{body::Bytes, extract::multipart::Field};
use futures::{Stream, StreamExt};
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use tempfile::TempPath;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct UploadedTempFile {
    pub path: TempPath,
    pub size: u64,
    pub sha256: String,
}

impl UploadedTempFile {
    pub fn into_persisted_path(self) -> Result<PathBuf> {
        self.path.keep().map_err(|error| AppError::Io(error.error))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TempUploadOptions<'a> {
    pub directory: &'a Path,
    pub prefix: &'a str,
    pub max_size: Option<u64>,
}

impl<'a> TempUploadOptions<'a> {
    pub fn new(directory: &'a Path, prefix: &'a str) -> Self {
        Self {
            directory,
            prefix,
            max_size: None,
        }
    }

    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }
}

pub async fn stream_field_to_temp_file(
    field: &mut Field<'_>,
    options: TempUploadOptions<'_>,
) -> Result<UploadedTempFile> {
    stream_chunks_to_temp_file(field, options).await
}

pub async fn stream_chunks_to_temp_file<S, E>(
    chunks: S,
    options: TempUploadOptions<'_>,
) -> Result<UploadedTempFile>
where
    S: Stream<Item = std::result::Result<Bytes, E>>,
    E: Display,
{
    tokio::fs::create_dir_all(options.directory)
        .await
        .map_err(AppError::Io)?;
    let file = tempfile::Builder::new()
        .prefix(options.prefix)
        .tempfile_in(options.directory)
        .map_err(AppError::Io)?;
    let (file, path) = file.into_parts();
    let mut output = tokio::fs::File::from_std(file);
    let mut size = 0_u64;
    let mut hasher = Sha256::new();
    futures::pin_mut!(chunks);

    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|error| AppError::ValidationError(error.to_string()))?;
        size = size.saturating_add(chunk.len() as u64);
        if let Some(limit) = options.max_size
            && size > limit
        {
            return Err(AppError::FileSizeLimitExceeded(format!(
                "Uploaded file exceeds the {limit} byte limit"
            )));
        }
        hasher.update(&chunk);
        output.write_all(&chunk).await.map_err(AppError::Io)?;
    }

    output.sync_all().await.map_err(AppError::Io)?;
    drop(output);
    Ok(UploadedTempFile {
        path,
        size,
        sha256: hex::encode(hasher.finalize()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn writes_large_input_incrementally_and_hashes_it() {
        const CHUNK_SIZE: usize = 64 * 1024;
        const CHUNK_COUNT: usize = 256;
        let directory = tempfile::tempdir().unwrap();
        let chunks = stream::iter(
            (0..CHUNK_COUNT)
                .map(|_| Ok::<Bytes, std::io::Error>(Bytes::from(vec![0x5a; CHUNK_SIZE]))),
        );

        let upload = stream_chunks_to_temp_file(
            chunks,
            TempUploadOptions::new(directory.path(), ".stream-test-"),
        )
        .await
        .unwrap();

        let mut expected = Sha256::new();
        for _ in 0..CHUNK_COUNT {
            expected.update(vec![0x5a; CHUNK_SIZE]);
        }
        assert_eq!(upload.size, (CHUNK_SIZE * CHUNK_COUNT) as u64);
        assert_eq!(upload.sha256, hex::encode(expected.finalize()));
        let upload_path: &Path = upload.path.as_ref();
        assert_eq!(std::fs::metadata(upload_path).unwrap().len(), upload.size);
    }

    #[tokio::test]
    async fn removes_partial_file_when_size_limit_is_exceeded() {
        let directory = tempfile::tempdir().unwrap();
        let chunks = stream::iter([
            Ok::<Bytes, std::io::Error>(Bytes::from_static(b"1234")),
            Ok(Bytes::from_static(b"5678")),
        ]);

        let error = stream_chunks_to_temp_file(
            chunks,
            TempUploadOptions::new(directory.path(), ".limit-test-").with_max_size(6),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, AppError::FileSizeLimitExceeded(_)));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn removes_partial_file_when_input_stream_fails() {
        let directory = tempfile::tempdir().unwrap();
        let chunks = stream::iter([
            Ok(Bytes::from_static(b"partial")),
            Err::<Bytes, _>(std::io::Error::other("interrupted")),
        ]);

        let error = stream_chunks_to_temp_file(
            chunks,
            TempUploadOptions::new(directory.path(), ".error-test-"),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("interrupted"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
