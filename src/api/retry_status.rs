//! API endpoints for retry and resilience status

use axum::{
    extract::Extension,
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::retry_config::ApplicationRetryConfig;

/// Response for retry configuration status
#[derive(Debug, Serialize, Deserialize)]
pub struct RetryStatusResponse {
    /// Whether retry is globally enabled
    pub retry_enabled: bool,
    /// Whether circuit breakers are enabled
    pub circuit_breakers_enabled: bool,
    /// Retry configurations by category
    pub configurations: RetryConfigurations,
    /// Current circuit breaker states (if available)
    pub circuit_breaker_states: Vec<CircuitBreakerState>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RetryConfigurations {
    pub api: CategoryConfig,
    pub database: CategoryConfig,
    pub downloads: CategoryConfig,
    pub imports: CategoryConfig,
    pub indexers: CategoryConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryConfig {
    pub enabled: bool,
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    pub service: String,
    pub state: String, // "closed", "open", "half_open"
    pub failure_count: u32,
    pub last_failure: Option<String>,
}

/// Get current retry configuration and status
pub async fn get_retry_status(
    Extension(config): Extension<Arc<ApplicationRetryConfig>>
) -> Result<Json<RetryStatusResponse>, StatusCode> {
    let response = RetryStatusResponse {
        retry_enabled: config.api_retries.enabled 
            || config.database_retries.enabled 
            || config.download_retries.enabled,
        circuit_breakers_enabled: config.circuit_breakers.enabled,
        configurations: RetryConfigurations {
            api: CategoryConfig {
                enabled: config.api_retries.enabled,
                max_attempts: config.api_retries.max_attempts,
                initial_delay_ms: config.api_retries.initial_delay_ms,
                max_delay_ms: config.api_retries.max_delay_ms,
            },
            database: CategoryConfig {
                enabled: config.database_retries.enabled,
                max_attempts: config.database_retries.max_attempts,
                initial_delay_ms: config.database_retries.initial_delay_ms,
                max_delay_ms: config.database_retries.max_delay_ms,
            },
            downloads: CategoryConfig {
                enabled: config.download_retries.enabled,
                max_attempts: config.download_retries.max_attempts,
                initial_delay_ms: config.download_retries.initial_delay_ms,
                max_delay_ms: config.download_retries.max_delay_ms,
            },
            imports: CategoryConfig {
                enabled: config.import_retries.enabled,
                max_attempts: config.import_retries.max_attempts,
                initial_delay_ms: config.import_retries.initial_delay_ms,
                max_delay_ms: config.import_retries.max_delay_ms,
            },
            indexers: CategoryConfig {
                enabled: config.indexer_retries.enabled,
                max_attempts: config.indexer_retries.max_attempts,
                initial_delay_ms: config.indexer_retries.initial_delay_ms,
                max_delay_ms: config.indexer_retries.max_delay_ms,
            },
        },
        circuit_breaker_states: vec![
            // In production, these would be fetched from active circuit breakers
            CircuitBreakerState {
                service: "download_client".to_string(),
                state: "closed".to_string(),
                failure_count: 0,
                last_failure: None,
            },
        ],
    };
    
    Ok(Json(response))
}