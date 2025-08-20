//! Core error types for Radarr domain

use thiserror::Error;

#[cfg(feature = "postgres")]
use sqlx;

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
    
    #[error("Database error: {message}")]
    DatabaseError { message: String },
    
    #[error("Import error: {operation} - {message}")]
    ImportError { operation: String, message: String },
    
    #[error("File system error: {path} - {error}")]
    FileSystemError { path: String, error: String },
    
    #[error("Configuration error: {field} - {message}")]
    ConfigurationError { field: String, message: String },
}

pub type Result<T> = std::result::Result<T, RadarrError>;

#[cfg(feature = "postgres")]
impl From<sqlx::Error> for RadarrError {
    fn from(err: sqlx::Error) -> Self {
        RadarrError::DatabaseError {
            message: err.to_string(),
        }
    }
}