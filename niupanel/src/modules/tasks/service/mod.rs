mod execution;
mod listing;
mod logs;
mod mutate;

use super::models::{LogPagination, LogResponse};
use niupanel_common::error::{AppError, Result};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub struct TaskService;

async fn read_log_response(
    path: String,
    pagination: LogPagination,
    default_tail_limit: Option<u64>,
) -> Result<LogResponse> {
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| AppError::NotFound("日志文件不存在".to_string()))?;
    let metadata = file.metadata().await.map_err(AppError::Io)?;
    let total_size = metadata.len();

    let (offset, limit) = match (pagination.offset, pagination.limit, default_tail_limit) {
        (Some(offset), Some(limit), _) => (offset, limit),
        (Some(offset), None, _) => (offset, total_size.saturating_sub(offset)),
        (None, Some(limit), _) => (total_size.saturating_sub(limit), limit),
        (None, None, Some(default_limit)) => {
            (total_size.saturating_sub(default_limit), default_limit)
        }
        (None, None, None) => (0, total_size),
    };

    let offset = offset.min(total_size);
    let limit = limit.min(total_size - offset);

    let content = if limit == 0 {
        String::new()
    } else {
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(AppError::Io)?;
        let mut buf = vec![0u8; limit as usize];
        file.read_exact(&mut buf).await.map_err(AppError::Io)?;
        String::from_utf8_lossy(&buf).to_string()
    };

    Ok(LogResponse {
        content,
        total_size,
        offset,
        length: limit,
    })
}

fn infer_env_type(filename: &str, content: &[u8]) -> &'static str {
    if filename.ends_with(".py") {
        return "Python";
    }
    if filename.ends_with(".js") || filename.ends_with(".ts") {
        return "Nodejs";
    }
    if filename.ends_with(".sh") {
        return "Shell";
    }

    let prefix = String::from_utf8_lossy(&content[0..std::cmp::min(content.len(), 100)]);
    if prefix.starts_with("#!") {
        if prefix.contains("python") {
            "Python"
        } else if prefix.contains("node") {
            "Nodejs"
        } else {
            "Shell"
        }
    } else {
        "Shell"
    }
}
