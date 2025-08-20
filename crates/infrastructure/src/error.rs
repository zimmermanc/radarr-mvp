//! Infrastructure error handling
//!
//! This module provides error conversion and handling for infrastructure operations.

use radarr_core::RadarrError;
use thiserror::Error;

/// Infrastructure-specific errors
#[derive(Error, Debug)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Connection pool error: {0}")]
    Pool(String),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("UUID parsing error: {0}")]
    UuidParsing(#[from] uuid::Error),
}

/// Convert infrastructure errors to domain errors
impl From<InfrastructureError> for RadarrError {
    fn from(err: InfrastructureError) -> Self {
        match err {
            InfrastructureError::Database(sqlx_err) => {
                RadarrError::ExternalServiceError {
                    service: "PostgreSQL".to_string(),
                    error: sqlx_err.to_string(),
                }
            }
            InfrastructureError::Pool(msg) => {
                RadarrError::ExternalServiceError {
                    service: "Database Pool".to_string(),
                    error: msg,
                }
            }
            InfrastructureError::Migration(msg) => {
                RadarrError::ConfigurationError {
                    field: "database_migration".to_string(),
                    message: msg,
                }
            }
            InfrastructureError::Serialization(err) => {
                RadarrError::ValidationError {
                    field: "json_data".to_string(),
                    message: err.to_string(),
                }
            }
            InfrastructureError::UuidParsing(err) => {
                RadarrError::ValidationError {
                    field: "uuid".to_string(),
                    message: err.to_string(),
                }
            }
        }
    }
}

// SQLx errors are now handled directly by the core crate