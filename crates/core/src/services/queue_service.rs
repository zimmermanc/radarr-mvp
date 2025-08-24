//! Queue service for managing download queue operations
//!
//! This service handles the business logic for queuing downloads,
//! monitoring progress, and coordinating with download clients.

use crate::models::{Movie, QueueItem, QueuePriority, QueueStats, QueueStatus, Release};
use crate::{RadarrError, Result};
use async_trait::async_trait;
use uuid::Uuid;

/// Repository trait for queue data persistence
#[async_trait]
pub trait QueueRepository: Send + Sync {
    /// Add a new queue item
    async fn add_queue_item(&self, item: &QueueItem) -> Result<()>;

    /// Get queue item by ID
    async fn get_queue_item(&self, id: Uuid) -> Result<Option<QueueItem>>;

    /// Get queue item by download client ID
    async fn get_queue_item_by_client_id(&self, client_id: &str) -> Result<Option<QueueItem>>;

    /// Get all queue items with optional status filter
    async fn get_queue_items(&self, status_filter: Option<QueueStatus>) -> Result<Vec<QueueItem>>;

    /// Get queue items for a specific movie
    async fn get_queue_items_for_movie(&self, movie_id: Uuid) -> Result<Vec<QueueItem>>;

    /// Update queue item
    async fn update_queue_item(&self, item: &QueueItem) -> Result<()>;

    /// Delete queue item
    async fn delete_queue_item(&self, id: Uuid) -> Result<()>;

    /// Get queue statistics
    async fn get_queue_stats(&self) -> Result<QueueStats>;

    /// Get items ready for retry
    async fn get_retry_items(&self) -> Result<Vec<QueueItem>>;
}

/// Service for download client operations
#[async_trait]
pub trait DownloadClientService: Send + Sync {
    /// Add a download to the client
    async fn add_download(
        &self,
        download_url: &str,
        category: Option<String>,
        save_path: Option<String>,
    ) -> Result<String>; // Returns client-specific ID

    /// Get download status from client
    async fn get_download_status(&self, client_id: &str) -> Result<Option<ClientDownloadStatus>>;

    /// Remove download from client
    async fn remove_download(&self, client_id: &str, delete_files: bool) -> Result<()>;

    /// Pause download
    async fn pause_download(&self, client_id: &str) -> Result<()>;

    /// Resume download
    async fn resume_download(&self, client_id: &str) -> Result<()>;

    /// Get all downloads from client
    async fn get_all_downloads(&self) -> Result<Vec<ClientDownloadStatus>>;
}

/// Download status information from client
#[derive(Debug, Clone)]
pub struct ClientDownloadStatus {
    pub client_id: String,
    pub name: String,
    pub status: String,
    pub progress: f64,
    pub download_speed: Option<u64>,
    pub upload_speed: Option<u64>,
    pub downloaded_bytes: Option<i64>,
    pub upload_bytes: Option<i64>,
    pub eta_seconds: Option<i64>,
    pub seeders: Option<i32>,
    pub leechers: Option<i32>,
    pub save_path: Option<String>,
}

/// Queue service for managing downloads
pub struct QueueService<Q: QueueRepository, D: DownloadClientService> {
    queue_repo: Q,
    download_client: D,
}

impl<Q: QueueRepository, D: DownloadClientService> QueueService<Q, D> {
    /// Create a new queue service
    pub fn new(queue_repo: Q, download_client: D) -> Self {
        Self {
            queue_repo,
            download_client,
        }
    }

    /// Add a release to the download queue
    pub async fn grab_release(
        &self,
        movie: &Movie,
        release: &Release,
        priority: Option<QueuePriority>,
        category: Option<String>,
    ) -> Result<QueueItem> {
        // Create queue item
        let mut queue_item = QueueItem::new(
            movie.id,
            release.id,
            release.title.clone(),
            release.download_url.clone(),
        );

        // Set optional parameters
        if let Some(p) = priority {
            queue_item.priority = p;
        }

        if let Some(size) = release.size_bytes {
            queue_item.size_bytes = Some(size);
        }

        if release.download_url.starts_with("magnet:") {
            queue_item.magnet_url = Some(release.download_url.clone());
        }

        // Set category
        if let Some(cat) = category {
            queue_item.category = Some(cat);
        }

        // Save to database
        self.queue_repo.add_queue_item(&queue_item).await?;

        // Start download immediately if high priority
        if queue_item.priority == QueuePriority::High
            || queue_item.priority == QueuePriority::VeryHigh
        {
            self.process_queue_item(queue_item.id).await?;
        }

        Ok(queue_item)
    }

    /// Process a specific queue item (send to download client)
    pub async fn process_queue_item(&self, queue_id: Uuid) -> Result<()> {
        let mut queue_item = self
            .queue_repo
            .get_queue_item(queue_id)
            .await?
            .ok_or_else(|| RadarrError::ValidationError {
                field: "queue_id".to_string(),
                message: format!("Queue item {} not found", queue_id),
            })?;

        if queue_item.status != QueueStatus::Queued {
            return Err(RadarrError::ValidationError {
                field: "status".to_string(),
                message: format!("Queue item {} is not in queued status", queue_id),
            });
        }

        // Add to download client
        let client_id = self
            .download_client
            .add_download(
                &queue_item.download_url,
                queue_item.category.clone(),
                queue_item.download_path.clone(),
            )
            .await?;

        // Update queue item
        queue_item.set_download_client_id(client_id);
        queue_item.update_status(QueueStatus::Downloading);

        self.queue_repo.update_queue_item(&queue_item).await?;

        Ok(())
    }

    /// Process queued items (called by scheduler)
    pub async fn process_queue(&self, max_concurrent: Option<usize>) -> Result<Vec<Uuid>> {
        let queued_items = self
            .queue_repo
            .get_queue_items(Some(QueueStatus::Queued))
            .await?;
        let downloading_items = self
            .queue_repo
            .get_queue_items(Some(QueueStatus::Downloading))
            .await?;

        let concurrent_limit = max_concurrent.unwrap_or(5);
        let slots_available = concurrent_limit.saturating_sub(downloading_items.len());

        if slots_available == 0 {
            return Ok(vec![]);
        }

        // Sort by priority (highest first) then by created time (oldest first)
        let mut sorted_items = queued_items;
        sorted_items.sort_by(|a, b| {
            use std::cmp::Ordering;
            use QueuePriority::*;

            let priority_order = |p: &QueuePriority| match p {
                VeryHigh => 0,
                High => 1,
                Normal => 2,
                Low => 3,
            };

            match priority_order(&b.priority).cmp(&priority_order(&a.priority)) {
                Ordering::Equal => a.created_at.cmp(&b.created_at),
                other => other,
            }
        });

        let mut processed_ids = Vec::new();

        for item in sorted_items.iter().take(slots_available) {
            match self.process_queue_item(item.id).await {
                Ok(()) => processed_ids.push(item.id),
                Err(e) => {
                    tracing::error!("Failed to process queue item {}: {}", item.id, e);

                    // Mark as failed
                    let mut failed_item = item.clone();
                    failed_item.set_error(format!("Failed to start download: {}", e));
                    let _ = self.queue_repo.update_queue_item(&failed_item).await;
                }
            }
        }

        Ok(processed_ids)
    }

    /// Update queue items with status from download client
    pub async fn sync_with_download_client(&self) -> Result<Vec<Uuid>> {
        let active_items = self
            .queue_repo
            .get_queue_items(Some(QueueStatus::Downloading))
            .await?;
        let mut updated_items = Vec::new();

        for mut item in active_items {
            if let Some(client_id) = &item.download_client_id {
                match self.download_client.get_download_status(client_id).await? {
                    Some(status) => {
                        let old_status = item.status;
                        self.update_queue_item_from_client_status(&mut item, &status)?;

                        if item.status != old_status || item.progress != status.progress {
                            self.queue_repo.update_queue_item(&item).await?;
                            updated_items.push(item.id);
                        }
                    }
                    None => {
                        // Download not found in client, mark as failed
                        item.set_error("Download not found in client".to_string());
                        self.queue_repo.update_queue_item(&item).await?;
                        updated_items.push(item.id);
                    }
                }
            }
        }

        Ok(updated_items)
    }

    /// Update queue item from client status
    fn update_queue_item_from_client_status(
        &self,
        queue_item: &mut QueueItem,
        client_status: &ClientDownloadStatus,
    ) -> Result<()> {
        // Map client status to queue status
        let new_status = match client_status.status.to_lowercase().as_str() {
            "downloading" | "stalledDL" => QueueStatus::Downloading,
            "completed" | "seeding" | "uploading" => {
                if client_status.progress >= 1.0 {
                    QueueStatus::Completed
                } else {
                    QueueStatus::Downloading
                }
            }
            "pausedDL" | "pausedUP" => QueueStatus::Paused,
            "error" => QueueStatus::Failed,
            "stalled" | "stalledUP" => QueueStatus::Stalled,
            _ => queue_item.status,
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

    /// Remove item from queue
    pub async fn remove_queue_item(&self, queue_id: Uuid, delete_files: bool) -> Result<()> {
        let queue_item = self
            .queue_repo
            .get_queue_item(queue_id)
            .await?
            .ok_or_else(|| RadarrError::ValidationError {
                field: "queue_id".to_string(),
                message: format!("Queue item {} not found", queue_id),
            })?;

        // Remove from download client if present
        if let Some(client_id) = &queue_item.download_client_id {
            let _ = self
                .download_client
                .remove_download(client_id, delete_files)
                .await;
        }

        // Remove from database
        self.queue_repo.delete_queue_item(queue_id).await?;

        Ok(())
    }

    /// Pause a queue item
    pub async fn pause_queue_item(&self, queue_id: Uuid) -> Result<()> {
        let mut queue_item = self
            .queue_repo
            .get_queue_item(queue_id)
            .await?
            .ok_or_else(|| RadarrError::ValidationError {
                field: "queue_id".to_string(),
                message: format!("Queue item {} not found", queue_id),
            })?;

        if let Some(client_id) = &queue_item.download_client_id {
            self.download_client.pause_download(client_id).await?;
        }

        queue_item.update_status(QueueStatus::Paused);
        self.queue_repo.update_queue_item(&queue_item).await?;

        Ok(())
    }

    /// Resume a queue item
    pub async fn resume_queue_item(&self, queue_id: Uuid) -> Result<()> {
        let mut queue_item = self
            .queue_repo
            .get_queue_item(queue_id)
            .await?
            .ok_or_else(|| RadarrError::ValidationError {
                field: "queue_id".to_string(),
                message: format!("Queue item {} not found", queue_id),
            })?;

        if let Some(client_id) = &queue_item.download_client_id {
            self.download_client.resume_download(client_id).await?;
        }

        queue_item.update_status(QueueStatus::Downloading);
        self.queue_repo.update_queue_item(&queue_item).await?;

        Ok(())
    }

    /// Retry failed downloads
    pub async fn retry_failed_downloads(&self) -> Result<Vec<Uuid>> {
        let retry_items = self.queue_repo.get_retry_items().await?;
        let mut retried_ids = Vec::new();

        for mut item in retry_items {
            if item.can_retry() {
                item.reset_for_retry();
                self.queue_repo.update_queue_item(&item).await?;
                retried_ids.push(item.id);
            }
        }

        Ok(retried_ids)
    }

    /// Get queue items with filtering and pagination
    pub async fn get_queue_items(
        &self,
        status_filter: Option<QueueStatus>,
        movie_id_filter: Option<Uuid>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<QueueItem>> {
        let mut items = if let Some(movie_id) = movie_id_filter {
            let all_items = self.queue_repo.get_queue_items_for_movie(movie_id).await?;
            // Apply status filtering if specified
            if let Some(status) = status_filter {
                all_items
                    .into_iter()
                    .filter(|item| item.status == status)
                    .collect()
            } else {
                all_items
            }
        } else {
            self.queue_repo.get_queue_items(status_filter).await?
        };

        // Sort by priority then by creation time
        items.sort_by(|a, b| {
            use std::cmp::Ordering;
            use QueuePriority::*;

            let priority_order = |p: &QueuePriority| match p {
                VeryHigh => 0,
                High => 1,
                Normal => 2,
                Low => 3,
            };

            match priority_order(&b.priority).cmp(&priority_order(&a.priority)) {
                Ordering::Equal => b.created_at.cmp(&a.created_at),
                other => other,
            }
        });

        // Apply pagination
        let start = offset.unwrap_or(0);
        let end = if let Some(limit) = limit {
            start.saturating_add(limit).min(items.len())
        } else {
            items.len()
        };

        Ok(items.into_iter().skip(start).take(end - start).collect())
    }

    /// Get queue statistics
    pub async fn get_queue_statistics(&self) -> Result<QueueStats> {
        self.queue_repo.get_queue_stats().await
    }

    /// Clear completed downloads older than specified days
    pub async fn cleanup_completed_items(&self, days_old: i64) -> Result<usize> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(days_old);
        let completed_items = self
            .queue_repo
            .get_queue_items(Some(QueueStatus::Completed))
            .await?;

        let mut cleaned_count = 0;
        for item in completed_items {
            if let Some(completed_at) = item.completed_at {
                if completed_at < cutoff_date {
                    self.queue_repo.delete_queue_item(item.id).await?;
                    cleaned_count += 1;
                }
            }
        }

        Ok(cleaned_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Movie, Release, ReleaseProtocol};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

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
            let mut items = self.items.write().await;
            items.insert(item.id, item.clone());
            Ok(())
        }

        async fn get_queue_item(&self, id: Uuid) -> Result<Option<QueueItem>> {
            let items = self.items.read().await;
            Ok(items.get(&id).cloned())
        }

        async fn get_queue_item_by_client_id(&self, client_id: &str) -> Result<Option<QueueItem>> {
            let items = self.items.read().await;
            Ok(items
                .values()
                .find(|item| item.download_client_id.as_deref() == Some(client_id))
                .cloned())
        }

        async fn get_queue_items(
            &self,
            status_filter: Option<QueueStatus>,
        ) -> Result<Vec<QueueItem>> {
            let items = self.items.read().await;
            let mut result: Vec<_> = items.values().cloned().collect();

            if let Some(status) = status_filter {
                result.retain(|item| item.status == status);
            }

            Ok(result)
        }

        async fn get_queue_items_for_movie(&self, movie_id: Uuid) -> Result<Vec<QueueItem>> {
            let items = self.items.read().await;
            Ok(items
                .values()
                .filter(|item| item.movie_id == movie_id)
                .cloned()
                .collect())
        }

        async fn update_queue_item(&self, item: &QueueItem) -> Result<()> {
            let mut items = self.items.write().await;
            items.insert(item.id, item.clone());
            Ok(())
        }

        async fn delete_queue_item(&self, id: Uuid) -> Result<()> {
            let mut items = self.items.write().await;
            items.remove(&id);
            Ok(())
        }

        async fn get_queue_stats(&self) -> Result<QueueStats> {
            let items = self.items.read().await;
            let mut stats = QueueStats::default();

            stats.total_count = items.len() as i64;
            for item in items.values() {
                match item.status {
                    QueueStatus::Queued => stats.queued_count += 1,
                    QueueStatus::Downloading => stats.downloading_count += 1,
                    QueueStatus::Completed => stats.completed_count += 1,
                    QueueStatus::Failed => stats.failed_count += 1,
                    QueueStatus::Paused => stats.paused_count += 1,
                    _ => {}
                }
            }

            Ok(stats)
        }

        async fn get_retry_items(&self) -> Result<Vec<QueueItem>> {
            let items = self.items.read().await;
            Ok(items
                .values()
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

        async fn get_download_status(
            &self,
            _client_id: &str,
        ) -> Result<Option<ClientDownloadStatus>> {
            Ok(Some(ClientDownloadStatus {
                client_id: "mock_client_id_123".to_string(),
                name: "Test Movie".to_string(),
                status: "downloading".to_string(),
                progress: 0.5,
                download_speed: Some(1024 * 1024),         // 1MB/s
                upload_speed: Some(512 * 1024),            // 512KB/s
                downloaded_bytes: Some(500 * 1024 * 1024), // 500MB
                upload_bytes: Some(100 * 1024 * 1024),     // 100MB
                eta_seconds: Some(600),                    // 10 minutes
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
    async fn test_grab_release() {
        let repo = MockQueueRepository::new();
        let client = MockDownloadClient;
        let service = QueueService::new(repo, client);

        let movie = Movie::new(123, "Test Movie".to_string());
        let release = Release::new(
            1,
            "Test Movie 2023 1080p".to_string(),
            "magnet:?xt=urn:btih:test".to_string(),
            "test-guid".to_string(),
            ReleaseProtocol::Torrent,
        );

        let result = service
            .grab_release(
                &movie,
                &release,
                Some(QueuePriority::High),
                Some("movies".to_string()),
            )
            .await;

        assert!(result.is_ok());
        let queue_item = result.unwrap();
        assert_eq!(queue_item.movie_id, movie.id);
        assert_eq!(queue_item.title, release.title);
        assert_eq!(queue_item.priority, QueuePriority::High);
        assert_eq!(queue_item.category, Some("movies".to_string()));
    }

    #[tokio::test]
    async fn test_process_queue_item() {
        let repo = MockQueueRepository::new();
        let client = MockDownloadClient;
        let service = QueueService::new(repo, client);

        // Create a test queue item
        let movie = Movie::new(123, "Test Movie".to_string());
        let release = Release::new(
            1,
            "Test Movie 2023 1080p".to_string(),
            "magnet:?xt=urn:btih:test".to_string(),
            "test-guid".to_string(),
            ReleaseProtocol::Torrent,
        );

        let queue_item = service
            .grab_release(&movie, &release, None, None)
            .await
            .unwrap();

        // Process the queue item
        let result = service.process_queue_item(queue_item.id).await;
        assert!(result.is_ok());

        // Verify it was updated
        let updated_item = service
            .queue_repo
            .get_queue_item(queue_item.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated_item.status, QueueStatus::Downloading);
        assert!(updated_item.download_client_id.is_some());
    }
}
