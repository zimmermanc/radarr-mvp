//! Integration tests for Enhanced Sync Handler
//!
//! Tests the complete sync workflow including performance tracking,
//! conflict resolution, and repository integration.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Movie;
    use chrono::{NaiveDate, Utc};
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{Mutex, RwLock};
    use uuid::Uuid;

    /// Mock movie repository for testing
    #[derive(Clone)]
    struct MockMovieRepository {
        movies: Arc<RwLock<HashMap<i32, Movie>>>, // tmdb_id -> Movie
    }

    impl MockMovieRepository {
        fn new() -> Self {
            Self {
                movies: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        async fn add_movie(&self, movie: Movie) {
            if let Some(tmdb_id) = movie.tmdb_id {
                let mut movies = self.movies.write().await;
                movies.insert(tmdb_id, movie);
            }
        }
    }

    #[async_trait::async_trait]
    impl MovieRepository for MockMovieRepository {
        async fn find_by_tmdb_id(&self, tmdb_id: i32) -> Result<Option<Movie>, SyncError> {
            let movies = self.movies.read().await;
            Ok(movies.get(&tmdb_id).cloned())
        }

        async fn find_by_imdb_id(&self, imdb_id: &str) -> Result<Option<Movie>, SyncError> {
            let movies = self.movies.read().await;
            Ok(movies
                .values()
                .find(|m| m.imdb_id.as_ref() == Some(&imdb_id.to_string()))
                .cloned())
        }

        async fn create(&self, movie: &Movie) -> Result<Movie, SyncError> {
            if let Some(tmdb_id) = movie.tmdb_id {
                let mut movies = self.movies.write().await;
                movies.insert(tmdb_id, movie.clone());
            }
            Ok(movie.clone())
        }

        async fn update(&self, movie: &Movie) -> Result<Movie, SyncError> {
            if let Some(tmdb_id) = movie.tmdb_id {
                let mut movies = self.movies.write().await;
                movies.insert(tmdb_id, movie.clone());
            }
            Ok(movie.clone())
        }
    }

    /// Mock list sync repository for testing
    #[derive(Clone)]
    struct MockListSyncRepository {
        sync_history: Arc<RwLock<Vec<SyncHistoryRecord>>>,
        performance_metrics: Arc<RwLock<Vec<(PerformanceMetrics, Uuid)>>>,
    }

    #[derive(Debug, Clone)]
    struct SyncHistoryRecord {
        id: Uuid,
        list_id: Uuid,
        status: String,
        started_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
        items_found: i32,
        items_added: i32,
        items_updated: i32,
        items_removed: i32,
        items_excluded: i32,
        error_message: Option<String>,
    }

    impl MockListSyncRepository {
        fn new() -> Self {
            Self {
                sync_history: Arc::new(RwLock::new(Vec::new())),
                performance_metrics: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn get_sync_records(&self) -> Vec<SyncHistoryRecord> {
            self.sync_history.read().await.clone()
        }

        async fn get_performance_records(&self) -> Vec<(PerformanceMetrics, Uuid)> {
            self.performance_metrics.read().await.clone()
        }
    }

    #[async_trait::async_trait]
    impl ListSyncRepository for MockListSyncRepository {
        async fn start_sync(&self, list_id: Uuid, _metadata: Option<serde_json::Value>) -> Result<Uuid, SyncError> {
            let sync_id = Uuid::new_v4();
            let mut history = self.sync_history.write().await;
            history.push(SyncHistoryRecord {
                id: sync_id,
                list_id,
                status: "started".to_string(),
                started_at: Utc::now(),
                completed_at: None,
                items_found: 0,
                items_added: 0,
                items_updated: 0,
                items_removed: 0,
                items_excluded: 0,
                error_message: None,
            });
            Ok(sync_id)
        }

        async fn complete_sync(
            &self,
            sync_id: Uuid,
            status: &str,
            items_found: i32,
            items_added: i32,
            items_updated: i32,
            items_removed: i32,
            items_excluded: i32,
            error_message: Option<String>,
            _error_details: Option<serde_json::Value>,
        ) -> Result<(), SyncError> {
            let mut history = self.sync_history.write().await;
            if let Some(record) = history.iter_mut().find(|r| r.id == sync_id) {
                record.status = status.to_string();
                record.completed_at = Some(Utc::now());
                record.items_found = items_found;
                record.items_added = items_added;
                record.items_updated = items_updated;
                record.items_removed = items_removed;
                record.items_excluded = items_excluded;
                record.error_message = error_message;
            }
            Ok(())
        }

        async fn record_performance_metrics(
            &self, 
            metrics: &PerformanceMetrics, 
            list_id: Uuid
        ) -> Result<(), SyncError> {
            let mut perf_metrics = self.performance_metrics.write().await;
            perf_metrics.push((metrics.clone(), list_id));
            Ok(())
        }
    }

    /// Mock sync monitoring for testing
    #[derive(Clone)]
    struct MockSyncMonitoring {
        recorded_operations: Arc<Mutex<Vec<SyncOperationRecord>>>,
        recorded_api_requests: Arc<Mutex<Vec<ApiRequestRecord>>>,
        recorded_cache_accesses: Arc<Mutex<Vec<CacheAccessRecord>>>,
    }

    #[derive(Debug, Clone)]
    struct SyncOperationRecord {
        source: String,
        success: bool,
        duration: Duration,
        items_added: u64,
        items_updated: u64,
        items_total: u64,
    }

    #[derive(Debug, Clone)]
    struct ApiRequestRecord {
        service: String,
        duration: Duration,
        rate_limited: bool,
    }

    #[derive(Debug, Clone)]
    struct CacheAccessRecord {
        cache_type: String,
        hit: bool,
    }

    impl MockSyncMonitoring {
        fn new() -> Self {
            Self {
                recorded_operations: Arc::new(Mutex::new(Vec::new())),
                recorded_api_requests: Arc::new(Mutex::new(Vec::new())),
                recorded_cache_accesses: Arc::new(Mutex::new(Vec::new())),
            }
        }

        async fn get_recorded_operations(&self) -> Vec<SyncOperationRecord> {
            self.recorded_operations.lock().await.clone()
        }
    }

    #[async_trait::async_trait]
    impl SyncMonitoring for MockSyncMonitoring {
        async fn record_sync_operation(
            &self,
            source: &str,
            success: bool,
            duration: Duration,
            items_added: u64,
            items_updated: u64,
            items_total: u64,
        ) {
            let mut operations = self.recorded_operations.lock().await;
            operations.push(SyncOperationRecord {
                source: source.to_string(),
                success,
                duration,
                items_added,
                items_updated,
                items_total,
            });
        }

        async fn record_api_request(&self, service: &str, duration: Duration, rate_limited: bool) {
            let mut requests = self.recorded_api_requests.lock().await;
            requests.push(ApiRequestRecord {
                service: service.to_string(),
                duration,
                rate_limited,
            });
        }

        async fn record_cache_access(&self, cache_type: &str, hit: bool) {
            let mut accesses = self.recorded_cache_accesses.lock().await;
            accesses.push(CacheAccessRecord {
                cache_type: cache_type.to_string(),
                hit,
            });
        }
    }

    /// Create a test movie with specified quality level
    fn create_test_movie(tmdb_id: i32, quality_level: &str) -> Movie {
        match quality_level {
            "high" => Movie {
                id: Uuid::new_v4(),
                title: "High Quality Movie".to_string(),
                tmdb_id: Some(tmdb_id),
                imdb_id: Some(format!("tt{:07}", tmdb_id)),
                year: Some(2023),
                overview: Some("This is a high quality movie with complete metadata".to_string()),
                poster_path: Some("/high-quality-poster.jpg".to_string()),
                backdrop_path: Some("/high-quality-backdrop.jpg".to_string()),
                release_date: Some(NaiveDate::from_ymd_opt(2023, 6, 15).unwrap()),
                runtime: Some(120),
                genres: vec!["Action".to_string(), "Drama".to_string(), "Thriller".to_string()],
                vote_average: Some(8.7),
                vote_count: Some(15420),
                popularity: Some(125.8),
                original_language: Some("en".to_string()),
                status: Some("Released".to_string()),
                updated_at: Some(Utc::now()),
                ..Default::default()
            },
            "medium" => Movie {
                id: Uuid::new_v4(),
                title: "Medium Quality Movie".to_string(),
                tmdb_id: Some(tmdb_id),
                year: Some(2023),
                overview: Some("Medium quality metadata".to_string()),
                poster_path: Some("/medium-poster.jpg".to_string()),
                runtime: Some(95),
                genres: vec!["Comedy".to_string()],
                vote_average: Some(7.2),
                vote_count: Some(832),
                updated_at: Some(Utc::now() - chrono::Duration::days(30)),
                ..Default::default()
            },
            "low" => Movie {
                id: Uuid::new_v4(),
                title: "Low Quality Movie".to_string(),
                tmdb_id: Some(tmdb_id),
                updated_at: Some(Utc::now() - chrono::Duration::days(90)),
                ..Default::default()
            },
            _ => Movie {
                id: Uuid::new_v4(),
                title: "Default Movie".to_string(),
                tmdb_id: Some(tmdb_id),
                ..Default::default()
            }
        }
    }

    #[tokio::test]
    async fn test_enhanced_sync_handler_basic_workflow() {
        let movie_repo = Arc::new(MockMovieRepository::new());
        let list_sync_repo = Arc::new(MockListSyncRepository::new());
        let monitoring = Arc::new(MockSyncMonitoring::new());
        
        let config = SyncHandlerConfig {
            enable_performance_tracking: true,
            ..Default::default()
        };

        let handler = EnhancedSyncHandler::new(
            movie_repo.clone(),
            list_sync_repo.clone(),
            monitoring.clone(),
            config,
        );

        let job = SyncJob {
            id: Uuid::new_v4(),
            list_id: Uuid::new_v4(),
            list_name: "Test List".to_string(),
            source_type: "imdb".to_string(),
            enabled: true,
            sync_interval: chrono::Duration::hours(6),
            next_sync: Utc::now(),
            last_sync: None,
            priority: 5,
            retry_count: 0,
            max_retries: 3,
        };

        // Execute sync
        let result = handler.execute_sync(&job).await.unwrap();

        // Verify sync result
        assert_eq!(result.status, SyncStatus::Success);
        assert_eq!(result.job_id, job.id);
        assert_eq!(result.list_id, job.list_id);
        assert!(result.duration_ms > 0);

        // Verify repository integration
        let sync_records = list_sync_repo.get_sync_records().await;
        assert_eq!(sync_records.len(), 1);
        assert_eq!(sync_records[0].status, "success");

        // Verify monitoring integration
        let operations = monitoring.get_recorded_operations().await;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].source, "imdb");
        assert!(operations[0].success);

        // Verify performance metrics recorded
        let perf_records = list_sync_repo.get_performance_records().await;
        assert_eq!(perf_records.len(), 1);
        assert_eq!(perf_records[0].1, job.list_id);
    }

    #[tokio::test]
    async fn test_intelligent_conflict_resolution() {
        let movie_repo = Arc::new(MockMovieRepository::new());
        let list_sync_repo = Arc::new(MockListSyncRepository::new());
        let monitoring = Arc::new(MockSyncMonitoring::new());
        
        let config = SyncHandlerConfig {
            conflict_strategy: ConflictStrategy::Intelligent,
            ..Default::default()
        };

        let handler = EnhancedSyncHandler::new(
            movie_repo.clone(),
            list_sync_repo.clone(),
            monitoring.clone(),
            config,
        );

        // Test scenarios
        let existing_high = create_test_movie(12345, "high");
        let new_low = create_test_movie(12345, "low");
        let new_high = create_test_movie(12345, "high");
        let existing_medium = create_test_movie(12345, "medium");

        // High quality existing vs low quality new -> Keep
        let resolution = handler.resolve_conflict(&existing_high, &new_low).await;
        assert_eq!(resolution, ConflictResolution::Keep);

        // Medium quality existing vs high quality new -> Update
        let resolution = handler.resolve_conflict(&existing_medium, &new_high).await;
        assert_eq!(resolution, ConflictResolution::Update);

        // Similar quality -> should result in Merge
        let resolution = handler.resolve_conflict(&existing_high, &new_high).await;
        assert_eq!(resolution, ConflictResolution::Merge);
    }

    #[tokio::test]
    async fn test_rules_based_conflict_resolution() {
        let movie_repo = Arc::new(MockMovieRepository::new());
        let list_sync_repo = Arc::new(MockListSyncRepository::new());
        let monitoring = Arc::new(MockSyncMonitoring::new());
        
        let config = SyncHandlerConfig {
            conflict_strategy: ConflictStrategy::RulesBased,
            ..Default::default()
        };

        let handler = EnhancedSyncHandler::new(
            movie_repo,
            list_sync_repo,
            monitoring,
            config,
        );

        let existing_with_images = Movie {
            id: Uuid::new_v4(),
            title: "Movie with Images".to_string(),
            tmdb_id: Some(12345),
            poster_path: Some("/poster.jpg".to_string()),
            backdrop_path: Some("/backdrop.jpg".to_string()),
            year: Some(2020),
            ..Default::default()
        };

        let new_without_images = Movie {
            id: Uuid::new_v4(),
            title: "Movie without Images".to_string(),
            tmdb_id: Some(12345),
            overview: Some("New detailed overview".to_string()),
            runtime: Some(120),
            genres: vec!["Action".to_string(), "Drama".to_string()],
            ..Default::default()
        };

        let resolution = handler.resolve_conflict(&existing_with_images, &new_without_images).await;
        
        // Rules-based should prefer merge to combine best of both
        // (existing has images, new has better metadata)
        assert_eq!(resolution, ConflictResolution::Merge);
    }

    #[tokio::test]
    async fn test_performance_tracking() {
        let movie_repo = Arc::new(MockMovieRepository::new());
        let list_sync_repo = Arc::new(MockListSyncRepository::new());
        let monitoring = Arc::new(MockSyncMonitoring::new());
        
        let config = SyncHandlerConfig {
            enable_performance_tracking: true,
            batch_size: 50,
            ..Default::default()
        };

        let handler = EnhancedSyncHandler::new(
            movie_repo,
            list_sync_repo.clone(),
            monitoring,
            config,
        );

        // Simulate performance tracking
        {
            let mut tracker = handler.performance_tracker.write().await;
            tracker.start_sync();
            tracker.record_batch_processed(100, Duration::from_millis(250));
            tracker.record_api_request();
            tracker.record_api_request();
            tracker.record_cache_hit();
            tracker.record_cache_miss();
            tracker.record_memory_sample(256.7);
        }

        let job = SyncJob {
            id: Uuid::new_v4(),
            list_id: Uuid::new_v4(),
            list_name: "Performance Test List".to_string(),
            source_type: "tmdb".to_string(),
            enabled: true,
            sync_interval: chrono::Duration::hours(6),
            next_sync: Utc::now(),
            last_sync: None,
            priority: 5,
            retry_count: 0,
            max_retries: 3,
        };

        let result = handler.execute_sync(&job).await.unwrap();
        assert_eq!(result.status, SyncStatus::Success);

        // Verify performance metrics were recorded
        let perf_records = list_sync_repo.get_performance_records().await;
        assert_eq!(perf_records.len(), 1);
        
        let metrics = &perf_records[0].0;
        assert!(metrics.duration_ms > 0);
        assert_eq!(metrics.network_requests, 2);
        assert_eq!(metrics.cache_hit_rate, Some(0.5)); // 1 hit, 1 miss = 50%
        assert_eq!(metrics.memory_peak_mb, Some(256.7));
        assert!(metrics.items_per_second > 0.0);
    }

    #[tokio::test]
    async fn test_conflict_strategy_variants() {
        let movie_repo = Arc::new(MockMovieRepository::new());
        let list_sync_repo = Arc::new(MockListSyncRepository::new());
        let monitoring = Arc::new(MockSyncMonitoring::new());

        let existing = create_test_movie(12345, "medium");
        let new = create_test_movie(12345, "high");

        // Test KeepExisting strategy
        let config = SyncHandlerConfig {
            conflict_strategy: ConflictStrategy::KeepExisting,
            ..Default::default()
        };
        let handler = EnhancedSyncHandler::new(
            movie_repo.clone(),
            list_sync_repo.clone(),
            monitoring.clone(),
            config,
        );
        let resolution = handler.resolve_conflict(&existing, &new).await;
        assert_eq!(resolution, ConflictResolution::Keep);

        // Test UseNew strategy
        let config = SyncHandlerConfig {
            conflict_strategy: ConflictStrategy::UseNew,
            ..Default::default()
        };
        let handler = EnhancedSyncHandler::new(
            movie_repo.clone(),
            list_sync_repo.clone(),
            monitoring.clone(),
            config,
        );
        let resolution = handler.resolve_conflict(&existing, &new).await;
        assert_eq!(resolution, ConflictResolution::Update);
    }

    #[tokio::test]
    async fn test_data_quality_scoring() {
        let resolver = ConflictResolver::new(ConflictStrategy::Intelligent);

        // Test complete movie scoring
        let complete_movie = create_test_movie(12345, "high");
        let score = resolver.calculate_data_quality_score(&complete_movie);
        assert!(score > 0.8, "Complete movie should have high score: {}", score);

        // Test minimal movie scoring
        let minimal_movie = create_test_movie(67890, "low");
        let minimal_score = resolver.calculate_data_quality_score(&minimal_movie);
        assert!(minimal_score < 0.3, "Minimal movie should have low score: {}", minimal_score);

        // Complete should score higher than minimal
        assert!(score > minimal_score);
    }

    #[tokio::test]
    async fn test_metadata_completeness_calculation() {
        let resolver = ConflictResolver::new(ConflictStrategy::RulesBased);

        let complete_movie = create_test_movie(12345, "high");
        let incomplete_movie = create_test_movie(67890, "low");

        let complete_score = resolver.calculate_metadata_completeness(&complete_movie);
        let incomplete_score = resolver.calculate_metadata_completeness(&incomplete_movie);

        assert!(complete_score > incomplete_score);
        assert!(complete_score > 0.7); // Should be quite complete
        assert!(incomplete_score < 0.3); // Should be quite incomplete
    }

    #[tokio::test]
    async fn test_performance_tracker_lifecycle() {
        let mut tracker = PerformanceTracker::default();

        // Test initial state
        let initial_metrics = tracker.get_metrics();
        assert_eq!(initial_metrics.duration_ms, 0);
        assert_eq!(initial_metrics.items_per_second, 0.0);

        // Start tracking
        tracker.start_sync();
        
        // Record some activity
        tracker.record_batch_processed(50, Duration::from_millis(100));
        tracker.record_batch_processed(30, Duration::from_millis(75));
        tracker.record_api_request();
        tracker.record_api_request();
        tracker.record_cache_hit();
        tracker.record_cache_miss();
        tracker.record_cache_hit();
        tracker.record_memory_sample(128.0);
        tracker.record_memory_sample(256.0);
        tracker.record_memory_sample(192.0);
        tracker.record_error("Test error".to_string());

        // Wait a bit to get measurable duration
        tokio::time::sleep(Duration::from_millis(10)).await;

        let final_metrics = tracker.get_metrics();

        // Verify metrics
        assert!(final_metrics.duration_ms > 0);
        assert_eq!(final_metrics.network_requests, 2);
        assert_eq!(final_metrics.cache_hit_rate, Some(2.0/3.0)); // 2 hits out of 3 total
        assert_eq!(final_metrics.memory_peak_mb, Some(256.0));
        assert_eq!(final_metrics.error_rate, 1.0/80.0); // 1 error out of 80 items
        assert!(final_metrics.items_per_second > 0.0);
        assert_eq!(final_metrics.batch_processing_times.len(), 2);
    }
}