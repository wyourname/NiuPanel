mod execution;
mod listing;
mod logs;
mod mutate;

use super::models::{LogPagination, LogResponse};
use niupanel_common::error::{AppError, Result};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub struct TaskService;

pub(crate) async fn read_log_response(
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

    let (content, length) = if limit == 0 {
        (String::new(), 0)
    } else {
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(AppError::Io)?;
        let capacity =
            usize::try_from(limit).map_err(|error| AppError::Generic(error.to_string()))?;
        let mut buf = vec![0u8; capacity];
        file.read_exact(&mut buf).await.map_err(AppError::Io)?;
        // A resumed read starts on a character boundary; leave an incomplete suffix
        // for the next chunk so Chinese text is not replaced at chunk boundaries.
        while let Err(error) = std::str::from_utf8(&buf) {
            if error.error_len().is_some() || offset + buf.len() as u64 >= total_size {
                break;
            }
            if error.valid_up_to() > 0 {
                buf.truncate(error.valid_up_to());
                break;
            }
            // Even a one-byte page must advance by a complete character.
            buf.push(file.read_u8().await.map_err(AppError::Io)?);
        }
        let length =
            u64::try_from(buf.len()).map_err(|error| AppError::Generic(error.to_string()))?;
        (String::from_utf8_lossy(&buf).to_string(), length)
    };

    Ok(LogResponse {
        content,
        total_size,
        offset,
        length,
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

#[cfg(test)]
mod log_tests {
    use super::*;

    #[tokio::test]
    async fn resumed_pages_preserve_utf8_and_byte_offsets() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let expected = format!("{}中文🙂\n结尾", "a".repeat(65_535));
        std::fs::write(file.path(), &expected).unwrap();
        for limit in [1, 65_536] {
            let mut offset = 0;
            let mut restored = String::new();
            // Exercise the tiny-page boundary without reading the ASCII prefix one byte at a time.
            if limit == 1 {
                offset = 65_535;
                restored.push_str(&expected[..65_535]);
            }
            while offset < expected.len() as u64 {
                let page = read_log_response(
                    file.path().to_string_lossy().into_owned(),
                    LogPagination {
                        offset: Some(offset),
                        limit: Some(limit),
                    },
                    None,
                )
                .await
                .unwrap();
                assert_eq!(page.offset, offset);
                assert!(page.length > 0);
                offset += page.length;
                restored.push_str(&page.content);
            }
            assert_eq!(restored, expected);
        }
    }
}
