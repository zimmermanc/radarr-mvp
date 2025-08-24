//! Simple fault injection test to verify basic functionality

use radarr_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use radarr_core::{RadarrError, Result};
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Simple test context for fault injection
struct SimpleFaultContext {
    pub mock_server: MockServer,
    pub circuit_breaker: CircuitBreaker,
}

impl SimpleFaultContext {
    async fn new(service_name: &str) -> Self {
        let mock_server = MockServer::start().await;
        let config = CircuitBreakerConfig::new(service_name)
            .with_failure_threshold(3)
            .with_timeout(Duration::from_millis(100));
        let circuit_breaker = CircuitBreaker::new(config);

        Self {
            mock_server,
            circuit_breaker,
        }
    }

    fn base_url(&self) -> String {
        self.mock_server.uri()
    }

    async fn make_request(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::new();

        self.circuit_breaker
            .call(async {
                let response =
                    client
                        .get(url)
                        .send()
                        .await
                        .map_err(|e| RadarrError::NetworkError {
                            message: format!("HTTP request failed: {}", e),
                        })?;

                let status = response.status();
                if status.is_server_error() {
                    return Err(RadarrError::ExternalServiceError {
                        service: "test_service".to_string(),
                        error: format!("Server error: {}", status),
                    });
                }

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response body: {}", e),
                    })?;

                Ok::<String, RadarrError>(body)
            })
            .await
    }
}

#[tokio::test]
async fn test_basic_circuit_breaker_functionality() {
    println!("=== Basic Circuit Breaker Test ===");

    let context = SimpleFaultContext::new("basic_test").await;

    // Setup failing endpoint
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&context.mock_server)
        .await;

    let test_url = format!("{}/api/test", context.base_url());

    // Initial state should be closed
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );
    println!("✓ Circuit breaker starts in Closed state");

    // Make requests to trigger circuit breaker
    let mut failures = 0;
    for i in 0..6 {
        let result = context.make_request(&test_url).await;

        match result {
            Err(RadarrError::ExternalServiceError { .. }) => {
                failures += 1;
                println!("Request {}: Failed as expected", i + 1);
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                println!(
                    "Request {}: Circuit breaker opened (service: {})",
                    i + 1,
                    service
                );
                break;
            }
            other => {
                println!("Request {}: Unexpected result: {:?}", i + 1, other);
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Circuit should be open
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Open
    );
    println!("✓ Circuit breaker opened after {} failures", failures);

    let metrics = context.circuit_breaker.get_metrics().await;
    println!(
        "✓ Final metrics: {} total, {} successful, {} failed, {} rejected",
        metrics.total_requests,
        metrics.successful_requests,
        metrics.failed_requests,
        metrics.rejected_requests
    );

    assert!(failures >= 3, "Should have at least 3 failures");
    assert!(
        metrics.failed_requests >= 3,
        "Metrics should show at least 3 failures"
    );

    println!("✓ Basic circuit breaker functionality test passed!");
}

#[tokio::test]
async fn test_service_recovery_scenario() {
    println!("=== Service Recovery Test ===");

    let context = SimpleFaultContext::new("recovery_test").await;

    // First setup failing endpoint
    Mock::given(method("GET"))
        .and(path("/api/recover"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Service Down"))
        .up_to_n_times(3)
        .mount(&context.mock_server)
        .await;

    // Then setup working endpoint
    Mock::given(method("GET"))
        .and(path("/api/recover"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "recovered",
            "message": "Service is back online"
        })))
        .mount(&context.mock_server)
        .await;

    let recover_url = format!("{}/api/recover", context.base_url());

    // Trigger failures to open circuit
    for i in 0..3 {
        let result = context.make_request(&recover_url).await;
        assert!(result.is_err(), "Request {} should fail", i + 1);
        println!("Request {}: Failed as expected", i + 1);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Open
    );
    println!("✓ Circuit breaker opened due to failures");

    // Wait for circuit to transition to half-open
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Make request that should succeed
    let recovery_result = context.make_request(&recover_url).await;
    assert!(recovery_result.is_ok(), "Recovery request should succeed");

    if let Ok(body) = recovery_result {
        assert!(body.contains("recovered"));
        println!("✓ Service recovered: {}", body);
    }

    // Circuit should be closed again
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );
    println!("✓ Circuit breaker closed after successful recovery");

    let metrics = context.circuit_breaker.get_metrics().await;
    println!(
        "✓ Recovery metrics: {} total, {} successful, {} failed",
        metrics.total_requests, metrics.successful_requests, metrics.failed_requests
    );

    assert!(
        metrics.successful_requests > 0,
        "Should have successful requests"
    );
    assert!(metrics.failed_requests >= 3, "Should have failed requests");

    println!("✓ Service recovery scenario test passed!");
}

#[tokio::test]
async fn test_timeout_scenario() {
    println!("=== Timeout Scenario Test ===");

    let context = SimpleFaultContext::new("timeout_test").await;

    // Setup endpoint that times out
    Mock::given(method("GET"))
        .and(path("/api/timeout"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30)) // Much longer than circuit breaker timeout
                .set_body_string("Too slow"),
        )
        .mount(&context.mock_server)
        .await;

    let timeout_url = format!("{}/api/timeout", context.base_url());

    let start_time = Instant::now();
    let result = context.make_request(&timeout_url).await;
    let elapsed = start_time.elapsed();

    assert!(result.is_err(), "Timeout request should fail");
    assert!(
        elapsed < Duration::from_secs(10),
        "Should timeout quickly: {:?}",
        elapsed
    );

    if let Err(error) = result {
        match error {
            RadarrError::Timeout { operation } => {
                println!("✓ Correctly detected timeout: {}", operation);
            }
            other => {
                println!("Timeout detected as: {:?}", other);
            }
        }
    }

    let metrics = context.circuit_breaker.get_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);

    println!("✓ Timeout scenario test passed!");
}

#[tokio::test]
async fn test_rate_limiting_scenario() {
    println!("=== Rate Limiting Scenario Test ===");

    let context = SimpleFaultContext::new("rate_limit_test").await;

    // Setup rate limited endpoint
    Mock::given(method("GET"))
        .and(path("/api/limited"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded")
                .insert_header("Retry-After", "60"),
        )
        .mount(&context.mock_server)
        .await;

    let limited_url = format!("{}/api/limited", context.base_url());

    let client = reqwest::Client::new();
    let result =
        context
            .circuit_breaker
            .call(async {
                let response = client.get(&limited_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("HTTP request failed: {}", e),
                    }
                })?;

                let status = response.status();
                if status == 429 {
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse().ok());

                    return Err(RadarrError::RateLimited {
                        service: "test_service".to_string(),
                        retry_after,
                    });
                }

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response body: {}", e),
                    })?;

                Ok::<String, RadarrError>(body)
            })
            .await;

    assert!(result.is_err(), "Rate limited request should fail");

    if let Err(RadarrError::RateLimited {
        service,
        retry_after,
    }) = result
    {
        assert_eq!(service, "test_service");
        assert_eq!(retry_after, Some(60));
        println!(
            "✓ Rate limiting detected correctly: retry after {} seconds",
            retry_after.unwrap()
        );
    } else {
        panic!("Expected RateLimited error, got: {:?}", result);
    }

    let metrics = context.circuit_breaker.get_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);

    println!("✓ Rate limiting scenario test passed!");
}

#[tokio::test]
async fn test_concurrent_fault_scenarios() {
    println!("=== Concurrent Fault Scenarios Test ===");

    let context = SimpleFaultContext::new("concurrent_test").await;

    // Setup failing endpoint
    Mock::given(method("GET"))
        .and(path("/api/concurrent"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&context.mock_server)
        .await;

    let concurrent_url = format!("{}/api/concurrent", context.base_url());

    // Make concurrent requests
    let mut handles = Vec::new();
    for i in 0..5 {
        let context_ref = &context;
        let url = concurrent_url.clone();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = context_ref.make_request(&url).await;
            let elapsed = start.elapsed();
            (i, result, elapsed)
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    let mut service_errors = 0;
    let mut circuit_rejections = 0;

    for result in results {
        let (request_id, response, elapsed) = result.unwrap();

        match response {
            Err(RadarrError::ExternalServiceError { .. }) => {
                service_errors += 1;
                println!("Request {}: Service error in {:?}", request_id, elapsed);
            }
            Err(RadarrError::TemporaryError { .. }) => {
                service_errors += 1;
                println!("Request {}: Temporary error in {:?}", request_id, elapsed);
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                circuit_rejections += 1;
                println!(
                    "Request {}: Circuit breaker rejection in {:?}",
                    request_id, elapsed
                );
            }
            other => {
                println!(
                    "Request {}: Unexpected result: {:?} in {:?}",
                    request_id, other, elapsed
                );
            }
        }
    }

    assert!(service_errors > 0, "Should have service errors");
    println!(
        "✓ Concurrent requests handled: {} service errors, {} circuit rejections",
        service_errors, circuit_rejections
    );

    let metrics = context.circuit_breaker.get_metrics().await;
    assert_eq!(metrics.total_requests, 5);
    assert!(metrics.failed_requests > 0 || metrics.rejected_requests > 0);

    println!("✓ Concurrent fault scenarios test passed!");
}

#[tokio::test]
async fn test_complete_fault_injection_workflow() {
    println!("=== Complete Fault Injection Workflow Test ===");

    let indexer = SimpleFaultContext::new("workflow_indexer").await;
    let downloader = SimpleFaultContext::new("workflow_downloader").await;

    // Phase 1: Both services working
    println!("\n--- Phase 1: Normal Operation ---");

    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title": "Test Movie", "quality": "1080p"}]
        })))
        .up_to_n_times(2)
        .mount(&indexer.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "downloading", "progress": 0.5
        })))
        .up_to_n_times(2)
        .mount(&downloader.mock_server)
        .await;

    let indexer_url = format!("{}/search", indexer.base_url());
    let downloader_url = format!("{}/status", downloader.base_url());

    // Test normal operation
    let indexer_result = indexer.make_request(&indexer_url).await;
    let downloader_result = downloader.make_request(&downloader_url).await;

    assert!(indexer_result.is_ok(), "Indexer should work");
    assert!(downloader_result.is_ok(), "Downloader should work");
    println!("✓ Both services operational");

    // Phase 2: Introduce failures
    println!("\n--- Phase 2: Fault Injection ---");

    // Indexer starts failing
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Indexer Error"))
        .mount(&indexer.mock_server)
        .await;

    // Downloader gets rate limited
    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(ResponseTemplate::new(429).set_body_string("Rate Limited"))
        .mount(&downloader.mock_server)
        .await;

    // Test fault behavior
    for i in 0..4 {
        let indexer_result = indexer.make_request(&indexer_url).await;
        let downloader_result = downloader.make_request(&downloader_url).await;

        println!(
            "Fault test {}: indexer={}, downloader={}",
            i + 1,
            indexer_result.is_ok(),
            downloader_result.is_ok()
        );

        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Phase 3: Check circuit breaker states
    println!("\n--- Phase 3: Circuit Breaker States ---");

    let indexer_state = indexer.circuit_breaker.get_state().await;
    let downloader_state = downloader.circuit_breaker.get_state().await;

    println!("Indexer circuit: {:?}", indexer_state);
    println!("Downloader circuit: {:?}", downloader_state);

    // At least one should have opened
    assert!(
        indexer_state == CircuitBreakerState::Open || downloader_state == CircuitBreakerState::Open
    );

    // Phase 4: Service recovery
    println!("\n--- Phase 4: Service Recovery ---");

    tokio::time::sleep(Duration::from_millis(150)).await;

    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{"title": "Recovered Movie"}]
        })))
        .mount(&indexer.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "completed", "progress": 1.0
        })))
        .mount(&downloader.mock_server)
        .await;

    // Test recovery
    let indexer_recovery = indexer.make_request(&indexer_url).await;
    let downloader_recovery = downloader.make_request(&downloader_url).await;

    println!(
        "Recovery: indexer={}, downloader={}",
        indexer_recovery.is_ok(),
        downloader_recovery.is_ok()
    );

    // Final metrics
    let indexer_metrics = indexer.circuit_breaker.get_metrics().await;
    let downloader_metrics = downloader.circuit_breaker.get_metrics().await;

    println!("\n--- Final Metrics ---");
    println!(
        "Indexer: {} total, {} success, {} failed",
        indexer_metrics.total_requests,
        indexer_metrics.successful_requests,
        indexer_metrics.failed_requests
    );
    println!(
        "Downloader: {} total, {} success, {} failed",
        downloader_metrics.total_requests,
        downloader_metrics.successful_requests,
        downloader_metrics.failed_requests
    );

    println!("✓ Complete fault injection workflow test passed!");
}
