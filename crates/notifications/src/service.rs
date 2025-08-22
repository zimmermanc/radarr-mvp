use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::models::{Notification, NotificationProvider, Result};

/// Central notification service that manages multiple providers
pub struct NotificationService {
    providers: Vec<Arc<dyn NotificationProvider>>,
    sender: broadcast::Sender<Notification>,
}

impl NotificationService {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        
        Self {
            providers: Vec::new(),
            sender,
        }
    }

    /// Add a notification provider
    pub fn add_provider(&mut self, provider: Arc<dyn NotificationProvider>) {
        info!("Adding notification provider: {}", provider.name());
        self.providers.push(provider);
    }

    /// Subscribe to notifications
    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.sender.subscribe()
    }

    /// Send a notification to all enabled providers
    pub async fn send(&self, notification: Notification) -> Result<()> {
        debug!(
            "Sending notification: {} to {} providers",
            notification.title,
            self.providers.len()
        );

        // Broadcast to subscribers first
        if let Err(e) = self.sender.send(notification.clone()) {
            warn!("Failed to broadcast notification: {}", e);
        }

        let mut errors = Vec::new();
        let mut sent_count = 0;

        // Send to all enabled providers
        for provider in &self.providers {
            if !provider.is_enabled() {
                debug!("Skipping disabled provider: {}", provider.name());
                continue;
            }

            match provider.send(&notification).await {
                Ok(_) => {
                    debug!("Successfully sent notification via {}", provider.name());
                    sent_count += 1;
                }
                Err(e) => {
                    error!(
                        "Failed to send notification via {}: {}",
                        provider.name(),
                        e
                    );
                    errors.push(format!("{}: {}", provider.name(), e));
                }
            }
        }

        if sent_count == 0 && !errors.is_empty() {
            return Err(crate::NotificationError::SendFailed(format!(
                "All providers failed: {}",
                errors.join(", ")
            )));
        }

        info!(
            "Notification sent successfully to {}/{} providers",
            sent_count,
            self.providers.len()
        );

        Ok(())
    }

    /// Test all notification providers
    pub async fn test_all(&self) -> Vec<(String, Result<()>)> {
        let mut results = Vec::new();

        for provider in &self.providers {
            let result = provider.test().await;
            results.push((provider.name().to_string(), result));
        }

        results
    }

    /// Get list of enabled providers
    pub fn get_enabled_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .filter(|p| p.is_enabled())
            .map(|p| p.name().to_string())
            .collect()
    }

    /// Get total number of providers
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Get number of enabled providers
    pub fn enabled_provider_count(&self) -> usize {
        self.providers.iter().filter(|p| p.is_enabled()).count()
    }
}