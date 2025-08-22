//! Service layer for Radarr application
//!
//! This module provides the service layer that coordinates between all components:
//! - MediaService: Main service orchestrating movie workflows
//! - Component initialization and dependency injection
//! - Business logic coordination

use std::sync::Arc;
use radarr_core::{RadarrError, Result, EventBus, EventProcessor, QueueProcessor, QueueProcessorConfig};
use radarr_indexers::{IndexerClient};
use radarr_downloaders::QBittorrentClient;
use radarr_import::ImportPipeline;
use radarr_infrastructure::{DatabasePool, PostgresQueueRepository, QBittorrentDownloadClient};
use tracing::{info, debug, error, warn, instrument};

pub mod simplified_media_service;
pub mod workflow;
pub mod rss_service;

pub use simplified_media_service::*;
pub use workflow::*;
pub use rss_service::*;

/// Application services container
#[derive(Clone)]
pub struct AppServices {
    /// Media service for movie operations
    pub media_service: Arc<SimplifiedMediaService>,
    /// Database pool
    pub database_pool: DatabasePool,
    /// Indexer client for direct API access
    pub indexer_client: Arc<dyn IndexerClient + Send + Sync>,
    /// Event bus for inter-component communication
    pub event_bus: Arc<EventBus>,
    /// Queue processor for background download processing
    pub queue_processor: Option<Arc<QueueProcessor<PostgresQueueRepository, QBittorrentDownloadClient>>>,
    /// RSS monitoring service
    pub rss_service: Option<Arc<RssService>>,
}

impl AppServices {
    /// Create new application services with all dependencies
    pub async fn new(
        database_pool: DatabasePool,
        prowlarr_client: Arc<dyn IndexerClient + Send + Sync>,
        qbittorrent_client: Arc<QBittorrentClient>,
        import_pipeline: Arc<ImportPipeline>,
    ) -> Result<Self> {
        // Create event bus
        let event_bus = Arc::new(EventBus::new());
        
        let media_service = Arc::new(SimplifiedMediaService::new(
            database_pool.clone(),
            prowlarr_client.clone(),
            qbittorrent_client.clone(),
            import_pipeline,
        ));
        
        Ok(Self {
            media_service,
            database_pool,
            indexer_client: prowlarr_client,
            event_bus,
            queue_processor: None, // Will be initialized separately
            rss_service: None, // Will be initialized separately
        })
    }
    
    /// Initialize queue processor with proper configuration
    pub fn initialize_queue_processor(
        &mut self,
        qbittorrent_config: radarr_downloaders::QBittorrentConfig,
    ) -> Result<()> {
        // Create queue repository
        let queue_repo = Arc::new(PostgresQueueRepository::new(self.database_pool.clone()));
        
        // Create download client service adapter
        let download_client = Arc::new(QBittorrentDownloadClient::new(qbittorrent_config)?);
        
        // Create queue processor
        let queue_config = QueueProcessorConfig::default();
        let queue_processor = Arc::new(QueueProcessor::new(
            queue_config,
            queue_repo,
            download_client,
        ));
        
        self.queue_processor = Some(queue_processor);
        Ok(())
    }
    
    /// Initialize RSS service
    pub fn initialize_rss_service(
        &mut self,
        config: RssServiceConfig,
    ) -> Result<()> {
        let rss_service = Arc::new(RssService::new(
            config,
            self.indexer_client.clone(),
            self.database_pool.clone(),
        ).with_event_bus(self.event_bus.clone()));
        
        self.rss_service = Some(rss_service);
        Ok(())
    }
    
    /// Start the RSS service in the background
    pub async fn start_rss_service(&self) -> Result<()> {
        if let Some(rss_service) = &self.rss_service {
            rss_service.clone().start().await?;
            info!("RSS monitoring service started successfully");
        } else {
            warn!("RSS service not initialized, skipping start");
        }
        Ok(())
    }
    
    /// Start the queue processor in the background
    pub async fn start_queue_processor(&mut self) -> Result<()> {
        if let Some(queue_processor) = self.queue_processor.take() {
            // Extract the processor from Arc to pass ownership to start()
            let processor = Arc::try_unwrap(queue_processor)
                .map_err(|_| RadarrError::ValidationError {
                    field: "queue_processor".to_string(),
                    message: "Cannot extract queue processor from Arc - multiple references exist".to_string(),
                })?;
            
            tokio::spawn(async move {
                info!("Starting queue processor...");
                if let Err(e) = processor.start().await {
                    error!("Queue processor failed: {}", e);
                }
            });
            info!("Queue processor started successfully");
        } else {
            return Err(RadarrError::ValidationError {
                field: "queue_processor".to_string(),
                message: "Queue processor not initialized".to_string(),
            });
        }
        Ok(())
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

    /// Start event processing with all handlers
    pub async fn start_event_processing(&self) -> Result<()> {
        info!("Starting event processing system");
        
        // Create event handlers
        let logging_handler = Arc::new(LoggingEventHandler::new());
        let download_import_handler = Arc::new(DownloadImportHandler::new(
            self.media_service.import_pipeline.clone(),
            self.database_pool.clone(),
        ));
        
        // Create event processor
        let event_processor = EventProcessor::new(&self.event_bus)
            .add_handler(logging_handler)
            .add_handler(download_import_handler);
        
        // Start event processor in background
        let event_bus = self.event_bus.clone();
        tokio::spawn(async move {
            if let Err(e) = event_processor.run().await {
                error!("Event processor failed: {}", e);
            }
        });
        
        info!("Event processing system started with {} subscribers", 
              self.event_bus.subscriber_count());
        
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
    qbittorrent_config: Option<radarr_downloaders::QBittorrentConfig>,
    import_pipeline: Option<Arc<ImportPipeline>>,
}

impl ServiceBuilder {
    pub fn new() -> Self {
        Self {
            database_pool: None,
            prowlarr_client: None,
            qbittorrent_client: None,
            qbittorrent_config: None,
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
    
    pub fn with_qbittorrent_config(mut self, config: radarr_downloaders::QBittorrentConfig) -> Self {
        self.qbittorrent_config = Some(config);
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
        
        let mut services = AppServices::new(
            database_pool.clone(),
            prowlarr_client,
            qbittorrent_client,
            import_pipeline,
        ).await?;
        
        // Initialize queue processor if config is provided
        if let Some(qbittorrent_config) = self.qbittorrent_config {
            services.initialize_queue_processor(qbittorrent_config)?;
        }
        
        Ok(services)
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