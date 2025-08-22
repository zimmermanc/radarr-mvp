use async_trait::async_trait;
use radarr_core::models::Movie;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Failed to send notification: {0}")]
    SendFailed(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Template error: {0}")]
    TemplateError(String),
    
    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Email error: {0}")]
    EmailError(String),
}

pub type Result<T> = std::result::Result<T, NotificationError>;

#[async_trait]
pub trait NotificationProvider: Send + Sync {
    /// Get the name of this provider
    fn name(&self) -> &str;
    
    /// Test the connection/configuration
    async fn test(&self) -> Result<()>;
    
    /// Send a notification
    async fn send(&self, notification: &Notification) -> Result<()>;
    
    /// Check if this provider is enabled
    fn is_enabled(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub event_type: NotificationEventType,
    pub title: String,
    pub message: String,
    pub data: NotificationData,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationEventType {
    MovieAdded,
    MovieDeleted,
    DownloadStarted,
    DownloadCompleted,
    DownloadFailed,
    ImportStarted,
    ImportCompleted,
    ImportFailed,
    HealthCheckFailed,
    UpdateAvailable,
}

impl NotificationEventType {
    pub fn emoji(&self) -> &str {
        match self {
            Self::MovieAdded => "🎬",
            Self::MovieDeleted => "🗑️",
            Self::DownloadStarted => "⬇️",
            Self::DownloadCompleted => "✅",
            Self::DownloadFailed => "❌",
            Self::ImportStarted => "📁",
            Self::ImportCompleted => "✅",
            Self::ImportFailed => "❌",
            Self::HealthCheckFailed => "⚠️",
            Self::UpdateAvailable => "🆕",
        }
    }
    
    pub fn color(&self) -> u32 {
        match self {
            Self::MovieAdded | Self::DownloadCompleted | Self::ImportCompleted => 0x00FF00, // Green
            Self::DownloadStarted | Self::ImportStarted => 0x0099FF, // Blue
            Self::MovieDeleted => 0xFFFF00, // Yellow
            Self::DownloadFailed | Self::ImportFailed | Self::HealthCheckFailed => 0xFF0000, // Red
            Self::UpdateAvailable => 0x9933FF, // Purple
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NotificationData {
    Movie(MovieNotificationData),
    Download(DownloadNotificationData),
    Import(ImportNotificationData),
    Health(HealthNotificationData),
    Update(UpdateNotificationData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieNotificationData {
    pub movie: Movie,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadNotificationData {
    pub movie_id: i32,
    pub movie_title: String,
    pub quality: String,
    pub size: i64,
    pub indexer: String,
    pub download_client: String,
    pub status: String,
    pub progress: Option<f32>,
    pub eta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportNotificationData {
    pub movie_id: i32,
    pub movie_title: String,
    pub source_path: String,
    pub destination_path: String,
    pub quality: String,
    pub size: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthNotificationData {
    pub check_name: String,
    pub status: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNotificationData {
    pub current_version: String,
    pub new_version: String,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
}

impl Notification {
    pub fn new(
        event_type: NotificationEventType,
        title: String,
        message: String,
        data: NotificationData,
    ) -> Self {
        Self {
            event_type,
            title,
            message,
            data,
            timestamp: chrono::Utc::now(),
        }
    }
    
    pub fn movie_added(movie: Movie) -> Self {
        let title = format!("Movie Added: {}", movie.title);
        let message = format!(
            "{} ({}) has been added to your library",
            movie.title,
            movie.year.unwrap_or(0)
        );
        
        Self::new(
            NotificationEventType::MovieAdded,
            title,
            message,
            NotificationData::Movie(MovieNotificationData {
                movie,
                action: "added".to_string(),
            }),
        )
    }
    
    pub fn download_completed(data: DownloadNotificationData) -> Self {
        let title = format!("Download Complete: {}", data.movie_title);
        let message = format!(
            "{} ({}) has finished downloading from {}",
            data.movie_title,
            data.quality,
            data.indexer
        );
        
        Self::new(
            NotificationEventType::DownloadCompleted,
            title,
            message,
            NotificationData::Download(data),
        )
    }
}