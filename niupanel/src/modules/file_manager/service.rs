use super::archive;
use super::models::{
    CopyItemRequest, CreateDirectoryRequest, CreateFileRequest, DownloadBatchRequest,
    DownloadUrlRequest, ExtractArchiveRequest, FileSystemItem, RenameItemRequest, WriteFileRequest,
};
use axum::extract::Multipart;
use axum::response::Response;
use axum::{body::Body, http};
use chrono::Local;
use niupanel_common::error::{AppError, Result};
use niupanel_common::filesystem::{get_relative_path, resolve_path};
use niupanel_common::info;
use sha2::Digest;
use tokio::fs;

pub struct FileManagerService;

impl FileManagerService {
    pub async fn list_directory(
        relative_path: String,
        query: Option<String>,
    ) -> Result<Vec<FileSystemItem>> {
        let target_path = resolve_path(&relative_path)?;
        let mut items = Vec::new();

        if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
            let q_lower = q.to_lowercase();
            let search_items =
                tokio::task::spawn_blocking(move || -> Result<Vec<FileSystemItem>> {
                    let mut results = Vec::new();
                    for entry in walkdir::WalkDir::new(&target_path) {
                        let entry = entry.map_err(|err| {
                            AppError::Io(
                                err.into_io_error()
                                    .unwrap_or_else(|| std::io::Error::other("遍历搜索目录失败")),
                            )
                        })?;
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&q_lower)
                        {
                            let metadata = entry.metadata().map_err(|err| {
                                AppError::Io(err.into_io_error().unwrap_or_else(|| {
                                    std::io::Error::other("读取搜索结果元数据失败")
                                }))
                            })?;
                            let is_dir = metadata.is_dir();
                            let size = metadata.len();
                            let mtime = metadata
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs() as i64);

                            let name = entry.file_name().to_string_lossy().to_string();
                            let path_buf = entry.path().to_path_buf();
                            let rel_path = get_relative_path(&path_buf)?;
                            results.push(FileSystemItem {
                                name,
                                path: rel_path,
                                size,
                                is_dir,
                                mtime,
                            });

                            if results.len() >= 1000 {
                                break;
                            }
                        }
                    }
                    Ok(results)
                })
                .await
                .map_err(|e| AppError::Generic(e.to_string()))??;

            items.extend(search_items);
        } else {
            let mut entries = fs::read_dir(&target_path).await.map_err(AppError::Io)?;
            while let Some(entry) = entries.next_entry().await.map_err(AppError::Io)? {
                let path = entry.path();
                let file_type = entry.file_type().await.map_err(AppError::Io)?;
                let name = path
                    .file_name()
                    .and_then(|os_str| os_str.to_str())
                    .ok_or(AppError::Generic("Invalid filename".to_string()))?
                    .to_string();

                let metadata = entry.metadata().await.map_err(AppError::Io)?;
                let size = metadata.len();
                let mtime = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64);

                items.push(FileSystemItem {
                    name,
                    path: get_relative_path(&path)?,
                    size,
                    is_dir: file_type.is_dir(),
                    mtime,
                });
            }
        }

        Ok(items)
    }

    pub async fn read_file_content(relative_path: String) -> Result<String> {
        let target_path = resolve_path(&relative_path)?;
        let metadata = fs::metadata(&target_path).await.map_err(AppError::Io)?;
        if metadata.len() > 2 * 1024 * 1024 {
            return Err(AppError::Generic(
                "文件过大 (超过 2MB)，请使用下载功能查看。".to_string(),
            ));
        }

        fs::read_to_string(&target_path).await.map_err(AppError::Io)
    }

    pub async fn write_file_content(payload: WriteFileRequest) -> Result<()> {
        let target_path = resolve_path(&payload.path)?;
        fs::write(&target_path, &payload.content)
            .await
            .map_err(AppError::Io)?;
        Ok(())
    }

    pub async fn resolve_download_file(
        relative_path: String,
    ) -> Result<(std::path::PathBuf, String)> {
        let target_path = resolve_path(&relative_path)?;
        if !target_path.exists() {
            return Err(AppError::NotFound("File not found".to_string()));
        }
        if !target_path.is_file() {
            return Err(AppError::Generic("Not a file".to_string()));
        }

        let filename = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();

        Ok((target_path, filename))
    }

    pub async fn upload_file(relative_path: String, mut multipart: Multipart) -> Result<()> {
        let target_dir = resolve_path(&relative_path)?;

        if !target_dir.exists() {
            fs::create_dir_all(&target_dir)
                .await
                .map_err(AppError::Io)?;
        } else if !target_dir.is_dir() {
            return Err(AppError::Generic(
                "Target path is not a directory".to_string(),
            ));
        }

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| AppError::Generic(e.to_string()))?
        {
            let file_name = field
                .file_name()
                .ok_or(AppError::Generic("No filename provided".to_string()))?
                .to_string();

            if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
                return Err(AppError::Generic(
                    "Invalid filename: contains slashes or traversal components".to_string(),
                ));
            }

            let file_path = target_dir.join(&file_name);
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::Generic(e.to_string()))?;

            if file_path.exists() && file_path.is_file() {
                let existing_data = fs::read(&file_path).await.map_err(AppError::Io)?;

                let mut hasher = sha2::Sha256::new();
                sha2::Digest::update(&mut hasher, &existing_data);
                let existing_hash = sha2::Digest::finalize(hasher);

                let mut hasher = sha2::Sha256::new();
                sha2::Digest::update(&mut hasher, &data);
                let new_hash = sha2::Digest::finalize(hasher);

                if existing_hash != new_hash {
                    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
                    let backup_path = target_dir.join(format!("{}.bak_{}", file_name, timestamp));
                    info!(
                        "File content changed for {}, creating backup at {:?}",
                        file_name, backup_path
                    );
                    fs::rename(&file_path, &backup_path)
                        .await
                        .map_err(AppError::Io)?;
                } else {
                    info!(
                        "File content identical for {}, skipping overwrite/backup",
                        file_name
                    );
                    continue;
                }
            }

            fs::write(&file_path, &data).await.map_err(AppError::Io)?;
        }

        Ok(())
    }

    pub async fn create_file(payload: CreateFileRequest) -> Result<()> {
        let target_path = resolve_path(&payload.path)?;
        fs::write(&target_path, payload.content.unwrap_or_default())
            .await
            .map_err(AppError::Io)?;
        Ok(())
    }

    pub async fn create_directory(payload: CreateDirectoryRequest) -> Result<()> {
        let target_path = resolve_path(&payload.path)?;
        fs::create_dir(&target_path).await.map_err(AppError::Io)?;
        Ok(())
    }

    pub async fn delete_item(relative_path: String) -> Result<()> {
        if relative_path.trim().is_empty() || relative_path == "/" || relative_path == "." {
            return Err(AppError::Generic(
                "Cannot delete root directory".to_string(),
            ));
        }

        let target_path = resolve_path(&relative_path)?;
        if target_path.is_dir() {
            fs::remove_dir_all(&target_path)
                .await
                .map_err(AppError::Io)?;
        } else {
            fs::remove_file(&target_path).await.map_err(AppError::Io)?;
        }
        Ok(())
    }

    pub async fn rename_item(payload: RenameItemRequest) -> Result<()> {
        let old_path = resolve_path(&payload.old_path)?;
        let new_path = resolve_path(&payload.new_path)?;

        match fs::rename(&old_path, &new_path).await {
            Ok(_) => Ok(()),
            Err(e) => {
                info!(
                    "Failed to rename {:?} to {:?} (Error: {}). Falling back to copy then delete.",
                    old_path, new_path, e
                );
                if old_path.is_dir() {
                    copy_dir_recursive(&old_path, &new_path)
                        .await
                        .map_err(AppError::Io)?;
                    fs::remove_dir_all(&old_path).await.map_err(AppError::Io)?;
                } else {
                    fs::copy(&old_path, &new_path).await.map_err(AppError::Io)?;
                    fs::remove_file(&old_path).await.map_err(AppError::Io)?;
                }
                Ok(())
            }
        }
    }

    pub async fn copy_item(payload: CopyItemRequest) -> Result<()> {
        let from_path = resolve_path(&payload.from_path)?;
        let mut to_path = resolve_path(&payload.to_path)?;

        if to_path.exists() && to_path.is_dir() {
            if let Some(file_name) = from_path.file_name() {
                to_path = to_path.join(file_name);
            }
        }

        if from_path.is_dir() {
            copy_dir_recursive(&from_path, &to_path)
                .await
                .map_err(AppError::Io)?;
        } else {
            fs::copy(&from_path, &to_path).await.map_err(AppError::Io)?;
        }
        Ok(())
    }

    pub async fn extract_archive(payload: ExtractArchiveRequest) -> Result<usize> {
        let source_path = resolve_path(&payload.path)?;
        let destination_path = match payload.destination_path {
            Some(path) if !path.trim().is_empty() => resolve_path(&path)?,
            _ => source_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .ok_or_else(|| {
                    AppError::Generic("Archive file has no parent directory".to_string())
                })?,
        };

        if !destination_path.exists() {
            fs::create_dir_all(&destination_path)
                .await
                .map_err(AppError::Io)?;
        }
        if !destination_path.is_dir() {
            return Err(AppError::Generic(
                "Destination path is not a directory".to_string(),
            ));
        }

        tokio::task::spawn_blocking(move || {
            archive::extract_archive(&source_path, &destination_path)
        })
        .await
        .map_err(|error| AppError::Generic(error.to_string()))?
    }

    pub async fn download_from_url(
        http_client: &reqwest::Client,
        payload: DownloadUrlRequest,
    ) -> Result<()> {
        let target_dir = resolve_path(&payload.path)?;
        if !target_dir.is_dir() {
            return Err(AppError::Generic(
                "Target path is not a directory or does not exist".to_string(),
            ));
        }

        let url = payload.url;
        let filename = if let Some(name) = payload.filename {
            name
        } else {
            url.split('/')
                .last()
                .unwrap_or("downloaded_file")
                .to_string()
        };

        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(AppError::Generic(
                "Invalid filename: contains slashes or traversal components".to_string(),
            ));
        }

        let target_path = target_dir.join(&filename);
        let response = http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Generic(e.to_string()))?;
        if !response.status().is_success() {
            return Err(AppError::Generic(format!(
                "Download failed: {}",
                response.status()
            )));
        }

        let content = response
            .bytes()
            .await
            .map_err(|e| AppError::Generic(e.to_string()))?;
        tokio::fs::write(&target_path, &content)
            .await
            .map_err(AppError::Io)?;
        Ok(())
    }

    pub async fn download_batch(payload: DownloadBatchRequest) -> Result<Response> {
        if payload.paths.is_empty() {
            return Err(AppError::Generic("No paths provided".to_string()));
        }

        let mut resolved_paths = Vec::new();
        for rel_path in &payload.paths {
            let path = resolve_path(rel_path)?;
            if !path.exists() {
                return Err(AppError::NotFound(format!("Path not found: {}", rel_path)));
            }
            resolved_paths.push((rel_path.clone(), path));
        }

        let mut tar_builder = tar::Builder::new(Vec::new());
        for (_rel_path, abs_path) in resolved_paths {
            let base_name = abs_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            if abs_path.is_dir() {
                tar_builder
                    .append_dir_all(&base_name, &abs_path)
                    .map_err(|e| AppError::Generic(e.to_string()))?;
            } else {
                let mut file = std::fs::File::open(&abs_path).map_err(AppError::Io)?;
                tar_builder
                    .append_file(&base_name, &mut file)
                    .map_err(|e| AppError::Generic(e.to_string()))?;
            }
        }

        let tar_data = tar_builder
            .into_inner()
            .map_err(|e| AppError::Generic(e.to_string()))?;
        let filename = format!(
            "batch_download_{}.tar",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );

        let mut res = Response::new(Body::from(tar_data));
        res.headers_mut().insert(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("application/x-tar"),
        );
        res.headers_mut().insert(
            http::header::CONTENT_DISPOSITION,
            http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
                .map_err(|_| AppError::Generic("Invalid filename".to_string()))?,
        );
        Ok(res)
    }
}

async fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).await?;
    }
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let entry_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry_path.is_dir() {
            Box::pin(copy_dir_recursive(&entry_path, &dst_path)).await?;
        } else {
            fs::copy(&entry_path, &dst_path).await?;
        }
    }
    Ok(())
}

pub fn no_cache_headers() -> http::HeaderMap {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CACHE_CONTROL,
        http::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
    );
    headers.insert(
        http::header::PRAGMA,
        http::HeaderValue::from_static("no-cache"),
    );
    headers.insert(http::header::EXPIRES, http::HeaderValue::from_static("0"));
    headers
}
