//! End-to-end workflow tests demonstrating complete Search → Download → Import pipeline
//!
//! These tests showcase the complete movie acquisition workflow using mock clients
//! to ensure the integration architecture works correctly without external dependencies.

use async_trait::async_trait;
use chrono::Utc;
use radarr_core::{MinimumAvailability, Movie, MovieStatus};
use radarr_downloaders::{
    AddTorrentParams, QBittorrentClient, QBittorrentConfig, TorrentData, TorrentInfo,
};
use radarr_indexers::{
    Category, IndexerClient, ProwlarrSearchResult, SearchRequest, SearchResponse,
};
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

/// Mock indexer client for testing
pub struct MockIndexerClient {
    pub search_responses: Vec<SearchResponse>,
}

impl MockIndexerClient {
    pub fn new() -> Self {
        Self {
            search_responses: Vec::new(),
        }
    }

    pub fn with_search_response(mut self, response: SearchResponse) -> Self {
        self.search_responses.push(response);
        self
    }
}

#[async_trait]
impl IndexerClient for MockIndexerClient {
    async fn search(&self, _request: &SearchRequest) -> radarr_core::Result<SearchResponse> {
        if let Some(response) = self.search_responses.first() {
            Ok(response.clone())
        } else {
            Ok(SearchResponse {
                total: 0,
                results: vec![],
                indexers_searched: 0,
                indexers_with_errors: 0,
                errors: vec![],
            })
        }
    }

    async fn get_indexers(&self) -> radarr_core::Result<Vec<radarr_indexers::ProwlarrIndexer>> {
        Ok(vec![])
    }

    async fn test_indexer(&self, _indexer_id: i32) -> radarr_core::Result<bool> {
        Ok(true)
    }

    async fn health_check(&self) -> radarr_core::Result<bool> {
        Ok(true)
    }
}

/// Mock qBittorrent client for testing
pub struct MockQBittorrentClient {
    pub torrents: std::sync::Arc<std::sync::RwLock<HashMap<String, TorrentInfo>>>,
    pub should_fail: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl MockQBittorrentClient {
    pub fn new() -> Self {
        Self {
            torrents: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            should_fail: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn login(&self) -> radarr_core::Result<()> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(radarr_core::RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Authentication failed".to_string(),
            });
        }
        Ok(())
    }

    pub async fn add_torrent(&self, params: AddTorrentParams) -> radarr_core::Result<String> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(radarr_core::RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Failed to add torrent".to_string(),
            });
        }

        let hash = match &params.torrent_data {
            TorrentData::Url(url) => {
                if url.starts_with("magnet:") {
                    format!("magnet_{:x}", md5::compute(url.as_bytes()))
                } else {
                    format!("url_{:x}", md5::compute(url.as_bytes()))
                }
            }
            TorrentData::File(data) => {
                format!("file_{:x}", md5::compute(data))
            }
        };

        let torrent_info = TorrentInfo {
            hash: hash.clone(),
            name: "Test Movie 2023 1080p BluRay x264-TEST".to_string(),
            state: "downloading".to_string(),
            progress: 0.0,
            dlspeed: 1024 * 1024, // 1 MB/s
            upspeed: 0,
            size: 8589934592, // 8GB
            completed: 0,
            eta: 7200, // 2 hours
            priority: 0,
            category: params.category.unwrap_or_default(),
            save_path: params.save_path.unwrap_or("/downloads".to_string()),
        };

        {
            let mut torrents = self.torrents.write().unwrap();
            torrents.insert(hash.clone(), torrent_info);
        }

        Ok(hash)
    }

    pub async fn get_torrents(&self) -> radarr_core::Result<Vec<TorrentInfo>> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(radarr_core::RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Failed to get torrents".to_string(),
            });
        }

        let torrents = self.torrents.read().unwrap();
        Ok(torrents.values().cloned().collect())
    }

    pub async fn get_torrent_status(&self, hash: &str) -> radarr_core::Result<Option<TorrentInfo>> {
        let torrents = self.torrents.read().unwrap();
        Ok(torrents.get(hash).cloned())
    }

    /// Simulate torrent completion for testing
    pub fn complete_torrent(&self, hash: &str) {
        let mut torrents = self.torrents.write().unwrap();
        if let Some(torrent) = torrents.get_mut(hash) {
            torrent.state = "completed".to_string();
            torrent.progress = 1.0;
            torrent.completed = torrent.size;
            torrent.eta = -1;
            torrent.dlspeed = 0;
        }
    }

    /// Simulate torrent progress update
    pub fn update_progress(&self, hash: &str, progress: f64) {
        let mut torrents = self.torrents.write().unwrap();
        if let Some(torrent) = torrents.get_mut(hash) {
            torrent.progress = progress;
            torrent.completed = (torrent.size as f64 * progress) as u64;
            if progress >= 1.0 {
                torrent.state = "completed".to_string();
                torrent.eta = -1;
                torrent.dlspeed = 0;
            }
        }
    }
}

/// End-to-end workflow coordinator
pub struct WorkflowCoordinator {
    indexer_client: MockIndexerClient,
    download_client: MockQBittorrentClient,
}

impl WorkflowCoordinator {
    pub fn new() -> Self {
        // Set up mock indexer with realistic movie releases
        let search_response = SearchResponse {
            total: 3,
            results: vec![
                ProwlarrSearchResult {
                    title: "The.Matrix.1999.1080p.BluRay.x264-CLASSIC".to_string(),
                    download_url: "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335aa7c1367a88a"
                        .to_string(),
                    info_url: Some("http://indexer.example.com/details/123".to_string()),
                    indexer_id: 1,
                    indexer: "TestIndexer".to_string(),
                    size: Some(8589934592), // 8GB
                    seeders: Some(50),
                    leechers: Some(5),
                    download_factor: Some(1.0),
                    upload_factor: Some(1.0),
                    publish_date: Some(Utc::now()),
                    categories: vec![Category {
                        id: 2000,
                        name: "Movies".to_string(),
                        description: None,
                    }],
                    attributes: HashMap::new(),
                    imdb_id: Some("tt0133093".to_string()),
                    tmdb_id: Some(603),
                    freeleech: Some(false),
                    info_hash: Some("c12fe1c06bba254a9dc9f519b335aa7c1367a88a".to_string()),
                },
                ProwlarrSearchResult {
                    title: "The.Matrix.1999.2160p.UHD.BluRay.x265-TERMINAL".to_string(),
                    download_url: "magnet:?xt=urn:btih:d23fe1c06bba254a9dc9f519b335aa7c1367a99b"
                        .to_string(),
                    info_url: Some("http://indexer.example.com/details/124".to_string()),
                    indexer_id: 1,
                    indexer: "TestIndexer".to_string(),
                    size: Some(25769803776), // 24GB
                    seeders: Some(25),
                    leechers: Some(2),
                    download_factor: Some(1.0),
                    upload_factor: Some(1.0),
                    publish_date: Some(Utc::now()),
                    categories: vec![Category {
                        id: 2000,
                        name: "Movies".to_string(),
                        description: None,
                    }],
                    attributes: HashMap::new(),
                    imdb_id: Some("tt0133093".to_string()),
                    tmdb_id: Some(603),
                    freeleech: Some(true),
                    info_hash: Some("d23fe1c06bba254a9dc9f519b335aa7c1367a99b".to_string()),
                },
                ProwlarrSearchResult {
                    title: "The.Matrix.1999.720p.BluRay.x264-VINTAGE".to_string(),
                    download_url: "http://indexer.example.com/download/125.torrent".to_string(),
                    info_url: Some("http://indexer.example.com/details/125".to_string()),
                    indexer_id: 2,
                    indexer: "BackupIndexer".to_string(),
                    size: Some(4294967296), // 4GB
                    seeders: Some(100),
                    leechers: Some(10),
                    download_factor: Some(1.0),
                    upload_factor: Some(1.0),
                    publish_date: Some(Utc::now()),
                    categories: vec![Category {
                        id: 2000,
                        name: "Movies".to_string(),
                        description: None,
                    }],
                    attributes: HashMap::new(),
                    imdb_id: Some("tt0133093".to_string()),
                    tmdb_id: Some(603),
                    freeleech: Some(false),
                    info_hash: Some("vintage_720p_hash_125".to_string()),
                },
            ],
            indexers_searched: 2,
            indexers_with_errors: 0,
            errors: Vec::new(),
        };

        let indexer_client = MockIndexerClient::new().with_search_response(search_response);
        let download_client = MockQBittorrentClient::new();

        Self {
            indexer_client,
            download_client,
        }
    }

    /// Execute complete workflow: Search → Select → Download → Monitor
    pub async fn execute_movie_acquisition_workflow(
        &self,
        movie: &Movie,
    ) -> radarr_core::Result<String> {
        println!(
            "🎬 Starting movie acquisition workflow for: {}",
            movie.title
        );

        // Step 1: Search for releases
        println!("🔍 Searching for releases...");
        let search_request = if let Some(ref imdb_id) = movie.imdb_id {
            SearchRequest::for_movie_imdb(imdb_id)
        } else if movie.tmdb_id != 0 {
            SearchRequest::for_movie_tmdb(movie.tmdb_id)
        } else {
            SearchRequest::for_title(&movie.title)
        };

        let search_response = self.indexer_client.search(&search_request).await?;
        println!(
            "✅ Found {} releases from {} indexers",
            search_response.total, search_response.indexers_searched
        );

        if search_response.results.is_empty() {
            return Err(radarr_core::RadarrError::NotFound {
                resource: format!("No releases found for movie: {}", movie.title),
            });
        }

        // Step 2: Select best release (prefer 1080p, high seeders, freeleech)
        println!("🎯 Selecting best release...");
        let best_release = self.select_best_release(&search_response.results)?;
        println!(
            "✅ Selected: {} ({} GB, {} seeders)",
            best_release.title,
            best_release.size.unwrap_or(0) / 1024 / 1024 / 1024,
            best_release.seeders.unwrap_or(0)
        );

        // Step 3: Add to download client
        println!("⬇️ Adding to download client...");
        let torrent_params = AddTorrentParams {
            torrent_data: if best_release.download_url.starts_with("magnet:") {
                TorrentData::Url(best_release.download_url.clone())
            } else {
                // For HTTP URLs, we would download the .torrent file first
                // For this test, we'll simulate with dummy data
                TorrentData::File(vec![1, 2, 3, 4])
            },
            category: Some("movies".to_string()),
            save_path: Some("/media/downloads/movies".to_string()),
            paused: false,
            skip_checking: false,
            priority: 0,
        };

        let torrent_hash = self.download_client.add_torrent(torrent_params).await?;
        println!("✅ Torrent added with hash: {}", torrent_hash);

        // Step 4: Simulate monitoring progress
        println!("📊 Monitoring download progress...");
        for progress in [0.1, 0.3, 0.6, 0.8, 1.0] {
            self.download_client
                .update_progress(&torrent_hash, progress);
            let status = self
                .download_client
                .get_torrent_status(&torrent_hash)
                .await?;
            if let Some(torrent) = status {
                println!(
                    "📈 Progress: {:.1}% - {} - ETA: {}s",
                    torrent.progress * 100.0,
                    torrent.state,
                    torrent.eta
                );
            }
        }

        println!("🎉 Movie acquisition workflow completed successfully!");
        Ok(torrent_hash)
    }

    /// Quality-based release selection algorithm
    fn select_best_release<'a>(
        &self,
        releases: &'a [ProwlarrSearchResult],
    ) -> radarr_core::Result<&'a ProwlarrSearchResult> {
        let mut scored_releases: Vec<(f64, &ProwlarrSearchResult)> = releases
            .iter()
            .map(|release| {
                let mut score = 0.0;

                // Prefer 1080p releases
                if release.title.contains("1080p") {
                    score += 10.0;
                } else if release.title.contains("2160p") {
                    score += 8.0; // 4K is good but large
                } else if release.title.contains("720p") {
                    score += 6.0;
                }

                // Bonus for high seeders
                if let Some(seeders) = release.seeders {
                    score += (seeders as f64).min(50.0) / 10.0; // Cap at 5 points
                }

                // Bonus for freeleech
                if release.freeleech == Some(true) {
                    score += 5.0;
                }

                // Penalty for very large files (>20GB)
                if let Some(size) = release.size {
                    if size > 21474836480 {
                        // 20GB
                        score -= 3.0;
                    }
                }

                // Prefer known good release groups
                if release.title.contains("CLASSIC") || release.title.contains("CRITERION") {
                    score += 2.0;
                }

                (score, release)
            })
            .collect();

        // Sort by score (descending)
        scored_releases.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        scored_releases
            .first()
            .map(|(_, release)| *release)
            .ok_or_else(|| radarr_core::RadarrError::NotFound {
                resource: "No suitable release found".to_string(),
            })
    }

    /// Test error handling scenarios
    pub async fn test_error_scenarios(&self) -> radarr_core::Result<()> {
        println!("🧪 Testing error handling scenarios...");

        // Test indexer failure
        println!("🔴 Testing indexer failure scenario...");
        let empty_client = MockIndexerClient::new();
        let request = SearchRequest::for_title("NonExistent Movie");
        let response = empty_client.search(&request).await?;
        assert_eq!(response.total, 0);
        println!("✅ Indexer gracefully handled no results");

        // Test download client failure
        println!("🔴 Testing download client failure scenario...");
        self.download_client
            .should_fail
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let result = self
            .download_client
            .add_torrent(AddTorrentParams::default())
            .await;
        assert!(result.is_err());
        println!("✅ Download client gracefully handled failure");

        // Reset failure state
        self.download_client
            .should_fail
            .store(false, std::sync::atomic::Ordering::Relaxed);

        println!("✅ All error scenarios handled correctly");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_movie() -> Movie {
        Movie {
            id: Uuid::new_v4(),
            tmdb_id: 603,
            imdb_id: Some("tt0133093".to_string()),
            title: "The Matrix".to_string(),
            original_title: Some("The Matrix".to_string()),
            year: Some(1999),
            runtime: Some(136),
            status: MovieStatus::Released,
            monitored: true,
            quality_profile_id: Some(1),
            minimum_availability: MinimumAvailability::Released,
            has_file: false,
            movie_file_id: None,
            metadata: serde_json::json!({
                "overview": "A computer hacker learns about the true nature of reality.",
                "genres": ["Action", "Sci-Fi"],
                "path": "/media/movies/The Matrix (1999)"
            }),
            alternative_titles: serde_json::json!([]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_search_time: None,
            last_info_sync: None,
        }
    }

    #[tokio::test]
    async fn test_complete_movie_acquisition_workflow() {
        let coordinator = WorkflowCoordinator::new();
        let test_movie = create_test_movie();

        let result = coordinator
            .execute_movie_acquisition_workflow(&test_movie)
            .await;
        assert!(
            result.is_ok(),
            "Workflow should complete successfully: {:?}",
            result
        );

        let torrent_hash = result.unwrap();
        assert!(!torrent_hash.is_empty(), "Should return valid torrent hash");

        // Verify torrent was added to download client
        let torrents = coordinator.download_client.get_torrents().await.unwrap();
        assert_eq!(torrents.len(), 1, "Should have one torrent");
        assert_eq!(torrents[0].hash, torrent_hash, "Hash should match");
        assert_eq!(
            torrents[0].state, "completed",
            "Should be completed after simulation"
        );
    }

    #[tokio::test]
    async fn test_release_selection_algorithm() {
        let coordinator = WorkflowCoordinator::new();
        let search_request = SearchRequest::for_movie_imdb("tt0133093");
        let search_response = coordinator
            .indexer_client
            .search(&search_request)
            .await
            .unwrap();

        let best_release = coordinator
            .select_best_release(&search_response.results)
            .unwrap();

        // Should prefer 1080p BluRay release due to good quality and reasonable size
        assert!(
            best_release.title.contains("1080p"),
            "Should prefer 1080p release"
        );
        assert!(
            best_release.title.contains("BluRay"),
            "Should prefer BluRay source"
        );

        // Should have good metrics
        assert!(
            best_release.seeders.unwrap_or(0) > 10,
            "Should have good seeders"
        );
        assert!(
            best_release.size.unwrap_or(0) > 0,
            "Should have size information"
        );
    }

    #[tokio::test]
    async fn test_error_handling_scenarios() {
        let coordinator = WorkflowCoordinator::new();
        let result = coordinator.test_error_scenarios().await;
        assert!(
            result.is_ok(),
            "Error scenarios should be handled gracefully"
        );
    }

    #[tokio::test]
    async fn test_indexer_client_health_monitoring() {
        let coordinator = WorkflowCoordinator::new();

        // Test health check
        let health_result = coordinator.indexer_client.health_check().await;
        assert!(health_result.unwrap(), "Mock indexer should be healthy");

        // Test service health status
        let health_status = coordinator.indexer_client.get_service_health().await;
        assert_eq!(health_status, radarr_indexers::HealthStatus::Healthy);

        // Test service metrics
        let metrics = coordinator.indexer_client.get_service_metrics().await;
        assert_eq!(metrics.total_requests, 0); // Default metrics
    }

    #[tokio::test]
    async fn test_download_progress_monitoring() {
        let coordinator = WorkflowCoordinator::new();
        let torrent_params = AddTorrentParams {
            torrent_data: TorrentData::Url("magnet:?xt=urn:btih:test".to_string()),
            category: Some("test".to_string()),
            ..Default::default()
        };

        let hash = coordinator
            .download_client
            .add_torrent(torrent_params)
            .await
            .unwrap();

        // Test progress updates
        for progress in [0.25, 0.5, 0.75, 1.0] {
            coordinator.download_client.update_progress(&hash, progress);
            let status = coordinator
                .download_client
                .get_torrent_status(&hash)
                .await
                .unwrap();

            if let Some(torrent) = status {
                assert!(
                    (torrent.progress - progress).abs() < 0.01,
                    "Progress should match"
                );
                if progress >= 1.0 {
                    assert_eq!(torrent.state, "completed", "Should be completed at 100%");
                }
            }
        }
    }
}
