//! Notification system for user awareness
//!
//! This module provides notification capabilities for various events
//! like movie downloads, errors, and system status updates.

pub mod webhook;
pub mod discord;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::{Result, Movie};

/// Notification event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationEvent {
    /// Movie was successfully downloaded
    MovieDownloaded {
        movie: Movie,
        quality: String,
        size_mb: Option<u64>,
    },
    /// Download started
    DownloadStarted {
        movie: Movie,
        release_title: String,
    },
    /// Download failed
    DownloadFailed {
        movie: Movie,
        error: String,
    },
    /// Movie imported to library
    MovieImported {
        movie: Movie,
        file_path: String,
    },
    /// Health check failed
    HealthCheckFailed {
        service: String,
        error: String,
    },
    /// Application started
    ApplicationStarted,
    /// Application stopped
    ApplicationStopped,
}

/// Notification provider trait
#[async_trait]
pub trait NotificationProvider: Send + Sync {
    /// Send a notification for the given event
    async fn send_notification(&self, event: &NotificationEvent) -> Result<()>;
    
    /// Test the notification provider configuration
    async fn test_notification(&self) -> Result<()>;
    
    /// Get the provider name
    fn provider_name(&self) -> &'static str;
}

/// Main notification service
pub struct NotificationService {
    providers: Vec<Box<dyn NotificationProvider>>,
}

impl NotificationService {
    /// Create a new notification service
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }
    
    /// Add a notification provider
    pub fn add_provider(mut self, provider: Box<dyn NotificationProvider>) -> Self {
        self.providers.push(provider);
        self
    }
    
    /// Send notification to all configured providers
    pub async fn notify(&self, event: NotificationEvent) -> Vec<Result<()>> {
        let mut results = Vec::new();
        
        for provider in &self.providers {
            let result = provider.send_notification(&event).await;
            if let Err(ref e) = result {
                tracing::warn!(
                    "Notification failed for provider {}: {}",
                    provider.provider_name(),
                    e
                );
            }
            results.push(result);
        }
        
        results
    }
    
    /// Test all notification providers
    pub async fn test_all_providers(&self) -> Vec<(String, Result<()>)> {
        let mut results = Vec::new();
        
        for provider in &self.providers {
            let result = provider.test_notification().await;
            results.push((provider.provider_name().to_string(), result));
        }
        
        results
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}