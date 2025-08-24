//! API request and response models
//!
//! This module contains all the data transfer objects (DTOs) used for
//! API requests and responses, implementing proper serialization and validation.

use chrono::{DateTime, Utc};
use radarr_core::{Download, MinimumAvailability, Movie, MovieStatus};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Pagination parameters for list endpoints
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of items per page
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    /// Sort field
    pub sort_key: Option<String>,
    /// Sort direction
    pub sort_dir: Option<SortDirection>,
}

/// Sort direction for pagination
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    50
}

impl PaginationParams {
    /// Convert to SQL LIMIT and OFFSET
    pub fn to_sql_params(&self) -> (i32, i64) {
        let limit = self.page_size.min(1000) as i32; // Cap at 1000
        let offset = ((self.page.saturating_sub(1)) * self.page_size) as i64;
        (limit, offset)
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Current page number
    pub page: u32,
    /// Number of items per page  
    pub page_size: u32,
    /// Total number of items
    pub total_records: i64,
    /// Total number of pages
    pub total_pages: u32,
    /// Items in this page
    pub records: Vec<T>,
}

impl<T> PaginatedResponse<T> {
    pub fn new(page: u32, page_size: u32, total_count: i64, records: Vec<T>) -> Self {
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as u32;
        Self {
            page,
            page_size,
            total_records: total_count,
            total_pages,
            records,
        }
    }
}

/// Movie API response model
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieResponse {
    pub id: Uuid,
    pub tmdb_id: i32,
    pub imdb_id: Option<String>,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub runtime: Option<i32>,
    pub status: MovieStatus,
    pub monitored: bool,
    pub quality_profile_id: Option<i32>,
    pub minimum_availability: MinimumAvailability,
    pub has_file: bool,
    pub movie_file_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub alternative_titles: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_search_time: Option<DateTime<Utc>>,
    pub last_info_sync: Option<DateTime<Utc>>,
    /// Computed fields
    pub rating: Option<f64>,
    pub overview: Option<String>,
}

impl From<Movie> for MovieResponse {
    fn from(movie: Movie) -> Self {
        let rating = movie.rating();
        let overview = movie.overview().map(|s| s.to_string());

        Self {
            id: movie.id,
            tmdb_id: movie.tmdb_id,
            imdb_id: movie.imdb_id,
            title: movie.title,
            original_title: movie.original_title,
            year: movie.year,
            runtime: movie.runtime,
            status: movie.status,
            monitored: movie.monitored,
            quality_profile_id: movie.quality_profile_id,
            minimum_availability: movie.minimum_availability,
            has_file: movie.has_file,
            movie_file_id: movie.movie_file_id,
            metadata: movie.metadata,
            alternative_titles: movie.alternative_titles,
            created_at: movie.created_at,
            updated_at: movie.updated_at,
            last_search_time: movie.last_search_time,
            last_info_sync: movie.last_info_sync,
            rating,
            overview,
        }
    }
}

/// Movie creation request
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMovieRequest {
    pub tmdb_id: i32,
    #[serde(default = "default_monitored")]
    pub monitored: bool,
    pub quality_profile_id: Option<i32>,
    pub minimum_availability: Option<MinimumAvailability>,
    /// Custom title override (optional)
    pub title: Option<String>,
    /// Additional metadata to merge
    pub metadata: Option<serde_json::Value>,
}

fn default_monitored() -> bool {
    true
}

/// Movie update request
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMovieRequest {
    pub monitored: Option<bool>,
    pub quality_profile_id: Option<i32>,
    pub minimum_availability: Option<MinimumAvailability>,
    pub metadata: Option<serde_json::Value>,
}

/// Search request for movies via indexers
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    /// Movie title to search for
    pub title: String,
    /// Release year (optional)
    pub year: Option<i32>,
    /// IMDB ID (optional)
    pub imdb_id: Option<String>,
    /// TMDB ID (optional)
    pub tmdb_id: Option<i32>,
    /// Quality profile ID for filtering
    pub quality_profile_id: Option<Uuid>,
    /// Minimum number of seeders
    pub min_seeders: Option<u32>,
    /// Maximum number of results
    pub max_results: Option<u32>,
    /// Specific indexer IDs to search (empty = all)
    pub indexer_ids: Option<Vec<i32>>,
}

/// Search response containing found releases
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    /// Total number of results found
    pub total: i32,
    /// Search results
    pub releases: Vec<ReleaseResponse>,
    /// Number of indexers searched
    pub indexers_searched: i32,
    /// Number of indexers with errors
    pub indexers_with_errors: i32,
    /// Search execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Individual release/torrent result
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResponse {
    pub guid: String,
    pub title: String,
    pub download_url: String,
    pub info_url: Option<String>,
    pub indexer: String,
    pub indexer_id: i32,
    pub size: Option<i64>,
    pub seeders: Option<i32>,
    pub leechers: Option<i32>,
    pub download_factor: Option<f64>,
    pub upload_factor: Option<f64>,
    pub publish_date: Option<DateTime<Utc>>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i32>,
    pub freeleech: Option<bool>,
    /// Computed quality score (0-100)
    pub quality_score: Option<u8>,
    /// Progress percentage for active downloads
    pub progress: f64,
}

/// Download request to start downloading a release
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    /// Release GUID
    pub guid: String,
    /// Indexer ID
    pub indexer_id: i32,
}

/// Download status response matching the core Download model
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResponse {
    pub id: Uuid,
    pub movie_id: Option<Uuid>,
    pub download_client_id: i32,
    pub indexer_id: Option<i32>,
    pub download_id: String,
    pub title: String,
    pub category: Option<String>,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub size_left: Option<i64>,
    pub quality: serde_json::Value,
    pub download_time: Option<DateTime<Utc>>,
    pub completion_time: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub imported: bool,
    pub import_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Computed progress percentage (0-100)
    pub progress: f64,
    /// Computed ETA in seconds
    pub eta_seconds: Option<u64>,
}

impl From<Download> for DownloadResponse {
    fn from(download: Download) -> Self {
        let progress = download.progress_percentage().unwrap_or(0.0);
        let eta_seconds = download.eta_seconds();

        Self {
            id: download.id,
            movie_id: Some(download.movie_id),
            download_client_id: download.download_client_id,
            indexer_id: download.indexer_id,
            download_id: download.download_id,
            title: download.title,
            category: download.category,
            status: download.status.to_string(),
            size_bytes: download.size_bytes,
            size_left: download.size_left,
            quality: download.quality,
            download_time: download.download_time,
            completion_time: download.completion_time,
            error_message: download.error_message,
            imported: download.imported,
            import_time: download.import_time,
            created_at: download.created_at,
            updated_at: download.updated_at,
            progress,
            eta_seconds,
        }
    }
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub services: Vec<ServiceHealth>,
}

/// Individual service health status
#[derive(Debug, Serialize)]
pub struct ServiceHealth {
    pub name: String,
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
}

/// Generic API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.clone()),
            message: Some(message),
        }
    }
}

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    pub sort_key: Option<String>,
    pub sort_dir: Option<SortDirection>,
}

impl PaginationQuery {
    pub fn to_sql_params(&self) -> (i32, i64) {
        let limit = self.page_size.min(1000) as i32;
        let offset = ((self.page.saturating_sub(1)) * self.page_size) as i64;
        (limit, offset)
    }
}
