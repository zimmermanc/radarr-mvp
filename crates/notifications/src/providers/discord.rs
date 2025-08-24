use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, info};

use crate::{
    models::{Notification, NotificationError, NotificationProvider, Result},
    NotificationData, NotificationEventType,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_url: String,
    pub enabled: bool,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub mention_everyone: bool,
    pub mention_roles: Vec<String>,
}

pub struct DiscordProvider {
    config: DiscordConfig,
    client: reqwest::Client,
}

impl DiscordProvider {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn create_embed(&self, notification: &Notification) -> serde_json::Value {
        let color = notification.event_type.color();
        let emoji = notification.event_type.emoji();

        let mut embed = json!({
            "title": format!("{} {}", emoji, notification.title),
            "description": notification.message,
            "color": color,
            "timestamp": notification.timestamp.to_rfc3339(),
            "footer": {
                "text": "Radarr MVP",
                "icon_url": self.config.avatar_url.as_deref().unwrap_or("https://radarr.video/img/logo.png")
            }
        });

        // Add fields based on notification data
        if let Some(fields) = self.create_fields(&notification.data) {
            embed["fields"] = fields;
        }

        // Add thumbnail for movie notifications
        if let NotificationData::Movie(data) = &notification.data {
            if let Some(poster_path) = data
                .movie
                .metadata
                .get("poster_path")
                .and_then(|v| v.as_str())
            {
                embed["thumbnail"] = json!({
                    "url": format!("https://image.tmdb.org/t/p/w342{}", poster_path)
                });
            }
        }

        embed
    }

    fn create_fields(&self, data: &NotificationData) -> Option<serde_json::Value> {
        match data {
            NotificationData::Movie(movie_data) => Some(json!([
                {
                    "name": "Title",
                    "value": movie_data.movie.title.clone(),
                    "inline": true
                },
                {
                    "name": "Year",
                    "value": movie_data.movie.year.map(|y| y.to_string()).unwrap_or_else(|| "Unknown".to_string()),
                    "inline": true
                },
                {
                    "name": "TMDB ID",
                    "value": movie_data.movie.tmdb_id.to_string(),
                    "inline": true
                }
            ])),

            NotificationData::Download(dl_data) => Some(json!([
                {
                    "name": "Movie",
                    "value": dl_data.movie_title.clone(),
                    "inline": true
                },
                {
                    "name": "Quality",
                    "value": dl_data.quality.clone(),
                    "inline": true
                },
                {
                    "name": "Size",
                    "value": format_size(dl_data.size),
                    "inline": true
                },
                {
                    "name": "Indexer",
                    "value": dl_data.indexer.clone(),
                    "inline": true
                },
                {
                    "name": "Download Client",
                    "value": dl_data.download_client.clone(),
                    "inline": true
                },
                {
                    "name": "Progress",
                    "value": dl_data.progress.map(|p| format!("{}%", p))
                        .unwrap_or_else(|| "N/A".to_string()),
                    "inline": true
                }
            ])),

            NotificationData::Import(import_data) => Some(json!([
                {
                    "name": "Movie",
                    "value": import_data.movie_title.clone(),
                    "inline": true
                },
                {
                    "name": "Quality",
                    "value": import_data.quality.clone(),
                    "inline": true
                },
                {
                    "name": "Size",
                    "value": format_size(import_data.size),
                    "inline": true
                },
                {
                    "name": "Destination",
                    "value": truncate_path(&import_data.destination_path),
                    "inline": false
                }
            ])),

            _ => None,
        }
    }
}

#[async_trait]
impl NotificationProvider for DiscordProvider {
    fn name(&self) -> &str {
        "Discord"
    }

    fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    async fn test(&self) -> Result<()> {
        if !self.is_enabled() {
            return Err(NotificationError::ProviderUnavailable(
                "Discord provider is disabled".to_string(),
            ));
        }

        let test_notification = Notification::new(
            NotificationEventType::HealthCheckFailed,
            "Test Notification".to_string(),
            "This is a test notification from Radarr MVP".to_string(),
            NotificationData::Health(crate::HealthNotificationData {
                check_name: "Discord Integration".to_string(),
                status: "success".to_string(),
                message: "Discord webhook is configured correctly".to_string(),
                details: None,
            }),
        );

        self.send(&test_notification).await
    }

    async fn send(&self, notification: &Notification) -> Result<()> {
        if !self.is_enabled() {
            debug!("Discord provider is disabled, skipping notification");
            return Ok(());
        }

        let embed = self.create_embed(notification);

        let mut content = String::new();
        if self.config.mention_everyone {
            content.push_str("@everyone ");
        }
        for role in &self.config.mention_roles {
            content.push_str(&format!("<@&{}> ", role));
        }

        let webhook_data = json!({
            "username": self.config.username.as_deref().unwrap_or("Radarr MVP"),
            "avatar_url": self.config.avatar_url.as_deref(),
            "content": if content.is_empty() { None } else { Some(content) },
            "embeds": [embed]
        });

        let response = self
            .client
            .post(&self.config.webhook_url)
            .json(&webhook_data)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Discord webhook failed with status {}: {}", status, text);
            return Err(NotificationError::SendFailed(format!(
                "Discord webhook returned status {}: {}",
                status, text
            )));
        }

        info!(
            "Successfully sent Discord notification for event: {:?}",
            notification.event_type
        );
        Ok(())
    }
}

fn format_size(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

fn truncate_path(path: &str) -> String {
    if path.len() <= 50 {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - 47..])
    }
}
