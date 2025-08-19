//! Core error types for Radarr domain

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RadarrError {
    #[error("Movie not found: {id}")]
    MovieNotFound { id: String },
    
    #[error("Invalid quality profile: {profile}")]
    InvalidQualityProfile { profile: String },
    
    #[error("Indexer error: {message}")]
    IndexerError { message: String },
    
    #[error("Domain validation error: {field} - {message}")]
    ValidationError { field: String, message: String },
    
    #[error("External service error: {service} - {error}")]
    ExternalServiceError { service: String, error: String },
}

pub type Result<T> = std::result::Result<T, RadarrError>;