//! Core error types for Radarr domain

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RadarrError {
    #[error("Movie not found: {id}")]
    MovieNotFound { id: String },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Invalid quality profile: {profile}")]
    InvalidQualityProfile { profile: String },

    #[error("Indexer error: {message}")]
    IndexerError { message: String },

    #[error("Domain validation error: {field} - {message}")]
    ValidationError { field: String, message: String },

    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },

    #[error("Configuration error: {field} - {message}")]
    ConfigurationError { field: String, message: String },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Not found: {entity} with id {id}")]
    NotFoundError { entity: String, id: String },

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {message}")]
    NetworkError { message: String },

    #[error("Operation timeout: {operation}")]
    Timeout { operation: String },

    #[error("Temporary error: {message}")]
    TemporaryError { message: String },

    #[error("Retry exhausted after {attempts} attempts for {operation}: {last_error}")]
    RetryExhausted {
        operation: String,
        attempts: u32,
        #[source]
        last_error: Box<RadarrError>,
    },

    #[error("Circuit breaker open for service: {service}")]
    CircuitBreakerOpen { service: String },

    #[error("Authentication required for {service}: {message}")]
    AuthenticationRequired { service: String, message: String },

    #[error("Rate limited by {service}")]
    RateLimited {
        service: String,
        retry_after: Option<u64>,
    },
}

pub type Result<T> = std::result::Result<T, RadarrError>;

// From implementations for external error types
#[cfg(feature = "postgres")]
impl From<sqlx::Error> for RadarrError {
    fn from(err: sqlx::Error) -> Self {
        RadarrError::DatabaseError {
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for RadarrError {
    fn from(err: std::io::Error) -> Self {
        RadarrError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for RadarrError {
    fn from(err: serde_json::Error) -> Self {
        RadarrError::SerializationError(err.to_string())
    }
}

impl From<tokio::task::JoinError> for RadarrError {
    fn from(err: tokio::task::JoinError) -> Self {
        RadarrError::ExternalServiceError {
            service: "tokio".to_string(),
            error: err.to_string(),
        }
    }
}
