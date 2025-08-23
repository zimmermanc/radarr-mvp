# List Sync Monitoring System

This module provides comprehensive monitoring infrastructure for the List Sync system, including:

- **Prometheus Metrics**: Track sync performance, API usage, cache hit rates
- **Alerting System**: Rule-based alerting for failures and performance issues
- **Health Checks**: Monitor external service availability (IMDb, TMDb, Trakt, Plex)
- **Circuit Breakers**: Prevent cascading failures with automatic service protection

## Quick Start

```rust
use radarr_infrastructure::monitoring::*;
use std::time::Duration;

// Initialize monitoring system
let config = ListSyncMonitorConfig::default();
let mut monitor = ListSyncMonitor::new(config).await?;

// Setup health checkers for external services
monitor.setup_default_health_checkers().await?;

// Start monitoring components
monitor.start_monitoring().await?;

// Record sync operations
monitor.record_sync_operation(
    "imdb",      // source
    true,        // success
    Duration::from_millis(1500), // duration
    5,           // items_added
    2,           // items_updated
    10,          // items_total
).await;

// Use circuit breaker protection
let result = monitor.execute_with_circuit_breaker("tmdb", async {
    // Your API call here
    Ok::<String, radarr_core::RadarrError>("success".to_string())
}).await?;

// Get Prometheus metrics
let metrics = monitor.get_prometheus_metrics().await;
println!("{}", metrics);

// Get monitoring status
let status = monitor.get_monitoring_status().await;
println!("Active alerts: {}", status.active_critical_alerts);
println!("Healthy services: {}", status.health_summary.healthy_services);
```

## Architecture

### Components

1. **ListSyncMonitor**: Main coordinator orchestrating all monitoring components
2. **PrometheusMetrics**: Collects and exports metrics in Prometheus format
3. **AlertManager**: Evaluates alert rules and manages alert lifecycle
4. **HealthChecker**: Monitors external service health with HTTP checks
5. **Circuit Breakers**: Integrated with existing circuit breaker system

### Metrics Collected

#### Sync Performance
- `radarr_list_sync_sync_operations_total{source="imdb"}` - Total sync operations
- `radarr_list_sync_sync_operations_success_total{source="imdb"}` - Successful syncs
- `radarr_list_sync_sync_operations_failed_total{source="imdb"}` - Failed syncs
- `radarr_list_sync_sync_duration_seconds{source="imdb"}` - Sync duration histogram

#### API Performance  
- `radarr_list_sync_api_requests_total{service="tmdb"}` - Total API requests
- `radarr_list_sync_api_rate_limit_hits_total{service="tmdb"}` - Rate limit hits
- `radarr_list_sync_api_request_duration_seconds{service="tmdb"}` - Request duration

#### Cache Metrics
- `radarr_list_sync_cache_hits_total{type="movie_cache"}` - Cache hits
- `radarr_list_sync_cache_misses_total{type="movie_cache"}` - Cache misses

#### Service Health
- `radarr_list_sync_service_up{service="imdb"}` - Service health (1=up, 0=down)
- `radarr_list_sync_circuit_breaker_failures{service="imdb"}` - Circuit breaker failures
- `radarr_list_sync_last_successful_sync_timestamp{source="imdb"}` - Last success

### Default Alert Rules

The system includes pre-configured alert rules for common scenarios:

#### Consecutive Sync Failures
- **Threshold**: 3 consecutive failures
- **Level**: Warning
- **Rate Limit**: 30 minutes

#### Slow Sync Operations
- **Threshold**: 5 minutes
- **Level**: Warning  
- **Auto-resolve**: Yes (30 minutes)

#### High Rate Limit Hits
- **Threshold**: 10 hits per hour
- **Level**: Critical
- **Rate Limit**: 2 hours

#### Service Down
- **Detection**: Health check failures
- **Level**: Critical
- **Auto-resolve**: When service recovers

#### Circuit Breaker Open
- **Detection**: Circuit breaker state change
- **Level**: Warning
- **Auto-resolve**: When circuit closes

## Configuration

### ListSyncMonitorConfig

```rust
let config = ListSyncMonitorConfig {
    metrics: MetricsConfig {
        namespace: "radarr".to_string(),
        subsystem: "list_sync".to_string(),
        enable_timing_histograms: true,
        ..Default::default()
    },
    health_checks: HealthCheckConfig {
        check_interval: Duration::from_secs(300), // 5 minutes
        request_timeout: Duration::from_secs(30),
        failure_threshold: 3,
        recovery_threshold: 2,
        ..Default::default()
    },
    enable_default_alerts: true,
    alert_evaluation_interval: Duration::from_secs(60),
    alert_retention_days: 30,
    circuit_breaker_configs: HashMap::new(), // Uses defaults
};
```

### Circuit Breaker Defaults

- **IMDb**: 5 failures, 60s timeout, 30s request timeout
- **TMDb**: 3 failures, 30s timeout, 10s request timeout  
- **Trakt**: 3 failures, 45s timeout, 15s request timeout
- **Plex**: 2 failures, 30s timeout, 20s request timeout

## Integration with List Sync Jobs

```rust
use radarr_core::jobs::list_sync::{ListSyncScheduler, SyncHandler};
use std::sync::Arc;

struct MonitoredSyncHandler {
    monitor: Arc<ListSyncMonitor>,
    // ... other fields
}

#[async_trait::async_trait]
impl SyncHandler for MonitoredSyncHandler {
    async fn execute_sync(&self, job: &SyncJob) -> Result<SyncResult, SyncError> {
        let start = std::time::Instant::now();
        
        // Execute sync with circuit breaker protection
        let result = self.monitor.execute_with_circuit_breaker(
            &job.source_type,
            async {
                // Your sync logic here
                self.perform_sync(job).await
            }
        ).await;
        
        let duration = start.elapsed();
        
        // Record metrics
        match &result {
            Ok(sync_result) => {
                self.monitor.record_sync_operation(
                    &job.source_type,
                    true, // success
                    duration,
                    sync_result.items_added as u64,
                    sync_result.items_updated as u64,
                    sync_result.items_found as u64,
                ).await;
            }
            Err(_) => {
                self.monitor.record_sync_operation(
                    &job.source_type,
                    false, // failure
                    duration,
                    0, 0, 0,
                ).await;
            }
        }
        
        result
    }
}
```

## Prometheus Integration

### Scrape Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'radarr-list-sync'
    static_configs:
      - targets: ['localhost:7878']
    metrics_path: '/metrics'
    scrape_interval: 30s
```

### Example Grafana Queries

#### Sync Success Rate
```promql
rate(radarr_list_sync_sync_operations_success_total[5m]) / 
rate(radarr_list_sync_sync_operations_total[5m])
```

#### Average Sync Duration
```promql
rate(radarr_list_sync_sync_duration_seconds_sum[5m]) / 
rate(radarr_list_sync_sync_duration_seconds_count[5m])
```

#### Cache Hit Rate
```promql
rate(radarr_list_sync_cache_hits_total[5m]) / 
(rate(radarr_list_sync_cache_hits_total[5m]) + rate(radarr_list_sync_cache_misses_total[5m]))
```

#### Service Health Overview
```promql
radarr_list_sync_service_up
```

## API Endpoints

The monitoring system integrates with the existing API to provide monitoring endpoints:

- `GET /metrics` - Prometheus metrics (handled by ListSyncMonitor)
- `GET /api/v3/monitoring/status` - Comprehensive monitoring status
- `GET /api/v3/monitoring/alerts` - Active alerts
- `GET /api/v3/monitoring/health` - Service health summary
- `POST /api/v3/monitoring/alerts/{id}/acknowledge` - Acknowledge alert
- `POST /api/v3/monitoring/health/{service}/check` - Force health check

## Alerting Integration

### Custom Alert Rules

```rust
use radarr_infrastructure::monitoring::alert_manager::*;

let rule = AlertRule {
    name: "custom_rule".to_string(),
    level: AlertLevel::Warning,
    description: "Custom alert for specific condition".to_string(),
    labels: HashMap::from([("component".to_string(), "custom".to_string())]),
    condition: AlertCondition::ConsecutiveFailures {
        service: "my_service".to_string(),
        count: 5,
    },
    threshold: 5.0,
    evaluation_window: Duration::from_minutes(10),
    rate_limit: Some(Duration::from_minutes(30)),
    auto_resolve: false,
    auto_resolve_after: None,
    enabled: true,
};

monitor.alert_manager.add_rule(rule).await?;
```

### Notification Handlers

```rust
use radarr_infrastructure::monitoring::alert_manager::*;

struct DiscordNotificationHandler {
    webhook_url: String,
}

#[async_trait::async_trait]
impl AlertNotificationHandler for DiscordNotificationHandler {
    async fn send_notification(&self, alert: &Alert) -> Result<(), AlertError> {
        // Send Discord webhook notification
        let payload = format!(
            "🚨 **{}** - {}\n{}",
            alert.level.as_str().to_uppercase(),
            alert.title,
            alert.description
        );
        
        // HTTP request to Discord webhook...
        Ok(())
    }
    
    fn name(&self) -> &str {
        "discord"
    }
}
```

## Performance Considerations

- **Metrics Collection**: Uses lock-free atomic counters where possible
- **Health Checks**: Configurable intervals, parallel execution
- **Alert Evaluation**: Runs on separate background task
- **Memory Usage**: Automatic cleanup of old alerts and metrics
- **Circuit Breakers**: Shared instances, minimal overhead

## Troubleshooting

### High Memory Usage
- Reduce `alert_retention_days` in configuration
- Disable `enable_timing_histograms` if not needed
- Increase health check intervals

### Missing Metrics
- Check that monitoring was started with `start_monitoring()`
- Verify circuit breaker configurations
- Check logs for initialization errors

### Alert Spam
- Adjust `rate_limit` settings in alert rules
- Increase thresholds for noisy conditions
- Use alert acknowledgment to suppress known issues

This monitoring system provides production-ready observability for the List Sync system with minimal performance impact and comprehensive coverage of failure scenarios.