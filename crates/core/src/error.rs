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
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, RadarrError>;

// From implementations for external error types
#[cfg(feature = "postgres")]
impl From<sqlx::Error> for RadarrError {
    fn from(err: sqlx::Error) -> Self {
        RadarrError::DatabaseError(err.to_string())
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