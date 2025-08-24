//! Retry configuration for the application

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Application-wide retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRetryConfig {
    /// Retry configuration for API calls
    pub api_retries: RetrySettings,
    /// Retry configuration for database operations
    pub database_retries: RetrySettings,
    /// Retry configuration for download operations
    pub download_retries: RetrySettings,
    /// Retry configuration for import operations
    pub import_retries: RetrySettings,
    /// Retry configuration for indexer searches
    pub indexer_retries: RetrySettings,
    /// Circuit breaker settings
    pub circuit_breakers: CircuitBreakerSettings,
}

/// Settings for a specific retry category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySettings {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Whether to add jitter
    pub jitter: bool,
    /// Whether retries are enabled
    pub enabled: bool,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerSettings {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before attempting to close circuit (seconds)
    pub reset_timeout_seconds: u64,
    /// Number of successful calls needed to close circuit
    pub success_threshold: u32,
    /// Whether circuit breakers are enabled
    pub enabled: bool,
}

impl Default for ApplicationRetryConfig {
    fn default() -> Self {
        Self {
            api_retries: RetrySettings {
                max_attempts: 3,
                initial_delay_ms: 100,
                max_delay_ms: 5000,
                backoff_multiplier: 2.0,
                jitter: true,
                enabled: true,
            },
            database_retries: RetrySettings {
                max_attempts: 3,
                initial_delay_ms: 50,
                max_delay_ms: 1000,
                backoff_multiplier: 2.0,
                jitter: false,
                enabled: true,
            },
            download_retries: RetrySettings {
                max_attempts: 5,
                initial_delay_ms: 5000,
                max_delay_ms: 300000, // 5 minutes
                backoff_multiplier: 2.0,
                jitter: true,
                enabled: true,
            },
            import_retries: RetrySettings {
                max_attempts: 3,
                initial_delay_ms: 2000,
                max_delay_ms: 60000, // 1 minute
                backoff_multiplier: 2.0,
                jitter: true,
                enabled: true,
            },
            indexer_retries: RetrySettings {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000, // 30 seconds
                backoff_multiplier: 2.0,
                jitter: true,
                enabled: true,
            },
            circuit_breakers: CircuitBreakerSettings {
                failure_threshold: 5,
                reset_timeout_seconds: 60,
                success_threshold: 3,
                enabled: true,
            },
        }
    }
}

impl ApplicationRetryConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Override with environment variables if present
        if let Ok(val) = std::env::var("RETRY_MAX_ATTEMPTS") {
            if let Ok(max) = val.parse() {
                config.api_retries.max_attempts = max;
                config.database_retries.max_attempts = max;
                config.indexer_retries.max_attempts = max;
            }
        }

        if let Ok(val) = std::env::var("RETRY_ENABLED") {
            let enabled = val.to_lowercase() == "true" || val == "1";
            config.api_retries.enabled = enabled;
            config.database_retries.enabled = enabled;
            config.download_retries.enabled = enabled;
            config.import_retries.enabled = enabled;
            config.indexer_retries.enabled = enabled;
        }

        if let Ok(val) = std::env::var("CIRCUIT_BREAKER_ENABLED") {
            config.circuit_breakers.enabled = val.to_lowercase() == "true" || val == "1";
        }

        config
    }

    /// Convert retry settings to core RetryConfig
    pub fn to_core_config(&self, settings: &RetrySettings) -> radarr_core::retry::RetryConfig {
        radarr_core::retry::RetryConfig {
            max_attempts: settings.max_attempts,
            initial_delay: Duration::from_millis(settings.initial_delay_ms),
            max_delay: Duration::from_millis(settings.max_delay_ms),
            backoff_multiplier: settings.backoff_multiplier,
            jitter: settings.jitter,
        }
    }
}
