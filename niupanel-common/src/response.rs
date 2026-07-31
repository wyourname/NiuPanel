#[cfg(feature = "axum")]
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct PaginatedData<T> {
    pub items: Vec<T>,
    pub total: u64,
}

impl<T> ApiResponse<T> {
    pub fn new(code: i32, message: String, data: Option<T>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    pub fn success(data: T) -> Self {
        Self::new(0, "Success".to_string(), Some(data))
    }

    pub fn error(code: i32, message: String) -> Self {
        Self::new(code, message, None)
    }
}

#[cfg(feature = "axum")]
impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = match self.code {
            0 => StatusCode::OK,
            401 => StatusCode::UNAUTHORIZED,
            403 | 5020 | 5021 => StatusCode::FORBIDDEN,
            404 => StatusCode::NOT_FOUND,
            400 | 5008 | 5009 | 5017 => StatusCode::BAD_REQUEST,
            _ => {
                if self.code >= 5000 {
                    StatusCode::INTERNAL_SERVER_ERROR
                } else if self.code >= 400 && self.code < 500 {
                    StatusCode::from_u16(self.code as u16).unwrap_or(StatusCode::BAD_REQUEST)
                } else {
                    StatusCode::OK
                }
            }
        };
        (status, Json(self)).into_response()
    }
}
