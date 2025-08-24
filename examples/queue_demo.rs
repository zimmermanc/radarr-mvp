//! Example demonstrating the download queue management system
//!
//! This example shows how to:
//! 1. Set up a queue service with mock implementations
//! 2. Add releases to the queue
//! 3. Process queue items
//! 4. Monitor download progress
//! 5. Handle queue operations via API endpoints

use std::sync::Arc;
use uuid::Uuid;

// For this example, we'll create simplified versions without the full database setup
use async_trait::async_trait;
use radarr_core::{
    ClientDownloadStatus, DownloadClientService, Movie, QueueItem, QueuePriority, QueueRepository,
    QueueService, QueueStats, QueueStatus, Release, ReleaseProtocol,
};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Simple in-memory implementation of QueueRepository for testing
#[derive(Default)]
struct MockQueueRepository {
    items: Arc<RwLock<HashMap<Uuid, QueueItem>>>,
}

#[async_trait]
impl QueueRepository for MockQueueRepository {
    async fn add_queue_item(&self, item: &QueueItem) -> radarr_core::Result<()> {
        let mut items = self.items.write().await;
        items.insert(item.id, item.clone());
        println!("✓ Added queue item: {}", item.title);
        Ok(())
    }

    async fn get_queue_item(&self, id: Uuid) -> radarr_core::Result<Option<QueueItem>> {
        let items = self.items.read().await;
        Ok(items.get(&id).cloned())
    }

    async fn get_queue_item_by_client_id(
        &self,
        client_id: &str,
    ) -> radarr_core::Result<Option<QueueItem>> {
        let items = self.items.read().await;
        Ok(items
            .values()
            .find(|item| item.download_client_id.as_deref() == Some(client_id))
            .cloned())
    }

    async fn get_queue_items(
        &self,
        status_filter: Option<QueueStatus>,
    ) -> radarr_core::Result<Vec<QueueItem>> {
        let items = self.items.read().await;
        let mut result: Vec<_> = items.values().cloned().collect();

        if let Some(status) = status_filter {
            result.retain(|item| item.status == status);
        }

        Ok(result)
    }

    async fn get_queue_items_for_movie(
        &self,
        movie_id: Uuid,
    ) -> radarr_core::Result<Vec<QueueItem>> {
        let items = self.items.read().await;
        Ok(items
            .values()
            .filter(|item| item.movie_id == movie_id)
            .cloned()
            .collect())
    }

    async fn update_queue_item(&self, item: &QueueItem) -> radarr_core::Result<()> {
        let mut items = self.items.write().await;
        items.insert(item.id, item.clone());
        println!(
            "✓ Updated queue item: {} (status: {})",
            item.title, item.status
        );
        Ok(())
    }

    async fn delete_queue_item(&self, id: Uuid) -> radarr_core::Result<()> {
        let mut items = self.items.write().await;
        if let Some(item) = items.remove(&id) {
            println!("✓ Removed queue item: {}", item.title);
        }
        Ok(())
    }

    async fn get_queue_stats(&self) -> radarr_core::Result<QueueStats> {
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

    async fn get_retry_items(&self) -> radarr_core::Result<Vec<QueueItem>> {
        let items = self.items.read().await;
        Ok(items
            .values()
            .filter(|item| item.can_retry())
            .cloned()
            .collect())
    }
}

/// Mock download client that simulates qBittorrent behavior
#[derive(Default)]
struct MockDownloadClient {
    downloads: Arc<RwLock<HashMap<String, ClientDownloadStatus>>>,
}

#[async_trait]
impl DownloadClientService for MockDownloadClient {
    async fn add_download(
        &self,
        download_url: &str,
        _category: Option<String>,
        _save_path: Option<String>,
    ) -> radarr_core::Result<String> {
        let client_id = format!(
            "mock_{}_{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            md5::compute(download_url)
        );

        let status = ClientDownloadStatus {
            client_id: client_id.clone(),
            name: "Mock Download".to_string(),
            status: "downloading".to_string(),
            progress: 0.0,
            download_speed: Some(1024 * 1024), // 1MB/s
            upload_speed: Some(0),
            downloaded_bytes: Some(0),
            upload_bytes: Some(0),
            eta_seconds: Some(3600), // 1 hour
            seeders: Some(10),
            leechers: Some(3),
            save_path: Some("/downloads".to_string()),
        };

        let mut downloads = self.downloads.write().await;
        downloads.insert(client_id.clone(), status);

        println!("✓ Added to download client: {}", client_id);
        Ok(client_id)
    }

    async fn get_download_status(
        &self,
        client_id: &str,
    ) -> radarr_core::Result<Option<ClientDownloadStatus>> {
        let downloads = self.downloads.read().await;
        Ok(downloads.get(client_id).cloned())
    }

    async fn remove_download(
        &self,
        client_id: &str,
        _delete_files: bool,
    ) -> radarr_core::Result<()> {
        let mut downloads = self.downloads.write().await;
        if downloads.remove(client_id).is_some() {
            println!("✓ Removed from download client: {}", client_id);
        }
        Ok(())
    }

    async fn pause_download(&self, client_id: &str) -> radarr_core::Result<()> {
        let mut downloads = self.downloads.write().await;
        if let Some(status) = downloads.get_mut(client_id) {
            status.status = "paused".to_string();
            println!("✓ Paused download: {}", client_id);
        }
        Ok(())
    }

    async fn resume_download(&self, client_id: &str) -> radarr_core::Result<()> {
        let mut downloads = self.downloads.write().await;
        if let Some(status) = downloads.get_mut(client_id) {
            status.status = "downloading".to_string();
            println!("✓ Resumed download: {}", client_id);
        }
        Ok(())
    }

    async fn get_all_downloads(&self) -> radarr_core::Result<Vec<ClientDownloadStatus>> {
        let downloads = self.downloads.read().await;
        Ok(downloads.values().cloned().collect())
    }
}

#[tokio::main]
async fn main() -> radarr_core::Result<()> {
    println!("🎬 Radarr Queue Management Demo");
    println!("================================\n");

    // Set up mock services
    let queue_repo = MockQueueRepository::default();
    let download_client = MockDownloadClient::default();
    let queue_service = QueueService::new(queue_repo, download_client);

    // Create a test movie
    let movie = Movie::new(12345, "The Matrix Resurrections".to_string());
    println!("📽️  Created movie: {} (ID: {})", movie.title, movie.id);

    // Create some test releases
    let releases = vec![
        {
            let mut release = Release::new(
                1,
                "The.Matrix.Resurrections.2021.2160p.BluRay.x265.HDR-IMAX".to_string(),
                "magnet:?xt=urn:btih:high_quality_release".to_string(),
                "hq-release-guid".to_string(),
                ReleaseProtocol::Torrent,
            );
            release.size_bytes = Some(25 * 1024 * 1024 * 1024); // 25GB
            release.seeders = Some(150);
            release.leechers = Some(25);
            release
        },
        {
            let mut release = Release::new(
                1,
                "The.Matrix.Resurrections.2021.1080p.WEBRip.x264-MOVIE".to_string(),
                "magnet:?xt=urn:btih:medium_quality_release".to_string(),
                "med-release-guid".to_string(),
                ReleaseProtocol::Torrent,
            );
            release.size_bytes = Some(8 * 1024 * 1024 * 1024); // 8GB
            release.seeders = Some(75);
            release.leechers = Some(15);
            release
        },
    ];

    println!("\n📦 Available releases:");
    for (i, release) in releases.iter().enumerate() {
        println!(
            "  {}. {} ({})",
            i + 1,
            release.title,
            release
                .human_readable_size()
                .unwrap_or("Unknown size".to_string())
        );
    }

    // Grab the high-quality release
    println!("\n⬇️  Adding high-quality release to queue...");
    let queue_item = queue_service
        .grab_release(
            &movie,
            &releases[0],
            Some(QueuePriority::High),
            Some("movies".to_string()),
        )
        .await?;

    // Show queue stats
    let stats = queue_service.get_queue_statistics().await?;
    println!("\n📊 Queue Statistics:");
    println!("  Total items: {}", stats.total_count);
    println!("  Queued: {}", stats.queued_count);
    println!("  Downloading: {}", stats.downloading_count);
    println!("  Completed: {}", stats.completed_count);

    // Process the queue (simulate starting downloads)
    println!("\n🔄 Processing queue...");
    let processed_ids = queue_service.process_queue(Some(5)).await?;
    println!("  Started {} downloads", processed_ids.len());

    // Simulate some download progress
    println!("\n⏱️  Simulating download progress...");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Sync with download client (simulate progress updates)
    let updated_ids = queue_service.sync_with_download_client().await?;
    println!("  Updated {} items from download client", updated_ids.len());

    // Show updated stats
    let updated_stats = queue_service.get_queue_statistics().await?;
    println!("\n📊 Updated Queue Statistics:");
    println!("  Total items: {}", updated_stats.total_count);
    println!("  Queued: {}", updated_stats.queued_count);
    println!("  Downloading: {}", updated_stats.downloading_count);
    println!("  Completed: {}", updated_stats.completed_count);

    // Get all queue items to show current state
    let queue_items = queue_service
        .get_queue_items(None, None, None, None)
        .await?;
    println!("\n📋 Current Queue Items:");
    for item in queue_items {
        println!(
            "  • {} - {} ({:.1}% complete)",
            item.title,
            item.status,
            item.progress * 100.0
        );
        if let Some(speed) = item.human_readable_download_speed() {
            println!("    Speed: {}", speed);
        }
        if let Some(eta) = item.human_readable_eta() {
            println!("    ETA: {}", eta);
        }
    }

    // Demonstrate queue operations
    println!("\n🎮 Queue Operations Demo:");

    // Pause the download
    println!("  Pausing download...");
    if let Err(e) = queue_service.pause_queue_item(queue_item.id).await {
        println!("    ⚠️  Could not pause: {}", e);
    }

    // Resume the download
    println!("  Resuming download...");
    if let Err(e) = queue_service.resume_queue_item(queue_item.id).await {
        println!("    ⚠️  Could not resume: {}", e);
    }

    // Show final stats
    let final_stats = queue_service.get_queue_statistics().await?;
    println!("\n✅ Demo Complete!");
    println!("📊 Final Queue Statistics:");
    println!("  Total items: {}", final_stats.total_count);
    println!(
        "  Active downloads: {}",
        final_stats.downloading_count + final_stats.queued_count
    );

    println!("\n🎉 Queue management system is working!");
    println!("Next steps:");
    println!("  1. Set up PostgreSQL database and run migrations");
    println!("  2. Configure qBittorrent connection");
    println!("  3. Wire the API endpoints in your web server");
    println!("  4. Set up background queue processor");

    Ok(())
}
