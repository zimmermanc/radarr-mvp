//! Prowlarr API models and response types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prowlarr search result representing a torrent/NZB release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProwlarrSearchResult {
    /// Title of the release
    pub title: String,
    
    /// Download URL for the torrent/NZB file
    pub download_url: String,
    
    /// Info URL for the release page
    pub info_url: Option<String>,
    
    /// Indexer ID that provided this result
    pub indexer_id: i32,
    
    /// Indexer name
    pub indexer: String,
    
    /// Size of the release in bytes
    pub size: Option<i64>,
    
    /// Number of seeders (torrents only)
    pub seeders: Option<i32>,
    
    /// Number of leechers (torrents only)
    pub leechers: Option<i32>,
    
    /// Download factor (private trackers)
    pub download_factor: Option<f64>,
    
    /// Upload factor (private trackers)
    pub upload_factor: Option<f64>,
    
    /// Publication date
    pub publish_date: Option<DateTime<Utc>>,
    
    /// Categories (movie, tv, etc.)
    pub categories: Vec<Category>,
    
    /// Additional attributes from the indexer
    pub attributes: HashMap<String, serde_json::Value>,
    
    /// IMDB ID if available
    pub imdb_id: Option<String>,
    
    /// TMDB ID if available
    pub tmdb_id: Option<i32>,
    
    /// Whether this is a freeleech release
    pub freeleech: Option<bool>,
}

/// Category information for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
}

/// Prowlarr indexer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProwlarrIndexer {
    /// Unique identifier for the indexer
    pub id: i32,
    
    /// Human-readable name
    pub name: String,
    
    /// Implementation type (e.g., "torznab", "newznab")
    pub implementation: String,
    
    /// Base URL of the indexer
    pub base_url: String,
    
    /// Whether the indexer is currently enabled
    pub enable: bool,
    
    /// Current status of the indexer
    pub status: IndexerStatus,
    
    /// Supported categories
    pub categories: Vec<Category>,
    
    /// Capabilities of the indexer
    pub capabilities: IndexerCapabilities,
    
    /// Priority for this indexer (lower = higher priority)
    pub priority: i32,
    
    /// Whether this indexer supports RSS feeds
    pub supports_rss: bool,
    
    /// Whether this indexer supports search
    pub supports_search: bool,
    
    /// Last sync time
    pub last_sync: Option<DateTime<Utc>>,
}

/// Status of an indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerStatus {
    /// Overall health status
    pub status: String,
    
    /// Last error message if any
    pub last_error: Option<String>,
    
    /// Number of recent failures
    pub failure_count: i32,
    
    /// When the indexer was last tested
    pub last_test: Option<DateTime<Utc>>,
    
    /// Whether the indexer is currently disabled due to failures
    pub disabled_till: Option<DateTime<Utc>>,
}

/// Capabilities of an indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerCapabilities {
    /// Supported search parameters
    pub search_params: Vec<String>,
    
    /// Whether TV search is supported
    pub tv_search: bool,
    
    /// Whether movie search is supported  
    pub movie_search: bool,
    
    /// Whether music search is supported
    pub music_search: bool,
    
    /// Whether book search is supported
    pub book_search: bool,
    
    /// Maximum number of results per query
    pub limits: Option<SearchLimits>,
}

/// Search limits for an indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchLimits {
    /// Maximum results per page
    pub max: Option<i32>,
    
    /// Default results per page
    pub default: Option<i32>,
}

/// Search request parameters for Prowlarr
#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    /// Search query text
    pub query: Option<String>,
    
    /// IMDB ID to search for
    pub imdb_id: Option<String>,
    
    /// TMDB ID to search for
    pub tmdb_id: Option<i32>,
    
    /// Categories to search in
    pub categories: Vec<i32>,
    
    /// Specific indexer IDs to search (empty = all enabled)
    pub indexer_ids: Vec<i32>,
    
    /// Maximum number of results to return
    pub limit: Option<i32>,
    
    /// Offset for pagination
    pub offset: Option<i32>,
    
    /// Minimum seeders (torrents only)
    pub min_seeders: Option<i32>,
    
    /// Minimum size in bytes
    pub min_size: Option<i64>,
    
    /// Maximum size in bytes
    pub max_size: Option<i64>,
}

impl SearchRequest {
    /// Create a new search request for a movie by IMDB ID
    pub fn for_movie_imdb(imdb_id: &str) -> Self {
        Self {
            imdb_id: Some(imdb_id.to_string()),
            categories: vec![2000], // Movie category
            ..Default::default()
        }
    }
    
    /// Create a new search request for a movie by TMDB ID
    pub fn for_movie_tmdb(tmdb_id: i32) -> Self {
        Self {
            tmdb_id: Some(tmdb_id),
            categories: vec![2000], // Movie category
            ..Default::default()
        }
    }
    
    /// Create a new search request by title
    pub fn for_title(title: &str) -> Self {
        Self {
            query: Some(title.to_string()),
            categories: vec![2000], // Movie category
            ..Default::default()
        }
    }
    
    /// Set minimum seeders requirement
    pub fn with_min_seeders(mut self, min_seeders: i32) -> Self {
        self.min_seeders = Some(min_seeders);
        self
    }
    
    /// Limit results count
    pub fn with_limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Add specific indexer IDs to search
    pub fn with_indexers(mut self, indexer_ids: Vec<i32>) -> Self {
        self.indexer_ids = indexer_ids;
        self
    }
}

/// Response from a Prowlarr search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Total number of results found
    pub total: i32,
    
    /// Results in this page
    pub results: Vec<ProwlarrSearchResult>,
    
    /// Number of indexers that responded
    pub indexers_searched: i32,
    
    /// Number of indexers that had errors
    pub indexers_with_errors: i32,
    
    /// Any errors that occurred during search
    pub errors: Vec<SearchError>,
}

/// Error that occurred during search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchError {
    /// Indexer that had the error
    pub indexer: String,
    
    /// Error message
    pub message: String,
    
    /// Error code if available
    pub code: Option<String>,
}

/// Statistics about indexer performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerStats {
    /// Indexer ID
    pub indexer_id: i32,
    
    /// Total queries in the time period
    pub total_queries: i32,
    
    /// Successful queries
    pub successful_queries: i32,
    
    /// Failed queries
    pub failed_queries: i32,
    
    /// Average response time in milliseconds
    pub avg_response_time: f64,
    
    /// Success rate as percentage
    pub success_rate: f64,
    
    /// Time period these stats cover
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}