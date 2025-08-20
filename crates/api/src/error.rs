//! API error handling and response types

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use radarr_core::RadarrError;
use serde_json::json;
use thiserror::Error;

/// API-specific error types
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Core error: {0}")]
    CoreError(#[from] RadarrError),
    
    #[error("Validation error in field '{field}': {message}")]
    ValidationError {
        field: String,
        message: String,
    },
    
    #[error("Bad request: {message}")]
    BadRequest {
        message: String,
    },
    
    #[error("Resource not found: {resource}")]
    NotFound {
        resource: String,
    },
    
    #[error("Resource conflict: {resource}")]
    Conflict {
        resource: String,
    },
    
    #[error("External service '{service}' error: {error}")]
    ExternalServiceError {
        service: String,
        error: String,
    },
    
    #[error("Authentication required")]
    Unauthorized,
    
    #[error("Access forbidden")]
    Forbidden,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Internal server error: {message}")]
    InternalError {
        message: String,
    },
}

/// Type alias for API results
pub type ApiResult<T> = Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::CoreError(core_error) => {
                match core_error {
                    RadarrError::ValidationError { field, message } => {
                        (StatusCode::BAD_REQUEST, format!("Validation error in field '{}': {}", field, message))
                    }
                    RadarrError::NotFound { resource } => {
                        (StatusCode::NOT_FOUND, format!("Resource not found: {}", resource))
                    }
                    RadarrError::DatabaseError { .. } => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred".to_string())
                    }
                    RadarrError::ExternalServiceError { service, error } => {
                        (StatusCode::INTERNAL_SERVER_ERROR, format!("External service '{}' error: {}", service, error))
                    }
                    _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
                }
            }
            ApiError::ValidationError { field, message } => {
                (StatusCode::BAD_REQUEST, format!("Validation error in field '{}': {}", field, message))
            }
            ApiError::BadRequest { message } => {
                (StatusCode::BAD_REQUEST, format!("Bad request: {}", message))
            }
            ApiError::NotFound { resource } => {
                (StatusCode::NOT_FOUND, format!("Resource not found: {}", resource))
            }
            ApiError::Conflict { resource } => {
                (StatusCode::CONFLICT, format!("Resource conflict: {}", resource))
            }
            ApiError::ExternalServiceError { service, error } => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("External service '{}' error: {}", service, error))
            }
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Authentication required".to_string())
            }
            ApiError::Forbidden => {
                (StatusCode::FORBIDDEN, "Access forbidden".to_string())
            }
            ApiError::RateLimitExceeded => {
                (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string())
            }
            ApiError::InternalError { message } => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", message))
            }
        };
        
        let body = Json(json!({
            "error": {
                "message": error_message,
                "code": status.as_u16(),
            }
        }));
        
        (status, body).into_response()
    }
}