//! Service unavailable fault injection tests
//!
//! Tests the system's behavior when external services become completely unavailable.
//! Validates:
//! - Handling of 503 Service Unavailable responses
//! - Connection refused scenarios (service down)
//! - DNS resolution failures
//! - Network partition scenarios
//! - Circuit breaker activation for service outages
//! - Graceful degradation and fallback mechanisms
//! - Service recovery detection and circuit breaker reset

use super::*;
use radarr_core::circuit_breaker::CircuitBreakerState;
use radarr_core::RadarrError;
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

/// Test HTTP 503 Service Unavailable responses
#[tokio::test]
async fn test_http_503_service_unavailable() {
    let context = FaultInjectionTestContext::new("unavailable_service").await;

    // Setup service that returns 503
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(
            ResponseTemplate::new(503)
                .set_body_json(json!({
                    "error": "Service temporarily unavailable",
                    "code": "SERVICE_UNAVAILABLE",
                    "retry_after": 60
                }))
                .insert_header("Retry-After", "60"),
        )
        .mount(&context.mock_server)
        .await;

    let health_url = format!("{}/api/health", context.base_url());

    let start_time = Instant::now();
    let result = context.make_request(&health_url).await;
    let elapsed = start_time.elapsed();

    // Should fail with service error
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            RadarrError::ExternalServiceError {
                service,
                error: err_msg,
            } => {
                assert_eq!(service, "test_service");
                assert!(err_msg.contains("503"));
                println!("Correctly detected 503 Service Unavailable: {}", err_msg);
            }
            RadarrError::TemporaryError { message } => {
                assert!(message.contains("503") || message.contains("unavailable"));
                println!("Detected as temporary error: {}", message);
            }
            other => {
                println!("Got service unavailable error (may be valid): {:?}", other);
            }
        }
    }

    // Should respond quickly (not hang)
    assert!(elapsed < Duration::from_secs(2));

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test service that becomes completely unresponsive (connection refused)
#[tokio::test]
async fn test_connection_refused_service_down() {
    let context = FaultInjectionTestContext::new("connection_refused").await;

    // Don't setup any mock endpoints - this will cause connection refused
    let nonexistent_url = format!("http://localhost:{}/api/status", 65432); // Use unlikely port

    let start_time = Instant::now();

    let client = reqwest::Client::new();
    let result = context
        .circuit_breaker
        .call(async {
            let response = client
                .get(&nonexistent_url)
                .timeout(Duration::from_secs(2))
                .send()
                .await
                .map_err(|e| {
                    if e.is_connect() {
                        RadarrError::NetworkError {
                            message: format!("Connection refused - service down: {}", e),
                        }
                    } else if e.is_timeout() {
                        RadarrError::Timeout {
                            operation: "service connection".to_string(),
                        }
                    } else {
                        RadarrError::NetworkError {
                            message: format!("Network error: {}", e),
                        }
                    }
                })?;

            let body = response
                .text()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Failed to read response: {}", e),
                })?;

            Ok::<String, RadarrError>(body)
        })
        .await;

    let elapsed = start_time.elapsed();

    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            RadarrError::NetworkError { message } => {
                assert!(message.contains("Connection refused") || message.contains("connect"));
                println!("Correctly detected connection refused: {}", message);
            }
            RadarrError::Timeout { operation } => {
                println!("Detected as timeout: {}", operation);
            }
            other => {
                panic!("Expected NetworkError or Timeout, got: {:?}", other);
            }
        }
    }

    // Should fail quickly due to connection refused
    assert!(elapsed < Duration::from_secs(3));

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test multiple service unavailable responses leading to circuit breaker
#[tokio::test]
async fn test_service_outage_opens_circuit() {
    let context = FaultInjectionTestContext::new("service_outage").await;

    // Setup service that consistently returns 503
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(503)
                .set_body_string("Service maintenance in progress")
                .insert_header("Retry-After", "300"),
        )
        .mount(&context.mock_server)
        .await;

    let data_url = format!("{}/api/data", context.base_url());

    // Make multiple requests to trigger circuit breaker
    let mut unavailable_count = 0;
    for i in 0..6 {
        println!("Making request {}", i + 1);
        let result = context.make_request(&data_url).await;

        match result {
            Err(RadarrError::ExternalServiceError { error, .. }) if error.contains("503") => {
                unavailable_count += 1;
                println!("Request {} service unavailable", i + 1);
            }
            Err(RadarrError::TemporaryError { .. }) => {
                unavailable_count += 1;
                println!("Request {} temporary error", i + 1);
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                println!(
                    "Circuit breaker opened after {} unavailable responses",
                    unavailable_count
                );
                assert_eq!(service, "service_outage");
                break;
            }
            other => {
                println!("Unexpected result for request {}: {:?}", i + 1, other);
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Circuit should be open due to service outage
    let state = context.circuit_breaker.get_state().await;
    assert_eq!(state, CircuitBreakerState::Open);

    let metrics = context.get_test_metrics().await;
    assert!(metrics.failed_requests >= 3);
    assert!(unavailable_count >= 3);
    assert!(metrics.rejected_requests > 0);
}

/// Test service recovery after outage
#[tokio::test]
async fn test_service_recovery_after_outage() {
    let context = FaultInjectionTestContext::new("service_recovery").await;

    // First, service is unavailable
    Mock::given(method("GET"))
        .and(path("/api/status"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service temporarily unavailable"))
        .up_to_n_times(3)
        .mount(&context.mock_server)
        .await;

    // Then service comes back online
    Mock::given(method("GET"))
        .and(path("/api/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "operational",
            "message": "Service restored after maintenance",
            "uptime": "00:00:30"
        })))
        .mount(&context.mock_server)
        .await;

    let status_url = format!("{}/api/status", context.base_url());

    // Trigger service unavailable errors to open circuit
    for i in 0..3 {
        let result = context.make_request(&status_url).await;
        assert!(
            result.is_err(),
            "Request {} should fail - service unavailable",
            i + 1
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Verify circuit is open
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Open
    );

    // Wait for circuit to transition to half-open
    assert!(
        context
            .wait_for_circuit_state(CircuitBreakerState::HalfOpen, Duration::from_millis(200))
            .await
    );

    // Make request that should succeed (service restored)
    let recovery_result = context.make_request(&status_url).await;
    assert!(recovery_result.is_ok(), "Recovery request should succeed");

    if let Ok(body) = recovery_result {
        assert!(body.contains("operational"));
        assert!(body.contains("Service restored"));
        println!("Service successfully restored: {}", body);
    }

    // Circuit should be closed again
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );

    let metrics = context.get_test_metrics().await;
    assert!(metrics.successful_requests > 0);
    assert!(metrics.failed_requests >= 3);
    assert!(metrics.is_resilient());
}

/// Test intermittent service availability
#[tokio::test]
async fn test_intermittent_service_availability() {
    let context = FaultInjectionTestContext::new("intermittent_service").await;

    // Setup alternating availability pattern
    Mock::given(method("GET"))
        .and(path("/api/ping"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"pong": true, "sequence": 1})),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/ping"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Temporarily unavailable"))
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/ping"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"pong": true, "sequence": 3})),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/ping"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service overloaded"))
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/ping"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"pong": true, "sequence": 5})),
        )
        .mount(&context.mock_server)
        .await;

    let ping_url = format!("{}/api/ping", context.base_url());
    let mut results = Vec::new();

    // Make requests and track availability pattern
    for i in 0..6 {
        let result = context.make_request(&ping_url).await;
        let is_available = result.is_ok();

        if is_available {
            if let Ok(body) = result {
                println!("Request {}: Service available - {}", i + 1, body);
            }
        } else {
            if let Err(RadarrError::CircuitBreakerOpen { .. }) = result {
                println!("Request {}: Circuit breaker open", i + 1);
                break;
            } else {
                println!("Request {}: Service unavailable - {:?}", i + 1, result);
            }
        }

        results.push((i + 1, is_available));
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let metrics = context.get_test_metrics().await;

    // Should have mix of successful and failed requests
    let available_count = results.iter().filter(|(_, available)| *available).count();
    let unavailable_count = results.iter().filter(|(_, available)| !*available).count();

    assert!(available_count > 0, "Should have some successful requests");
    assert!(unavailable_count > 0, "Should have some failed requests");

    println!("Intermittent availability pattern:");
    for (request_num, available) in results {
        println!(
            "  Request {}: {}",
            request_num,
            if available {
                "AVAILABLE"
            } else {
                "UNAVAILABLE"
            }
        );
    }

    println!(
        "Final metrics - available: {}, unavailable: {}",
        available_count, unavailable_count
    );
}

/// Test concurrent requests during service outage
#[tokio::test]
async fn test_concurrent_requests_service_down() {
    let context = FaultInjectionTestContext::new("concurrent_outage").await;

    // Setup service that's completely down
    Mock::given(method("GET"))
        .and(path("/api/endpoint"))
        .respond_with(
            ResponseTemplate::new(503).set_body_string("All systems down for maintenance"),
        )
        .mount(&context.mock_server)
        .await;

    let endpoint_url = format!("{}/api/endpoint", context.base_url());

    // Make concurrent requests
    let mut handles = Vec::new();
    for i in 0..5 {
        let context_ref = &context;
        let url = endpoint_url.clone();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = context_ref.make_request(&url).await;
            let elapsed = start.elapsed();
            (i, result, elapsed)
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    let mut unavailable_count = 0;
    let mut circuit_open_count = 0;
    let mut total_time = Duration::ZERO;

    for result in results {
        let (request_id, response, elapsed) = result.unwrap();
        total_time += elapsed;

        match response {
            Err(RadarrError::ExternalServiceError { .. }) => {
                unavailable_count += 1;
                println!(
                    "Concurrent request {} service unavailable in {:?}",
                    request_id, elapsed
                );
            }
            Err(RadarrError::TemporaryError { .. }) => {
                unavailable_count += 1;
                println!(
                    "Concurrent request {} temporary error in {:?}",
                    request_id, elapsed
                );
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                circuit_open_count += 1;
                println!(
                    "Concurrent request {} circuit breaker open in {:?}",
                    request_id, elapsed
                );
            }
            other => {
                println!(
                    "Concurrent request {} unexpected result: {:?} in {:?}",
                    request_id, other, elapsed
                );
            }
        }
    }

    let avg_time = total_time / 5;

    // All requests should fail, either as service unavailable or circuit breaker open
    assert!(
        unavailable_count + circuit_open_count == 5,
        "All requests should fail: {} unavailable, {} circuit open",
        unavailable_count,
        circuit_open_count
    );

    // Requests should fail relatively quickly
    assert!(
        avg_time < Duration::from_secs(2),
        "Average response time should be fast: {:?}",
        avg_time
    );

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 5);
    assert!(metrics.failed_requests > 0 || metrics.rejected_requests > 0);

    println!(
        "Concurrent outage results - unavailable: {}, circuit open: {}, avg time: {:?}",
        unavailable_count, circuit_open_count, avg_time
    );
}

/// Test service degradation levels (different error codes)
#[tokio::test]
async fn test_service_degradation_levels() {
    let context = FaultInjectionTestContext::new("service_degradation").await;

    // Setup different levels of service degradation
    Mock::given(method("GET"))
        .and(path("/api/level1"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Minor service degradation"))
        .mount(&context.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/level2"))
        .respond_with(
            ResponseTemplate::new(502).set_body_string("Bad Gateway - upstream service failed"),
        )
        .mount(&context.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/level3"))
        .respond_with(
            ResponseTemplate::new(504).set_body_string("Gateway timeout - service unreachable"),
        )
        .mount(&context.mock_server)
        .await;

    let degradation_levels = vec![
        ("/api/level1", "503 Service Unavailable"),
        ("/api/level2", "502 Bad Gateway"),
        ("/api/level3", "504 Gateway Timeout"),
    ];

    for (endpoint, expected_error) in degradation_levels {
        let url = format!("{}{}", context.base_url(), endpoint);
        let result = context.make_request(&url).await;

        assert!(result.is_err(), "Request to {} should fail", endpoint);

        if let Err(error) = result {
            match error {
                RadarrError::ExternalServiceError { error: err_msg, .. } => {
                    println!("Detected {}: {}", expected_error, err_msg);
                    assert!(err_msg.contains("50") || err_msg.contains("server error"));
                }
                RadarrError::TemporaryError { message } => {
                    println!(
                        "Detected as temporary error for {}: {}",
                        expected_error, message
                    );
                }
                other => {
                    println!("Error for {} (may be valid): {:?}", expected_error, other);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 3);
    assert_eq!(metrics.failed_requests, 3);
}

/// Test service health check during outage
#[tokio::test]
async fn test_health_check_during_outage() {
    let context = FaultInjectionTestContext::new("health_check_outage").await;

    // Setup health endpoint that's down
    Mock::given(method("GET"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "status": "unhealthy",
            "checks": {
                "database": "failing",
                "cache": "unavailable",
                "external_apis": "timeout"
            },
            "uptime": 0
        })))
        .mount(&context.mock_server)
        .await;

    let health_url = format!("{}/health", context.base_url());

    // Perform health check
    let client = reqwest::Client::new();
    let health_result =
        context
            .circuit_breaker
            .call(async {
                let response = client.get(&health_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("Health check failed: {}", e),
                    }
                })?;

                let status = response.status();
                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read health response: {}", e),
                    })?;

                if status == 503 {
                    // Parse the unhealthy status
                    if let Ok(health_data) = serde_json::from_str::<serde_json::Value>(&body) {
                        let status = health_data["status"].as_str().unwrap_or("unknown");
                        if status == "unhealthy" {
                            return Err(RadarrError::ExternalServiceError {
                                service: "health_check".to_string(),
                                error: format!("Service unhealthy: {}", body),
                            });
                        }
                    }

                    return Err(RadarrError::TemporaryError {
                        message: format!("Health check returned 503: {}", body),
                    });
                }

                Ok::<String, RadarrError>(body)
            })
            .await;

    assert!(health_result.is_err());

    if let Err(error) = health_result {
        match error {
            RadarrError::ExternalServiceError {
                service,
                error: err_msg,
            } => {
                assert_eq!(service, "health_check");
                assert!(err_msg.contains("unhealthy"));
                println!("Health check detected service as unhealthy: {}", err_msg);
            }
            RadarrError::TemporaryError { message } => {
                assert!(message.contains("503"));
                println!("Health check failed with temporary error: {}", message);
            }
            other => {
                panic!("Expected service or temporary error, got: {:?}", other);
            }
        }
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

#[cfg(test)]
mod service_outage_integration_tests {
    use super::*;

    /// Integration test: Service outage affecting multiple components
    #[tokio::test]
    async fn test_cascading_service_outage() {
        let indexer_service = FaultInjectionTestContext::new("indexer_outage").await;
        let metadata_service = FaultInjectionTestContext::new("metadata_outage").await;
        let download_service = FaultInjectionTestContext::new("download_available").await;

        // Indexer service is down
        Mock::given(method("GET"))
            .and(path("/api/search"))
            .respond_with(
                ResponseTemplate::new(503).set_body_string("Indexer maintenance in progress"),
            )
            .mount(&indexer_service.mock_server)
            .await;

        // Metadata service is also down
        Mock::given(method("GET"))
            .and(path("/api/movie/123"))
            .respond_with(
                ResponseTemplate::new(502).set_body_string("Upstream metadata provider failed"),
            )
            .mount(&metadata_service.mock_server)
            .await;

        // But download service is still working
        Mock::given(method("GET"))
            .and(path("/api/v2/torrents/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"hash": "abc123", "state": "downloading", "progress": 0.5}
            ])))
            .mount(&download_service.mock_server)
            .await;

        let indexer_url = format!("{}/api/search", indexer_service.base_url());
        let metadata_url = format!("{}/api/movie/123", metadata_service.base_url());
        let download_url = format!("{}/api/v2/torrents/info", download_service.base_url());

        // Test each service
        let indexer_result = indexer_service.make_request(&indexer_url).await;
        let metadata_result = metadata_service.make_request(&metadata_url).await;
        let download_result = download_service.make_request(&download_url).await;

        // Indexer and metadata should fail
        assert!(indexer_result.is_err(), "Indexer should be unavailable");
        assert!(
            metadata_result.is_err(),
            "Metadata service should be unavailable"
        );

        // Download service should still work
        assert!(
            download_result.is_ok(),
            "Download service should still be available"
        );

        if let Ok(body) = download_result {
            assert!(body.contains("downloading"));
            println!("Download service operational despite other outages");
        }

        // Check metrics
        let indexer_metrics = indexer_service.get_test_metrics().await;
        let metadata_metrics = metadata_service.get_test_metrics().await;
        let download_metrics = download_service.get_test_metrics().await;

        assert_eq!(indexer_metrics.failed_requests, 1);
        assert_eq!(metadata_metrics.failed_requests, 1);
        assert_eq!(download_metrics.successful_requests, 1);

        println!("Cascading outage handled correctly:");
        println!("  Indexer: DOWN (503)");
        println!("  Metadata: DOWN (502)");
        println!("  Download: UP (200)");
    }

    /// Integration test: Complete service restoration workflow
    #[tokio::test]
    async fn test_complete_service_restoration() {
        let context = FaultInjectionTestContext::new("complete_restoration").await;

        // Phase 1: Service completely down
        Mock::given(method("GET"))
            .and(path("/api/status"))
            .respond_with(ResponseTemplate::new(503).set_body_string("All services offline"))
            .up_to_n_times(4)
            .mount(&context.mock_server)
            .await;

        // Phase 2: Partial recovery
        Mock::given(method("GET"))
            .and(path("/api/status"))
            .respond_with(ResponseTemplate::new(503).set_body_json(json!({
                "status": "degraded",
                "services": {
                    "core": "online",
                    "search": "offline",
                    "download": "online"
                }
            })))
            .up_to_n_times(2)
            .mount(&context.mock_server)
            .await;

        // Phase 3: Full recovery
        Mock::given(method("GET"))
            .and(path("/api/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "healthy",
                "services": {
                    "core": "online",
                    "search": "online",
                    "download": "online"
                },
                "uptime": "00:05:00"
            })))
            .mount(&context.mock_server)
            .await;

        let status_url = format!("{}/api/status", context.base_url());

        // Phase 1: Complete outage (trigger circuit breaker)
        println!("Phase 1: Complete service outage");
        for i in 0..4 {
            let result = context.make_request(&status_url).await;
            assert!(
                result.is_err(),
                "Request {} should fail during outage",
                i + 1
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Open
        );
        println!("  Circuit breaker opened due to outage");

        // Wait for circuit to transition to half-open
        assert!(
            context
                .wait_for_circuit_state(CircuitBreakerState::HalfOpen, Duration::from_millis(200))
                .await
        );
        println!("  Circuit breaker transitioned to half-open");

        // Phase 2: Partial recovery (may still fail)
        println!("Phase 2: Partial service recovery");
        for i in 0..2 {
            let result = context.make_request(&status_url).await;
            // May succeed or fail depending on what we consider "degraded" as
            println!("  Partial recovery request {}: {:?}", i + 1, result.is_ok());
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Phase 3: Full recovery
        println!("Phase 3: Full service recovery");
        let recovery_result = context.make_request(&status_url).await;
        assert!(recovery_result.is_ok(), "Full recovery should succeed");

        if let Ok(body) = recovery_result {
            assert!(body.contains("healthy"));
            assert!(body.contains("online"));
            println!("  Service fully recovered: all systems online");
        }

        // Circuit should be closed
        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Closed
        );

        let metrics = context.get_test_metrics().await;
        assert!(metrics.successful_requests > 0);
        assert!(metrics.failed_requests >= 4);
        assert!(metrics.is_resilient());

        println!("Complete restoration workflow successful");
    }
}
