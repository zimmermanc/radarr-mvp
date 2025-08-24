//! Simplified List Sync Integration Example
//!
//! Shows how the components work together without cross-crate dependencies.
//! This is suitable for demonstration and testing within the core crate.

use crate::jobs::{
    EnhancedSyncHandler, ListSyncScheduler, SyncHandlerConfig, ConflictStrategy,
    SyncJob, SyncError, ConflictResolution,
    // SyncResult, SyncStatus, // Currently unused
    enhanced_sync_handler::{MovieRepository, ListSyncRepository, SyncMonitoring, PerformanceMetrics},
};
use crate::models::Movie;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Mock implementations for demonstration
pub struct MockSetup {
    pub movie_repo: Arc<MockMovieRepo>,
    pub sync_repo: Arc<MockSyncRepo>,
    pub monitoring: Arc<MockMonitoring>,
    pub scheduler: ListSyncScheduler,
}

#[derive(Clone)]
pub struct MockMovieRepo {
    movies: Arc<RwLock<HashMap<i32, Movie>>>,
}

#[derive(Clone)]
pub struct MockSyncRepo {
    syncs: Arc<RwLock<Vec<SyncRecord>>>,
}

#[derive(Clone)]
pub struct MockMonitoring {
    operations: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct SyncRecord {
    id: Uuid,
    list_id: Uuid,
    status: String,
    items_found: i32,
    items_added: i32,
}

impl MockMovieRepo {
    fn new() -> Self {
        Self {
            movies: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl MockSyncRepo {
    fn new() -> Self {
        Self {
            syncs: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl MockMonitoring {
    fn new() -> Self {
        Self {
            operations: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl MovieRepository for MockMovieRepo {
    async fn find_by_tmdb_id(&self, tmdb_id: i32) -> Result<Option<Movie>, SyncError> {
        let movies = self.movies.read().await;
        Ok(movies.get(&tmdb_id).cloned())
    }

    async fn find_by_imdb_id(&self, _imdb_id: &str) -> Result<Option<Movie>, SyncError> {
        Ok(None) // Simplified for demo
    }

    async fn create(&self, movie: &Movie) -> Result<Movie, SyncError> {
        let mut movies = self.movies.write().await;
        movies.insert(movie.tmdb_id, movie.clone());
        Ok(movie.clone())
    }

    async fn update(&self, movie: &Movie) -> Result<Movie, SyncError> {
        let mut movies = self.movies.write().await;
        movies.insert(movie.tmdb_id, movie.clone());
        Ok(movie.clone())
    }
}

#[async_trait::async_trait]
impl ListSyncRepository for MockSyncRepo {
    async fn start_sync(&self, list_id: Uuid, _metadata: Option<serde_json::Value>) -> Result<Uuid, SyncError> {
        let sync_id = Uuid::new_v4();
        let mut syncs = self.syncs.write().await;
        syncs.push(SyncRecord {
            id: sync_id,
            list_id,
            status: "started".to_string(),
            items_found: 0,
            items_added: 0,
        });
        Ok(sync_id)
    }

    async fn complete_sync(
        &self,
        sync_id: Uuid,
        status: &str,
        items_found: i32,
        items_added: i32,
        _items_updated: i32,
        _items_removed: i32,
        _items_excluded: i32,
        _error_message: Option<String>,
        _error_details: Option<serde_json::Value>,
    ) -> Result<(), SyncError> {
        let mut syncs = self.syncs.write().await;
        if let Some(sync) = syncs.iter_mut().find(|s| s.id == sync_id) {
            sync.status = status.to_string();
            sync.items_found = items_found;
            sync.items_added = items_added;
        }
        Ok(())
    }

    async fn record_performance_metrics(&self, _metrics: &PerformanceMetrics, _list_id: Uuid) -> Result<(), SyncError> {
        Ok(()) // No-op for demo
    }
}

#[async_trait::async_trait]
impl SyncMonitoring for MockMonitoring {
    async fn record_sync_operation(
        &self,
        source: &str,
        success: bool,
        _duration: std::time::Duration,
        items_added: u64,
        _items_updated: u64,
        items_total: u64,
    ) {
        let mut ops = self.operations.lock().await;
        ops.push(format!("Sync {}: success={}, added={}, total={}", source, success, items_added, items_total));
    }

    async fn record_api_request(&self, service: &str, _duration: std::time::Duration, rate_limited: bool) {
        let mut ops = self.operations.lock().await;
        ops.push(format!("API {}: rate_limited={}", service, rate_limited));
    }

    async fn record_cache_access(&self, cache_type: &str, hit: bool) {
        let mut ops = self.operations.lock().await;
        ops.push(format!("Cache {}: hit={}", cache_type, hit));
    }
}

impl Default for MockSetup {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSetup {
    /// Create a complete mock setup for testing
    pub fn new() -> Self {
        let movie_repo = Arc::new(MockMovieRepo::new());
        let sync_repo = Arc::new(MockSyncRepo::new());
        let monitoring = Arc::new(MockMonitoring::new());

        let config = SyncHandlerConfig {
            enable_performance_tracking: true,
            conflict_strategy: ConflictStrategy::Intelligent,
            ..Default::default()
        };

        let handler = Arc::new(EnhancedSyncHandler::new(
            movie_repo.clone(),
            sync_repo.clone(),
            monitoring.clone(),
            config,
        ));

        let scheduler = ListSyncScheduler::new(handler);

        Self {
            movie_repo,
            sync_repo,
            monitoring,
            scheduler,
        }
    }

    /// Add a sample sync job
    pub async fn add_sample_job(&self, name: &str, source: &str) -> Result<Uuid, SyncError> {
        let job = SyncJob {
            id: Uuid::new_v4(),
            list_id: Uuid::new_v4(),
            list_name: name.to_string(),
            source_type: source.to_string(),
            enabled: true,
            sync_interval: Duration::hours(6),
            next_sync: Utc::now() + Duration::minutes(1),
            last_sync: None,
            priority: 5,
            retry_count: 0,
            max_retries: 3,
        };

        self.scheduler.add_job(job.clone()).await?;
        Ok(job.id)
    }

    /// Trigger a sync manually
    pub async fn trigger_sync(&self, job_id: Uuid) -> Result<(), SyncError> {
        self.scheduler.trigger_sync(job_id).await
    }

    /// Get all recorded monitoring operations
    pub async fn get_monitoring_logs(&self) -> Vec<String> {
        self.monitoring.operations.lock().await.clone()
    }

    /// Get sync history
    pub async fn get_sync_history(&self) -> Vec<SyncRecord> {
        self.sync_repo.syncs.read().await.clone()
    }

    /// Demonstrate conflict resolution with sample movies
    pub async fn demo_conflict_resolution(&self) -> Vec<(ConflictStrategy, ConflictResolution)> {
        let existing = Movie {
            id: Uuid::new_v4(),
            tmdb_id: 603,
            title: "The Matrix".to_string(),
            year: Some(1999),
            runtime: Some(136),
            metadata: serde_json::json!({
                "tmdb": {
                    "overview": "Basic overview",
                    "poster_path": "/poster.jpg",
                    "vote_average": 8.7,
                    "vote_count": 25000
                }
            }),
            created_at: Utc::now() - Duration::days(30),
            updated_at: Utc::now() - Duration::days(30),
            ..Movie::new(603, "The Matrix".to_string())
        };

        let new = Movie {
            id: Uuid::new_v4(),
            tmdb_id: 603,
            title: "The Matrix".to_string(),
            year: Some(1999),
            runtime: Some(136),
            metadata: serde_json::json!({
                "tmdb": {
                    "overview": "Revolutionary sci-fi film about reality and choice",
                    "poster_path": "/better_poster.jpg",
                    "backdrop_path": "/backdrop.jpg",
                    "genres": ["Action", "Sci-Fi"],
                    "vote_average": 8.7,
                    "vote_count": 25500,
                    "popularity": 89.2
                }
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Movie::new(603, "The Matrix".to_string())
        };

        let mut results = Vec::new();
        
        for strategy in [
            ConflictStrategy::KeepExisting,
            ConflictStrategy::UseNew,
            ConflictStrategy::Intelligent,
            ConflictStrategy::RulesBased,
        ] {
            use crate::jobs::enhanced_sync_handler::ConflictResolver;
            let resolver = ConflictResolver::new(strategy.clone());
            let resolution = resolver.resolve_conflict(&existing, &new).await;
            results.push((strategy, resolution));
        }

        results
    }
}

/// Demonstrate the complete system
pub async fn run_integration_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting List Sync Integration Demo");
    
    // Setup the system
    let setup = MockSetup::new();
    
    // Add some jobs
    let imdb_job = setup.add_sample_job("IMDb Top 250", "imdb").await?;
    let tmdb_job = setup.add_sample_job("TMDb Popular", "tmdb").await?;
    
    println!("✅ Added {} sync jobs", 2);
    
    // Trigger syncs
    setup.trigger_sync(imdb_job).await?;
    setup.trigger_sync(tmdb_job).await?;
    
    println!("✅ Triggered sync jobs");
    
    // Check results
    let sync_history = setup.get_sync_history().await;
    println!("📊 Sync History ({} entries):", sync_history.len());
    for sync in &sync_history {
        println!("  - {}: {} (found: {}, added: {})", 
                 sync.list_id, sync.status, sync.items_found, sync.items_added);
    }
    
    let monitoring_logs = setup.get_monitoring_logs().await;
    println!("📈 Monitoring Logs ({} entries):", monitoring_logs.len());
    for log in &monitoring_logs {
        println!("  - {}", log);
    }
    
    // Demo conflict resolution
    println!("🔄 Conflict Resolution Demo:");
    let conflict_results = setup.demo_conflict_resolution().await;
    for (strategy, resolution) in conflict_results {
        println!("  - Strategy {:?} → Resolution {:?}", strategy, resolution);
    }
    
    println!("✨ Integration demo completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_setup_creation() {
        let setup = MockSetup::new();
        assert!(setup.movie_repo.movies.read().await.is_empty());
        assert!(setup.sync_repo.syncs.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let setup = MockSetup::new();
        
        // Add job
        let job_id = setup.add_sample_job("Test List", "test").await.unwrap();
        
        // Trigger sync
        setup.trigger_sync(job_id).await.unwrap();
        
        // Check sync history
        let history = setup.get_sync_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, "success");
        
        // Check monitoring
        let logs = setup.get_monitoring_logs().await;
        assert!(!logs.is_empty());
    }

    #[tokio::test]
    async fn test_conflict_resolution_demo() {
        let setup = MockSetup::new();
        let results = setup.demo_conflict_resolution().await;
        
        assert_eq!(results.len(), 4);
        
        // Verify each strategy produces a different result pattern
        let keep_existing = results.iter().find(|(s, _)| matches!(s, ConflictStrategy::KeepExisting)).unwrap();
        let use_new = results.iter().find(|(s, _)| matches!(s, ConflictStrategy::UseNew)).unwrap();
        
        assert_eq!(keep_existing.1, ConflictResolution::Keep);
        assert_eq!(use_new.1, ConflictResolution::Update);
    }

    #[tokio::test]
    async fn test_full_integration_demo() {
        // This should run without errors
        run_integration_demo().await.unwrap();
    }
}