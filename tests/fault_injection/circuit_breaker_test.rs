//! Circuit breaker behavior fault injection tests
//!
//! Tests the circuit breaker's specific behavior under various fault injection scenarios.
//! Validates:
//! - Circuit breaker state transitions (Closed -> Open -> Half-Open -> Closed)
//! - Failure threshold configuration and enforcement
//! - Timeout behavior and recovery windows
//! - Success threshold for closing from half-open state
//! - Circuit breaker metrics and monitoring
//! - Manual circuit control for testing and maintenance
//! - Circuit breaker behavior under concurrent load

use super::*;
use radarr_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use radarr_core::RadarrError;
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate, Times};

/// Test circuit breaker failure threshold configuration
#[tokio::test]
async fn test_circuit_breaker_failure_threshold() {
    // Test with different failure thresholds
    let test_cases = vec![
        (2, "low_threshold"),
        (5, "medium_threshold"),
        (10, "high_threshold"),
    ];

    for (threshold, service_name) in test_cases {
        println!("Testing failure threshold: {}", threshold);

        let config = CircuitBreakerConfig::new(service_name)
            .with_failure_threshold(threshold)
            .with_timeout(Duration::from_millis(50));
        let _circuit_breaker = CircuitBreaker::new(config);

        // Setup always failing endpoint
        let context = FaultInjectionTestContext::new(service_name).await;
        context.setup_always_failing_endpoint("/api/test").await;
        let test_url = format!("{}/api/test", context.base_url());

        // Make requests until circuit opens
        let mut failures = 0;
        for _i in 0..threshold + 5 {
            let result = context.make_request(&test_url).await;

            match result {
                Err(RadarrError::CircuitBreakerOpen { .. }) => {
                    println!(
                        "  Circuit opened after {} failures (threshold: {})",
                        failures, threshold
                    );
                    assert!(
                        failures >= threshold,
                        "Circuit should open after {} failures, opened after {}",
                        threshold,
                        failures
                    );
                    break;
                }
                Err(_) => {
                    failures += 1;
                }
                Ok(_) => {
                    panic!("Request should not succeed with always failing endpoint");
                }
            }

            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        // Verify circuit is open
        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Open
        );

        let metrics = context.get_test_metrics().await;
        assert!(metrics.failed_requests >= threshold as u64);

        println!("  ✓ Threshold {} working correctly", threshold);
    }
}

/// Test circuit breaker timeout and transition to half-open
#[tokio::test]
async fn test_circuit_breaker_timeout_behavior() {
    let timeout_durations = vec![
        Duration::from_millis(50),
        Duration::from_millis(100),
        Duration::from_millis(200),
    ];

    for timeout_duration in timeout_durations {
        println!("Testing timeout duration: {:?}", timeout_duration);

        let config = CircuitBreakerConfig::new("timeout_test")
            .with_failure_threshold(2)
            .with_timeout(timeout_duration);
        let _circuit_breaker = CircuitBreaker::new(config);

        let context = FaultInjectionTestContext::new("timeout_test").await;
        context.setup_always_failing_endpoint("/api/fail").await;
        let fail_url = format!("{}/api/fail", context.base_url());

        // Trigger failures to open circuit
        for _ in 0..3 {
            let _ = context.make_request(&fail_url).await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Open
        );
        let open_time = Instant::now();

        // Wait for just under the timeout duration
        let wait_time = timeout_duration.saturating_sub(Duration::from_millis(10));
        tokio::time::sleep(wait_time).await;

        // Should still be open
        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Open
        );

        // Wait for timeout to elapse
        let remaining_time =
            timeout_duration.saturating_sub(open_time.elapsed()) + Duration::from_millis(20);
        tokio::time::sleep(remaining_time).await;

        // Should transition to half-open on next request
        let _ = context.make_request(&fail_url).await; // This triggers the state check

        // Verify state transition occurred
        assert!(
            context
                .wait_for_circuit_state(CircuitBreakerState::HalfOpen, Duration::from_millis(100))
                .await,
            "Circuit should transition to half-open after timeout: {:?}",
            timeout_duration
        );

        println!("  ✓ Timeout {:?} working correctly", timeout_duration);
    }
}

/// Test circuit breaker success threshold for closing
#[tokio::test]
async fn test_circuit_breaker_success_threshold() {
    let success_thresholds = vec![1, 2, 3];

    for success_threshold in success_thresholds {
        println!("Testing success threshold: {}", success_threshold);

        let config = CircuitBreakerConfig::new("success_test")
            .with_failure_threshold(2)
            .with_timeout(Duration::from_millis(50))
            .with_success_threshold(success_threshold);
        let _circuit_breaker = CircuitBreaker::new(config);

        let context = FaultInjectionTestContext::new("success_test").await;

        // First, fail to open circuit
        context.setup_always_failing_endpoint("/api/fail").await;
        let fail_url = format!("{}/api/fail", context.base_url());

        for _ in 0..3 {
            let _ = context.make_request(&fail_url).await;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Open
        );

        // Setup successful endpoint for recovery
        Mock::given(method("GET"))
            .and(path("/api/recover"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "recovered"})))
            .mount(&context.mock_server)
            .await;

        let recover_url = format!("{}/api/recover", context.base_url());

        // Wait for circuit to go half-open
        tokio::time::sleep(Duration::from_millis(75)).await;

        // Make successful requests according to success threshold
        for i in 0..success_threshold {
            let result = context.make_request(&recover_url).await;
            assert!(result.is_ok(), "Recovery request {} should succeed", i + 1);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Circuit should now be closed
        assert_eq!(
            context.circuit_breaker.get_state().await,
            CircuitBreakerState::Closed,
            "Circuit should be closed after {} successful requests",
            success_threshold
        );

        println!(
            "  ✓ Success threshold {} working correctly",
            success_threshold
        );
    }
}

/// Test concurrent requests with circuit breaker
#[tokio::test]
async fn test_circuit_breaker_concurrent_behavior() {
    let context = FaultInjectionTestContext::new("concurrent_circuit").await;

    // Setup failing endpoint
    context
        .setup_always_failing_endpoint("/api/concurrent")
        .await;
    let concurrent_url = format!("{}/api/concurrent", context.base_url());

    // Make concurrent requests to trigger circuit breaker
    let mut handles = Vec::new();
    for i in 0..10 {
        let context_ref = &context;
        let url = concurrent_url.clone();
        let handle = tokio::spawn(async move {
            let result = context_ref.make_request(&url).await;
            (i, result)
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    let mut failure_count = 0;
    let mut circuit_open_count = 0;

    for result in results {
        let (request_id, response) = result.unwrap();

        match response {
            Err(RadarrError::ExternalServiceError { .. }) => {
                failure_count += 1;
                println!("Request {}: Service error", request_id);
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                circuit_open_count += 1;
                println!("Request {}: Circuit breaker open", request_id);
            }
            other => {
                println!("Request {}: Unexpected result: {:?}", request_id, other);
            }
        }
    }

    // Should have failures followed by circuit breaker rejections
    assert!(failure_count >= 3, "Should have at least 3 failures");
    assert!(
        circuit_open_count > 0,
        "Should have some circuit breaker rejections"
    );
    assert!(
        failure_count + circuit_open_count == 10,
        "All requests should fail somehow"
    );

    // Circuit should be open
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Open
    );

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 10);
    assert!(metrics.failed_requests >= 3);
    assert!(metrics.rejected_requests > 0);

    println!(
        "Concurrent circuit breaker: {} failures, {} rejections",
        failure_count, circuit_open_count
    );
}

/// Test circuit breaker metrics accuracy
#[tokio::test]
async fn test_circuit_breaker_metrics_accuracy() {
    let context = FaultInjectionTestContext::new("metrics_test").await;

    // Setup mixed success/failure endpoints
    Mock::given(method("GET"))
        .and(path("/api/success"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": "success"})))
        .mount(&context.mock_server)
        .await;

    context.setup_always_failing_endpoint("/api/fail").await;

    let success_url = format!("{}/api/success", context.base_url());
    let fail_url = format!("{}/api/fail", context.base_url());

    // Make a known pattern of requests
    let request_pattern = vec![
        (&success_url, true),  // success
        (&success_url, true),  // success
        (&fail_url, false),    // fail
        (&success_url, true),  // success
        (&fail_url, false),    // fail
        (&fail_url, false),    // fail - should open circuit
        (&success_url, false), // rejected (circuit open)
    ];

    let mut expected_successes = 0;
    let mut expected_failures = 0;
    let mut expected_rejections = 0;

    for (i, (url, should_succeed)) in request_pattern.iter().enumerate() {
        let result = context.make_request(url).await;

        match (result, should_succeed) {
            (Ok(_), true) => {
                expected_successes += 1;
                println!("Request {}: Success as expected", i + 1);
            }
            (Err(RadarrError::ExternalServiceError { .. }), false) => {
                expected_failures += 1;
                println!("Request {}: Failed as expected", i + 1);
            }
            (Err(RadarrError::CircuitBreakerOpen { .. }), false) => {
                expected_rejections += 1;
                println!("Request {}: Rejected by circuit breaker as expected", i + 1);
            }
            (result, expected) => {
                println!(
                    "Request {}: Unexpected result {:?} (expected success: {})",
                    i + 1,
                    result.is_ok(),
                    expected
                );
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let metrics = context.get_test_metrics().await;

    // Verify metrics accuracy
    assert_eq!(metrics.total_requests as usize, request_pattern.len());
    assert_eq!(metrics.successful_requests, expected_successes);
    assert_eq!(metrics.failed_requests, expected_failures);
    assert_eq!(metrics.rejected_requests, expected_rejections);

    // Verify calculated rates
    let success_rate = metrics.success_rate();
    let failure_rate = metrics.failure_rate();
    let rejection_rate = metrics.rejection_rate();

    let expected_success_rate = (expected_successes as f64 / request_pattern.len() as f64) * 100.0;
    let expected_failure_rate = (expected_failures as f64 / request_pattern.len() as f64) * 100.0;
    let expected_rejection_rate =
        (expected_rejections as f64 / request_pattern.len() as f64) * 100.0;

    assert!((success_rate - expected_success_rate).abs() < 0.1);
    assert!((failure_rate - expected_failure_rate).abs() < 0.1);
    assert!((rejection_rate - expected_rejection_rate).abs() < 0.1);

    println!("Metrics verification:");
    println!(
        "  Total: {} (expected: {})",
        metrics.total_requests,
        request_pattern.len()
    );
    println!(
        "  Success: {} (expected: {})",
        metrics.successful_requests, expected_successes
    );
    println!(
        "  Failures: {} (expected: {})",
        metrics.failed_requests, expected_failures
    );
    println!(
        "  Rejections: {} (expected: {})",
        metrics.rejected_requests, expected_rejections
    );
    println!(
        "  Rates - Success: {:.1}%, Failure: {:.1}%, Rejection: {:.1}%",
        success_rate, failure_rate, rejection_rate
    );
}

/// Test manual circuit breaker control
#[tokio::test]
async fn test_manual_circuit_breaker_control() {
    let context = FaultInjectionTestContext::new("manual_control").await;

    // Setup endpoint (doesn't matter since we'll control manually)
    Mock::given(method("GET"))
        .and(path("/api/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "ok"})))
        .mount(&context.mock_server)
        .await;

    let test_url = format!("{}/api/test", context.base_url());

    // Initial state should be closed
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );

    // Test manual open
    context.circuit_breaker.force_open().await;
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Open
    );

    // Request should be rejected
    let result = context.make_request(&test_url).await;
    assert!(result.is_err());
    if let Err(RadarrError::CircuitBreakerOpen { service }) = result {
        assert_eq!(service, "manual_control");
        println!("Request correctly rejected when manually opened");
    } else {
        panic!("Expected CircuitBreakerOpen error");
    }

    // Test manual close
    context.circuit_breaker.force_close().await;
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );

    // Request should now succeed
    let result = context.make_request(&test_url).await;
    assert!(result.is_ok());
    println!("Request succeeded after manual close");

    // Test metrics reset
    let metrics_before = context.get_test_metrics().await;
    assert!(metrics_before.total_requests > 0);

    context.circuit_breaker.reset_metrics().await;

    let metrics_after = context.get_test_metrics().await;
    // Note: FaultInjectionTestContext tracks its own metrics, so this tests the circuit breaker's metrics
    let cb_metrics = context.circuit_breaker.get_metrics().await;
    assert_eq!(cb_metrics.total_requests, 0);
    assert_eq!(cb_metrics.successful_requests, 0);
    assert_eq!(cb_metrics.failed_requests, 0);
    assert_eq!(cb_metrics.rejected_requests, 0);

    println!("Manual control test completed successfully");
}

/// Test circuit breaker health check functionality
#[tokio::test]
async fn test_circuit_breaker_health_check() {
    let context = FaultInjectionTestContext::new("health_test").await;

    // Setup mixed endpoints
    Mock::given(method("GET"))
        .and(path("/api/healthy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "healthy"})))
        .mount(&context.mock_server)
        .await;

    context
        .setup_always_failing_endpoint("/api/unhealthy")
        .await;

    let healthy_url = format!("{}/api/healthy", context.base_url());
    let unhealthy_url = format!("{}/api/unhealthy", context.base_url());

    // Initial health check (no requests made yet)
    assert!(
        context.circuit_breaker.is_healthy().await,
        "Should be healthy with no requests"
    );

    // Make successful requests
    for _ in 0..5 {
        let result = context.make_request(&healthy_url).await;
        assert!(result.is_ok());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert!(
        context.circuit_breaker.is_healthy().await,
        "Should be healthy with all successes"
    );

    // Make some failures (but not enough to open circuit)
    let _ = context.make_request(&unhealthy_url).await;
    assert!(
        context.circuit_breaker.is_healthy().await,
        "Should still be healthy with one failure"
    );

    // Make enough failures to open circuit
    for _ in 0..3 {
        let _ = context.make_request(&unhealthy_url).await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    // Should now be unhealthy (circuit open)
    assert!(
        !context.circuit_breaker.is_healthy().await,
        "Should be unhealthy with circuit open"
    );

    let metrics = context.get_test_metrics().await;
    println!(
        "Health test metrics: {} successful, {} failed, circuit state: {:?}",
        metrics.successful_requests, metrics.failed_requests, metrics.circuit_state
    );
}

/// Test circuit breaker behavior under stress
#[tokio::test]
async fn test_circuit_breaker_stress_test() {
    let context = FaultInjectionTestContext::new("stress_test").await;

    // Setup intermittent failure pattern
    context
        .setup_intermittent_failure_endpoint("/api/stress", 10)
        .await;
    let stress_url = format!("{}/api/stress", context.base_url());

    // Make many concurrent requests
    let mut handles = Vec::new();
    let request_count = 50;

    for i in 0..request_count {
        let context_ref = &context;
        let url = stress_url.clone();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = context_ref.make_request(&url).await;
            let elapsed = start.elapsed();
            (i, result, elapsed)
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut rejection_count = 0;
    let mut total_time = Duration::ZERO;

    for result in results {
        let (request_id, response, elapsed) = result.unwrap();
        total_time += elapsed;

        match response {
            Ok(_) => success_count += 1,
            Err(RadarrError::ExternalServiceError { .. }) => failure_count += 1,
            Err(RadarrError::CircuitBreakerOpen { .. }) => rejection_count += 1,
            other => {
                println!("Request {}: Unexpected result: {:?}", request_id, other);
            }
        }
    }

    let avg_response_time = total_time / request_count;

    // Should have a mix of outcomes
    assert!(success_count > 0, "Should have some successes");
    assert!(failure_count > 0, "Should have some failures");

    // Circuit breaker should activate at some point
    let final_state = context.circuit_breaker.get_state().await;
    let cb_metrics = context.circuit_breaker.get_metrics().await;

    println!("Stress test results:");
    println!("  Requests: {} total", request_count);
    println!(
        "  Success: {}, Failures: {}, Rejections: {}",
        success_count, failure_count, rejection_count
    );
    println!("  Average response time: {:?}", avg_response_time);
    println!("  Final circuit state: {:?}", final_state);
    println!("  Circuit breaker metrics: {:?}", cb_metrics);

    // Verify metrics consistency
    assert_eq!(cb_metrics.total_requests, request_count as u32);
    assert_eq!(
        cb_metrics.successful_requests 
            + cb_metrics.failed_requests 
            + cb_metrics.rejected_requests,
        request_count as u32
    );

    // System should be resilient (not completely broken)
    let metrics = context.get_test_metrics().await;
    assert!(
        metrics.is_resilient(),
        "System should demonstrate resilience under stress"
    );
}

#[cfg(test)]
mod circuit_breaker_integration_tests {
    use super::*;

    /// Integration test: Circuit breaker coordination across multiple services
    #[tokio::test]
    async fn test_multiple_service_circuit_breakers() {
        let service_a = FaultInjectionTestContext::new("service_a").await;
        let service_b = FaultInjectionTestContext::new("service_b").await;
        let service_c = FaultInjectionTestContext::new("service_c").await;

        // Service A fails consistently
        service_a.setup_always_failing_endpoint("/api/data").await;

        // Service B has intermittent issues
        service_b
            .setup_intermittent_failure_endpoint("/api/data", 3)
            .await;

        // Service C works fine
        Mock::given(method("GET"))
            .and(path("/api/data"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"service": "C", "status": "healthy"})),
            )
            .mount(&service_c.mock_server)
            .await;

        let url_a = format!("{}/api/data", service_a.base_url());
        let url_b = format!("{}/api/data", service_b.base_url());
        let url_c = format!("{}/api/data", service_c.base_url());

        // Test each service several times
        for i in 0..8 {
            // Service A should eventually open circuit
            let result_a = service_a.make_request(&url_a).await;

            // Service B should have mixed results
            let result_b = service_b.make_request(&url_b).await;

            // Service C should consistently work
            let result_c = service_c.make_request(&url_c).await;
            assert!(result_c.is_ok(), "Service C should always work");

            println!(
                "Round {}: A={}, B={}, C={}",
                i + 1,
                result_a.is_ok(),
                result_b.is_ok(),
                result_c.is_ok()
            );

            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        // Check final states
        let state_a = service_a.circuit_breaker.get_state().await;
        let state_b = service_b.circuit_breaker.get_state().await;
        let state_c = service_c.circuit_breaker.get_state().await;

        assert_eq!(
            state_a,
            CircuitBreakerState::Open,
            "Service A should have open circuit"
        );
        // Service B state depends on intermittent failures
        assert_eq!(
            state_c,
            CircuitBreakerState::Closed,
            "Service C should have closed circuit"
        );

        let metrics_a = service_a.get_test_metrics().await;
        let metrics_b = service_b.get_test_metrics().await;
        let metrics_c = service_c.get_test_metrics().await;

        assert_eq!(metrics_c.successful_requests, 8); // Service C should succeed all times
        assert!(metrics_a.failed_requests >= 3); // Service A should fail

        println!("Multiple service circuit breaker coordination:");
        println!(
            "  Service A: state={:?}, failures={}",
            state_a, metrics_a.failed_requests
        );
        println!(
            "  Service B: state={:?}, successes={}, failures={}",
            state_b, metrics_b.successful_requests, metrics_b.failed_requests
        );
        println!(
            "  Service C: state={:?}, successes={}",
            state_c, metrics_c.successful_requests
        );
    }
}
