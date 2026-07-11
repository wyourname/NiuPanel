#[cfg(feature = "axum")]
use crate::response::ApiResponse;
#[cfg(feature = "axum")]
use axum::response::{IntoResponse, Response};
use thiserror::Error;
#[cfg(feature = "axum")]
use tracing::error;

#[derive(Error, Debug)]
pub enum AppError {
    // --- 1. System/Internal Errors (Usually 5xx) ---
    #[error("Internal database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Internal IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP request error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] envy::Error),

    #[error("Process start failed for '{command}': {source}")]
    ProcessStart {
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Process '{command}' failed (exit code {exit_code:?}): {stderr}")]
    ProcessFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Process has no PID")]
    ProcessHasNoPid,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("External API error: {0}")]
    ExternalApi(String),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("Invalid input or parameter: {0}")]
    ValidationError(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Cancelled")]
    Cancelled,

    // --- 2. Business/User Errors (Usually 4xx) ---
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Auth(String),

    #[error("Token expired")]
    ExpiredToken(String),

    #[error("Share token expired")]
    ExpiredShareToken(String),

    #[error("Permission denied")]
    Forbidden(String),

    #[error("File size limit exceeded: {0}")]
    FileSizeLimitExceeded(String),

    #[error("Download limit reached: {0}")]
    LimitDownload(String),

    #[error("Concurrency limit reached: {0}")]
    ConcurrencyLimitExceeded(String),

    #[error("Environment setup error: {0}")]
    Environment(String),

    #[error("Python environment error: {0}")]
    PythonEnv(String),

    #[error("Station error:{0}")]
    Station(String),

    // --- 3. Legacy/Generic Support ---
    #[error("{0}")]
    Generic(String),
}

impl AppError {
    /// Helper to get the semantic error code and HTTP status hint
    pub fn get_meta(&self) -> (u16, i32, String) {
        match self {
            // System Errors -> 500
            AppError::Database(e) => (500, 5001, format!("Database error: {}", e)),
            AppError::Io(e) => (500, 5002, format!("IO error: {}", e)),
            AppError::Json(e) => (500, 5012, format!("JSON error: {}", e)),
            AppError::Reqwest(e) => (500, 5013, format!("HTTP error: {}", e)),
            AppError::Config(e) => (500, 5003, format!("Configuration error: {}", e)),
            AppError::ProcessStart { command, source } => (
                500,
                5004,
                format!("Failed to start process '{}': {}", command, source),
            ),
            AppError::ProcessFailed {
                command,
                exit_code,
                stderr,
            } => (
                500,
                5005,
                format!("Process '{}' failed ({:?}): {}", command, exit_code, stderr),
            ),
            AppError::ProcessHasNoPid => (500, 5006, "Process missing PID".to_string()),
            AppError::Serialization(e) => (500, 5012, e.clone()),
            AppError::ExternalApi(e) => (500, 5013, e.clone()),
            AppError::PasswordHash(e) => (500, 5014, e.clone()),
            AppError::ValidationError(e) => (400, 400, e.clone()),
            AppError::Internal(e) => (500, 5000, e.clone()),
            AppError::Cancelled => (400, 400, "Operation cancelled".to_string()),

            // User Errors -> 4xx
            AppError::NotFound(e) => (404, 404, e.clone()),
            AppError::Auth(e) => (401, 401, e.clone()),
            AppError::ExpiredToken(e) => (401, 401, e.clone()),
            AppError::ExpiredShareToken(e) => (401, 5021, e.clone()),
            AppError::Forbidden(e) => (403, 403, e.clone()),
            AppError::FileSizeLimitExceeded(e) => (400, 400, e.clone()),
            AppError::LimitDownload(e) => (403, 5020, e.clone()),
            AppError::ConcurrencyLimitExceeded(e) => (429, 5015, e.clone()),
            AppError::Environment(e) => (400, 5008, e.clone()),
            AppError::PythonEnv(e) => (400, 5009, e.clone()),
            AppError::Station(e) => (400, 5017, e.clone()),

            // Generic mapping
            AppError::Generic(e) => (400, 400, e.clone()),
        }
    }
}

#[cfg(feature = "axum")]
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, biz_code, message) = self.get_meta();

        //如果是 5xx 错误，自动记录日志
        if status_code >= 500 {
            error!(
                error_type = ?self,
                biz_code = biz_code,
                "Internal System Error: {}", message
            );
        }

        ApiResponse::<()>::error(biz_code, message).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
