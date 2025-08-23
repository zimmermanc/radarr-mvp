//! Integration tests for monitoring API endpoints
//!
//! This module tests the REST API endpoints that expose monitoring data.
//! Tests focus on the existing placeholder implementation in the monitoring handlers.

use crate::common::TestContext;
use radarr_api::handlers::monitoring::*;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    Router,
    routing::get,
};
use tower::ServiceExt;
use serde_json::Value;

async fn create_test_monitoring_app() -> (Router, TestContext) {
    let test_ctx = TestContext::new().await;
    
    // Create monitoring app with test routes
    let app = Router::new()
        .route("/metrics", get(get_prometheus_metrics))
        .route("/api/v3/monitoring/status", get(get_monitoring_status));
    
    (app, test_ctx)
}

#[tokio::test]
async fn test_prometheus_metrics_endpoint() {
    // Test the /metrics endpoint for Prometheus metrics export
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let request = Request::builder()
        .uri("/metrics")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Verify response status
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify content type
    let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
    assert_eq!(content_type, "text/plain; version=0.0.4; charset=utf-8");
    
    // Read response body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // Verify Prometheus format
    assert!(body_str.contains("# HELP"), "Should contain HELP comments");
    assert!(body_str.contains("# TYPE"), "Should contain TYPE declarations");
    assert!(body_str.contains("radarr_list_sync"), "Should contain radarr metrics");
    assert!(body_str.contains("sync_operations_total"), "Should contain sync operation metrics");
    assert!(body_str.contains("api_requests_total"), "Should contain API request metrics");
    assert!(body_str.contains("cache_hits_total"), "Should contain cache metrics");
    
    // Verify generation timestamp
    assert!(body_str.contains("# Generated at"), "Should include generation timestamp");
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_monitoring_status_endpoint() {
    // Test the /api/v3/monitoring/status endpoint
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let request = Request::builder()
        .uri("/api/v3/monitoring/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    
    // Verify response status
    assert_eq!(response.status(), StatusCode::OK);
    
    // Verify content type
    let content_type = response.headers().get(header::CONTENT_TYPE).unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
    
    // Read and parse response body
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    let status_response: MonitoringStatusResponse = serde_json::from_str(&body_str)
        .expect("Should parse monitoring status response");
    
    // Verify response structure
    assert!(!status_response.status.is_empty());
    assert!(status_response.uptime_seconds >= 0);
    assert!(status_response.total_operations_monitored >= 0);
    
    // Verify sync metrics
    assert!(status_response.sync_metrics.total_sync_operations >= 0);
    assert!(status_response.sync_metrics.successful_sync_operations >= 0);
    assert!(status_response.sync_metrics.failed_sync_operations >= 0);
    assert!(status_response.sync_metrics.average_sync_duration_ms >= 0.0);
    assert!(status_response.sync_metrics.items_processed_total >= 0);
    assert!(status_response.sync_metrics.cache_hit_rate >= 0.0);
    assert!(status_response.sync_metrics.cache_hit_rate <= 1.0);
    
    // Verify alert stats
    assert!(status_response.alert_stats.total_alerts >= 0);
    assert!(status_response.alert_stats.active_alerts >= 0);
    assert!(status_response.alert_stats.critical_alerts >= 0);
    assert!(status_response.alert_stats.warning_alerts >= 0);
    assert!(status_response.alert_stats.resolved_alerts >= 0);
    
    // Verify health summary
    assert!(!status_response.health_summary.overall_status.is_empty());
    assert!(status_response.health_summary.healthy_services >= 0);
    assert!(status_response.health_summary.unhealthy_services >= 0);
    assert!(status_response.health_summary.unknown_services >= 0);
    
    // Verify circuit breaker status
    assert!(!status_response.circuit_breaker_status.is_empty());
    for (service_name, cb_status) in &status_response.circuit_breaker_status {
        assert!(!service_name.is_empty());
        assert!(!cb_status.state.is_empty());
        assert!(cb_status.consecutive_failures >= 0);
        assert!(cb_status.success_rate >= 0.0);
        assert!(cb_status.success_rate <= 100.0);
    }
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_metrics_endpoint_performance() {
    // Test metrics endpoint performance under load
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let concurrent_requests = 10;
    let mut handles = Vec::new();
    
    let start_time = std::time::Instant::now();
    
    // Make multiple concurrent requests
    for _i in 0..concurrent_requests {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .uri("/metrics")
                .method("GET")
                .body(Body::empty())
                .unwrap();
            
            app_clone.oneshot(request).await.unwrap()
        });
        handles.push(handle);
    }
    
    // Wait for all requests to complete
    let mut successful_requests = 0;
    for handle in handles {
        let response = handle.await.expect("Request should complete");
        if response.status() == StatusCode::OK {
            successful_requests += 1;
        }
    }
    
    let total_time = start_time.elapsed();
    
    // Verify all requests succeeded
    assert_eq!(successful_requests, concurrent_requests);
    
    // Verify reasonable performance (should handle concurrent requests quickly)
    assert!(total_time.as_millis() < 5000, "Should handle {} concurrent requests in under 5 seconds", concurrent_requests);
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_monitoring_status_endpoint_with_filters() {
    // Test monitoring status endpoint with query parameters (if implemented)
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    // Test basic status endpoint
    let request = Request::builder()
        .uri("/api/v3/monitoring/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    // Test with query parameters (these might not be implemented yet)
    let request_with_params = Request::builder()
        .uri("/api/v3/monitoring/status?include=metrics,alerts&format=detailed")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response_with_params = app.clone().oneshot(request_with_params).await.unwrap();
    // Should still return OK even if parameters are ignored
    assert_eq!(response_with_params.status(), StatusCode::OK);
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_api_error_handling() {
    // Test API error handling for various scenarios
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    // Test invalid endpoints
    let invalid_request = Request::builder()
        .uri("/api/v3/monitoring/nonexistent")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let invalid_response = app.oneshot(invalid_request).await.unwrap();
    assert_eq!(invalid_response.status(), StatusCode::NOT_FOUND);
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_metrics_content_validation() {
    // Test detailed validation of metrics content
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let request = Request::builder()
        .uri("/metrics")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // Parse and validate Prometheus format
    let lines: Vec<&str> = body_str.lines().collect();
    
    let mut current_metric = None;
    let mut help_found = false;
    let mut type_found = false;
    
    for line in lines {
        if line.starts_with("# HELP") {
            help_found = true;
            current_metric = line.split_whitespace().nth(2);
        } else if line.starts_with("# TYPE") {
            type_found = true;
            let metric_name = line.split_whitespace().nth(2);
            assert_eq!(current_metric, metric_name, "TYPE should match previous HELP metric");
        } else if !line.starts_with("#") && !line.trim().is_empty() {
            // Validate metric line format: metric_name{labels} value [timestamp]
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert!(!parts.is_empty(), "Metric line should not be empty");
            
            // Last part should be a valid number (the metric value)
            let value_str = parts.last().unwrap();
            assert!(
                value_str.parse::<f64>().is_ok() || value_str == "+Inf" || value_str == "-Inf" || value_str == "NaN",
                "Metric value should be a valid number: {}",
                value_str
            );
            
            // First part should contain metric name and optional labels
            let metric_part = parts[0];
            assert!(!metric_part.is_empty(), "Metric name should not be empty");
        }
    }
    
    assert!(help_found, "Should contain at least one HELP comment");
    assert!(type_found, "Should contain at least one TYPE comment");
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_status_response_json_structure() {
    // Test detailed JSON structure validation for status endpoint
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let request = Request::builder()
        .uri("/api/v3/monitoring/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();
    
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
    
    // Parse as generic JSON first to inspect structure
    let json_value: Value = serde_json::from_str(&body_str)
        .expect("Response should be valid JSON");
    
    // Verify top-level structure
    assert!(json_value.is_object(), "Response should be a JSON object");
    let obj = json_value.as_object().unwrap();
    
    // Required fields
    assert!(obj.contains_key("status"), "Should contain status field");
    assert!(obj.contains_key("startedAt"), "Should contain startedAt field");
    assert!(obj.contains_key("uptimeSeconds"), "Should contain uptimeSeconds field");
    assert!(obj.contains_key("totalOperationsMonitored"), "Should contain totalOperationsMonitored field");
    assert!(obj.contains_key("syncMetrics"), "Should contain syncMetrics field");
    assert!(obj.contains_key("alertStats"), "Should contain alertStats field");
    assert!(obj.contains_key("healthSummary"), "Should contain healthSummary field");
    assert!(obj.contains_key("circuitBreakerStatus"), "Should contain circuitBreakerStatus field");
    
    // Verify nested objects have correct structure
    let sync_metrics = obj.get("syncMetrics").unwrap().as_object().unwrap();
    assert!(sync_metrics.contains_key("totalSyncOperations"));
    assert!(sync_metrics.contains_key("successfulSyncOperations"));
    assert!(sync_metrics.contains_key("failedSyncOperations"));
    assert!(sync_metrics.contains_key("averageSyncDurationMs"));
    assert!(sync_metrics.contains_key("itemsProcessedTotal"));
    assert!(sync_metrics.contains_key("cacheHitRate"));
    
    let alert_stats = obj.get("alertStats").unwrap().as_object().unwrap();
    assert!(alert_stats.contains_key("totalAlerts"));
    assert!(alert_stats.contains_key("activeAlerts"));
    assert!(alert_stats.contains_key("criticalAlerts"));
    assert!(alert_stats.contains_key("warningAlerts"));
    assert!(alert_stats.contains_key("resolvedAlerts"));
    
    let health_summary = obj.get("healthSummary").unwrap().as_object().unwrap();
    assert!(health_summary.contains_key("overallStatus"));
    assert!(health_summary.contains_key("healthyServices"));
    assert!(health_summary.contains_key("unhealthyServices"));
    assert!(health_summary.contains_key("unknownServices"));
    assert!(health_summary.contains_key("lastCheck"));
    
    let circuit_breaker_status = obj.get("circuitBreakerStatus").unwrap().as_object().unwrap();
    assert!(!circuit_breaker_status.is_empty(), "Should have circuit breaker data");
    
    // Verify circuit breaker structure
    for (service_name, cb_data) in circuit_breaker_status {
        assert!(!service_name.is_empty(), "Service name should not be empty");
        let cb_obj = cb_data.as_object().unwrap();
        assert!(cb_obj.contains_key("state"));
        assert!(cb_obj.contains_key("consecutiveFailures"));
        assert!(cb_obj.contains_key("successRate"));
        assert!(cb_obj.contains_key("isHealthy"));
    }
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_endpoint_response_times() {
    // Test that endpoints respond within reasonable time limits
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let endpoints = vec![
        "/metrics",
        "/api/v3/monitoring/status",
    ];
    
    for endpoint in endpoints {
        let start_time = std::time::Instant::now();
        
        let request = Request::builder()
            .uri(endpoint)
            .method("GET")
            .body(Body::empty())
            .unwrap();
        
        let response = app.clone().oneshot(request).await.unwrap();
        let response_time = start_time.elapsed();
        
        assert_eq!(response.status(), StatusCode::OK, "Endpoint {} should return OK", endpoint);
        assert!(
            response_time.as_millis() < 1000, 
            "Endpoint {} should respond within 1 second, took {:?}", 
            endpoint, response_time
        );
        
        println!("Endpoint {} responded in {:?}", endpoint, response_time);
    }
    
    test_ctx.cleanup().await;
}

#[tokio::test]
async fn test_concurrent_api_access() {
    // Test concurrent access to multiple endpoints
    let (app, test_ctx) = create_test_monitoring_app().await;
    
    let endpoints = vec![
        "/metrics",
        "/api/v3/monitoring/status",
        "/api/v3/monitoring/status",  // Test same endpoint multiple times
        "/metrics",
    ];
    
    let mut handles = Vec::new();
    let start_time = std::time::Instant::now();
    
    // Make concurrent requests to different endpoints
    for endpoint in endpoints {
        let app_clone = app.clone();
        let endpoint_owned = endpoint.to_string();
        
        let handle = tokio::spawn(async move {
            let request = Request::builder()
                .uri(&endpoint_owned)
                .method("GET")
                .body(Body::empty())
                .unwrap();
            
            let response = app_clone.oneshot(request).await.unwrap();
            (endpoint_owned, response.status())
        });
        
        handles.push(handle);
    }
    
    // Collect all responses
    let mut results = Vec::new();
    for handle in handles {
        let (endpoint, status) = handle.await.expect("Request should complete");
        results.push((endpoint, status));
    }
    
    let total_time = start_time.elapsed();
    
    // Verify all requests succeeded
    for (endpoint, status) in results {
        assert_eq!(status, StatusCode::OK, "Endpoint {} should succeed", endpoint);
    }
    
    // Should handle concurrent requests efficiently
    assert!(total_time.as_millis() < 3000, "Concurrent requests should complete within 3 seconds");
    
    test_ctx.cleanup().await;
}