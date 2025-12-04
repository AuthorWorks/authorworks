//! Error types for the Storage Service

use spin_sdk::http::Response;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("S3 error: {0}")]
    S3Error(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

impl ServiceError {
    pub fn status_code(&self) -> u16 {
        match self {
            ServiceError::BadRequest(_) => 400,
            ServiceError::Unauthorized(_) => 401,
            ServiceError::Forbidden(_) => 403,
            ServiceError::NotFound(_) => 404,
            ServiceError::PayloadTooLarge(_) => 413,
            ServiceError::Internal(_) => 500,
            ServiceError::S3Error(_) => 502,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            ServiceError::BadRequest(_) => "BAD_REQUEST",
            ServiceError::Unauthorized(_) => "UNAUTHORIZED",
            ServiceError::Forbidden(_) => "FORBIDDEN",
            ServiceError::NotFound(_) => "NOT_FOUND",
            ServiceError::PayloadTooLarge(_) => "PAYLOAD_TOO_LARGE",
            ServiceError::Internal(_) => "INTERNAL_ERROR",
            ServiceError::S3Error(_) => "S3_ERROR",
        }
    }

    pub fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error: self.to_string(),
            code: self.error_code().to_string(),
        };

        let json = serde_json::to_string(&body).unwrap_or_else(|_| {
            r#"{"error":"Internal error","code":"INTERNAL_ERROR"}"#.to_string()
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(json)
            .build()
    }
}

