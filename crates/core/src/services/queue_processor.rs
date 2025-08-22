//! Background queue processor service
//!
//! This service runs in the background to automatically process queued items,
//! monitor download progress, and sync with download clients.

use crate::services::{QueueRepository, DownloadClientService};
use crate::{Result, RadarrError};
use crate::retry::{retry_with_backoff, RetryConfig, RetryPolicy, CircuitBreaker};
use crate::progress::{ProgressTracker, OperationType};
use crate::events::{EventBus, SystemEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for queue processor
#[derive(Debug, Clone)]
pub struct QueueProcessorConfig {
    /// Maximum concurrent downloads
    pub max_concurrent_downloads: usize,
    /// How often to check for new queue items (seconds)
    pub check_interval_seconds: u64,
    /// How often to sync with download client (seconds)  
    pub sync_interval_seconds: u64,
    /// How often to retry failed downloads (seconds)
    pub retry_interval_seconds: u64,
    /// Whether the processor is enabled
    pub enabled: bool,
}

impl Default for QueueProcessorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_downloads: 5,
            check_interval_seconds: 30,
            sync_interval_seconds: 60,
            retry_interval_seconds: 300, // 5 minutes
            enabled: true,
        }
    }
}

/// Background queue processor
pub struct QueueProcessor<Q: QueueRepository, D: DownloadClientService> {
    config: QueueProcessorConfig,
    queue_repo: Arc<Q>,
    download_client: Arc<D>,
    download_circuit_breaker: Arc<Mutex<CircuitBreaker>>,
    progress_tracker: Option<Arc<ProgressTracker>>,
    event_bus: Option<Arc<EventBus>>,
}

impl<Q: QueueRepository, D: DownloadClientService> QueueProcessor<Q, D>
where
    Q: 'static,
    D: 'static,
{
    /// Create a new queue processor
    pub fn new(config: QueueProcessorConfig, queue_repo: Arc<Q>, download_client: Arc<D>) -> Self {
        let download_circuit_breaker = Arc::new(Mutex::new(
            CircuitBreaker::new(
                "download_client",
                3, // Open after 3 failures
                Duration::from_secs(60), // Reset after 60 seconds
            )
        ));
        
        Self {
            config,
            queue_repo,
            download_client,
            download_circuit_breaker,
            progress_tracker: None,
            event_bus: None,
        }
    }
    
    /// Set progress tracker for this processor
    pub fn with_progress_tracker(mut self, tracker: Arc<ProgressTracker>) -> Self {
        self.progress_tracker = Some(tracker);
        self
    }
    
    /// Set event bus for this processor
    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.event_bus = Some(bus);
        self
    }
    
    /// Start the background processor
    pub async fn start(self) -> Result<()> {
        if !self.config.enabled {
            info!("Queue processor is disabled");
            return Ok(());
        }
        
        info!("Starting queue processor with config: {:?}", self.config);
        
        let processor = Arc::new(self);
        
        // Spawn queue processing task
        let process_task = {
            let processor = processor.clone();
            tokio::spawn(async move {
                processor.run_queue_processing().await;
            })
        };
        
        // Spawn sync task
        let sync_task = {
            let processor = processor.clone();
            tokio::spawn(async move {
                processor.run_sync_task().await;
            })
        };
        
        // Spawn retry task
        let retry_task = {
            let processor = processor.clone();
            tokio::spawn(async move {
                processor.run_retry_task().await;
            })
        };
        
        // Wait for all tasks to complete (they shouldn't unless there's an error)
        let _ = tokio::try_join!(process_task, sync_task, retry_task)?;
        
        Ok(())
    }
    
    /// Run the queue processing loop
    async fn run_queue_processing(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.check_interval_seconds));
        
        loop {
            interval.tick().await;
            
            // Use retry logic for queue processing
            let retry_config = RetryConfig::quick();
            match retry_with_backoff(
                retry_config,
                RetryPolicy::Transient,
                "process_queue_items",
                || self.process_queue_items(),
            ).await {
                Ok(processed_count) => {
                    if processed_count > 0 {
                        debug!("Processed {} queue items", processed_count);
                    }
                }
                Err(e) => {
                    error!("Error processing queue items after retries: {}", e);
                }
            }
        }
    }
    
    /// Run the sync task loop
    async fn run_sync_task(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.sync_interval_seconds));
        
        loop {
            interval.tick().await;
            
            // Use retry logic for sync operations
            let retry_config = RetryConfig::quick();
            match retry_with_backoff(
                retry_config,
                RetryPolicy::Transient,
                "sync_download_client",
                || self.sync_with_download_client(),
            ).await {
                Ok(updated_count) => {
                    if updated_count > 0 {
                        debug!("Updated {} items from download client", updated_count);
                    }
                }
                Err(e) => {
                    error!("Error syncing with download client after retries: {}", e);
                }
            }
        }
    }
    
    /// Run the retry task loop
    async fn run_retry_task(&self) {
        let mut interval = time::interval(Duration::from_secs(self.config.retry_interval_seconds));
        
        loop {
            interval.tick().await;
            
            match self.retry_failed_items().await {
                Ok(retried_count) => {
                    if retried_count > 0 {
                        info!("Retried {} failed downloads", retried_count);
                    }
                }
                Err(e) => {
                    error!("Error retrying failed downloads: {}", e);
                }
            }
        }
    }
    
    /// Process queued items
    async fn process_queue_items(&self) -> Result<usize> {
        use crate::models::QueueStatus;
        
        // Get current downloading count
        let downloading_items = self.queue_repo.get_queue_items(Some(QueueStatus::Downloading)).await?;
        let downloading_count = downloading_items.len();
        
        if downloading_count >= self.config.max_concurrent_downloads {
            return Ok(0); // No slots available
        }
        
        // Get queued items
        let queued_items = self.queue_repo.get_queue_items(Some(QueueStatus::Queued)).await?;
        if queued_items.is_empty() {
            return Ok(0);
        }
        
        let slots_available = self.config.max_concurrent_downloads - downloading_count;
        let mut processed_count = 0;
        
        // Sort by priority and creation time
        let mut sorted_items = queued_items;
        sorted_items.sort_by(|a, b| {
            use std::cmp::Ordering;
            use crate::models::QueuePriority;
            
            let priority_order = |p: &QueuePriority| match p {
                QueuePriority::VeryHigh => 0,
                QueuePriority::High => 1,
                QueuePriority::Normal => 2,
                QueuePriority::Low => 3,
            };
            
            match priority_order(&b.priority).cmp(&priority_order(&a.priority)) {
                Ordering::Equal => a.created_at.cmp(&b.created_at),
                other => other,
            }
        });
        
        for item in sorted_items.iter().take(slots_available) {
            match self.start_download(&item).await {
                Ok(()) => {
                    processed_count += 1;
                    info!("Started download for: {}", item.title);
                }
                Err(e) => {
                    warn!("Failed to start download for {}: {}", item.title, e);
                    
                    // Mark as failed
                    let mut failed_item = item.clone();
                    failed_item.set_error(format!("Failed to start download: {}", e));
                    let _ = self.queue_repo.update_queue_item(&failed_item).await;
                }
            }
        }
        
        Ok(processed_count)
    }
    
    /// Start a download for a queue item with retry and circuit breaker
    async fn start_download(&self, queue_item: &crate::models::QueueItem) -> Result<()> {
        use crate::models::QueueStatus;
        
        // Start progress tracking if available
        let progress_id = if let Some(tracker) = &self.progress_tracker {
            let id = tracker.start_operation(
                OperationType::Download,
                format!("Downloading: {}", queue_item.title)
            ).await;
            
            // Emit progress event
            if let Some(bus) = &self.event_bus {
                let _ = bus.publish(SystemEvent::ProgressUpdate {
                    operation_id: id,
                    operation_type: OperationType::Download,
                    percentage: 0.0,
                    message: "Initiating download".to_string(),
                    eta_seconds: None,
                }).await;
            }
            
            Some(id)
        } else {
            None
        };
        
        // Use circuit breaker for download client operations
        let mut circuit_breaker = self.download_circuit_breaker.lock().await;
        
        let client_id = circuit_breaker.execute(|| async {
            // Retry logic for adding download
            let retry_config = RetryConfig::slow();
            retry_with_backoff(
                retry_config,
                RetryPolicy::Transient,
                "add_download",
                || self.download_client.add_download(
                    &queue_item.download_url,
                    queue_item.category.clone(),
                    queue_item.download_path.clone(),
                ),
            ).await
        }).await?;
        
        // Update queue item with retry
        let mut updated_item = queue_item.clone();
        updated_item.set_download_client_id(client_id);
        updated_item.update_status(QueueStatus::Downloading);
        
        let retry_config = RetryConfig::quick();
        retry_with_backoff(
            retry_config,
            RetryPolicy::Transient,
            "update_queue_item",
            || self.queue_repo.update_queue_item(&updated_item),
        ).await?;
        
        // Update progress
        if let Some(id) = progress_id {
            if let Some(tracker) = &self.progress_tracker {
                tracker.update_progress(
                    id,
                    10.0,
                    "Download started, monitoring progress"
                ).await;
                
                // Emit progress event
                if let Some(bus) = &self.event_bus {
                    let _ = bus.publish(SystemEvent::ProgressUpdate {
                        operation_id: id,
                        operation_type: OperationType::Download,
                        percentage: 10.0,
                        message: "Download started".to_string(),
                        eta_seconds: None,
                    }).await;
                }
            }
        }
        
        Ok(())
    }
    
    /// Sync with download client
    async fn sync_with_download_client(&self) -> Result<usize> {
        use crate::models::QueueStatus;
        
        let active_items = self.queue_repo.get_queue_items(Some(QueueStatus::Downloading)).await?;
        let mut updated_count = 0;
        
        for mut item in active_items {
            if let Some(client_id) = &item.download_client_id {
                match self.download_client.get_download_status(client_id).await? {
                    Some(status) => {
                        let old_progress = item.progress;
                        let old_status = item.status.clone();
                        
                        self.update_queue_item_from_client_status(&mut item, &status)?;
                        
                        // Only update if something changed
                        if item.status != old_status || (item.progress - old_progress).abs() > 0.01 {
                            self.queue_repo.update_queue_item(&item).await?;
                            updated_count += 1;
                            
                            // Log completion
                            if item.is_completed() && old_status != item.status {
                                info!("Download completed: {}", item.title);
                            }
                        }
                    }
                    None => {
                        // Download not found in client, mark as failed
                        warn!("Download not found in client: {}", item.title);
                        item.set_error("Download not found in client".to_string());
                        self.queue_repo.update_queue_item(&item).await?;
                        updated_count += 1;
                    }
                }
            }
        }
        
        Ok(updated_count)
    }
    
    /// Update queue item from client status
    fn update_queue_item_from_client_status(
        &self,
        queue_item: &mut crate::models::QueueItem,
        client_status: &crate::services::ClientDownloadStatus,
    ) -> Result<()> {
        use crate::models::QueueStatus;
        
        // Map client status to queue status
        let new_status = match client_status.status.to_lowercase().as_str() {
            "downloading" | "stalled_dl" => QueueStatus::Downloading,
            "completed" | "seeding" | "uploading" => {
                if client_status.progress >= 1.0 {
                    QueueStatus::Completed
                } else {
                    QueueStatus::Downloading
                }
            }
            "paused_dl" | "paused_up" => QueueStatus::Paused,
            "error" => QueueStatus::Failed,
            "stalled" | "stalled_up" => QueueStatus::Stalled,
            _ => queue_item.status.clone(),
        };
        
        queue_item.update_status(new_status);
        queue_item.update_progress(
            client_status.progress,
            client_status.downloaded_bytes,
            client_status.download_speed,
            client_status.eta_seconds,
        );
        queue_item.update_seeding_info(
            client_status.upload_bytes,
            client_status.upload_speed,
            client_status.seeders,
            client_status.leechers,
        );
        
        Ok(())
    }
    
    /// Retry failed items
    async fn retry_failed_items(&self) -> Result<usize> {
        let retry_items = self.queue_repo.get_retry_items().await?;
        let mut retried_count = 0;
        
        for mut item in retry_items {
            if item.can_retry() {
                info!("Retrying failed download: {} (attempt {}/{})", 
                     item.title, item.retry_count + 1, item.max_retries);
                
                item.reset_for_retry();
                self.queue_repo.update_queue_item(&item).await?;
                retried_count += 1;
            }
        }
        
        Ok(retried_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{QueueItem, QueueStatus, QueuePriority};
    use crate::services::{QueueRepository, DownloadClientService, ClientDownloadStatus};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use uuid::Uuid;
    
    // Mock implementations for testing
    struct MockQueueRepository {
        items: Arc<RwLock<HashMap<Uuid, QueueItem>>>,
    }
    
    impl MockQueueRepository {
        fn new() -> Self {
            Self {
                items: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }
    
    #[async_trait]
    impl QueueRepository for MockQueueRepository {
        async fn add_queue_item(&self, item: &QueueItem) -> Result<()> {
            let mut items = self.items.write().unwrap();
            items.insert(item.id, item.clone());
            Ok(())
        }
        
        async fn get_queue_item(&self, id: Uuid) -> Result<Option<QueueItem>> {
            let items = self.items.read().unwrap();
            Ok(items.get(&id).cloned())
        }
        
        async fn get_queue_item_by_client_id(&self, client_id: &str) -> Result<Option<QueueItem>> {
            let items = self.items.read().unwrap();
            Ok(items.values()
                .find(|item| item.download_client_id.as_deref() == Some(client_id))
                .cloned())
        }
        
        async fn get_queue_items(&self, status_filter: Option<QueueStatus>) -> Result<Vec<QueueItem>> {
            let items = self.items.read().unwrap();
            let mut result: Vec<_> = items.values().cloned().collect();
            
            if let Some(status) = status_filter {
                result.retain(|item| item.status == status);
            }
            
            Ok(result)
        }
        
        async fn get_queue_items_for_movie(&self, movie_id: Uuid) -> Result<Vec<QueueItem>> {
            let items = self.items.read().unwrap();
            Ok(items.values()
                .filter(|item| item.movie_id == movie_id)
                .cloned()
                .collect())
        }
        
        async fn update_queue_item(&self, item: &QueueItem) -> Result<()> {
            let mut items = self.items.write().unwrap();
            items.insert(item.id, item.clone());
            Ok(())
        }
        
        async fn delete_queue_item(&self, id: Uuid) -> Result<()> {
            let mut items = self.items.write().unwrap();
            items.remove(&id);
            Ok(())
        }
        
        async fn get_queue_stats(&self) -> Result<crate::models::QueueStats> {
            Ok(crate::models::QueueStats::default())
        }
        
        async fn get_retry_items(&self) -> Result<Vec<QueueItem>> {
            let items = self.items.read().unwrap();
            Ok(items.values()
                .filter(|item| item.can_retry())
                .cloned()
                .collect())
        }
    }
    
    struct MockDownloadClient;
    
    #[async_trait]
    impl DownloadClientService for MockDownloadClient {
        async fn add_download(
            &self,
            _download_url: &str,
            _category: Option<String>,
            _save_path: Option<String>,
        ) -> Result<String> {
            Ok("mock_client_id_123".to_string())
        }
        
        async fn get_download_status(&self, _client_id: &str) -> Result<Option<ClientDownloadStatus>> {
            Ok(Some(ClientDownloadStatus {
                client_id: "mock_client_id_123".to_string(),
                name: "Test Movie".to_string(),
                status: "downloading".to_string(),
                progress: 0.5,
                download_speed: Some(1024 * 1024),
                upload_speed: Some(512 * 1024),
                downloaded_bytes: Some(500 * 1024 * 1024),
                upload_bytes: Some(100 * 1024 * 1024),
                eta_seconds: Some(600),
                seeders: Some(10),
                leechers: Some(5),
                save_path: Some("/downloads/movies".to_string()),
            }))
        }
        
        async fn remove_download(&self, _client_id: &str, _delete_files: bool) -> Result<()> {
            Ok(())
        }
        
        async fn pause_download(&self, _client_id: &str) -> Result<()> {
            Ok(())
        }
        
        async fn resume_download(&self, _client_id: &str) -> Result<()> {
            Ok(())
        }
        
        async fn get_all_downloads(&self) -> Result<Vec<ClientDownloadStatus>> {
            Ok(vec![])
        }
    }
    
    #[tokio::test]
    async fn test_queue_processor_creation() {
        let config = QueueProcessorConfig::default();
        let repo = Arc::new(MockQueueRepository::new());
        let client = Arc::new(MockDownloadClient);
        
        let _processor = QueueProcessor::new(config, repo, client);
    }
    
    #[tokio::test]
    async fn test_process_queue_items() {
        let config = QueueProcessorConfig {
            max_concurrent_downloads: 2,
            ..Default::default()
        };
        let repo = Arc::new(MockQueueRepository::new());
        let client = Arc::new(MockDownloadClient);
        
        // Add some queued items
        let movie_id = Uuid::new_v4();
        let release_id = Uuid::new_v4();
        let queue_item = QueueItem::new(
            movie_id,
            release_id,
            "Test Movie".to_string(),
            "magnet:test".to_string(),
        );
        
        repo.add_queue_item(&queue_item).await.unwrap();
        
        let processor = QueueProcessor::new(config, repo.clone(), client);
        
        let processed = processor.process_queue_items().await.unwrap();
        assert_eq!(processed, 1);
        
        // Verify item was updated to downloading
        let updated_item = repo.get_queue_item(queue_item.id).await.unwrap().unwrap();
        assert_eq!(updated_item.status, QueueStatus::Downloading);
        assert!(updated_item.download_client_id.is_some());
    }
}