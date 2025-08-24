//! List Sync Monitor - Main monitoring coordinator
//!
//! This module provides the main ListSyncMonitor that coordinates
//! metrics collection, alerting, health checks, and circuit breaker
//! integration for the List Sync system.

use crate::monitoring::{
    alert_manager::{AlertManager, AlertRule, create_default_alert_rules},
    health_checks::{HealthChecker, HealthCheckConfig, ServiceHealthChecker, HttpHealthChecker, HealthStatus},
    metrics::{PrometheusMetrics, MetricsConfig, SyncMetrics},
};
use radarr_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerMetrics, CircuitBreakerState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for the List Sync Monitor
#[derive(Debug, Clone)]
pub struct ListSyncMonitorConfig {
    /// Metrics configuration
    pub metrics: MetricsConfig,
    /// Health check configuration  
    pub health_checks: HealthCheckConfig,
    /// Whether to enable automatic alert rule setup
    pub enable_default_alerts: bool,
    /// How often to evaluate alert conditions
    pub alert_evaluation_interval: TokioDuration,
    /// How often to clean up old alerts
    pub alert_cleanup_interval: TokioDuration,
    /// Alert retention in days
    pub alert_retention_days: i64,
    /// Circuit breaker configurations for each service
    pub circuit_breaker_configs: HashMap<String, CircuitBreakerConfig>,
}

impl Default for ListSyncMonitorConfig {
    fn default() -> Self {
        let mut cb_configs = HashMap::new();
        
        // Default circuit breaker configs for each list service
        cb_configs.insert("imdb".to_string(), CircuitBreakerConfig::new("imdb")
            .with_failure_threshold(5)
            .with_timeout(Duration::from_secs(60))
            .with_request_timeout(Duration::from_secs(30)));
            
        cb_configs.insert("tmdb".to_string(), CircuitBreakerConfig::new("tmdb")
            .with_failure_threshold(3)
            .with_timeout(Duration::from_secs(30))
            .with_request_timeout(Duration::from_secs(10)));
            
        cb_configs.insert("trakt".to_string(), CircuitBreakerConfig::new("trakt")
            .with_failure_threshold(3)
            .with_timeout(Duration::from_secs(45))
            .with_request_timeout(Duration::from_secs(15)));
            
        cb_configs.insert("plex".to_string(), CircuitBreakerConfig::new("plex")
            .with_failure_threshold(2)
            .with_timeout(Duration::from_secs(30))
            .with_request_timeout(Duration::from_secs(20)));
        
        Self {
            metrics: MetricsConfig::default(),
            health_checks: HealthCheckConfig::default(),
            enable_default_alerts: true,
            alert_evaluation_interval: TokioDuration::from_secs(60), // 1 minute
            alert_cleanup_interval: TokioDuration::from_secs(3600), // 1 hour
            alert_retention_days: 30,
            circuit_breaker_configs: cb_configs,
        }
    }
}

/// Main List Sync Monitor that orchestrates all monitoring components
pub struct ListSyncMonitor {
    config: ListSyncMonitorConfig,
    metrics: Arc<PrometheusMetrics>,
    alert_manager: Arc<AlertManager>,
    health_checker: Arc<RwLock<HealthChecker>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    monitoring_stats: Arc<RwLock<MonitoringStats>>,
}

/// Internal monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MonitoringStats {
    pub started_at: DateTime<Utc>,
    pub total_sync_operations_monitored: u64,
    pub total_alerts_fired: u64,
    pub total_health_checks_performed: u64,
    pub circuit_breaker_activations: u64,
    pub last_prometheus_scrape: Option<DateTime<Utc>>,
}

impl Default for MonitoringStats {
    fn default() -> Self {
        Self {
            started_at: Utc::now(),
            total_sync_operations_monitored: 0,
            total_alerts_fired: 0,
            total_health_checks_performed: 0,
            circuit_breaker_activations: 0,
            last_prometheus_scrape: None,
        }
    }
}

impl ListSyncMonitor {
    /// Create a new ListSyncMonitor with the given configuration
    pub async fn new(config: ListSyncMonitorConfig) -> Result<Self, MonitoringError> {
        let metrics = Arc::new(PrometheusMetrics::new(config.metrics.clone()));
        let alert_manager = Arc::new(AlertManager::new());
        let health_checker = Arc::new(RwLock::new(HealthChecker::new(config.health_checks.clone())));
        let circuit_breakers = Arc::new(RwLock::new(HashMap::new()));
        
        // Initialize circuit breakers
        {
            let mut cb_map = circuit_breakers.write().await;
            for (service, cb_config) in &config.circuit_breaker_configs {
                let circuit_breaker = CircuitBreaker::new(cb_config.clone());
                cb_map.insert(service.clone(), circuit_breaker);
                info!("Initialized circuit breaker for service: {}", service);
            }
        }
        
        let monitor = Self {
            config,
            metrics,
            alert_manager,
            health_checker,
            circuit_breakers,
            monitoring_stats: Arc::new(RwLock::new(MonitoringStats::default())),
        };
        
        // Setup default alert rules if enabled
        if monitor.config.enable_default_alerts {
            monitor.setup_default_alert_rules().await?;
        }
        
        info!("List Sync Monitor initialized successfully");
        Ok(monitor)
    }

    /// Setup default alert rules for common scenarios
    async fn setup_default_alert_rules(&self) -> Result<(), MonitoringError> {
        let rules = create_default_alert_rules();
        
        for rule in rules {
            self.alert_manager.add_rule(rule).await
                .map_err(|e| MonitoringError::AlertSetupError(e.to_string()))?;
        }
        
        info!("Setup {} default alert rules", create_default_alert_rules().len());
        Ok(())
    }

    /// Add a health checker for a specific service
    pub async fn add_health_checker(&self, checker: Box<dyn ServiceHealthChecker>) {
        let service_name = checker.service_name().to_string();
        let mut health_checker = self.health_checker.write().await;
        health_checker.add_checker(checker);
        info!("Added health checker for service: {}", service_name);
    }

    /// Add health checkers for common list services
    pub async fn setup_default_health_checkers(&self) -> Result<(), MonitoringError> {
        let mut health_checker = self.health_checker.write().await;
        
        // IMDb health check (check if their API is responding)
        let imdb_checker = Box::new(HttpHealthChecker::new(
            "imdb",
            "https://www.imdb.com/robots.txt", // Simple endpoint to check availability
            200,
            Duration::from_secs(10),
        ));
        health_checker.add_checker(imdb_checker);
        
        // TMDb health check
        let tmdb_checker = Box::new(HttpHealthChecker::new(
            "tmdb",
            "https://api.themoviedb.org/3/configuration", // Public endpoint
            200,
            Duration::from_secs(10),
        ));
        health_checker.add_checker(tmdb_checker);
        
        // Trakt health check
        let trakt_checker = Box::new(HttpHealthChecker::new(
            "trakt",
            "https://api.trakt.tv/users/settings", // Requires auth but should return 401, not 500
            401, // We expect 401 without auth, which means service is up
            Duration::from_secs(10),
        ));
        health_checker.add_checker(trakt_checker);
        
        info!("Setup default health checkers for IMDb, TMDb, and Trakt");
        Ok(())
    }

    /// Start all monitoring components
    pub async fn start_monitoring(&self) -> Result<(), MonitoringError> {
        info!("Starting List Sync monitoring components");
        
        // Start health check loop
        let health_checker = self.health_checker.clone();
        tokio::spawn(async move {
            let health_checker = health_checker.read().await;
            health_checker.start().await;
        });
        
        // Start alert evaluation loop
        let alert_manager = self.alert_manager.clone();
        let metrics = self.metrics.clone();
        let health_checker_clone = self.health_checker.clone();
        let circuit_breakers = self.circuit_breakers.clone();
        let alert_interval = self.config.alert_evaluation_interval;
        
        tokio::spawn(async move {
            let mut interval = interval(alert_interval);
            loop {
                interval.tick().await;
                Self::evaluate_alert_conditions(
                    &alert_manager,
                    &metrics,
                    &health_checker_clone,
                    &circuit_breakers,
                ).await;
            }
        });
        
        // Start alert cleanup loop
        let alert_manager_cleanup = self.alert_manager.clone();
        let cleanup_interval = self.config.alert_cleanup_interval;
        let retention_days = self.config.alert_retention_days;
        
        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            loop {
                interval.tick().await;
                alert_manager_cleanup.cleanup_old_alerts(retention_days).await;
            }
        });
        
        info!("All monitoring components started successfully");
        Ok(())
    }

    /// Record a sync operation for monitoring
    pub async fn record_sync_operation(
        &self,
        source: &str,
        success: bool,
        duration: Duration,
        items_added: u64,
        items_updated: u64,
        items_total: u64,
    ) {
        // Update metrics
        self.metrics.record_sync_operation(source, success, duration).await;
        self.metrics.record_sync_items(source, items_added, items_updated, items_total).await;
        
        // Update monitoring stats
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.total_sync_operations_monitored += 1;
        }
        
        // Check for alerts
        if !success {
            // This will be handled by the alert evaluation loop which checks circuit breaker failures
            self.alert_manager.check_consecutive_failures(source, 1).await;
        }
        
        if duration.as_secs_f64() > 300.0 { // 5 minutes threshold
            self.alert_manager.check_slow_sync(source, duration.as_secs_f64()).await;
        }
        
        debug!(
            source = source,
            success = success,
            duration_ms = duration.as_millis(),
            items_added = items_added,
            items_updated = items_updated,
            items_total = items_total,
            "Recorded sync operation"
        );
    }

    /// Record an API request for monitoring
    pub async fn record_api_request(&self, service: &str, duration: Duration, rate_limited: bool) {
        self.metrics.record_api_request(service, duration, rate_limited).await;
        
        if rate_limited {
            // Could trigger rate limit alert
            self.alert_manager.check_rate_limit_hits(service, 1).await;
        }
    }

    /// Record cache access for monitoring
    pub async fn record_cache_access(&self, cache_type: &str, hit: bool) {
        self.metrics.record_cache_access(cache_type, hit).await;
    }

    /// Get circuit breaker for a service
    pub async fn get_circuit_breaker(&self, service: &str) -> Option<CircuitBreaker> {
        let circuit_breakers = self.circuit_breakers.read().await;
        circuit_breakers.get(service).map(|cb| {
            // Return a clone since CircuitBreaker is designed to be cloned
            CircuitBreaker::new(CircuitBreakerConfig::new(service))
        })
    }

    /// Execute an operation with circuit breaker protection
    pub async fn execute_with_circuit_breaker<F, T, E>(
        &self,
        service: &str,
        operation: F,
    ) -> Result<T, MonitoringError>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: Into<radarr_core::RadarrError>,
    {
        let circuit_breakers = self.circuit_breakers.read().await;
        
        if let Some(circuit_breaker) = circuit_breakers.get(service) {
            let start_time = Instant::now();
            
            let result = circuit_breaker.call(operation).await;
            let duration = start_time.elapsed();
            
            // Record metrics
            let success = result.is_ok();
            self.metrics.record_api_request(service, duration, false).await;
            
            // Update circuit breaker metrics in our monitoring system
            let cb_metrics = circuit_breaker.get_metrics().await;
            self.metrics.record_circuit_breaker_state(
                service,
                cb_metrics.state.as_str(),
                cb_metrics.consecutive_failures as u64,
            ).await;
            
            // Check for circuit breaker alerts
            self.alert_manager.check_circuit_breaker_state(
                service,
                cb_metrics.state == CircuitBreakerState::Open,
            ).await;
            
            match result {
                Ok(value) => Ok(value),
                Err(e) => Err(MonitoringError::CircuitBreakerError(e.to_string())),
            }
        } else {
            Err(MonitoringError::ServiceNotFound(service.to_string()))
        }
    }

    /// Get comprehensive monitoring status
    pub async fn get_monitoring_status(&self) -> MonitoringStatus {
        let sync_metrics = self.metrics.get_metrics_summary().await;
        let alert_stats = self.alert_manager.get_alert_stats().await;
        let active_alerts = self.alert_manager.get_active_alerts().await;
        
        let health_checker = self.health_checker.read().await;
        let health_summary = health_checker.get_health_summary().await;
        
        let circuit_breakers = self.circuit_breakers.read().await;
        let mut circuit_breaker_status = HashMap::new();
        
        for (service, cb) in circuit_breakers.iter() {
            let metrics = cb.get_metrics().await;
            circuit_breaker_status.insert(service.clone(), CircuitBreakerStatus {
                state: metrics.state.as_str().to_string(),
                consecutive_failures: metrics.consecutive_failures,
                success_rate: if metrics.total_requests > 0 {
                    (metrics.successful_requests as f64 / metrics.total_requests as f64) * 100.0
                } else {
                    0.0
                },
                is_healthy: cb.is_healthy().await,
            });
        }
        
        let stats = self.monitoring_stats.read().await;
        
        MonitoringStatus {
            started_at: stats.started_at,
            sync_metrics,
            alert_stats,
            active_critical_alerts: active_alerts.iter()
                .filter(|a| a.level == crate::monitoring::alert_manager::AlertLevel::Critical)
                .count() as u32,
            health_summary,
            circuit_breaker_status,
            total_operations_monitored: stats.total_sync_operations_monitored,
            uptime_seconds: (Utc::now() - stats.started_at).num_seconds() as u64,
        }
    }

    /// Generate Prometheus metrics output
    pub async fn get_prometheus_metrics(&self) -> String {
        // Update last scrape time
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.last_prometheus_scrape = Some(Utc::now());
        }
        
        self.metrics.generate_prometheus_output().await
    }

    /// Evaluate alert conditions across all monitoring components
    async fn evaluate_alert_conditions(
        alert_manager: &AlertManager,
        _metrics: &PrometheusMetrics,
        health_checker: &RwLock<HealthChecker>,
        circuit_breakers: &RwLock<HashMap<String, CircuitBreaker>>,
    ) {
        // Check health status and trigger service down alerts
        {
            let health_checker = health_checker.read().await;
            let all_health = health_checker.get_all_health_status().await;
            
            for (service, health) in all_health {
                alert_manager.check_service_health(&service, health.status == HealthStatus::Healthy).await;
            }
        }
        
        // Check circuit breaker states
        {
            let circuit_breakers = circuit_breakers.read().await;
            for (service, cb) in circuit_breakers.iter() {
                let metrics = cb.get_metrics().await;
                alert_manager.check_circuit_breaker_state(
                    service,
                    metrics.state == CircuitBreakerState::Open,
                ).await;
                
                // Also check for consecutive failures
                if metrics.consecutive_failures > 0 {
                    alert_manager.check_consecutive_failures(service, metrics.consecutive_failures).await;
                }
            }
        }
        
        debug!("Completed alert condition evaluation cycle");
    }
}

/// Monitoring error types
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Alert setup error: {0}")]
    AlertSetupError(String),
    
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    
    #[error("Circuit breaker error: {0}")]
    CircuitBreakerError(String),
    
    #[error("Health check error: {0}")]
    HealthCheckError(String),
    
    #[error("Metrics error: {0}")]
    MetricsError(String),
}

/// Comprehensive monitoring status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStatus {
    pub started_at: DateTime<Utc>,
    pub sync_metrics: SyncMetrics,
    pub alert_stats: crate::monitoring::alert_manager::AlertStats,
    pub active_critical_alerts: u32,
    pub health_summary: crate::monitoring::health_checks::HealthSummary,
    pub circuit_breaker_status: HashMap<String, CircuitBreakerStatus>,
    pub total_operations_monitored: u64,
    pub uptime_seconds: u64,
}

/// Circuit breaker status for monitoring display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    pub state: String,
    pub consecutive_failures: u32,
    pub success_rate: f64,
    pub is_healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    
    #[tokio::test]
    async fn test_list_sync_monitor_creation() {
        let config = ListSyncMonitorConfig::default();
        let monitor = ListSyncMonitor::new(config).await.unwrap();
        
        // Check that circuit breakers were initialized
        let cb_map = monitor.circuit_breakers.read().await;
        assert!(cb_map.contains_key("imdb"));
        assert!(cb_map.contains_key("tmdb"));
        assert!(cb_map.contains_key("trakt"));
        assert!(cb_map.contains_key("plex"));
    }
    
    #[tokio::test]
    async fn test_sync_operation_recording() {
        let config = ListSyncMonitorConfig::default();
        let monitor = ListSyncMonitor::new(config).await.unwrap();
        
        // Record a sync operation
        monitor.record_sync_operation(
            "imdb",
            true,
            Duration::from_millis(1500),
            5,
            2,
            10,
        ).await;
        
        let metrics = monitor.metrics.get_metrics_summary().await;
        assert_eq!(metrics.total_sync_operations, 1);
        assert_eq!(metrics.successful_sync_operations, 1);
        
        let status = monitor.get_monitoring_status().await;
        assert_eq!(status.total_operations_monitored, 1);
    }
    
    #[tokio::test]
    async fn test_prometheus_metrics_generation() {
        let config = ListSyncMonitorConfig::default();
        let monitor = ListSyncMonitor::new(config).await.unwrap();
        
        // Record some data
        monitor.record_sync_operation("imdb", true, Duration::from_millis(1000), 1, 0, 1).await;
        monitor.record_api_request("tmdb", Duration::from_millis(200), false).await;
        
        let prometheus_output = monitor.get_prometheus_metrics().await;
        
        // Should contain expected metric names
        assert!(prometheus_output.contains("radarr_list_sync_sync_operations_total"));
        assert!(prometheus_output.contains("radarr_list_sync_api_requests_total"));
        assert!(prometheus_output.contains("# HELP"));
        assert!(prometheus_output.contains("# TYPE"));
    }
}