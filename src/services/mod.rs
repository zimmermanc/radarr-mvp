//! Service layer for Radarr application
//!
//! This module provides the service layer that coordinates between all components:
//! - MediaService: Main service orchestrating movie workflows
//! - Component initialization and dependency injection
//! - Business logic coordination

use std::sync::Arc;
use radarr_core::{RadarrError, Result};
use radarr_indexers::{IndexerClient};
use radarr_downloaders::QBittorrentClient;
use radarr_import::ImportPipeline;
use radarr_infrastructure::DatabasePool;
use tracing::{info, debug};

pub mod simplified_media_service;

pub use simplified_media_service::*;

/// Application services container
#[derive(Clone)]
pub struct AppServices {
    /// Media service for movie operations
    pub media_service: Arc<SimplifiedMediaService>,
    /// Database pool
    pub database_pool: DatabasePool,
}

impl AppServices {
    /// Create new application services with all dependencies
    pub async fn new(
        database_pool: DatabasePool,
        prowlarr_client: Arc<dyn IndexerClient + Send + Sync>,
        qbittorrent_client: Arc<QBittorrentClient>,
        import_pipeline: Arc<ImportPipeline>,
    ) -> Result<Self> {
        let media_service = Arc::new(SimplifiedMediaService::new(
            database_pool.clone(),
            prowlarr_client,
            qbittorrent_client,
            import_pipeline,
        ));
        
        Ok(Self {
            media_service,
            database_pool,
        })
    }
    
    /// Initialize all services and test connectivity
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing application services");
        
        // Test database connectivity
        self.test_database().await?;
        
        // Initialize media service
        self.media_service.initialize().await?;
        
        info!("All services initialized successfully");
        Ok(())
    }
    
    /// Test database connectivity
    pub async fn test_database(&self) -> Result<()> {
        debug!("Testing database connectivity");
        
        // Try a simple query to verify database is accessible
        sqlx::query("SELECT 1")
            .fetch_one(&self.database_pool)
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "database_connectivity_test".to_string(),
                error: e.to_string(),
            })?;
            
        info!("Database connectivity verified");
        Ok(())
    }
}

/// Service initialization helper
pub struct ServiceBuilder {
    database_pool: Option<DatabasePool>,
    prowlarr_client: Option<Arc<dyn IndexerClient + Send + Sync>>,
    qbittorrent_client: Option<Arc<QBittorrentClient>>,
    import_pipeline: Option<Arc<ImportPipeline>>,
}

impl ServiceBuilder {
    pub fn new() -> Self {
        Self {
            database_pool: None,
            prowlarr_client: None,
            qbittorrent_client: None,
            import_pipeline: None,
        }
    }
    
    pub fn with_database(mut self, pool: DatabasePool) -> Self {
        self.database_pool = Some(pool);
        self
    }
    
    pub fn with_prowlarr(mut self, client: Arc<dyn IndexerClient + Send + Sync>) -> Self {
        self.prowlarr_client = Some(client);
        self
    }
    
    pub fn with_qbittorrent(mut self, client: Arc<QBittorrentClient>) -> Self {
        self.qbittorrent_client = Some(client);
        self
    }
    
    pub fn with_import_pipeline(mut self, pipeline: Arc<ImportPipeline>) -> Self {
        self.import_pipeline = Some(pipeline);
        self
    }
    
    pub async fn build(self) -> Result<AppServices> {
        let database_pool = self.database_pool.ok_or_else(|| RadarrError::ValidationError {
            field: "database_pool".to_string(),
            message: "Database pool is required".to_string(),
        })?;
        
        let prowlarr_client = self.prowlarr_client.ok_or_else(|| RadarrError::ValidationError {
            field: "prowlarr_client".to_string(),
            message: "Prowlarr client is required".to_string(),
        })?;
        
        let qbittorrent_client = self.qbittorrent_client.ok_or_else(|| RadarrError::ValidationError {
            field: "qbittorrent_client".to_string(),
            message: "qBittorrent client is required".to_string(),
        })?;
        
        let import_pipeline = self.import_pipeline.ok_or_else(|| RadarrError::ValidationError {
            field: "import_pipeline".to_string(),
            message: "Import pipeline is required".to_string(),
        })?;
        
        AppServices::new(
            database_pool,
            prowlarr_client,
            qbittorrent_client,
            import_pipeline,
        ).await
    }
}

impl Default for ServiceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_service_builder() {
        let db_config = radarr_infrastructure::DatabaseConfig {
            database_url: "sqlite::memory:".to_string(),
            max_connections: 1,
            ..radarr_infrastructure::DatabaseConfig::default()
        };
        let pool = radarr_infrastructure::create_pool(db_config).await.unwrap();
        let prowlarr_config = radarr_indexers::ProwlarrConfig::default();
        let prowlarr_client = Arc::new(radarr_indexers::ProwlarrClient::new(prowlarr_config).unwrap());
        let qbittorrent_config = radarr_downloaders::QBittorrentConfig::default();
        let qbittorrent_client = Arc::new(QBittorrentClient::new(qbittorrent_config).unwrap());
        let import_pipeline = Arc::new(ImportPipeline::default());
        
        let services = ServiceBuilder::new()
            .with_database(pool)
            .with_prowlarr(prowlarr_client)
            .with_qbittorrent(qbittorrent_client)
            .with_import_pipeline(import_pipeline)
            .build()
            .await;
            
        assert!(services.is_ok());
    }
    
    #[test]
    fn test_incomplete_service_builder() {
        let builder = ServiceBuilder::new();
        
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = builder.build().await;
            assert!(result.is_err());
        });
    }
}