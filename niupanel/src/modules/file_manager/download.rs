use axum::{
    body::{Body, Bytes},
    http::{HeaderValue, header},
    response::Response,
};
use niupanel_common::{
    error::{AppError, Result},
    filesystem::resolve_path,
};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use tokio::sync::mpsc;

pub(super) fn attachment(filename: &str) -> Result<HeaderValue> {
    let encoded: String = filename
        .as_bytes()
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect();
    HeaderValue::from_str(&format!(
        "attachment; filename=\"download\"; filename*=UTF-8''{encoded}"
    ))
    .map_err(|error| AppError::Generic(error.to_string()))
}

pub(super) fn resolve_batch(paths: &[String]) -> Result<Vec<PathBuf>> {
    if paths.is_empty() {
        return Err(AppError::Generic("请选择需要下载的文件".into()));
    }
    paths
        .iter()
        .map(|path| {
            let resolved = resolve_path(path)?;
            if !resolved.exists() {
                return Err(AppError::NotFound(format!("文件不存在：{path}")));
            }
            Ok(resolved)
        })
        .collect()
}

struct ArchiveWriter(mpsc::Sender<io::Result<Bytes>>);

impl Write for ArchiveWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // Bound both each allocation and queued data. Dropping the response stops the producer.
        for chunk in bytes.chunks(64 * 1024) {
            self.0
                .blocking_send(Ok(Bytes::copy_from_slice(chunk)))
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "下载已取消"))?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn archive_stream(paths: Vec<PathBuf>) -> mpsc::Receiver<io::Result<Bytes>> {
    let (sender, receiver) = mpsc::channel(2);
    tokio::task::spawn_blocking(move || {
        let result = (|| -> io::Result<()> {
            let mut archive = tar::Builder::new(ArchiveWriter(sender.clone()));
            archive.follow_symlinks(false);
            for path in paths {
                let name = path
                    .file_name()
                    .ok_or_else(|| io::Error::other("无效的文件名"))?;
                if path.is_dir() {
                    archive.append_dir_all(name, &path)?;
                } else {
                    archive.append_path_with_name(&path, name)?;
                }
            }
            archive.finish()
        })();
        if let Err(error) = result {
            let _ = sender.blocking_send(Err(error));
        }
    });
    receiver
}

pub(super) fn batch_response(paths: Vec<PathBuf>) -> Result<Response> {
    let mut receiver = archive_stream(paths);
    let stream =
        async_stream::stream! { while let Some(chunk) = receiver.recv().await { yield chunk; } };
    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/x-tar"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        attachment(&format!(
            "files_{}.tar",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ))?,
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn streams_archive_and_stops_after_cancel() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("中文.txt");
        let content = vec![b'x'; 300_000];
        std::fs::write(&path, &content).unwrap();
        let mut receiver = archive_stream(vec![path.clone()]);
        let mut data = Vec::new();
        while let Some(chunk) = receiver.recv().await {
            let chunk = chunk.unwrap();
            assert!(chunk.len() <= 64 * 1024);
            data.extend_from_slice(&chunk);
        }
        let mut archive = tar::Archive::new(data.as_slice());
        let mut entries = archive.entries().unwrap();
        let mut entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.path().unwrap().to_string_lossy(), "中文.txt");
        let mut restored = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut restored).unwrap();
        assert_eq!(restored, content);

        let (sender, receiver) = mpsc::channel(2);
        drop(receiver);
        let result = tokio::task::spawn_blocking(move || ArchiveWriter(sender).write(b"cancelled"))
            .await
            .unwrap();
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn encodes_unicode_and_header_characters() {
        let header = attachment("测试\"\r\n.py").unwrap();
        let value = header.to_str().unwrap();
        assert!(value.contains("filename*=UTF-8''"));
        assert!(value.contains("%22%0D%0A%2E%70%79"));
        assert!(!value.contains('\n'));
    }
}
