//! Movie domain model

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Movie status in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MovieStatus {
    #[default]
    Announced,
    InProduction,
    PostProduction,
    Released,
    Cancelled,
}

/// Minimum availability requirements
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MinimumAvailability {
    Announced,
    InCinemas,
    #[default]
    Released,
    Predb,
}

/// Core movie entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub id: Uuid,
    pub tmdb_id: i32,
    pub imdb_id: Option<String>,

    // Basic information
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub runtime: Option<i32>, // minutes

    // Status and monitoring
    pub status: MovieStatus,
    pub monitored: bool,

    // Quality and availability
    pub quality_profile_id: Option<i32>,
    pub minimum_availability: MinimumAvailability,

    // File information
    pub has_file: bool,
    pub movie_file_id: Option<Uuid>,

    // Flexible metadata storage
    pub metadata: serde_json::Value,
    pub alternative_titles: serde_json::Value,

    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub last_search_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_info_sync: Option<chrono::DateTime<chrono::Utc>>,
}

impl Movie {
    /// Create a new Movie with the given TMDB ID and title
    pub fn new(tmdb_id: i32, title: String) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            tmdb_id,
            imdb_id: None,
            title,
            original_title: None,
            year: None,
            runtime: None,
            status: MovieStatus::default(),
            monitored: true,
            quality_profile_id: None,
            minimum_availability: MinimumAvailability::default(),
            has_file: false,
            movie_file_id: None,
            metadata: serde_json::json!({}),
            alternative_titles: serde_json::json!([]),
            created_at: now,
            updated_at: now,
            last_search_time: None,
            last_info_sync: None,
        }
    }

    /// Update the movie's metadata
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.updated_at = chrono::Utc::now();
    }

    /// Mark the movie as having a file
    pub fn set_has_file(&mut self, movie_file_id: Uuid) {
        self.has_file = true;
        self.movie_file_id = Some(movie_file_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Get the movie's rating from metadata
    pub fn rating(&self) -> Option<f64> {
        self.metadata
            .get("tmdb")
            .and_then(|tmdb| tmdb.get("vote_average"))
            .and_then(|rating| rating.as_f64())
    }

    /// Get the movie's overview from metadata
    pub fn overview(&self) -> Option<&str> {
        self.metadata
            .get("tmdb")
            .and_then(|tmdb| tmdb.get("overview"))
            .and_then(|overview| overview.as_str())
    }
}

// Implement Display for enum serialization to string
impl std::fmt::Display for MovieStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MovieStatus::Announced => write!(f, "announced"),
            MovieStatus::InProduction => write!(f, "in_production"),
            MovieStatus::PostProduction => write!(f, "post_production"),
            MovieStatus::Released => write!(f, "released"),
            MovieStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::fmt::Display for MinimumAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MinimumAvailability::Announced => write!(f, "announced"),
            MinimumAvailability::InCinemas => write!(f, "in_cinemas"),
            MinimumAvailability::Released => write!(f, "released"),
            MinimumAvailability::Predb => write!(f, "predb"),
        }
    }
}
