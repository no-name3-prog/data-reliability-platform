//! API error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use drp_common::Error as PlatformError;

/// JSON error body.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// Machine-readable code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// API-layer error wrapper.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorBody,
}

impl ApiError {
    /// Construct from platform error.
    pub fn from_platform(err: PlatformError) -> Self {
        let status =
            StatusCode::from_u16(err.status_hint()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        Self {
            status,
            body: ErrorBody {
                code: err.code().to_string(),
                message: err.to_string(),
            },
        }
    }

    /// 400 Bad Request helper.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ErrorBody {
                code: "validation_error".into(),
                message: message.into(),
            },
        }
    }
}

impl From<PlatformError> for ApiError {
    fn from(value: PlatformError) -> Self {
        Self::from_platform(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

/// Result alias for handlers.
pub type ApiResult<T> = Result<T, ApiError>;
