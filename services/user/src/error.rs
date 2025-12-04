//! Error types for the User Service

use spin_sdk::http::Response;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Forbidden: {0}")]
    Forbidden(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Conflict: {0}")]
    Conflict(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl ServiceError {
    pub fn status_code(&self) -> u16 {
        match self {
            ServiceError::BadRequest(_) => 400,
            ServiceError::Unauthorized(_) => 401,
            ServiceError::Forbidden(_) => 403,
            ServiceError::NotFound(_) => 404,
            ServiceError::Conflict(_) => 409,
            ServiceError::Internal(_) => 500,
            ServiceError::ServiceUnavailable(_) => 503,
        }
    }
    
    pub fn error_type(&self) -> &'static str {
        match self {
            ServiceError::BadRequest(_) => "bad_request",
            ServiceError::Unauthorized(_) => "unauthorized",
            ServiceError::Forbidden(_) => "forbidden",
            ServiceError::NotFound(_) => "not_found",
            ServiceError::Conflict(_) => "conflict",
            ServiceError::Internal(_) => "internal_error",
            ServiceError::ServiceUnavailable(_) => "service_unavailable",
        }
    }
    
    pub fn into_response(self) -> Response {
        let status = self.status_code();
        let error_type = self.error_type();
        let message = self.to_string();
        
        let body = ErrorResponse {
            error: error_type.to_string(),
            message: message.clone(),
            details: if status >= 500 { None } else { Some(message) },
        };
        
        let json = serde_json::to_string(&body).unwrap_or_else(|_| {
            r#"{"error":"internal_error","message":"Failed to serialize error"}"#.to_string()
        });
        
        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(json)
            .build()
    }
}

