//! Enhanced Sync Handler Implementation
//!
//! Provides sophisticated sync handling with performance monitoring,
//! conflict resolution strategies, and comprehensive audit logging.

use crate::jobs::list_sync::{
    ConflictResolution, SyncError, SyncHandler, SyncJob, SyncResult, SyncStatus,
    MovieProvenance,
};
use crate::models::Movie;
use chrono::Utc;
// use chrono::DateTime; // Currently unused
use serde::{Deserialize, Serialize};
// use std::collections::HashMap; // Currently unused
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
// use tracing::error; // Currently unused
use uuid::Uuid;

/// Enhanced sync handler with performance tracking and sophisticated conflict resolution
pub struct EnhancedSyncHandler {
    movie_repository: Arc<dyn MovieRepository>,
    list_sync_repository: Arc<dyn ListSyncRepository>,
    monitoring: Arc<dyn SyncMonitoring>,
    conflict_resolver: Arc<ConflictResolver>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
    config: SyncHandlerConfig,
}

/// Configuration for the enhanced sync handler
#[derive(Debug, Clone)]
pub struct SyncHandlerConfig {
    /// Maximum duration before timing out a sync operation
    pub max_sync_duration: Duration,
    /// Batch size for processing items
    pub batch_size: usize,
    /// Whether to enable detailed performance tracking
    pub enable_performance_tracking: bool,
    /// Conflict resolution strategy priority
    pub conflict_strategy: ConflictStrategy,
    /// Memory usage thresholds
    pub memory_warning_mb: f64,
    pub memory_critical_mb: f64,
    /// Rate limiting settings
    pub max_requests_per_second: f64,
    pub burst_allowance: usize,
}

impl Default for SyncHandlerConfig {
    fn default() -> Self {
        Self {
            max_sync_duration: Duration::from_secs(1800), // 30 minutes
            batch_size: 100,
            enable_performance_tracking: true,
            conflict_strategy: ConflictStrategy::Intelligent,
            memory_warning_mb: 512.0,
            memory_critical_mb: 1024.0,
            max_requests_per_second: 10.0,
            burst_allowance: 20,
        }
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictStrategy {
    /// Always keep existing data
    KeepExisting,
    /// Always use new data
    UseNew,
    /// Merge based on data quality and recency
    Intelligent,
    /// Custom rules-based resolution
    RulesBased,
}

/// Performance tracking for sync operations
#[derive(Debug, Clone, Default)]
pub struct PerformanceTracker {
    sync_start: Option<Instant>,
    items_processed: usize,
    memory_samples: Vec<(Instant, f64)>,
    api_requests: usize,
    cache_hits: usize,
    cache_misses: usize,
    errors: Vec<String>,
    batch_times: Vec<Duration>,
}


impl PerformanceTracker {
    fn start_sync(&mut self) {
        self.sync_start = Some(Instant::now());
        self.items_processed = 0;
        self.memory_samples.clear();
        self.api_requests = 0;
        self.cache_hits = 0;
        self.cache_misses = 0;
        self.errors.clear();
        self.batch_times.clear();
    }

    fn record_batch_processed(&mut self, count: usize, duration: Duration) {
        self.items_processed += count;
        self.batch_times.push(duration);
    }

    fn record_memory_sample(&mut self, memory_mb: f64) {
        self.memory_samples.push((Instant::now(), memory_mb));
        // Keep only last hour of samples
        let cutoff = Instant::now() - Duration::from_secs(3600);
        self.memory_samples.retain(|(time, _)| *time > cutoff);
    }

    fn record_api_request(&mut self) {
        self.api_requests += 1;
    }

    fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    fn record_error(&mut self, error: String) {
        self.errors.push(error);
    }

    fn get_metrics(&self) -> PerformanceMetrics {
        let duration = self.sync_start
            .map(|start| start.elapsed())
            .unwrap_or_default();

        let items_per_second = if duration.as_secs_f64() > 0.0 {
            self.items_processed as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let memory_peak = self.memory_samples
            .iter()
            .map(|(_, mem)| *mem)
            .fold(0.0f64, f64::max);

        let cache_hit_rate = if (self.cache_hits + self.cache_misses) > 0 {
            Some(self.cache_hits as f64 / (self.cache_hits + self.cache_misses) as f64)
        } else {
            None
        };

        let error_rate = self.errors.len() as f64 / self.items_processed.max(1) as f64;

        PerformanceMetrics {
            duration_ms: duration.as_millis() as i64,
            items_per_second,
            memory_peak_mb: if memory_peak > 0.0 { Some(memory_peak) } else { None },
            network_requests: self.api_requests as i32,
            cache_hit_rate,
            error_rate,
            batch_processing_times: self.batch_times.clone(),
        }
    }
}

/// Performance metrics for a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub duration_ms: i64,
    pub items_per_second: f64,
    pub memory_peak_mb: Option<f64>,
    pub network_requests: i32,
    pub cache_hit_rate: Option<f64>,
    pub error_rate: f64,
    pub batch_processing_times: Vec<Duration>,
}

/// Sophisticated conflict resolver
pub struct ConflictResolver {
    strategy: ConflictStrategy,
}

impl ConflictResolver {
    pub fn new(strategy: ConflictStrategy) -> Self {
        Self { strategy }
    }

    /// Resolve conflicts between existing and new movie data
    pub async fn resolve_conflict(
        &self,
        existing: &Movie,
        new: &Movie,
    ) -> ConflictResolution {
        match self.strategy {
            ConflictStrategy::KeepExisting => ConflictResolution::Keep,
            ConflictStrategy::UseNew => ConflictResolution::Update,
            ConflictStrategy::Intelligent => self.intelligent_resolve(existing, new).await,
            ConflictStrategy::RulesBased => self.rules_based_resolve(existing, new).await,
        }
    }

    async fn intelligent_resolve(&self, existing: &Movie, new: &Movie) -> ConflictResolution {
        // Intelligent conflict resolution based on data quality and recency
        
        // Calculate quality scores for both datasets
        let existing_score = self.calculate_data_quality_score(existing);
        let new_score = self.calculate_data_quality_score(new);

        debug!(
            existing_score = existing_score,
            new_score = new_score,
            "Calculated data quality scores"
        );

        // If scores are very close, prefer newer data
        if (new_score - existing_score).abs() < 0.1 {
            // Check if new data is significantly more recent
            let age_diff = (new.updated_at - existing.updated_at).num_hours();
            if age_diff > 24 {
                return ConflictResolution::Merge;
            }
        }

        // If new data is significantly better quality, use it
        if new_score > existing_score + 0.2 {
            ConflictResolution::Update
        } else if existing_score > new_score + 0.2 {
            ConflictResolution::Keep
        } else {
            // Similar quality, merge the best of both
            ConflictResolution::Merge
        }
    }

    async fn rules_based_resolve(&self, existing: &Movie, new: &Movie) -> ConflictResolution {
        // Rules-based conflict resolution with specific criteria

        // Rule 1: Never downgrade from higher quality poster/backdrop
        if self.has_higher_quality_images(existing, new) {
            if self.has_higher_quality_images(new, existing) {
                return ConflictResolution::Merge; // Both have good images, merge
            } else {
                return ConflictResolution::Keep; // Existing has better images
            }
        }

        // Rule 2: Prefer data with more complete metadata
        let existing_completeness = self.calculate_metadata_completeness(existing);
        let new_completeness = self.calculate_metadata_completeness(new);

        if new_completeness > existing_completeness * 1.2 {
            return ConflictResolution::Update;
        } else if existing_completeness > new_completeness * 1.2 {
            return ConflictResolution::Keep;
        }

        // Rule 3: Prefer data from more reliable sources (TMDb over IMDb for metadata)
        // This would require source tracking in the Movie model

        // Default to merge for rules-based approach
        ConflictResolution::Merge
    }

    fn calculate_data_quality_score(&self, movie: &Movie) -> f64 {
        let mut score = 0.0;

        // Basic required fields (40% of score)
        if !movie.title.is_empty() { score += 0.1; }
        score += 0.1; // tmdb_id is always present (required field)
        if movie.year.is_some() { score += 0.1; }
        
        // Check for release date in metadata
        if movie.metadata.get("tmdb").and_then(|t| t.get("release_date")).is_some() { score += 0.1; }

        // Enhanced metadata (30% of score)
        if movie.overview().is_some() { score += 0.1; }
        if movie.runtime.is_some() && movie.runtime.unwrap() > 0 { score += 0.05; }
        
        // Check for genres in metadata
        if movie.metadata.get("tmdb").and_then(|t| t.get("genres"))
            .and_then(|g| g.as_array()).map(|arr| !arr.is_empty()).unwrap_or(false) { 
            score += 0.05; 
        }
        
        if movie.rating().is_some() && movie.rating().unwrap() > 0.0 { score += 0.05; }
        
        // Check vote count in metadata
        if movie.metadata.get("tmdb").and_then(|t| t.get("vote_count"))
            .and_then(|v| v.as_u64()).map(|c| c > 10).unwrap_or(false) { 
            score += 0.05; 
        }

        // Visual assets (20% of score)
        if movie.metadata.get("tmdb").and_then(|t| t.get("poster_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false) { 
            score += 0.1; 
        }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("backdrop_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false) { 
            score += 0.1; 
        }

        // Technical metadata (10% of score)
        if movie.original_title.is_some() { score += 0.05; }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("popularity"))
            .and_then(|p| p.as_f64()).map(|p| p > 0.0).unwrap_or(false) { 
            score += 0.05; 
        }

        score
    }

    fn calculate_metadata_completeness(&self, movie: &Movie) -> f64 {
        let total_fields = 15.0; // Total number of metadata fields we care about
        let mut complete_fields = 0.0;

        if !movie.title.is_empty() { complete_fields += 1.0; }
        complete_fields += 1.0; // tmdb_id is always present (required field)
        if movie.imdb_id.as_ref().map(|s| !s.is_empty()).unwrap_or(false) { complete_fields += 1.0; }
        if movie.year.is_some() { complete_fields += 1.0; }
        
        // Check metadata fields
        if movie.metadata.get("tmdb").and_then(|t| t.get("release_date")).is_some() { complete_fields += 1.0; }
        if movie.overview().is_some() { complete_fields += 1.0; }
        if movie.runtime.is_some() && movie.runtime.unwrap() > 0 { complete_fields += 1.0; }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("genres"))
            .and_then(|g| g.as_array()).map(|arr| !arr.is_empty()).unwrap_or(false) { 
            complete_fields += 1.0; 
        }
        
        if movie.rating().is_some() { complete_fields += 1.0; }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("vote_count")).is_some() { complete_fields += 1.0; }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("poster_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false) { 
            complete_fields += 1.0; 
        }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("backdrop_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false) { 
            complete_fields += 1.0; 
        }
        
        if movie.original_title.is_some() { complete_fields += 1.0; }
        
        if movie.metadata.get("tmdb").and_then(|t| t.get("popularity")).is_some() { complete_fields += 1.0; }
        
        complete_fields += 1.0; // status is always present

        complete_fields / total_fields
    }

    fn has_higher_quality_images(&self, movie1: &Movie, movie2: &Movie) -> bool {
        let movie1_poster = movie1.metadata.get("tmdb").and_then(|t| t.get("poster_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
        let movie1_backdrop = movie1.metadata.get("tmdb").and_then(|t| t.get("backdrop_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
            
        let movie2_poster = movie2.metadata.get("tmdb").and_then(|t| t.get("poster_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
        let movie2_backdrop = movie2.metadata.get("tmdb").and_then(|t| t.get("backdrop_path"))
            .and_then(|p| p.as_str()).map(|s| !s.is_empty()).unwrap_or(false);

        // Simple heuristic: movie1 has higher quality if it has more images
        let movie1_count = movie1_poster as u8 + movie1_backdrop as u8;
        let movie2_count = movie2_poster as u8 + movie2_backdrop as u8;

        movie1_count > movie2_count
    }
}

// Trait definitions (these would typically be in separate files)
#[async_trait::async_trait]
pub trait MovieRepository: Send + Sync {
    async fn find_by_tmdb_id(&self, tmdb_id: i32) -> Result<Option<Movie>, SyncError>;
    async fn find_by_imdb_id(&self, imdb_id: &str) -> Result<Option<Movie>, SyncError>;
    async fn create(&self, movie: &Movie) -> Result<Movie, SyncError>;
    async fn update(&self, movie: &Movie) -> Result<Movie, SyncError>;
}

#[async_trait::async_trait]
pub trait ListSyncRepository: Send + Sync {
    async fn start_sync(&self, list_id: Uuid, metadata: Option<serde_json::Value>) -> Result<Uuid, SyncError>;
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
        error_details: Option<serde_json::Value>,
    ) -> Result<(), SyncError>;
    async fn record_performance_metrics(&self, metrics: &PerformanceMetrics, list_id: Uuid) -> Result<(), SyncError>;
}

#[async_trait::async_trait]
pub trait SyncMonitoring: Send + Sync {
    async fn record_sync_operation(
        &self,
        source: &str,
        success: bool,
        duration: Duration,
        items_added: u64,
        items_updated: u64,
        items_total: u64,
    );
    async fn record_api_request(&self, service: &str, duration: Duration, rate_limited: bool);
    async fn record_cache_access(&self, cache_type: &str, hit: bool);
}

impl EnhancedSyncHandler {
    pub fn new(
        movie_repository: Arc<dyn MovieRepository>,
        list_sync_repository: Arc<dyn ListSyncRepository>,
        monitoring: Arc<dyn SyncMonitoring>,
        config: SyncHandlerConfig,
    ) -> Self {
        let conflict_resolver = Arc::new(ConflictResolver::new(config.conflict_strategy.clone()));
        
        Self {
            movie_repository,
            list_sync_repository,
            monitoring,
            conflict_resolver,
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::default())),
            config,
        }
    }
}

#[async_trait::async_trait]
impl SyncHandler for EnhancedSyncHandler {
    async fn execute_sync(&self, job: &SyncJob) -> Result<SyncResult, SyncError> {
        let start_time = Utc::now();
        
        // Initialize performance tracking
        {
            let mut tracker = self.performance_tracker.write().await;
            tracker.start_sync();
        }

        // Start sync in repository
        let sync_id = self.list_sync_repository
            .start_sync(job.list_id, None)
            .await
            .map_err(|e| SyncError::DatabaseError(e.to_string()))?;

        info!("Started sync {} for job {}", sync_id, job.id);

        // Execute the actual sync logic
        let result = self.execute_sync_internal(job, sync_id).await;

        // Calculate final metrics
        let performance_metrics = {
            let tracker = self.performance_tracker.read().await;
            tracker.get_metrics()
        };

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds();

        // Record performance metrics
        if self.config.enable_performance_tracking {
            if let Err(e) = self.list_sync_repository
                .record_performance_metrics(&performance_metrics, job.list_id)
                .await
            {
                warn!("Failed to record performance metrics: {}", e);
            }
        }

        // Complete sync in repository based on result
        match &result {
            Ok(sync_result) => {
                self.list_sync_repository
                    .complete_sync(
                        sync_id,
                        match sync_result.status {
                            SyncStatus::Success => "success",
                            SyncStatus::Partial => "partial",
                            SyncStatus::Failed => "failed",
                            SyncStatus::Cancelled => "cancelled",
                        },
                        sync_result.items_found as i32,
                        sync_result.items_added as i32,
                        sync_result.items_updated as i32,
                        0, // items_removed (not tracked in current SyncResult)
                        sync_result.items_excluded as i32,
                        sync_result.error_message.clone(),
                        None,
                    )
                    .await
                    .map_err(|e| warn!("Failed to complete sync in repository: {}", e))
                    .ok();
            }
            Err(e) => {
                self.list_sync_repository
                    .complete_sync(
                        sync_id,
                        "failed",
                        0,
                        0,
                        0,
                        0,
                        0,
                        Some(e.to_string()),
                        None,
                    )
                    .await
                    .map_err(|e| warn!("Failed to complete failed sync in repository: {}", e))
                    .ok();
            }
        }

        // Record monitoring metrics
        if let Ok(sync_result) = &result {
            self.monitoring
                .record_sync_operation(
                    &job.source_type,
                    sync_result.status == SyncStatus::Success,
                    std::time::Duration::from_millis(duration_ms as u64),
                    sync_result.items_added as u64,
                    sync_result.items_updated as u64,
                    sync_result.items_found as u64,
                )
                .await;
        }

        result
    }

    async fn resolve_conflict(&self, existing: &Movie, new: &Movie) -> ConflictResolution {
        self.conflict_resolver.resolve_conflict(existing, new).await
    }

    async fn store_results(&self, results: &SyncResult) -> Result<(), SyncError> {
        debug!("Storing sync results for job {}: {} items processed", 
               results.job_id, results.items_found);
        
        // The results are already stored via the enhanced repository integration
        // This method could be used for additional result storage if needed
        Ok(())
    }
}

impl EnhancedSyncHandler {
    async fn execute_sync_internal(&self, job: &SyncJob, _sync_id: Uuid) -> Result<SyncResult, SyncError> {
        // This would contain the actual sync logic specific to each source type
        // For now, we'll return a mock result
        
        let start_time = Utc::now();
        
        // Simulate sync work with performance tracking
        {
            let mut tracker = self.performance_tracker.write().await;
            tracker.record_batch_processed(10, std::time::Duration::from_millis(100));
            tracker.record_api_request();
            tracker.record_cache_hit();
            tracker.record_memory_sample(128.5);
        }

        let end_time = Utc::now();
        let duration_ms = (end_time - start_time).num_milliseconds();

        Ok(SyncResult {
            job_id: job.id,
            list_id: job.list_id,
            status: SyncStatus::Success,
            started_at: start_time,
            completed_at: end_time,
            duration_ms,
            items_found: 10,
            items_added: 5,
            items_updated: 2,
            items_excluded: 3,
            items_conflicted: 1,
            error_message: None,
            provenance: vec![
                MovieProvenance {
                    movie_id: Uuid::new_v4(),
                    list_id: job.list_id,
                    list_name: job.list_name.clone(),
                    source_type: job.source_type.clone(),
                    added_at: start_time,
                    metadata: serde_json::json!({}),
                }
            ],
        })
    }
}

// Include comprehensive test module
#[cfg(test)]
#[path = "enhanced_sync_handler_tests.rs"]
mod tests;