//! Download stall fault injection tests
//!
//! Tests the system's behavior when downloads become stalled or interrupted.
//! Validates:
//! - Detection of stalled downloads
//! - Timeout handling for stuck transfers
//! - Resource cleanup after download failures
//! - Retry logic for interrupted downloads
//! - Circuit breaker behavior with download client issues
//! - Graceful handling of download client disconnections

use super::*;
use radarr_core::circuit_breaker::CircuitBreakerState;
use radarr_core::RadarrError;
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, ResponseTemplate, Times};

/// Test download client connection timeout
#[tokio::test]
async fn test_download_client_connection_timeout() {
    let context = FaultInjectionTestContext::new("qbittorrent_client").await;

    // Setup download client endpoint that never responds
    context.setup_timeout_endpoint("/api/v2/auth/login").await;

    let login_url = format!("{}/api/v2/auth/login", context.base_url());

    let start_time = Instant::now();
    let result = context.make_request(&login_url).await;
    let elapsed = start_time.elapsed();

    // Should fail with timeout error
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            RadarrError::Timeout { operation } => {
                assert!(operation.contains("qbittorrent_client"));
            }
            _ => panic!("Expected timeout error, got: {:?}", error),
        }
    }

    // Should timeout within circuit breaker request timeout
    assert!(elapsed < Duration::from_secs(6));

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test download add request stalling
#[tokio::test]
async fn test_download_add_stall() {
    let context = FaultInjectionTestContext::new("download_client").await;

    // Setup successful login
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&context.mock_server)
        .await;

    // Setup add torrent endpoint that stalls (very long delay)
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30))
                .set_body_string("Ok."),
        )
        .mount(&context.mock_server)
        .await;

    // Test login first (should work)
    let login_url = format!("{}/api/v2/auth/login", context.base_url());
    let login_result = context.make_request(&login_url).await;
    assert!(login_result.is_ok());

    // Test add torrent (should timeout)
    let add_url = format!("{}/api/v2/torrents/add", context.base_url());
    let start_time = Instant::now();

    let client = reqwest::Client::new();
    let add_result = context
        .circuit_breaker
        .call(async {
            let response = client
                .post(&add_url)
                .form(&[("urls", "magnet:?xt=urn:btih:test"), ("category", "movies")])
                .send()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Add torrent request failed: {}", e),
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

    // Should fail with timeout
    assert!(add_result.is_err());
    if let Err(RadarrError::Timeout { .. }) = add_result {
        println!("Add torrent timed out as expected after {:?}", elapsed);
    } else {
        panic!("Expected timeout error, got: {:?}", add_result);
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 1); // Login succeeded
    assert_eq!(metrics.failed_requests, 1); // Add torrent failed
}

/// Test download progress monitoring with stalled transfers
#[tokio::test]
async fn test_stalled_download_detection() {
    let context = FaultInjectionTestContext::new("download_monitor").await;

    // Setup endpoints for download monitoring
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "abc123",
                "name": "Test Movie 2024",
                "size": 1073741824,
                "downloaded": 536870912,
                "progress": 0.5,
                "dlspeed": 0,
                "state": "stalledDL",
                "priority": 1,
                "eta": 8640000
            }
        ])))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());

    // Monitor for several iterations
    let mut stall_detected = false;
    for i in 0..3 {
        let result = context.make_request(&info_url).await;
        assert!(result.is_ok());

        if let Ok(body) = result {
            if body.contains("stalledDL") {
                stall_detected = true;
                println!("Iteration {}: Stalled download detected", i + 1);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(stall_detected, "Should have detected stalled download");

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 3);
    assert_eq!(metrics.failed_requests, 0);
}

/// Test download client disconnection during transfer
#[tokio::test]
async fn test_download_client_disconnection() {
    let context = FaultInjectionTestContext::new("download_client").await;

    // First few requests work (client is connected)
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"hash": "abc123", "state": "downloading", "progress": 0.3}
        ])))
        .up_to_n_times(2)
        .mount(&context.mock_server)
        .await;

    // Then client becomes unresponsive (connection lost)
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Connection refused"))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());

    // First requests should work
    for i in 0..2 {
        let result = context.make_request(&info_url).await;
        assert!(result.is_ok(), "Request {} should succeed", i + 1);

        if let Ok(body) = result {
            assert!(body.contains("downloading"));
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Subsequent requests should fail (client disconnected)
    for i in 2..5 {
        let result = context.make_request(&info_url).await;
        println!("Request {} result: {:?}", i + 1, result.is_ok());

        if result.is_err() {
            if let Err(RadarrError::CircuitBreakerOpen { .. }) = result {
                println!("Circuit breaker opened due to client disconnection");
                break;
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let metrics = context.get_test_metrics().await;
    assert!(metrics.successful_requests >= 2);
    assert!(metrics.failed_requests > 0);
}

/// Test download retry after stall recovery
#[tokio::test]
async fn test_download_stall_recovery() {
    let context = FaultInjectionTestContext::new("recovery_client").await;

    // First, simulate stalled downloads
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "abc123",
                "name": "Test Movie",
                "progress": 0.3,
                "dlspeed": 0,
                "state": "stalledDL"
            }
        ])))
        .up_to_n_times(3)
        .mount(&context.mock_server)
        .await;

    // Then simulate recovery (downloads resume)
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "abc123",
                "name": "Test Movie",
                "progress": 0.8,
                "dlspeed": 1048576,
                "state": "downloading"
            }
        ])))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());

    // Monitor stalled state
    for i in 0..3 {
        let result = context.make_request(&info_url).await;
        assert!(result.is_ok());

        if let Ok(body) = result {
            assert!(body.contains("stalledDL"));
            println!("Request {}: Download still stalled", i + 1);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Should still be in closed state (stalls aren't failures)
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );

    // Check recovery
    let recovery_result = context.make_request(&info_url).await;
    assert!(recovery_result.is_ok());

    if let Ok(body) = recovery_result {
        assert!(body.contains("downloading"));
        assert!(body.contains("1048576")); // Download speed > 0
        println!("Download recovered successfully");
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 4); // All monitoring requests succeeded
    assert_eq!(metrics.failed_requests, 0);
}

/// Test multiple concurrent download stalls
#[tokio::test]
async fn test_concurrent_download_stalls() {
    let context = FaultInjectionTestContext::new("concurrent_downloads").await;

    // Setup endpoint returning multiple stalled downloads
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "hash1",
                "name": "Movie 1",
                "progress": 0.2,
                "dlspeed": 0,
                "state": "stalledDL"
            },
            {
                "hash": "hash2",
                "name": "Movie 2",
                "progress": 0.5,
                "dlspeed": 0,
                "state": "stalledDL"
            },
            {
                "hash": "hash3",
                "name": "Movie 3",
                "progress": 0.1,
                "dlspeed": 1024000,
                "state": "downloading"
            }
        ])))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());

    // Make concurrent monitoring requests
    let mut handles = Vec::new();
    for i in 0..3 {
        let context_ref = &context;
        let url = info_url.clone();
        let handle = tokio::spawn(async move {
            let result = context_ref.make_request(&url).await;
            (i, result)
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(handles).await;

    let mut stalled_count = 0;
    let mut downloading_count = 0;

    for result in results {
        let (request_id, response) = result.unwrap();
        assert!(response.is_ok(), "Request {} should succeed", request_id);

        if let Ok(body) = response {
            // Count stalled vs active downloads in response
            let stalled_matches = body.matches("stalledDL").count();
            let downloading_matches = body.matches("\"downloading\"").count();

            stalled_count += stalled_matches;
            downloading_count += downloading_matches;

            println!(
                "Request {}: {} stalled, {} downloading",
                request_id, stalled_matches, downloading_matches
            );
        }
    }

    // Should detect multiple stalled downloads but at least one active
    assert!(stalled_count >= 6); // 2 stalled downloads * 3 requests
    assert!(downloading_count >= 3); // 1 downloading * 3 requests

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 3);
    assert_eq!(metrics.failed_requests, 0);
}

/// Test download bandwidth throttling detection
#[tokio::test]
async fn test_download_bandwidth_throttling() {
    let context = FaultInjectionTestContext::new("throttled_downloads").await;

    // Setup endpoint showing throttled downloads (very slow speed)
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "throttled1",
                "name": "Large Movie",
                "size": 5368709120, // 5GB
                "downloaded": 1073741824, // 1GB
                "progress": 0.2,
                "dlspeed": 1024, // Very slow: 1KB/s
                "upspeed": 0,
                "state": "downloading",
                "eta": 4194304 // Very long ETA
            }
        ])))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());

    // Monitor several times to detect consistent throttling
    let mut throttling_detected = false;
    let mut total_speed = 0u64;
    let monitor_count = 3;

    for i in 0..monitor_count {
        let result = context.make_request(&info_url).await;
        assert!(result.is_ok());

        if let Ok(body) = result {
            // Parse speed from response (in a real implementation this would be proper JSON parsing)
            if body.contains("\"dlspeed\": 1024") {
                total_speed += 1024;
                println!("Monitor {}: Detected throttled speed (1024 B/s)", i + 1);
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let avg_speed = total_speed / monitor_count as u64;
    if avg_speed <= 2048 {
        // Less than 2KB/s average is considered throttling
        throttling_detected = true;
    }

    assert!(
        throttling_detected,
        "Should have detected bandwidth throttling"
    );
    println!(
        "Average download speed: {} B/s (throttling detected: {})",
        avg_speed, throttling_detected
    );

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, monitor_count as u64);
    assert_eq!(metrics.failed_requests, 0);
}

/// Test download cleanup after client failure
#[tokio::test]
async fn test_download_cleanup_after_failure() {
    let context = FaultInjectionTestContext::new("cleanup_client").await;

    // Setup initial successful state
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"hash": "cleanup1", "state": "downloading"}
        ])))
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Setup delete torrent endpoint (for cleanup)
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&context.mock_server)
        .await;

    // Then client starts failing
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());
    let delete_url = format!("{}/api/v2/torrents/delete", context.base_url());

    // First request should work
    let result = context.make_request(&info_url).await;
    assert!(result.is_ok());

    // Subsequent requests should fail, triggering cleanup
    let mut failures = 0;
    for _ in 0..4 {
        let result = context.make_request(&info_url).await;
        if result.is_err() {
            failures += 1;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Should have failed enough to open circuit
    assert!(failures >= 3);
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Open
    );

    // Simulate cleanup operation (would normally be triggered by failure detection)
    let client = reqwest::Client::new();
    let cleanup_result = client
        .post(&delete_url)
        .form(&[("hashes", "cleanup1"), ("deleteFiles", "false")])
        .send()
        .await;

    assert!(cleanup_result.is_ok());
    if let Ok(response) = cleanup_result {
        assert_eq!(response.status(), 200);
        println!("Cleanup operation completed successfully");
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 1); // Only first request succeeded
    assert!(metrics.failed_requests >= 3);
    assert!(metrics.rejected_requests > 0);
}

/// Test download resume after interruption
#[tokio::test]
async fn test_download_resume_after_interruption() {
    let context = FaultInjectionTestContext::new("resume_client").await;

    // Phase 1: Download in progress
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "resume1",
                "progress": 0.4,
                "state": "downloading",
                "dlspeed": 5242880
            }
        ])))
        .up_to_n_times(2)
        .mount(&context.mock_server)
        .await;

    // Phase 2: Download interrupted (network error)
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .up_to_n_times(2)
        .mount(&context.mock_server)
        .await;

    // Phase 3: Download resumes
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "resume1",
                "progress": 0.4,
                "state": "downloading",
                "dlspeed": 3145728
            }
        ])))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());

    // Phase 1: Normal operation
    for i in 0..2 {
        let result = context.make_request(&info_url).await;
        assert!(result.is_ok(), "Phase 1 request {} should succeed", i + 1);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Phase 2: Interruption
    for i in 0..2 {
        let result = context.make_request(&info_url).await;
        println!("Phase 2 request {} result: {:?}", i + 1, result.is_ok());
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Circuit should be open due to failures
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

    // Phase 3: Resume
    let resume_result = context.make_request(&info_url).await;
    assert!(resume_result.is_ok(), "Resume request should succeed");

    if let Ok(body) = resume_result {
        assert!(body.contains("downloading"));
        println!("Download successfully resumed");
    }

    // Circuit should close again
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );

    let metrics = context.get_test_metrics().await;
    assert!(metrics.successful_requests >= 3); // Phase 1 + resume
    assert!(metrics.failed_requests >= 2); // Phase 2 interruptions
    assert!(metrics.is_resilient());
}

#[cfg(test)]
mod download_integration_tests {
    use super::*;

    /// Integration test: Download workflow with multiple failure points
    #[tokio::test]
    async fn test_complete_download_failure_workflow() {
        let download_client = FaultInjectionTestContext::new("complete_workflow").await;

        // Step 1: Add torrent (works)
        Mock::given(method("POST"))
            .and(path("/api/v2/torrents/add"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
            .mount(&download_client.mock_server)
            .await;

        // Step 2: Monitor download (fails after initial success)
        Mock::given(method("GET"))
            .and(path("/api/v2/torrents/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"hash": "workflow1", "state": "downloading", "progress": 0.1}
            ])))
            .up_to_n_times(2)
            .mount(&download_client.mock_server)
            .await;

        // Then monitoring fails (client issues)
        Mock::given(method("GET"))
            .and(path("/api/v2/torrents/info"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&download_client.mock_server)
            .await;

        let add_url = format!("{}/api/v2/torrents/add", download_client.base_url());
        let info_url = format!("{}/api/v2/torrents/info", download_client.base_url());

        // Step 1: Add torrent
        let client = reqwest::Client::new();
        let add_result = download_client
            .circuit_breaker
            .call(async {
                let response = client
                    .post(&add_url)
                    .form(&[("urls", "magnet:?xt=urn:btih:test")])
                    .send()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Add request failed: {}", e),
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

        assert!(add_result.is_ok(), "Adding torrent should succeed");

        // Step 2: Monitor download (initial success)
        for i in 0..2 {
            let result = download_client.make_request(&info_url).await;
            assert!(
                result.is_ok(),
                "Initial monitoring {} should succeed",
                i + 1
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Step 3: Monitor fails (triggers circuit breaker)
        for i in 0..4 {
            let result = download_client.make_request(&info_url).await;
            println!("Monitoring failure {}: {:?}", i + 1, result.is_ok());
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Circuit should be open
        assert_eq!(
            download_client.circuit_breaker.get_state().await,
            CircuitBreakerState::Open
        );

        let metrics = download_client.get_test_metrics().await;
        assert!(metrics.successful_requests >= 3); // Add + 2 successful monitors
        assert!(metrics.failed_requests >= 3); // Failed monitoring attempts
        assert!(metrics.rejected_requests > 0); // Circuit breaker rejections

        println!("Complete workflow metrics: {:?}", metrics);
    }
}
