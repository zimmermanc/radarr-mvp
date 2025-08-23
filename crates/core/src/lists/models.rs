//! Models for lists and discovery functionality

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// External list configuration and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalList {
    /// Unique identifier
    pub id: Uuid,
    /// User-friendly name for the list
    pub name: String,
    /// Type of list (trakt, imdb, tmdb)
    pub list_type: ListType,
    /// External list identifier (e.g., Trakt username, IMDb list ID)
    pub external_id: String,
    /// List URL if applicable
    pub url: Option<String>,
    /// Whether this list is enabled for sync
    pub enabled: bool,
    /// How often to sync this list (in hours)
    pub sync_interval_hours: u32,
    /// Whether to automatically add movies from this list
    pub auto_add: bool,
    /// Quality profile to use for auto-added movies
    pub quality_profile_id: Option<Uuid>,
    /// Root folder for auto-added movies
    pub root_folder: Option<String>,
    /// Tags to apply to auto-added movies
    pub tags: Vec<String>,
    /// Last successful sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
    /// Last sync status
    pub last_sync_status: SyncStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp  
    pub updated_at: DateTime<Utc>,
}

/// Types of external lists supported
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListType {
    /// Trakt lists (user lists, watchlists, etc.)
    Trakt,
    /// IMDb lists
    IMDb,
    /// TMDb lists
    TMDb,
    /// Custom/manual lists
    Custom,
}

impl ListType {
    /// Get the display name for this list type
    pub fn display_name(&self) -> &'static str {
        match self {
            ListType::Trakt => "Trakt",
            ListType::IMDb => "IMDb", 
            ListType::TMDb => "TMDb",
            ListType::Custom => "Custom",
        }
    }
    
    /// Get whether this list type supports OAuth authentication
    pub fn supports_oauth(&self) -> bool {
        matches!(self, ListType::Trakt)
    }
    
    /// Get whether this list type requires an API key
    pub fn requires_api_key(&self) -> bool {
        matches!(self, ListType::TMDb)
    }
}

/// Status of last sync operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Never synced
    Never,
    /// Sync completed successfully
    Success,
    /// Sync completed with warnings
    Warning { message: String },
    /// Sync failed
    Failed { error: String },
    /// Sync currently in progress
    InProgress,
}

/// Movie discovered from an external list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredMovie {
    /// External ID from the source list
    pub external_id: String,
    /// TMDb ID if available
    pub tmdb_id: Option<i32>,
    /// IMDb ID if available  
    pub imdb_id: Option<String>,
    /// Movie title
    pub title: String,
    /// Release year
    pub year: Option<i32>,
    /// Overview/plot summary
    pub overview: Option<String>,
    /// Poster image URL
    pub poster_path: Option<String>,
    /// Backdrop image URL
    pub backdrop_path: Option<String>,
    /// User rating on the source platform
    pub user_rating: Option<f32>,
    /// TMDB rating
    pub tmdb_rating: Option<f32>,
    /// Runtime in minutes
    pub runtime: Option<i32>,
    /// Genres
    pub genres: Vec<String>,
    /// Source list this movie came from
    pub source_list_id: Uuid,
    /// Source list name for display
    pub source_list_name: String,
    /// When this movie was discovered
    pub discovered_at: DateTime<Utc>,
    /// Whether this movie is already in the library
    pub in_library: bool,
    /// Whether this movie is monitored
    pub monitored: bool,
}

/// List sync operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// List that was synced
    pub list_id: Uuid,
    /// When the sync started
    pub started_at: DateTime<Utc>,
    /// When the sync completed
    pub completed_at: DateTime<Utc>,
    /// Whether the sync was successful
    pub success: bool,
    /// Error message if sync failed
    pub error: Option<String>,
    /// Number of movies discovered
    pub movies_discovered: u32,
    /// Number of new movies added
    pub movies_added: u32,
    /// Number of movies already in library
    pub movies_existing: u32,
    /// Number of movies skipped due to filters
    pub movies_skipped: u32,
    /// Detailed sync log
    pub sync_log: Vec<String>,
}

impl SyncResult {
    /// Create a new sync result
    pub fn new(list_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            list_id,
            started_at: now,
            completed_at: now,
            success: false,
            error: None,
            movies_discovered: 0,
            movies_added: 0,
            movies_existing: 0,
            movies_skipped: 0,
            sync_log: Vec::new(),
        }
    }
    
    /// Add a log entry
    pub fn add_log(&mut self, message: String) {
        self.sync_log.push(format!("[{}] {}", Utc::now().format("%H:%M:%S"), message));
    }
    
    /// Mark sync as completed successfully
    pub fn complete_success(mut self) -> Self {
        self.completed_at = Utc::now();
        self.success = true;
        self.add_log("Sync completed successfully".to_string());
        self
    }
    
    /// Mark sync as failed
    pub fn complete_failure(mut self, error: String) -> Self {
        self.completed_at = Utc::now();
        self.success = false;
        self.error = Some(error.clone());
        self.add_log(format!("Sync failed: {}", error));
        self
    }
}

/// List import configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListImportConfig {
    /// Maximum number of movies to import per sync
    pub max_movies_per_sync: u32,
    /// Whether to skip movies already in library
    pub skip_existing: bool,
    /// Minimum IMDb rating threshold
    pub min_imdb_rating: Option<f32>,
    /// Minimum TMDb rating threshold
    pub min_tmdb_rating: Option<f32>,
    /// Minimum year filter
    pub min_year: Option<i32>,
    /// Maximum year filter (None = no limit)
    pub max_year: Option<i32>,
    /// Genres to include (empty = all genres)
    pub include_genres: Vec<String>,
    /// Genres to exclude
    pub exclude_genres: Vec<String>,
    /// Languages to include (ISO 639-1 codes)
    pub include_languages: Vec<String>,
}

impl Default for ListImportConfig {
    fn default() -> Self {
        Self {
            max_movies_per_sync: 100,
            skip_existing: true,
            min_imdb_rating: Some(6.0),
            min_tmdb_rating: Some(6.0),
            min_year: Some(1980),
            max_year: None,
            include_genres: vec![],
            exclude_genres: vec!["Documentary".to_string(), "Short".to_string()],
            include_languages: vec!["en".to_string()],
        }
    }
}

/// Movie discovery source tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieProvenance {
    /// Movie ID in our system
    pub movie_id: Uuid,
    /// Source that discovered this movie
    pub source: DiscoverySource,
    /// External list ID if from a list
    pub list_id: Option<Uuid>,
    /// External ID on the source platform
    pub external_id: String,
    /// When the movie was discovered
    pub discovered_at: DateTime<Utc>,
    /// User who added the list (if applicable)
    pub added_by_user: Option<String>,
    /// Additional metadata about discovery
    pub metadata: serde_json::Value,
}

/// Source of movie discovery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Added manually by user
    Manual,
    /// Discovered from Trakt list
    TraktList,
    /// Discovered from IMDb list
    IMDbList,
    /// Discovered from TMDb list
    TMDbList,
    /// Recommended by algorithm
    Recommendation,
    /// Imported from RSS feed
    RSS,
}

impl DiscoverySource {
    /// Get display name for the source
    pub fn display_name(&self) -> &'static str {
        match self {
            DiscoverySource::Manual => "Manual",
            DiscoverySource::TraktList => "Trakt List",
            DiscoverySource::IMDbList => "IMDb List", 
            DiscoverySource::TMDbList => "TMDb List",
            DiscoverySource::Recommendation => "Recommendation",
            DiscoverySource::RSS => "RSS Feed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_type_properties() {
        assert_eq!(ListType::Trakt.display_name(), "Trakt");
        assert!(ListType::Trakt.supports_oauth());
        assert!(!ListType::Trakt.requires_api_key());
        
        assert_eq!(ListType::TMDb.display_name(), "TMDb");
        assert!(!ListType::TMDb.supports_oauth());
        assert!(ListType::TMDb.requires_api_key());
    }

    #[test]
    fn test_sync_result_lifecycle() {
        let list_id = Uuid::new_v4();
        let mut result = SyncResult::new(list_id);
        
        result.add_log("Starting sync".to_string());
        result.movies_discovered = 50;
        result.movies_added = 10;
        
        let completed = result.complete_success();
        assert!(completed.success);
        assert!(completed.sync_log.len() >= 2); // Initial log + completion log
    }

    #[test]
    fn test_list_import_config_defaults() {
        let config = ListImportConfig::default();
        assert_eq!(config.max_movies_per_sync, 100);
        assert!(config.skip_existing);
        assert_eq!(config.min_imdb_rating, Some(6.0));
        assert!(config.exclude_genres.contains(&"Documentary".to_string()));
    }

    #[test]
    fn test_discovery_source_display() {
        assert_eq!(DiscoverySource::TraktList.display_name(), "Trakt List");
        assert_eq!(DiscoverySource::Manual.display_name(), "Manual");
    }
}