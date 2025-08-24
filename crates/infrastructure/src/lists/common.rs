use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Common structure for list items across all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub title: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub original_language: Option<String>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f32>,
    pub source_metadata: serde_json::Value,
}

/// Supported list sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ListSource {
    IMDb,
    TMDb,
    Plex,
    Trakt,
    Letterboxd,
    Custom,
}

/// Result of a list sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListSyncResult {
    pub list_id: Uuid,
    pub source: ListSource,
    pub status: SyncStatus,
    pub items_found: usize,
    pub items_added: usize,
    pub items_updated: usize,
    pub items_excluded: usize,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Started,
    Success,
    Partial,
    Failed,
}

/// Trait for all list parsers
#[async_trait::async_trait]
pub trait ListParser: Send + Sync {
    /// Parse a list and return items
    async fn parse_list(&self, list_url: &str) -> Result<Vec<ListItem>, ListParseError>;

    /// Get the source type
    fn source_type(&self) -> ListSource;

    /// Validate a list URL
    fn validate_url(&self, url: &str) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum ListParseError {
    #[error("Invalid list URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Authentication required")]
    AuthRequired,

    #[error("List not found")]
    NotFound,

    #[error("Unknown error: {0}")]
    Unknown(String),
}
