//! Webhook notification provider
//!
//! Sends HTTP POST notifications to configured webhook URLs

use super::{NotificationEvent, NotificationProvider};
use crate::{RadarrError, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

/// Webhook notification configuration
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Webhook URL to POST to
    pub url: String,
    /// Optional username for basic auth
    pub username: Option<String>,
    /// Optional password for basic auth
    pub password: Option<String>,
    /// Request timeout in seconds
    pub timeout: u64,
}

/// Webhook notification provider
#[derive(Debug)]
pub struct WebhookProvider {
    config: WebhookConfig,
    client: Client,
}

impl WebhookProvider {
    /// Create a new webhook provider
    pub fn new(config: WebhookConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "webhook".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl NotificationProvider for WebhookProvider {
    async fn send_notification(&self, event: &NotificationEvent) -> Result<()> {
        let payload = json!({
            "event_type": match event {
                NotificationEvent::MovieDownloaded { .. } => "movie_downloaded",
                NotificationEvent::DownloadStarted { .. } => "download_started",
                NotificationEvent::DownloadFailed { .. } => "download_failed",
                NotificationEvent::MovieImported { .. } => "movie_imported",
                NotificationEvent::HealthCheckFailed { .. } => "health_check_failed",
                NotificationEvent::ApplicationStarted => "application_started",
                NotificationEvent::ApplicationStopped => "application_stopped",
            },
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": event
        });

        let mut request = self.client.post(&self.config.url).json(&payload);

        // Add basic auth if configured
        if let (Some(ref username), Some(ref password)) =
            (&self.config.username, &self.config.password)
        {
            request = request.basic_auth(username, Some(password));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "webhook".to_string(),
                error: format!("Request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RadarrError::ExternalServiceError {
                service: "webhook".to_string(),
                error: format!("HTTP {}: {}", status, error_text),
            });
        }

        Ok(())
    }

    async fn test_notification(&self) -> Result<()> {
        let test_event = NotificationEvent::ApplicationStarted;
        self.send_notification(&test_event).await
    }

    fn provider_name(&self) -> &'static str {
        "webhook"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_creation() {
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            timeout: 30,
        };

        assert_eq!(config.url, "https://example.com/webhook");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.timeout, 30);
    }

    #[test]
    fn test_webhook_provider_creation() {
        let config = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            username: None,
            password: None,
            timeout: 30,
        };

        let provider = WebhookProvider::new(config);
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().provider_name(), "webhook");
    }
}
