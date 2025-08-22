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
}

impl RateLimiter {
    pub fn new(max_requests_per_hour: u32) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            max_requests_per_hour,
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