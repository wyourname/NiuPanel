use niupanel_common::error::AppError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config parse error: {0}")]
    ConfigError(#[from] serde_json::Error),

    #[error("WebSocket error: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Handshake error: {0}")]
    HandshakeError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl From<Errors> for AppError {
    fn from(err: Errors) -> Self {
        match err {
            Errors::IoError(e) => AppError::Io(e),
            Errors::ConfigError(e) => AppError::Json(e),
            Errors::WsError(e) => AppError::ExternalApi(format!("WebSocket error: {}", e)),
            Errors::TlsError(e) => AppError::ExternalApi(format!("TLS error: {}", e)),
            Errors::HandshakeError(e) => AppError::ExternalApi(format!("Handshake error: {}", e)),
            Errors::AuthError(e) => AppError::ExternalApi(format!("Auth error: {}", e)),
            Errors::InvalidHeader(e) => AppError::ExternalApi(format!("Invalid header: {}", e)),
            Errors::ValidationError(e) => AppError::ExternalApi(format!("Validation error: {}", e)),
        }
    }
}
