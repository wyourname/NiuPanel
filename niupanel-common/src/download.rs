use crate::error::{AppError, Result};
use futures::StreamExt;
use reqwest::header::HeaderMap;
use std::future::Future;
use std::path::Path;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct DownloadOptions {
    pub headers: HeaderMap,
    pub timeout: Option<Duration>,
    pub retries: u8,
    pub retry_delay: Duration,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            headers: HeaderMap::new(),
            timeout: None,
            retries: 1,
            retry_delay: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Attempt { attempt: u8, resumed_from: u64 },
    Progress(DownloadProgress),
    Completed { total_size: u64 },
}

#[derive(Debug, Clone, Copy)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total_size: u64,
    pub percent: u8,
    pub attempt: u8,
}

pub async fn download_with_resume<F, Fut, C>(
    client: &reqwest::Client,
    url: &str,
    dest_path: &Path,
    expected_size: Option<u64>,
    options: DownloadOptions,
    should_cancel: C,
    mut on_event: F,
) -> Result<()>
where
    F: FnMut(DownloadEvent) -> Fut,
    Fut: Future<Output = ()>,
    C: Fn() -> bool,
{
    let retries = options.retries.max(1);
    let mut last_error: Option<String> = None;

    for attempt in 1..=retries {
        if should_cancel() {
            return Err(AppError::Cancelled);
        }

        let existing_size = tokio::fs::metadata(dest_path)
            .await
            .map(|meta| meta.len())
            .unwrap_or(0);
        on_event(DownloadEvent::Attempt {
            attempt,
            resumed_from: existing_size,
        })
        .await;

        let mut request = client.get(url);
        if let Some(timeout) = options.timeout {
            request = request.timeout(timeout);
        }
        if !options.headers.is_empty() {
            request = request.headers(options.headers.clone());
        }
        if existing_size > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={}-", existing_size));
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                last_error = Some(err.to_string());
                if attempt < retries {
                    tokio::time::sleep(options.retry_delay).await;
                    continue;
                }
                break;
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            if expected_size.is_none() || existing_size >= expected_size.unwrap_or(0) {
                let total_size = expected_size.unwrap_or(existing_size);
                on_event(DownloadEvent::Completed { total_size }).await;
                return Ok(());
            }
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| status.to_string());
            last_error = Some(format!("HTTP {}: {}", status, body));
            if attempt < retries {
                tokio::time::sleep(options.retry_delay).await;
                continue;
            }
            break;
        }

        let resumed = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut downloaded = if resumed { existing_size } else { 0 };
        let total_size = parse_total_size(&response, expected_size, downloaded);
        let mut file = if resumed {
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(dest_path)
                .await
                .map_err(AppError::Io)?
        } else {
            tokio::fs::File::create(dest_path)
                .await
                .map_err(AppError::Io)?
        };

        let mut stream = response.bytes_stream();
        let mut stream_error: Option<String> = None;

        while let Some(item) = stream.next().await {
            if should_cancel() {
                return Err(AppError::Cancelled);
            }

            match item {
                Ok(chunk) => {
                    file.write_all(&chunk).await.map_err(AppError::Io)?;
                    downloaded += chunk.len() as u64;

                    let percent = calculate_percent(downloaded, total_size);
                    on_event(DownloadEvent::Progress(DownloadProgress {
                        downloaded,
                        total_size,
                        percent,
                        attempt,
                    }))
                    .await;
                }
                Err(err) => {
                    stream_error = Some(err.to_string());
                    break;
                }
            }
        }

        file.flush().await.map_err(AppError::Io)?;

        if stream_error.is_none() && (total_size == 0 || downloaded >= total_size) {
            on_event(DownloadEvent::Completed { total_size }).await;
            return Ok(());
        }

        last_error = Some(stream_error.unwrap_or_else(|| {
            format!(
                "Connection closed before full download completed ({}/{})",
                downloaded, total_size
            )
        }));
        if attempt < retries {
            tokio::time::sleep(options.retry_delay).await;
        }
    }

    Err(AppError::ExternalApi(format!(
        "Download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn calculate_percent(downloaded: u64, total_size: u64) -> u8 {
    if total_size == 0 {
        return 0;
    }

    ((downloaded as f64 / total_size as f64) * 100.0) as u8
}

fn parse_total_size(
    response: &reqwest::Response,
    expected_size: Option<u64>,
    resumed_size: u64,
) -> u64 {
    if let Some(content_range) = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split('/').nth(1))
        .and_then(|value| value.parse::<u64>().ok())
    {
        return content_range;
    }

    if let Some(expected_size) = expected_size {
        if expected_size > 0 {
            return expected_size;
        }
    }

    resumed_size + response.content_length().unwrap_or(0)
}
