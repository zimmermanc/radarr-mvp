//! Domain repositories
//!
//! This module defines the repository traits that provide abstraction
//! over data persistence for domain entities.

use crate::models::*;
use crate::error::Result;
use async_trait::async_trait;
use uuid::Uuid;

/// Repository trait for Movie entities
#[async_trait]
pub trait MovieRepository: Send + Sync {
    /// Find a movie by its ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Movie>>;
    
    /// Find a movie by its TMDB ID
    async fn find_by_tmdb_id(&self, tmdb_id: i32) -> Result<Option<Movie>>;
    
    /// Find a movie by its IMDB ID
    async fn find_by_imdb_id(&self, imdb_id: &str) -> Result<Option<Movie>>;
    
    /// Get all monitored movies
    async fn find_monitored(&self) -> Result<Vec<Movie>>;
    
    /// Get movies without files
    async fn find_missing_files(&self) -> Result<Vec<Movie>>;
    
    /// Search movies by title
    async fn search_by_title(&self, query: &str, limit: i32) -> Result<Vec<Movie>>;
    
    /// Create a new movie
    async fn create(&self, movie: &Movie) -> Result<Movie>;
    
    /// Update an existing movie
    async fn update(&self, movie: &Movie) -> Result<Movie>;
    
    /// Delete a movie by ID
    async fn delete(&self, id: Uuid) -> Result<()>;
    
    /// List all movies with pagination
    async fn list(&self, offset: i64, limit: i32) -> Result<Vec<Movie>>;
    
    /// Count total movies
    async fn count(&self) -> Result<i64>;
    
    /// Update last search time
    async fn update_last_search_time(&self, id: Uuid) -> Result<()>;
}

/// Repository trait for Indexer entities
#[async_trait]
pub trait IndexerRepository: Send + Sync {
    /// Find an indexer by its ID
    async fn find_by_id(&self, id: i32) -> Result<Option<Indexer>>;
    
    /// Find an indexer by its name
    async fn find_by_name(&self, name: &str) -> Result<Option<Indexer>>;
    
    /// Get all enabled indexers
    async fn find_enabled(&self) -> Result<Vec<Indexer>>;
    
    /// Create a new indexer
    async fn create(&self, indexer: &Indexer) -> Result<Indexer>;
    
    /// Update an existing indexer
    async fn update(&self, indexer: &Indexer) -> Result<Indexer>;
    
    /// Delete an indexer by ID
    async fn delete(&self, id: i32) -> Result<()>;
    
    /// List all indexers
    async fn list(&self) -> Result<Vec<Indexer>>;
    
    /// Test indexer connection
    async fn test_connection(&self, id: i32) -> Result<bool>;
}

/// Repository trait for QualityProfile entities
#[async_trait]
pub trait QualityProfileRepository: Send + Sync {
    /// Find a quality profile by its ID
    async fn find_by_id(&self, id: i32) -> Result<Option<QualityProfile>>;
    
    /// Find a quality profile by its name
    async fn find_by_name(&self, name: &str) -> Result<Option<QualityProfile>>;
    
    /// Create a new quality profile
    async fn create(&self, profile: &QualityProfile) -> Result<QualityProfile>;
    
    /// Update an existing quality profile
    async fn update(&self, profile: &QualityProfile) -> Result<QualityProfile>;
    
    /// Delete a quality profile by ID
    async fn delete(&self, id: i32) -> Result<()>;
    
    /// List all quality profiles
    async fn list(&self) -> Result<Vec<QualityProfile>>;
    
    /// Get the default quality profile
    async fn get_default(&self) -> Result<Option<QualityProfile>>;
}

/// Repository trait for Download entities
#[async_trait]
pub trait DownloadRepository: Send + Sync {
    /// Find a download by its ID
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Download>>;
    
    /// Find downloads by movie ID
    async fn find_by_movie_id(&self, movie_id: Uuid) -> Result<Vec<Download>>;
    
    /// Find downloads by status
    async fn find_by_status(&self, status: DownloadStatus) -> Result<Vec<Download>>;
    
    /// Find active downloads (downloading)
    async fn find_active(&self) -> Result<Vec<Download>>;
    
    /// Find completed but not imported downloads
    async fn find_completed_not_imported(&self) -> Result<Vec<Download>>;
    
    /// Create a new download
    async fn create(&self, download: &Download) -> Result<Download>;
    
    /// Update an existing download
    async fn update(&self, download: &Download) -> Result<Download>;
    
    /// Delete a download by ID
    async fn delete(&self, id: Uuid) -> Result<()>;
    
    /// List downloads with pagination
    async fn list(&self, offset: i64, limit: i32) -> Result<Vec<Download>>;
    
    /// Clean up old completed downloads
    async fn cleanup_old(&self, days: i32) -> Result<i64>;
}
