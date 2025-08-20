//! Simplified media service for initial integration
//!
//! This is a minimal implementation to get the application running.
//! TODO: Expand to full MediaService once repository implementations are complete.

use std::sync::Arc;
use radarr_core::{RadarrError, Result};
use radarr_indexers::IndexerClient;
use radarr_downloaders::QBittorrentClient;
use radarr_import::ImportPipeline;
use radarr_infrastructure::DatabasePool;
use tracing::{info, warn, debug, error, instrument};

/// Simplified media service for basic connectivity testing
pub struct SimplifiedMediaService {
    /// Database connection pool
    database_pool: DatabasePool,
    /// Indexer client for searching releases
    indexer_client: Arc<dyn IndexerClient + Send + Sync>,
    /// Download client for managing downloads
    download_client: Arc<QBittorrentClient>,
    /// Import pipeline for processing completed downloads
    import_pipeline: Arc<ImportPipeline>,
}

impl SimplifiedMediaService {
    /// Create a new simplified media service
    pub fn new(
        database_pool: DatabasePool,
        indexer_client: Arc<dyn IndexerClient + Send + Sync>,
        download_client: Arc<QBittorrentClient>,
        import_pipeline: Arc<ImportPipeline>,
    ) -> Self {
        Self {
            database_pool,
            indexer_client,
            download_client,
            import_pipeline,
        }
    }
    
    /// Initialize the service and test all components
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing simplified media service");
        
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
        
        info!("Simplified media service initialization complete");
        Ok(())
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
    use crate::config::{ProwlarrConfig, QBittorrentConfig};
    
    async fn create_test_service() -> SimplifiedMediaService {
        let db_config = radarr_infrastructure::DatabaseConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: 1,
            ..radarr_infrastructure::DatabaseConfig::default()
        };
        let pool = radarr_infrastructure::create_pool(db_config).await.unwrap();
        
        let prowlarr_config = radarr_indexers::ProwlarrConfig::default();
        let prowlarr_client = Arc::new(radarr_indexers::ProwlarrClient::new(prowlarr_config).unwrap());
        
        let qbittorrent_config = radarr_downloaders::QBittorrentConfig::default();
        let qbittorrent_client = Arc::new(radarr_downloaders::QBittorrentClient::new(qbittorrent_config).unwrap());
        
        let import_pipeline = Arc::new(ImportPipeline::default());
        
        SimplifiedMediaService::new(pool, prowlarr_client, qbittorrent_client, import_pipeline)
    }
    
    #[tokio::test]
    async fn test_simplified_service_creation() {
        let service = create_test_service().await;
        // Service should initialize without errors
        assert!(service.initialize().await.is_ok());
    }
}