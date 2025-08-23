//! Monitoring API route definitions
//!
//! This module defines the route configuration for monitoring-related endpoints,
//! including metrics, alerts, health checks, and circuit breaker states.

use crate::handlers::monitoring::{
    get_prometheus_metrics, get_monitoring_status, get_alerts, get_health_status,
    get_circuit_breaker_states, get_alert_by_id,
};
use axum::{
    routing::get,
    Router,
};
// Note: ListSyncMonitor integration is planned for future implementation

/// Create monitoring routes
///
/// This function creates all monitoring-related routes and returns a Router
/// that can be nested into the main application router.
///
/// Routes created:
/// - GET /metrics - Prometheus metrics endpoint
/// - GET /api/v3/monitoring/status - Comprehensive monitoring status
/// - GET /api/v3/monitoring/alerts - List alerts with filtering
/// - GET /api/v3/monitoring/alerts/{id} - Get specific alert by ID  
/// - GET /api/v3/monitoring/health - Service health status
/// - GET /api/v3/monitoring/circuit-breakers - Circuit breaker states
pub fn create_monitoring_routes() -> Router {
    Router::new()
        // Prometheus metrics endpoint (standard path for metrics scraping)
        .route("/metrics", get(get_prometheus_metrics))
        
        // Comprehensive monitoring status
        .route("/api/v3/monitoring/status", get(get_monitoring_status))
        
        // Alert management endpoints
        .route("/api/v3/monitoring/alerts", get(get_alerts))
        .route("/api/v3/monitoring/alerts/:id", get(get_alert_by_id))
        
        // Service health endpoints
        .route("/api/v3/monitoring/health", get(get_health_status))
        
        // Circuit breaker status
        .route("/api/v3/monitoring/circuit-breakers", get(get_circuit_breaker_states))
}

/// Create monitoring routes with middleware
///
/// This is an alternative that could be used if you want to add monitoring-specific
/// middleware like authentication, rate limiting, or logging.
#[allow(dead_code)]
pub fn create_monitoring_routes_with_middleware() -> Router {
    create_monitoring_routes()
        // Add any monitoring-specific middleware here
        // .layer(auth_layer)  // Example: authentication for sensitive endpoints
        // .layer(rate_limit_layer)  // Example: rate limiting for metrics endpoint
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_monitoring_routes_creation() {
        let router = create_monitoring_routes();
        
        // Verify the router can be created without errors
        let server = TestServer::new(router).unwrap();
        
        // Test that the routes are accessible and return proper responses
        let response = server.get("/metrics").await;
        // Should return OK with Prometheus format
        assert_eq!(response.status_code(), StatusCode::OK);
        let content_type = response.header("content-type");
        assert!(content_type.is_some());
        assert!(content_type.unwrap().to_str().unwrap().contains("text/plain"));
        
        let response = server.get("/api/v3/monitoring/status").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let response = server.get("/api/v3/monitoring/health").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let response = server.get("/api/v3/monitoring/circuit-breakers").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let response = server.get("/api/v3/monitoring/alerts").await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_alert_by_id_not_found() {
        let router = create_monitoring_routes();
        let server = TestServer::new(router).unwrap();
        
        let response = server.get("/api/v3/monitoring/alerts/nonexistent-id").await;
        assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
    }
}