//! HDBits private tracker indexer
//!
//! This module provides a direct integration with HDBits using their API
//! with passkey authentication. Includes rate limiting, error handling,
//! and quality parsing from release names.

use radarr_core::{Result, RadarrError};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub mod client;
pub mod models;
pub mod parser;

#[cfg(test)]
mod tests;

pub use client::HDBitsClient;
pub use models::*;

/// HDBits indexer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDBitsConfig {
    pub username: String,
    pub session_cookie: String,  // Changed from passkey to session_cookie
    pub rate_limit_per_hour: u32,
    pub timeout_seconds: u64,
}

impl Default for HDBitsConfig {
    fn default() -> Self {
        Self {
            username: "blargdiesel".to_string(),
            session_cookie: "your_session_cookie_here".to_string(),
            rate_limit_per_hour: 150,
            timeout_seconds: 30,
        }
    }
}

impl HDBitsConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let username = std::env::var("HDBITS_USERNAME")
            .unwrap_or_else(|_| "blargdiesel".to_string());
        
        let session_cookie = std::env::var("HDBITS_SESSION_COOKIE")
            .unwrap_or_else(|_| "your_session_cookie_here".to_string());
            
        let rate_limit_per_hour: u32 = std::env::var("HDBITS_RATE_LIMIT")
            .unwrap_or_else(|_| "150".to_string())
            .parse()
            .map_err(|e| RadarrError::ConfigurationError {
                field: "HDBITS_RATE_LIMIT".to_string(),
                message: format!("Invalid rate limit: {}", e),
            })?;

        let timeout_seconds: u64 = std::env::var("HDBITS_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .map_err(|e| RadarrError::ConfigurationError {
                field: "HDBITS_TIMEOUT".to_string(),
                message: format!("Invalid timeout: {}", e),
            })?;

        Ok(Self {
            username,
            session_cookie,
            rate_limit_per_hour,
            timeout_seconds,
        })
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.username.is_empty() {
            return Err(RadarrError::ConfigurationError {
                field: "username".to_string(),
                message: "Username cannot be empty".to_string(),
            });
        }
        
        if self.session_cookie.is_empty() {
            return Err(RadarrError::ConfigurationError {
                field: "session_cookie".to_string(),
                message: "Session cookie cannot be empty".to_string(),
            });
        }
        
        if self.rate_limit_per_hour == 0 {
            return Err(RadarrError::ConfigurationError {
                field: "rate_limit_per_hour".to_string(),
                message: "Rate limit must be greater than 0".to_string(),
            });
        }
        
        Ok(())
    }
}

/// Rate limiter for HDBits API calls
#[derive(Debug)]
pub struct RateLimiter {
    requests: Mutex<Vec<Instant>>,
    max_requests_per_hour: u32,
    failure_count: Mutex<u32>,
    last_failure: Mutex<Option<Instant>>,
}

impl RateLimiter {
    pub fn new(max_requests_per_hour: u32) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            max_requests_per_hour,
            failure_count: Mutex::new(0),
            last_failure: Mutex::new(None),
        }
    }
    
    /// Wait if necessary to respect rate limits
    pub async fn acquire(&self) -> Result<()> {
        loop {
            let mut requests = self.requests.lock().await;
            let now = Instant::now();
            let one_hour_ago = now - Duration::from_secs(3600);
            
            // Remove requests older than 1 hour
            requests.retain(|&timestamp| timestamp > one_hour_ago);
            
            // Check if we've hit the rate limit
            if requests.len() >= self.max_requests_per_hour as usize {
                let oldest_request = requests[0];
                let wait_time = oldest_request + Duration::from_secs(3600) - now;
                
                if wait_time > Duration::from_secs(0) {
                    warn!(
                        "Rate limit exceeded, waiting {} seconds",
                        wait_time.as_secs()
                    );
                    drop(requests); // Release lock before sleeping
                    tokio::time::sleep(wait_time).await;
                    continue; // Retry
                }
            }
            
            // Record this request
            requests.push(now);
            debug!("Rate limiter: {}/{} requests in last hour", 
                   requests.len(), self.max_requests_per_hour);
            
            break;
        }
        
        Ok(())
    }
    
    /// Record a successful request (resets failure count)
    pub async fn record_success(&self) {
        let mut failure_count = self.failure_count.lock().await;
        *failure_count = 0;
        debug!("Request succeeded, reset failure count");
    }
    
    /// Record a failed request and apply exponential backoff
    pub async fn record_failure(&self) -> Result<()> {
        let mut failure_count = self.failure_count.lock().await;
        let mut last_failure = self.last_failure.lock().await;
        
        *failure_count += 1;
        *last_failure = Some(Instant::now());
        
        let failures = *failure_count;
        drop(failure_count);
        drop(last_failure);
        
        if failures > 0 {
            // Exponential backoff: 2^failures seconds, max 300 seconds (5 minutes)
            let backoff_seconds = (2_u64.pow(failures.min(8))).min(300);
            
            warn!("HDBits request failed (failure #{failures}), backing off for {backoff_seconds} seconds");
            
            tokio::time::sleep(Duration::from_secs(backoff_seconds)).await;
        }
        
        Ok(())
    }
    
    /// Check if we should skip requests due to recent failures
    pub async fn should_skip_due_to_failures(&self) -> bool {
        let failure_count = self.failure_count.lock().await;
        let last_failure = self.last_failure.lock().await;
        
        // Skip if we have 5+ consecutive failures and last failure was within 10 minutes
        if *failure_count >= 5 {
            if let Some(last_fail_time) = *last_failure {
                if last_fail_time.elapsed() < Duration::from_secs(600) {
                    return true;
                }
            }
        }
        
        false
    }
}

/// Convert HDBits search error to RadarrError
pub fn map_hdbits_error(error: &str) -> RadarrError {
    match error {
        e if e.contains("login") || e.contains("session") => RadarrError::ExternalServiceError {
            service: "HDBits".to_string(),
            error: "Authentication failed - check session cookie".to_string(),
        },
        e if e.contains("Rate limit") => RadarrError::ExternalServiceError {
            service: "HDBits".to_string(),
            error: "Rate limit exceeded - slow down requests".to_string(),
        },
        e if e.contains("No results") => RadarrError::NotFound {
            resource: "HDBits search results".to_string(),
        },
        e => RadarrError::IndexerError {
            message: format!("HDBits scraping error: {}", e),
        },
    }
}