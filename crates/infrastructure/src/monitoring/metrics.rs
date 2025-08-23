//! Prometheus metrics implementation for List Sync monitoring
//!
//! This module defines and manages all Prometheus metrics for the List Sync system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for Prometheus metrics
#[derive(Debug, Clone)]
pub struct MetricsConfig {
    /// Namespace for all metrics (e.g., "radarr")
    pub namespace: String,
    /// Subsystem for list sync metrics (e.g., "list_sync")
    pub subsystem: String,
    /// Labels to add to all metrics
    pub default_labels: HashMap<String, String>,
    /// Whether to enable detailed timing metrics
    pub enable_timing_histograms: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            namespace: "radarr".to_string(),
            subsystem: "list_sync".to_string(),
            default_labels: HashMap::new(),
            enable_timing_histograms: true,
        }
    }
}

/// Core Prometheus metrics for List Sync operations
pub struct PrometheusMetrics {
    config: MetricsConfig,
    
    // Sync operation counters
    sync_operations_total: Arc<RwLock<HashMap<String, u64>>>,
    sync_operations_success: Arc<RwLock<HashMap<String, u64>>>,
    sync_operations_failed: Arc<RwLock<HashMap<String, u64>>>,
    
    // Sync performance metrics
    sync_duration_seconds: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    sync_items_processed: Arc<RwLock<HashMap<String, u64>>>,
    sync_items_added: Arc<RwLock<HashMap<String, u64>>>,
    sync_items_updated: Arc<RwLock<HashMap<String, u64>>>,
    
    // API performance metrics
    api_request_duration: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    api_requests_total: Arc<RwLock<HashMap<String, u64>>>,
    api_rate_limit_hits: Arc<RwLock<HashMap<String, u64>>>,
    
    // Cache metrics
    cache_hits_total: Arc<RwLock<HashMap<String, u64>>>,
    cache_misses_total: Arc<RwLock<HashMap<String, u64>>>,
    
    // Queue metrics
    queue_depth: Arc<RwLock<HashMap<String, u64>>>,
    queue_processing_time: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    
    // Circuit breaker metrics
    circuit_breaker_state: Arc<RwLock<HashMap<String, String>>>,
    circuit_breaker_failures: Arc<RwLock<HashMap<String, u64>>>,
    
    // Service health metrics
    service_up: Arc<RwLock<HashMap<String, u64>>>,
    last_successful_sync: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl PrometheusMetrics {
    /// Create a new PrometheusMetrics instance
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            config,
            sync_operations_total: Arc::new(RwLock::new(HashMap::new())),
            sync_operations_success: Arc::new(RwLock::new(HashMap::new())),
            sync_operations_failed: Arc::new(RwLock::new(HashMap::new())),
            sync_duration_seconds: Arc::new(RwLock::new(HashMap::new())),
            sync_items_processed: Arc::new(RwLock::new(HashMap::new())),
            sync_items_added: Arc::new(RwLock::new(HashMap::new())),
            sync_items_updated: Arc::new(RwLock::new(HashMap::new())),
            api_request_duration: Arc::new(RwLock::new(HashMap::new())),
            api_requests_total: Arc::new(RwLock::new(HashMap::new())),
            api_rate_limit_hits: Arc::new(RwLock::new(HashMap::new())),
            cache_hits_total: Arc::new(RwLock::new(HashMap::new())),
            cache_misses_total: Arc::new(RwLock::new(HashMap::new())),
            queue_depth: Arc::new(RwLock::new(HashMap::new())),
            queue_processing_time: Arc::new(RwLock::new(HashMap::new())),
            circuit_breaker_state: Arc::new(RwLock::new(HashMap::new())),
            circuit_breaker_failures: Arc::new(RwLock::new(HashMap::new())),
            service_up: Arc::new(RwLock::new(HashMap::new())),
            last_successful_sync: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a sync operation
    pub async fn record_sync_operation(&self, source: &str, success: bool, duration: Duration) {
        let label_key = format!("source={}", source);
        
        // Increment total operations
        {
            let mut total = self.sync_operations_total.write().await;
            *total.entry(label_key.clone()).or_insert(0) += 1;
        }
        
        // Increment success/failure counter
        if success {
            let mut success_map = self.sync_operations_success.write().await;
            *success_map.entry(label_key.clone()).or_insert(0) += 1;
            
            // Update last successful sync time
            let mut last_sync = self.last_successful_sync.write().await;
            last_sync.insert(label_key.clone(), Utc::now());
        } else {
            let mut failed = self.sync_operations_failed.write().await;
            *failed.entry(label_key.clone()).or_insert(0) += 1;
        }
        
        // Record duration
        if self.config.enable_timing_histograms {
            let mut durations = self.sync_duration_seconds.write().await;
            durations.entry(label_key).or_insert_with(Vec::new)
                .push(duration.as_secs_f64());
        }
        
        debug!(
            source = source,
            success = success,
            duration_ms = duration.as_millis(),
            "Recorded sync operation metrics"
        );
    }

    /// Record sync items processed
    pub async fn record_sync_items(&self, source: &str, added: u64, updated: u64, total: u64) {
        let label_key = format!("source={}", source);
        
        {
            let mut processed = self.sync_items_processed.write().await;
            *processed.entry(label_key.clone()).or_insert(0) += total;
        }
        
        {
            let mut added_map = self.sync_items_added.write().await;
            *added_map.entry(label_key.clone()).or_insert(0) += added;
        }
        
        {
            let mut updated_map = self.sync_items_updated.write().await;
            *updated_map.entry(label_key.clone()).or_insert(0) += updated;
        }
        
        debug!(
            source = source,
            items_added = added,
            items_updated = updated,
            items_total = total,
            "Recorded sync items metrics"
        );
    }

    /// Record API request performance
    pub async fn record_api_request(&self, service: &str, duration: Duration, rate_limited: bool) {
        let label_key = format!("service={}", service);
        
        {
            let mut requests = self.api_requests_total.write().await;
            *requests.entry(label_key.clone()).or_insert(0) += 1;
        }
        
        if rate_limited {
            let mut rate_limits = self.api_rate_limit_hits.write().await;
            *rate_limits.entry(label_key.clone()).or_insert(0) += 1;
            warn!(service = service, "API rate limit hit recorded");
        }
        
        if self.config.enable_timing_histograms {
            let mut durations = self.api_request_duration.write().await;
            durations.entry(label_key).or_insert_with(Vec::new)
                .push(duration.as_secs_f64());
        }
        
        debug!(
            service = service,
            duration_ms = duration.as_millis(),
            rate_limited = rate_limited,
            "Recorded API request metrics"
        );
    }

    /// Record cache hit or miss
    pub async fn record_cache_access(&self, cache_type: &str, hit: bool) {
        let label_key = format!("type={}", cache_type);
        
        if hit {
            let mut hits = self.cache_hits_total.write().await;
            *hits.entry(label_key).or_insert(0) += 1;
        } else {
            let mut misses = self.cache_misses_total.write().await;
            *misses.entry(label_key).or_insert(0) += 1;
        }
        
        debug!(
            cache_type = cache_type,
            hit = hit,
            "Recorded cache access metrics"
        );
    }

    /// Record queue metrics
    pub async fn record_queue_metrics(&self, queue_name: &str, depth: u64, processing_time: Duration) {
        let label_key = format!("queue={}", queue_name);
        
        {
            let mut queue_depths = self.queue_depth.write().await;
            queue_depths.insert(label_key.clone(), depth);
        }
        
        if self.config.enable_timing_histograms {
            let mut processing_times = self.queue_processing_time.write().await;
            processing_times.entry(label_key).or_insert_with(Vec::new)
                .push(processing_time.as_secs_f64());
        }
        
        debug!(
            queue = queue_name,
            depth = depth,
            processing_time_ms = processing_time.as_millis(),
            "Recorded queue metrics"
        );
    }

    /// Record circuit breaker state
    pub async fn record_circuit_breaker_state(&self, service: &str, state: &str, failure_count: u64) {
        let label_key = format!("service={}", service);
        
        {
            let mut states = self.circuit_breaker_state.write().await;
            states.insert(label_key.clone(), state.to_string());
        }
        
        {
            let mut failures = self.circuit_breaker_failures.write().await;
            failures.insert(label_key, failure_count);
        }
        
        debug!(
            service = service,
            state = state,
            failure_count = failure_count,
            "Recorded circuit breaker metrics"
        );
    }

    /// Record service health
    pub async fn record_service_health(&self, service: &str, healthy: bool) {
        let label_key = format!("service={}", service);
        
        let mut service_up = self.service_up.write().await;
        service_up.insert(label_key, if healthy { 1 } else { 0 });
        
        debug!(
            service = service,
            healthy = healthy,
            "Recorded service health metrics"
        );
    }

    /// Generate Prometheus metrics format output
    pub async fn generate_prometheus_output(&self) -> String {
        let mut output = String::new();
        let ns = &self.config.namespace;
        let subsys = &self.config.subsystem;
        
        // Helper function to add help and type comments
        let add_metric_header = |output: &mut String, name: &str, help: &str, metric_type: &str| {
            output.push_str(&format!("# HELP {}_{} {}\n", ns, name, help));
            output.push_str(&format!("# TYPE {}_{} {}\n", ns, name, metric_type));
        };
        
        // Sync operations total
        add_metric_header(&mut output, &format!("{}_sync_operations_total", subsys), 
                         "Total number of sync operations", "counter");
        let sync_ops = self.sync_operations_total.read().await;
        for (labels, value) in sync_ops.iter() {
            output.push_str(&format!("{}_{}_sync_operations_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Sync operations success
        add_metric_header(&mut output, &format!("{}_sync_operations_success_total", subsys), 
                         "Total number of successful sync operations", "counter");
        let sync_success = self.sync_operations_success.read().await;
        for (labels, value) in sync_success.iter() {
            output.push_str(&format!("{}_{}_sync_operations_success_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Sync operations failed
        add_metric_header(&mut output, &format!("{}_sync_operations_failed_total", subsys), 
                         "Total number of failed sync operations", "counter");
        let sync_failed = self.sync_operations_failed.read().await;
        for (labels, value) in sync_failed.iter() {
            output.push_str(&format!("{}_{}_sync_operations_failed_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // API requests total
        add_metric_header(&mut output, &format!("{}_api_requests_total", subsys), 
                         "Total number of API requests", "counter");
        let api_requests = self.api_requests_total.read().await;
        for (labels, value) in api_requests.iter() {
            output.push_str(&format!("{}_{}_api_requests_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // API rate limit hits
        add_metric_header(&mut output, &format!("{}_api_rate_limit_hits_total", subsys), 
                         "Total number of API rate limit hits", "counter");
        let rate_limits = self.api_rate_limit_hits.read().await;
        for (labels, value) in rate_limits.iter() {
            output.push_str(&format!("{}_{}_api_rate_limit_hits_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Cache hits and misses
        add_metric_header(&mut output, &format!("{}_cache_hits_total", subsys), 
                         "Total number of cache hits", "counter");
        let cache_hits = self.cache_hits_total.read().await;
        for (labels, value) in cache_hits.iter() {
            output.push_str(&format!("{}_{}_cache_hits_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        add_metric_header(&mut output, &format!("{}_cache_misses_total", subsys), 
                         "Total number of cache misses", "counter");
        let cache_misses = self.cache_misses_total.read().await;
        for (labels, value) in cache_misses.iter() {
            output.push_str(&format!("{}_{}_cache_misses_total{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Queue depth (gauge)
        add_metric_header(&mut output, &format!("{}_queue_depth", subsys), 
                         "Current queue depth", "gauge");
        let queue_depths = self.queue_depth.read().await;
        for (labels, value) in queue_depths.iter() {
            output.push_str(&format!("{}_{}_queue_depth{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Service health (gauge)
        add_metric_header(&mut output, &format!("{}_service_up", subsys), 
                         "Service health status (1=up, 0=down)", "gauge");
        let service_health = self.service_up.read().await;
        for (labels, value) in service_health.iter() {
            output.push_str(&format!("{}_{}_service_up{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Circuit breaker failures
        add_metric_header(&mut output, &format!("{}_circuit_breaker_failures", subsys), 
                         "Current circuit breaker failure count", "gauge");
        let cb_failures = self.circuit_breaker_failures.read().await;
        for (labels, value) in cb_failures.iter() {
            output.push_str(&format!("{}_{}_circuit_breaker_failures{{{}}} {}\n", 
                                   ns, subsys, labels, value));
        }
        
        // Last successful sync timestamp
        add_metric_header(&mut output, &format!("{}_last_successful_sync_timestamp", subsys), 
                         "Timestamp of last successful sync", "gauge");
        let last_syncs = self.last_successful_sync.read().await;
        for (labels, timestamp) in last_syncs.iter() {
            output.push_str(&format!("{}_{}_last_successful_sync_timestamp{{{}}} {}\n", 
                                   ns, subsys, labels, timestamp.timestamp()));
        }
        
        output
    }

    /// Get current metrics summary
    pub async fn get_metrics_summary(&self) -> SyncMetrics {
        let sync_ops_total = self.sync_operations_total.read().await;
        let sync_ops_success = self.sync_operations_success.read().await;
        let sync_ops_failed = self.sync_operations_failed.read().await;
        let api_requests = self.api_requests_total.read().await;
        let rate_limits = self.api_rate_limit_hits.read().await;
        let cache_hits = self.cache_hits_total.read().await;
        let cache_misses = self.cache_misses_total.read().await;
        
        SyncMetrics {
            total_sync_operations: sync_ops_total.values().sum(),
            successful_sync_operations: sync_ops_success.values().sum(),
            failed_sync_operations: sync_ops_failed.values().sum(),
            total_api_requests: api_requests.values().sum(),
            rate_limit_hits: rate_limits.values().sum(),
            cache_hits: cache_hits.values().sum(),
            cache_misses: cache_misses.values().sum(),
            cache_hit_rate: {
                let hits = cache_hits.values().sum::<u64>() as f64;
                let total_cache_requests = hits + cache_misses.values().sum::<u64>() as f64;
                if total_cache_requests > 0.0 {
                    hits / total_cache_requests
                } else {
                    0.0
                }
            },
        }
    }
}

/// Summary of sync metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetrics {
    pub total_sync_operations: u64,
    pub successful_sync_operations: u64,
    pub failed_sync_operations: u64,
    pub total_api_requests: u64,
    pub rate_limit_hits: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
}

/// Service-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    pub service_name: String,
    pub requests_total: u64,
    pub requests_failed: u64,
    pub rate_limit_hits: u64,
    pub avg_response_time_ms: f64,
    pub circuit_breaker_state: String,
    pub is_healthy: bool,
    pub last_success: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_record_sync_operation() {
        let config = MetricsConfig::default();
        let metrics = PrometheusMetrics::new(config);
        
        // Record successful operation
        metrics.record_sync_operation("imdb", true, Duration::from_millis(1500)).await;
        
        // Record failed operation
        metrics.record_sync_operation("imdb", false, Duration::from_millis(500)).await;
        
        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.total_sync_operations, 2);
        assert_eq!(summary.successful_sync_operations, 1);
        assert_eq!(summary.failed_sync_operations, 1);
    }
    
    #[tokio::test]
    async fn test_record_api_request() {
        let config = MetricsConfig::default();
        let metrics = PrometheusMetrics::new(config);
        
        // Record normal request
        metrics.record_api_request("tmdb", Duration::from_millis(200), false).await;
        
        // Record rate-limited request
        metrics.record_api_request("tmdb", Duration::from_millis(100), true).await;
        
        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.total_api_requests, 2);
        assert_eq!(summary.rate_limit_hits, 1);
    }
    
    #[tokio::test]
    async fn test_cache_metrics() {
        let config = MetricsConfig::default();
        let metrics = PrometheusMetrics::new(config);
        
        // Record cache hits and misses
        metrics.record_cache_access("movie_cache", true).await;
        metrics.record_cache_access("movie_cache", true).await;
        metrics.record_cache_access("movie_cache", false).await;
        
        let summary = metrics.get_metrics_summary().await;
        assert_eq!(summary.cache_hits, 2);
        assert_eq!(summary.cache_misses, 1);
        assert!((summary.cache_hit_rate - 0.666_666_666_666_666_7).abs() < f64::EPSILON);
    }
    
    #[tokio::test]
    async fn test_prometheus_output_generation() {
        let config = MetricsConfig::default();
        let metrics = PrometheusMetrics::new(config);
        
        // Record some metrics
        metrics.record_sync_operation("imdb", true, Duration::from_millis(1000)).await;
        metrics.record_api_request("tmdb", Duration::from_millis(200), false).await;
        
        let output = metrics.generate_prometheus_output().await;
        
        // Check that output contains expected metric names
        assert!(output.contains("radarr_list_sync_sync_operations_total"));
        assert!(output.contains("radarr_list_sync_api_requests_total"));
        assert!(output.contains("# HELP"));
        assert!(output.contains("# TYPE"));
    }
}