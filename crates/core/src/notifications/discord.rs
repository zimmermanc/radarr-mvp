//! Discord notification provider
//!
//! Sends notifications to Discord channels via webhook URLs

use super::{NotificationProvider, NotificationEvent, NotificationService};
use crate::{Result, RadarrError, Movie};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

/// Discord webhook configuration
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    /// Discord webhook URL
    pub webhook_url: String,
    /// Bot username to display
    pub username: Option<String>,
    /// Bot avatar URL
    pub avatar_url: Option<String>,
    /// Request timeout in seconds
    pub timeout: u64,
}

/// Discord notification provider
#[derive(Debug)]
pub struct DiscordProvider {
    config: DiscordConfig,
    client: Client,
}

impl DiscordProvider {
    /// Create a new Discord provider
    pub fn new(config: DiscordConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "discord".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;
            
        Ok(Self { config, client })
    }
    
    /// Format a movie notification message
    fn format_movie_message(&self, movie: &Movie, action: &str, details: Option<&str>) -> String {
        let mut message = format!("🎬 **{}** - {}", action, movie.title);
        
        if let Some(year) = movie.year {
            message.push_str(&format!(" ({})", year));
        }
        
        if let Some(details) = details {
            message.push_str(&format!("\n📋 {}", details));
        }
        
        message
    }
    
    /// Create Discord embed for rich formatting
    fn create_embed(&self, event: &NotificationEvent) -> Value {
        match event {
            NotificationEvent::MovieDownloaded { movie, quality, size_mb } => {
                let mut description = format!("Quality: {}", quality);
                if let Some(size) = size_mb {
                    description.push_str(&format!("\nSize: {}MB", size));
                }
                
                json!({
                    "title": format!("📥 Movie Downloaded"),
                    "description": format!("{} ({})", movie.title, movie.year.unwrap_or(0)),
                    "color": 0x00FF00, // Green
                    "fields": [
                        {
                            "name": "Quality",
                            "value": quality,
                            "inline": true
                        },
                        {
                            "name": "Size", 
                            "value": size_mb.map(|s| format!("{}MB", s)).unwrap_or("Unknown".to_string()),
                            "inline": true
                        }
                    ],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
            NotificationEvent::DownloadStarted { movie, release_title } => {
                json!({
                    "title": "📤 Download Started",
                    "description": format!("{} ({})", movie.title, movie.year.unwrap_or(0)),
                    "color": 0x0099FF, // Blue
                    "fields": [
                        {
                            "name": "Release",
                            "value": release_title,
                            "inline": false
                        }
                    ],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
            NotificationEvent::DownloadFailed { movie, error } => {
                json!({
                    "title": "❌ Download Failed",
                    "description": format!("{} ({})", movie.title, movie.year.unwrap_or(0)),
                    "color": 0xFF0000, // Red
                    "fields": [
                        {
                            "name": "Error",
                            "value": error,
                            "inline": false
                        }
                    ],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
            NotificationEvent::MovieImported { movie, file_path } => {
                json!({
                    "title": "📂 Movie Imported",
                    "description": format!("{} ({})", movie.title, movie.year.unwrap_or(0)),
                    "color": 0x9932CC, // Purple
                    "fields": [
                        {
                            "name": "Location",
                            "value": file_path,
                            "inline": false
                        }
                    ],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
            NotificationEvent::HealthCheckFailed { service, error } => {
                json!({
                    "title": "⚠️ Health Check Failed",
                    "description": format!("Service: {}", service),
                    "color": 0xFFA500, // Orange
                    "fields": [
                        {
                            "name": "Error",
                            "value": error,
                            "inline": false
                        }
                    ],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
            NotificationEvent::ApplicationStarted => {
                json!({
                    "title": "🚀 Radarr Started",
                    "description": "Application started successfully",
                    "color": 0x00FF00, // Green
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
            NotificationEvent::ApplicationStopped => {
                json!({
                    "title": "🛑 Radarr Stopped", 
                    "description": "Application stopped",
                    "color": 0x808080, // Gray
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
            },
        }
    }
}

#[async_trait]
impl NotificationProvider for DiscordProvider {
    async fn send_notification(&self, event: &NotificationEvent) -> Result<()> {
        let embed = self.create_embed(event);
        
        let mut payload = json!({
            "embeds": [embed]
        });
        
        // Add custom username and avatar if configured
        if let Some(ref username) = self.config.username {
            payload["username"] = Value::String(username.clone());
        }
        
        if let Some(ref avatar_url) = self.config.avatar_url {
            payload["avatar_url"] = Value::String(avatar_url.clone());
        }
        
        let response = self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "discord".to_string(),
                error: format!("Request failed: {}", e),
            })?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RadarrError::ExternalServiceError {
                service: "discord".to_string(),
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
        "discord"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_movie() -> Movie {
        let mut movie = Movie::new(12345, "Test Movie".to_string());
        movie.year = Some(2023);
        movie.imdb_id = Some("tt1234567".to_string());
        movie.status = crate::MovieStatus::Released;
        movie.minimum_availability = crate::MinimumAvailability::Released;
        movie
    }

    #[test]
    fn test_discord_config_creation() {
        let config = DiscordConfig {
            webhook_url: "https://discord.com/api/webhooks/123/abc".to_string(),
            username: Some("Radarr".to_string()),
            avatar_url: None,
            timeout: 30,
        };
        
        assert_eq!(config.webhook_url, "https://discord.com/api/webhooks/123/abc");
        assert_eq!(config.username, Some("Radarr".to_string()));
    }

    #[test]
    fn test_notification_service_creation() {
        let service = NotificationService::new();
        assert_eq!(service.providers.len(), 0);
    }

    #[test]
    fn test_embed_creation() {
        let config = DiscordConfig {
            webhook_url: "https://test.com".to_string(),
            username: None,
            avatar_url: None,
            timeout: 30,
        };
        
        let provider = DiscordProvider::new(config).unwrap();
        let movie = create_test_movie();
        
        let event = NotificationEvent::MovieDownloaded {
            movie,
            quality: "1080p BluRay".to_string(),
            size_mb: Some(8000),
        };
        
        let embed = provider.create_embed(&event);
        assert!(embed["title"].as_str().unwrap().contains("Movie Downloaded"));
        assert_eq!(embed["color"], 0x00FF00);
    }
}