//! Monitoring API handlers
//!
//! This module contains handlers for exposing monitoring data via REST API endpoints.
//! These handlers provide placeholder implementations for monitoring data and can be
//! enhanced to integrate with the ListSyncMonitor system when it's wired into the main application.

use crate::{
    error::{ApiError, ApiResult},
    models::PaginationParams,
};
use axum::{
    extract::{Query, Path, Extension},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use radarr_infrastructure::monitoring::list_sync_monitor::{ListSyncMonitor, MonitoringStatus};
use tracing::{debug, warn};

/// Response model for monitoring status
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringStatusResponse {
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub total_operations_monitored: u64,
    pub sync_metrics: SyncMetricsResponse,
    pub alert_stats: AlertStatsResponse,
    pub active_critical_alerts: u32,
    pub health_summary: HealthSummaryResponse,
    pub circuit_breaker_status: HashMap<String, CircuitBreakerStatusResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMetricsResponse {
    pub total_sync_operations: u64,
    pub successful_sync_operations: u64,
    pub failed_sync_operations: u64,
    pub average_sync_duration_ms: f64,
    pub items_processed_total: u64,
    pub cache_hit_rate: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertStatsResponse {
    pub total_alerts: u32,
    pub active_alerts: u32,
    pub critical_alerts: u32,
    pub warning_alerts: u32,
    pub resolved_alerts: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSummaryResponse {
    pub overall_status: String,
    pub healthy_services: u32,
    pub unhealthy_services: u32,
    pub unknown_services: u32,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitBreakerStatusResponse {
    pub state: String,
    pub consecutive_failures: u32,
    pub success_rate: f64,
    pub is_healthy: bool,
}

/// Response model for alerts
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertResponse {
    pub id: String,
    pub rule_name: String,
    pub level: String,
    pub status: String,
    pub title: String,
    pub description: String,
    pub service: String,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub fire_count: u32,
    pub last_fired: DateTime<Utc>,
}

/// Response model for service health
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthResponse {
    pub name: String,
    pub status: String,
    pub response_time_ms: Option<u64>,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
    pub is_healthy: bool,
}

/// Query parameters for alert filtering
#[derive(Debug, Deserialize)]
pub struct AlertFilterParams {
    /// Filter by alert severity
    pub severity: Option<String>,
    /// Filter by service name
    pub service: Option<String>,
    /// Filter by status (active, resolved, acknowledged, suppressed)
    pub status: Option<String>,
    /// Include resolved alerts (default: false for active only)
    #[serde(default)]
    pub include_resolved: bool,
}

/// GET /metrics - Prometheus metrics endpoint
pub async fn get_prometheus_metrics(
    monitor: Option<Extension<Arc<ListSyncMonitor>>>,
) -> impl IntoResponse {
    debug!("Serving Prometheus metrics");
    
    let metrics = if let Some(Extension(monitor)) = monitor {
        // Get real metrics from the monitor
        monitor.get_prometheus_metrics().await
    } else {
        // Fallback to placeholder if monitor not available
        warn!("ListSyncMonitor not available, using placeholder metrics");
        generate_placeholder_metrics()
    };
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(metrics)
        .unwrap()
}

/// GET /api/v3/monitoring/status - Comprehensive monitoring status
pub async fn get_monitoring_status(
    monitor: Option<Extension<Arc<ListSyncMonitor>>>,
) -> ApiResult<Json<MonitoringStatusResponse>> {
    debug!("Fetching comprehensive monitoring status");
    
    let response = if let Some(Extension(monitor)) = monitor {
        // Get real status from the monitor
        let status = monitor.get_monitoring_status().await;
        convert_monitoring_status_to_response(status)
    } else {
        // Fallback to placeholder if monitor not available
        warn!("ListSyncMonitor not available, using placeholder status");
        generate_placeholder_status()
    };
    
    Ok(Json(response))
}

// ========================================
// ListSyncMonitor Integration
// ========================================

/// Convert the internal MonitoringStatus to our API response format
fn convert_monitoring_status_to_response(status: MonitoringStatus) -> MonitoringStatusResponse {
    MonitoringStatusResponse {
        status: "active".to_string(), // Monitor is active if we got a response
        started_at: status.started_at,
        uptime_seconds: status.uptime_seconds,
        total_operations_monitored: status.total_operations_monitored,
        sync_metrics: SyncMetricsResponse {
            total_sync_operations: status.sync_metrics.total_sync_operations,
            successful_sync_operations: status.sync_metrics.successful_sync_operations,
            failed_sync_operations: status.sync_metrics.failed_sync_operations,
            average_sync_duration_ms: 0.0, // TODO: Calculate from actual data
            items_processed_total: 0, // TODO: Calculate from actual data
            cache_hit_rate: status.sync_metrics.cache_hit_rate,
        },
        alert_stats: AlertStatsResponse {
            total_alerts: status.alert_stats.total_active + status.alert_stats.total_resolved_today,
            active_alerts: status.alert_stats.total_active,
            critical_alerts: status.alert_stats.active_critical,
            warning_alerts: status.alert_stats.active_warning,
            resolved_alerts: status.alert_stats.total_resolved_today,
        },
        active_critical_alerts: status.active_critical_alerts,
        health_summary: HealthSummaryResponse {
            overall_status: if status.health_summary.unhealthy_services == 0 { "healthy".to_string() } else { "degraded".to_string() },
            healthy_services: status.health_summary.healthy_services,
            unhealthy_services: status.health_summary.unhealthy_services,
            unknown_services: status.health_summary.unknown_services,
            last_check: Utc::now(), // TODO: Get actual last check time
        },
        circuit_breaker_status: status.circuit_breaker_status
            .into_iter()
            .map(|(k, v)| {
                (k, CircuitBreakerStatusResponse {
                    state: v.state,
                    consecutive_failures: v.consecutive_failures,
                    success_rate: v.success_rate,
                    is_healthy: v.is_healthy,
                })
            })
            .collect(),
    }
}

// ========================================
// Placeholder implementations
// ========================================
// NOTE: These are fallbacks when ListSyncMonitor is not available

/// Generate placeholder Prometheus metrics
fn generate_placeholder_metrics() -> String {
    format!(
        r#"# HELP radarr_list_sync_sync_operations_total Total number of sync operations
# TYPE radarr_list_sync_sync_operations_total counter
radarr_list_sync_sync_operations_total{{source="placeholder"}} 0

# HELP radarr_list_sync_api_requests_total Total number of API requests
# TYPE radarr_list_sync_api_requests_total counter
radarr_list_sync_api_requests_total{{service="placeholder"}} 0

# HELP radarr_list_sync_cache_hits_total Total number of cache hits
# TYPE radarr_list_sync_cache_hits_total counter
radarr_list_sync_cache_hits_total{{cache_type="placeholder"}} 0

# HELP radarr_monitoring_status System monitoring status
# TYPE radarr_monitoring_status gauge
radarr_monitoring_status{{status="active"}} 1

# Generated at {}
"#,
        Utc::now().to_rfc3339()
    )
}

/// Generate placeholder monitoring status
fn generate_placeholder_status() -> MonitoringStatusResponse {
    let now = Utc::now();
    MonitoringStatusResponse {
        status: "healthy".to_string(),
        started_at: now - chrono::Duration::hours(1), // Mock: started 1 hour ago
        uptime_seconds: 3600,
        total_operations_monitored: 0,
        sync_metrics: SyncMetricsResponse {
            total_sync_operations: 0,
            successful_sync_operations: 0,
            failed_sync_operations: 0,
            average_sync_duration_ms: 0.0,
            items_processed_total: 0,
            cache_hit_rate: 0.0,
        },
        alert_stats: AlertStatsResponse {
            total_alerts: 0,
            active_alerts: 0,
            critical_alerts: 0,
            warning_alerts: 0,
            resolved_alerts: 0,
        },
        active_critical_alerts: 0,
        health_summary: HealthSummaryResponse {
            overall_status: "healthy".to_string(),
            healthy_services: 4,
            unhealthy_services: 0,
            unknown_services: 0,
            last_check: now,
        },
        circuit_breaker_status: generate_placeholder_circuit_breakers(),
    }
}

/// Generate placeholder alerts
fn generate_placeholder_alerts(_filter: &AlertFilterParams, _pagination: &PaginationParams) -> Vec<AlertResponse> {
    // Return empty for now - real implementation would query ListSyncMonitor
    Vec::new()
}

/// Generate placeholder health data
fn generate_placeholder_health() -> (serde_json::Value, Vec<ServiceHealthResponse>) {
    let now = Utc::now();
    
    let health_summary = json!({
        "status": "healthy",
        "healthyServices": 4,
        "unhealthyServices": 0,
        "unknownServices": 0,
        "lastCheck": now,
    });

    let services = vec![
        ServiceHealthResponse {
            name: "TMDB".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(150),
            last_check: now,
            error: None,
            is_healthy: true,
        },
        ServiceHealthResponse {
            name: "IMDB".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(200),
            last_check: now,
            error: None,
            is_healthy: true,
        },
        ServiceHealthResponse {
            name: "Trakt".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(300),
            last_check: now,
            error: None,
            is_healthy: true,
        },
        ServiceHealthResponse {
            name: "Database".to_string(),
            status: "healthy".to_string(),
            response_time_ms: Some(50),
            last_check: now,
            error: None,
            is_healthy: true,
        },
    ];

    (health_summary, services)
}

/// Generate placeholder circuit breaker states
fn generate_placeholder_circuit_breakers() -> HashMap<String, CircuitBreakerStatusResponse> {
    let mut circuit_breakers = HashMap::new();
    
    circuit_breakers.insert("tmdb".to_string(), CircuitBreakerStatusResponse {
        state: "closed".to_string(),
        consecutive_failures: 0,
        success_rate: 99.5,
        is_healthy: true,
    });
    
    circuit_breakers.insert("imdb".to_string(), CircuitBreakerStatusResponse {
        state: "closed".to_string(),
        consecutive_failures: 0,
        success_rate: 98.2,
        is_healthy: true,
    });
    
    circuit_breakers.insert("trakt".to_string(), CircuitBreakerStatusResponse {
        state: "closed".to_string(),
        consecutive_failures: 0,
        success_rate: 97.8,
        is_healthy: true,
    });

    circuit_breakers
}

/// Generate placeholder alert by ID
fn generate_placeholder_alert_by_id(_alert_id: &str) -> Option<AlertResponse> {
    // Return None for now - real implementation would query ListSyncMonitor
    None
}

// ========================================
// Additional Handler Functions
// ========================================

/// GET /api/v3/monitoring/alerts - List alerts with filtering
pub async fn get_alerts(
    Query(filter): Query<AlertFilterParams>,
    Query(pagination): Query<PaginationParams>,
    monitor: Option<Extension<Arc<ListSyncMonitor>>>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Fetching alerts with filter: {:?}", filter);
    
    let alerts = if let Some(Extension(monitor)) = monitor {
        // TODO: Implement real alert querying from monitor
        // For now, return empty as the monitor doesn't expose alerts directly yet
        Vec::new()
    } else {
        generate_placeholder_alerts(&filter, &pagination)
    };
    
    Ok(Json(json!({
        "data": alerts,
        "pagination": {
            "page": pagination.page,
            "pageSize": pagination.page_size,
            "totalCount": alerts.len()
        }
    })))
}

/// GET /api/v3/monitoring/alerts/{id} - Get specific alert by ID
pub async fn get_alert_by_id(
    Path(alert_id): Path<String>,
    monitor: Option<Extension<Arc<ListSyncMonitor>>>,
) -> ApiResult<Json<AlertResponse>> {
    debug!("Fetching alert by ID: {}", alert_id);
    
    let alert = if let Some(Extension(_monitor)) = monitor {
        // TODO: Implement real alert querying by ID from monitor
        generate_placeholder_alert_by_id(&alert_id)
    } else {
        generate_placeholder_alert_by_id(&alert_id)
    };
    
    match alert {
        Some(alert) => Ok(Json(alert)),
        None => Err(ApiError::NotFound {
            resource: "alert".to_string(),
        }),
    }
}

/// GET /api/v3/monitoring/health - Service health status
pub async fn get_health_status(
    monitor: Option<Extension<Arc<ListSyncMonitor>>>,
) -> ApiResult<Json<serde_json::Value>> {
    debug!("Fetching service health status");
    
    let (health_summary, services) = if let Some(Extension(monitor)) = monitor {
        // Get real health data from monitor
        let status = monitor.get_monitoring_status().await;
        let overall_status = if status.health_summary.unhealthy_services == 0 {
            "healthy"
        } else {
            "degraded"
        };
        
        let summary = json!({
            "status": overall_status,
            "healthyServices": status.health_summary.healthy_services,
            "unhealthyServices": status.health_summary.unhealthy_services,
            "unknownServices": status.health_summary.unknown_services,
            "lastCheck": Utc::now(),
        });
        
        // TODO: Get actual service details from monitor
        // For now, use placeholder services
        let (_, services) = generate_placeholder_health();
        (summary, services)
    } else {
        generate_placeholder_health()
    };
    
    Ok(Json(json!({
        "healthSummary": health_summary,
        "services": services
    })))
}

/// GET /api/v3/monitoring/circuit-breakers - Circuit breaker states
pub async fn get_circuit_breaker_states(
    monitor: Option<Extension<Arc<ListSyncMonitor>>>,
) -> ApiResult<Json<HashMap<String, CircuitBreakerStatusResponse>>> {
    debug!("Fetching circuit breaker states");
    
    let circuit_breakers = if let Some(Extension(monitor)) = monitor {
        // Get real circuit breaker data from monitor
        let status = monitor.get_monitoring_status().await;
        status.circuit_breaker_status
            .into_iter()
            .map(|(k, v)| {
                (k, CircuitBreakerStatusResponse {
                    state: v.state,
                    consecutive_failures: v.consecutive_failures,
                    success_rate: v.success_rate,
                    is_healthy: v.is_healthy,
                })
            })
            .collect()
    } else {
        generate_placeholder_circuit_breakers()
    };
    
    Ok(Json(circuit_breakers))
}