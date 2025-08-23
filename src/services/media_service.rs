//! Media service for coordinating movie operations
//!
//! This service orchestrates the complete movie workflow:
//! 1. Search for movies using indexers (Prowlarr)
//! 2. Download releases using download clients (qBittorrent)
//! 3. Import completed downloads into the media library

use std::sync::Arc;
use std::path::Path;
use radarr_core::{
    RadarrError, Result, Movie, Download, DownloadStatus, QualityProfile,
    repositories::{MovieRepository, DownloadRepository, QualityProfileRepository},
    correlation::{CorrelationContext, set_current_context},
    tracing::{info_with_correlation, debug_with_correlation, warn_with_correlation, error_with_correlation}
};
use radarr_indexers::{IndexerClient, SearchRequest, SearchResponse};
use radarr_downloaders::{QBittorrentClient, AddTorrentParams, TorrentData, TorrentInfo};
use radarr_import::{ImportPipeline, ImportResult, ImportStats};
use radarr_infrastructure::{
    DatabasePool, PostgresMovieRepository, PostgresDownloadRepository, 
    PostgresIndexerRepository, PostgresQualityProfileRepository
};
use tracing::{info, warn, debug, error, instrument};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Main media service coordinating all movie operations
pub struct MediaService {
    /// Database connection pool
    database_pool: DatabasePool,
    /// Indexer client for searching releases
    indexer_client: Arc<dyn IndexerClient + Send + Sync>,
    /// Download client for managing downloads
    download_client: Arc<QBittorrentClient>,
    /// Import pipeline for processing completed downloads
    import_pipeline: Arc<ImportPipeline>,
    /// Movie repository
    movie_repo: PostgresMovieRepository,
    /// Download repository
    download_repo: PostgresDownloadRepository,
    /// Quality profile repository
    quality_profile_repo: PostgresQualityProfileRepository,
}

/// Parameters for searching movies
#[derive(Debug, Clone)]
pub struct MovieSearchParams {
    /// Movie title
    pub title: String,
    /// Release year (optional)
    pub year: Option<i32>,
    /// IMDb ID (optional)
    pub imdb_id: Option<String>,
    /// TMDb ID (optional)
    pub tmdb_id: Option<i32>,
    /// Quality profile ID
    pub quality_profile_id: Uuid,
    /// Minimum seeders
    pub min_seeders: Option<u32>,
    /// Maximum results
    pub max_results: Option<u32>,
}

/// Result of a movie search operation
#[derive(Debug, Clone)]
pub struct MovieSearchResult {
    /// Search response from indexer
    pub search_response: SearchResponse,
    /// Number of results found
    pub result_count: usize,
    /// Quality profile used for filtering
    pub quality_profile: QualityProfile,
}

/// Parameters for downloading a release
#[derive(Debug, Clone)]
pub struct DownloadParams {
    /// Movie ID
    pub movie_id: Uuid,
    /// Release title
    pub release_title: String,
    /// Download URL (magnet or torrent file URL)
    pub download_url: String,
    /// File size in bytes
    pub size: u64,
    /// Quality profile ID
    pub quality_profile_id: Uuid,
    /// Indexer name
    pub indexer: String,
    /// Category (optional)
    pub category: Option<String>,
}

/// Status of a workflow operation
#[derive(Debug, Clone)]
pub struct WorkflowStatus {
    /// Operation ID
    pub id: Uuid,
    /// Current status
    pub status: String,
    /// Progress percentage (0-100)
    pub progress: f32,
    /// Status message
    pub message: String,
    /// Error if operation failed
    pub error: Option<String>,
    /// Timestamp of last update
    pub updated_at: DateTime<Utc>,
}

impl MediaService {
    /// Create a new media service
    pub fn new(
        database_pool: DatabasePool,
        indexer_client: Arc<dyn IndexerClient + Send + Sync>,
        download_client: Arc<QBittorrentClient>,
        import_pipeline: Arc<ImportPipeline>,
    ) -> Self {
        let movie_repo = PostgresMovieRepository::new(database_pool.clone());
        let download_repo = PostgresDownloadRepository::new(database_pool.clone());
        let quality_profile_repo = PostgresQualityProfileRepository::new(database_pool.clone());
        
        Self {
            database_pool,
            indexer_client,
            download_client,
            import_pipeline,
            movie_repo,
            download_repo,
            quality_profile_repo,
        }
    }
    
    /// Initialize the service and test all components
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing media service");
        
        // Test indexer connectivity
        match self.indexer_client.health_check().await {
            Ok(true) => info!("Indexer client connected successfully"),
            Ok(false) => warn!("Indexer client health check failed"),
            Err(e) => warn!("Indexer client connectivity test failed: {}", e),
        }
        
        // Test download client connectivity
        match self.download_client.test_connection().await {
            Ok(()) => info!("Download client connected successfully"),
            Err(e) => warn!("Download client connectivity test failed: {}", e),
        }
        
        // Validate import pipeline configuration
        match self.import_pipeline.validate_config() {
            Ok(()) => info!("Import pipeline configuration valid"),
            Err(e) => warn!("Import pipeline configuration invalid: {}", e),
        }
        
        info!("Media service initialization complete");
        Ok(())
    }
    
    /// Search for movies using the configured indexer
    #[instrument(skip(self), fields(title = %params.title))]
    pub async fn search_movie(&self, params: MovieSearchParams) -> Result<MovieSearchResult> {
        // Create correlation context for this search operation
        let context = CorrelationContext::new("media_service.search_movie");
        set_current_context(context);
        
        debug_with_correlation(format!("Searching for movie: {}", params.title));
        
        // Get quality profile for filtering
        let quality_profile = self.quality_profile_repo.get_by_id(params.quality_profile_id).await?
            .ok_or_else(|| RadarrError::NotFoundError {
                entity: "QualityProfile".to_string(),
                id: params.quality_profile_id.to_string(),
            })?;
        
        // Build search request
        let mut search_request = SearchRequest::for_movie_title(&params.title);
        
        if let Some(year) = params.year {
            // For title searches, we can append the year
            search_request.query = Some(format!("{} {}", params.title, year));
        }
        
        if let Some(imdb_id) = params.imdb_id {
            search_request.imdb_id = Some(imdb_id);
        }
        
        if let Some(tmdb_id) = params.tmdb_id {
            search_request.tmdb_id = Some(tmdb_id);
        }
        
        if let Some(min_seeders) = params.min_seeders {
            search_request.min_seeders = Some(min_seeders);
        }
        
        if let Some(limit) = params.max_results {
            search_request.limit = Some(limit);
        }
        
        // Execute search
        let search_response = self.indexer_client.search(&search_request).await?;
        let result_count = search_response.results.len();
        
        info_with_correlation(format!("Found {} results for movie search: {}", result_count, params.title));
        
        Ok(MovieSearchResult {
            search_response,
            result_count,
            quality_profile,
        })
    }
    
    /// Download a release
    #[instrument(skip(self), fields(movie_id = %params.movie_id, release = %params.release_title))]
    pub async fn download_release(&self, params: DownloadParams) -> Result<Uuid> {
        // Create correlation context for this download operation
        let context = CorrelationContext::new("media_service.download_release");
        set_current_context(context);
        
        info_with_correlation(format!("Starting download for release: {}", params.release_title));
        
        // Verify movie exists
        let movie = self.movie_repo.get_by_id(params.movie_id).await?
            .ok_or_else(|| RadarrError::NotFoundError {
                entity: "Movie".to_string(),
                id: params.movie_id.to_string(),
            })?;
        
        // Create download record
        let download_id = Uuid::new_v4();
        let download = Download {
            id: download_id,
            movie_id: params.movie_id,
            release_title: params.release_title.clone(),
            download_url: params.download_url.clone(),
            status: DownloadStatus::Queued,
            progress: 0.0,
            size: params.size,
            downloaded: 0,
            download_client: "qbittorrent".to_string(),
            download_client_id: None,
            indexer: params.indexer.clone(),
            quality_profile_id: params.quality_profile_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            error_message: None,
        };
        
        // Save download to database
        self.download_repo.create(&download).await?;
        
        // Prepare torrent parameters
        let torrent_params = AddTorrentParams {
            torrent_data: TorrentData::Url(params.download_url.clone()),
            category: params.category.or_else(|| Some("radarr".to_string())),
            save_path: None, // Use default
            paused: false,
            skip_checking: false,
            priority: 0,
        };
        
        // Add torrent to download client
        match self.download_client.add_torrent(torrent_params).await {
            Ok(client_id) => {
                // Update download record with client ID
                let mut updated_download = download;
                updated_download.status = DownloadStatus::Downloading;
                updated_download.download_client_id = Some(client_id);
                updated_download.updated_at = Utc::now();
                
                self.download_repo.update(&updated_download).await?;
                
                info!("Successfully added download to client: {}", params.release_title);
                Ok(download_id)
            }
            Err(e) => {
                // Update download record with error
                let mut failed_download = download;
                failed_download.status = DownloadStatus::Failed;
                failed_download.error_message = Some(e.to_string());
                failed_download.updated_at = Utc::now();
                
                self.download_repo.update(&failed_download).await?;
                
                error!("Failed to add download to client: {}", e);
                Err(e)
            }
        }
    }
    
    /// Import completed downloads
    #[instrument(skip(self))]
    pub async fn import_completed(&self, download_path: &Path, destination_path: &Path) -> Result<ImportStats> {
        info!("Starting import from {} to {}", download_path.display(), destination_path.display());
        
        // Execute import pipeline
        let import_stats = self.import_pipeline
            .import_directory(download_path, destination_path)
            .await?;
        
        info!("Import completed: {} successful, {} failed", 
              import_stats.successful_imports, import_stats.failed_imports);
        
        Ok(import_stats)
    }
    
    /// Process complete workflow: search, download, and queue for import
    #[instrument(skip(self), fields(title = %search_params.title))]
    pub async fn process_workflow(
        &self,
        search_params: MovieSearchParams,
        auto_download: bool,
    ) -> Result<WorkflowStatus> {
        let workflow_id = Uuid::new_v4();
        info!("Starting workflow for movie: {}", search_params.title);
        
        // Step 1: Search for movie
        let search_result = match self.search_movie(search_params.clone()).await {
            Ok(result) => result,
            Err(e) => {
                return Ok(WorkflowStatus {
                    id: workflow_id,
                    status: "failed".to_string(),
                    progress: 0.0,
                    message: "Search failed".to_string(),
                    error: Some(e.to_string()),
                    updated_at: Utc::now(),
                });
            }
        };
        
        if search_result.result_count == 0 {
            return Ok(WorkflowStatus {
                id: workflow_id,
                status: "completed".to_string(),
                progress: 100.0,
                message: "No releases found".to_string(),
                error: None,
                updated_at: Utc::now(),
            });
        }
        
        if !auto_download {
            return Ok(WorkflowStatus {
                id: workflow_id,
                status: "awaiting_selection".to_string(),
                progress: 50.0,
                message: format!("Found {} releases, awaiting user selection", search_result.result_count),
                error: None,
                updated_at: Utc::now(),
            });
        }
        
        // Step 2: Auto-select best release (simplified logic)
        let best_release = search_result.search_response.results
            .into_iter()
            .max_by_key(|r| (r.seeders, r.size));
        
        if let Some(release) = best_release {
            // Create a dummy movie for this workflow (in real implementation, this would be provided)
            let movie_id = Uuid::new_v4();
            
            let download_params = DownloadParams {
                movie_id,
                release_title: release.title.clone(),
                download_url: release.download_url.clone(),
                size: release.size,
                quality_profile_id: search_params.quality_profile_id,
                indexer: release.indexer.clone(),
                category: Some("radarr".to_string()),
            };
            
            // Step 3: Start download
            match self.download_release(download_params).await {
                Ok(_) => {
                    Ok(WorkflowStatus {
                        id: workflow_id,
                        status: "downloading".to_string(),
                        progress: 75.0,
                        message: format!("Started download: {}", release.title),
                        error: None,
                        updated_at: Utc::now(),
                    })
                }
                Err(e) => {
                    Ok(WorkflowStatus {
                        id: workflow_id,
                        status: "failed".to_string(),
                        progress: 50.0,
                        message: "Download failed".to_string(),
                        error: Some(e.to_string()),
                        updated_at: Utc::now(),
                    })
                }
            }
        } else {
            Ok(WorkflowStatus {
                id: workflow_id,
                status: "completed".to_string(),
                progress: 100.0,
                message: "No suitable releases found".to_string(),
                error: None,
                updated_at: Utc::now(),
            })
        }
    }
    
    /// Get download status
    pub async fn get_download_status(&self, download_id: Uuid) -> Result<Option<Download>> {
        self.download_repo.get_by_id(download_id).await
    }
    
    /// Update download progress by syncing with download client
    #[instrument(skip(self))]
    pub async fn sync_download_progress(&self) -> Result<usize> {
        debug!("Syncing download progress with client");
        
        // Get all active downloads
        let active_downloads = self.download_repo.get_by_status(DownloadStatus::Downloading).await?;
        
        if active_downloads.is_empty() {
            return Ok(0);
        }
        
        // Get current torrents from client
        let torrents = self.download_client.get_torrents().await?;
        let mut updated_count = 0;
        
        for mut download in active_downloads {
            if let Some(client_id) = &download.download_client_id {
                if let Some(torrent) = torrents.iter().find(|t| &t.hash == client_id || &t.name == client_id) {
                    let old_progress = download.progress;
                    download.progress = (torrent.progress * 100.0) as f32;
                    download.downloaded = torrent.completed;
                    download.updated_at = Utc::now();
                    
                    // Check if completed
                    if torrent.progress >= 1.0 && download.status != DownloadStatus::Completed {
                        download.status = DownloadStatus::Completed;
                        download.completed_at = Some(Utc::now());
                        info!("Download completed: {}", download.release_title);
                    }
                    
                    // Only update if progress changed significantly
                    if (download.progress - old_progress).abs() > 1.0 {
                        self.download_repo.update(&download).await?;
                        updated_count += 1;
                    }
                }
            }
        }
        
        if updated_count > 0 {
            debug!("Updated progress for {} downloads", updated_count);
        }
        
        Ok(updated_count)
    }
    
    /// Get all movies
    pub async fn get_movies(&self) -> Result<Vec<Movie>> {
        self.movie_repo.get_all().await
    }
    
    /// Get movie by ID
    pub async fn get_movie(&self, id: Uuid) -> Result<Option<Movie>> {
        self.movie_repo.get_by_id(id).await
    }
    
    /// Get all downloads
    pub async fn get_downloads(&self) -> Result<Vec<Download>> {
        self.download_repo.get_all().await
    }
    
    /// Get downloads by status
    pub async fn get_downloads_by_status(&self, status: DownloadStatus) -> Result<Vec<Download>> {
        self.download_repo.get_by_status(status).await
    }
    
    /// Test indexer connectivity (used by health checks)
    pub async fn test_indexer_connectivity(&self) -> Result<()> {
        debug!("Testing indexer connectivity");
        match self.indexer_client.health_check().await {
            Ok(true) => {
                debug!("Indexer connectivity test passed");
                Ok(())
            }
            Ok(false) => {
                Err(RadarrError::ExternalServiceError {
                    service: "indexer".to_string(),
                    error: "Health check returned false".to_string(),
                })
            }
            Err(e) => {
                error!("Indexer connectivity test failed: {}", e);
                Err(e)
            }
        }
    }
    
    /// Test downloader connectivity (used by health checks)
    pub async fn test_downloader_connectivity(&self) -> Result<()> {
        debug!("Testing downloader connectivity");
        match self.download_client.test_connection().await {
            Ok(()) => {
                debug!("Downloader connectivity test passed");
                Ok(())
            }
            Err(e) => {
                error!("Downloader connectivity test failed: {}", e);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radarr_indexers::ProwlarrConfig;
    use radarr_downloaders::QBittorrentConfig;
    
    async fn create_test_service() -> MediaService {
        let db_config = radarr_infrastructure::DatabaseConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: 1,
            ..radarr_infrastructure::DatabaseConfig::default()
        };
        let pool = radarr_infrastructure::create_pool(db_config).await.unwrap();
        let prowlarr_config = ProwlarrConfig::default();
        let prowlarr_client = Arc::new(radarr_indexers::ProwlarrClient::new(prowlarr_config).unwrap());
        let qbittorrent_config = QBittorrentConfig::default();
        let qbittorrent_client = Arc::new(QBittorrentClient::new(qbittorrent_config).unwrap());
        let import_pipeline = Arc::new(ImportPipeline::default());
        
        MediaService::new(pool, prowlarr_client, qbittorrent_client, import_pipeline)
    }
    
    #[tokio::test]
    async fn test_media_service_creation() {
        let service = create_test_service().await;
        // Service should initialize without errors
        assert!(service.initialize().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_search_params() {
        let params = MovieSearchParams {
            title: "The Matrix".to_string(),
            year: Some(1999),
            imdb_id: Some("tt0133093".to_string()),
            tmdb_id: Some(603),
            quality_profile_id: Uuid::new_v4(),
            min_seeders: Some(5),
            max_results: Some(10),
        };
        
        assert_eq!(params.title, "The Matrix");
        assert_eq!(params.year, Some(1999));
        assert_eq!(params.min_seeders, Some(5));
    }
}