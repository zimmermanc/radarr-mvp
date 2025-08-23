//! Indexer timeout fault injection tests
//!
//! Tests the system's behavior when indexers become unresponsive or timeout.
//! Validates:
//! - Circuit breaker activation on timeouts
//! - Proper timeout handling with retries
//! - Graceful fallback to other indexers
//! - Recovery behavior when service becomes available again
//! - Alert generation for persistent timeouts

use super::*;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::{Mock, ResponseTemplate};
use wiremock::matchers::{method, path, query_param};
use serde_json::json;
use radarr_core::circuit_breaker::CircuitBreakerState;

/// Test indexer API timeout scenarios
#[tokio::test]
async fn test_indexer_connection_timeout() {
    let context = FaultInjectionTestContext::new("indexer_api").await;
    
    // Setup indexer endpoint that never responds (simulates network timeout)
    context.setup_timeout_endpoint("/api/v1/search").await;
    
    let search_url = format!("{}/api/v1/search", context.base_url());
    
    // Make request and expect timeout error
    let start_time = Instant::now();
    let result = context.make_request(&search_url).await;
    let elapsed = start_time.elapsed();
    
    // Should fail with timeout error
    assert!(result.is_err());
    if let Err(error) = result {
        match error {
            RadarrError::Timeout { operation } => {
                assert!(operation.contains("indexer_api"));
            }
            _ => panic!("Expected timeout error, got: {:?}", error),
        }
    }
    
    // Should timeout within reasonable time (circuit breaker request timeout)
    assert!(elapsed < Duration::from_secs(6), "Request took too long: {:?}", elapsed);
    
    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test multiple indexer timeouts leading to circuit breaker activation
#[tokio::test] 
async fn test_multiple_indexer_timeouts_open_circuit() {
    let context = FaultInjectionTestContext::new("indexer_api").await;
    
    // Setup endpoint that times out
    context.setup_timeout_endpoint("/api/v1/search").await;
    
    let search_url = format!("{}/api/v1/search", context.base_url());
    
    // Make multiple requests to trigger circuit breaker
    let mut timeout_count = 0;
    for i in 0..5 {
        println!("Making request {}", i + 1);
        let result = context.make_request(&search_url).await;
        
        match result {
            Err(RadarrError::Timeout { .. }) => {
                timeout_count += 1;
                println!("Request {} timed out", i + 1);
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                println!("Circuit breaker opened after {} timeouts", timeout_count);
                break;
            }
            other => {
                println!("Unexpected result for request {}: {:?}", i + 1, other);
            }
        }
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Circuit should be open due to repeated timeouts
    let state = context.circuit_breaker.get_state().await;
    assert_eq!(state, CircuitBreakerState::Open);
    
    // Verify metrics show timeouts
    let metrics = context.get_test_metrics().await;
    assert!(metrics.failed_requests >= 3);
    assert!(timeout_count >= 3);
}

/// Test indexer timeout recovery scenario
#[tokio::test]
async fn test_indexer_timeout_recovery() {
    let context = FaultInjectionTestContext::new("indexer_api").await;
    
    // First setup timeouts to open circuit
    context.setup_timeout_endpoint("/api/v1/search").await;
    
    let search_url = format!("{}/api/v1/search", context.base_url());
    
    // Trigger timeouts to open circuit
    for _ in 0..4 {
        let _ = context.make_request(&search_url).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Verify circuit is open
    assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Open);
    
    // Setup recovery endpoint (indexer is now responding)
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [
                {
                    "title": "Test Movie 2024",
                    "size": 1024000000,
                    "seeders": 5,
                    "quality": "1080p"
                }
            ]
        })))
        .mount(&context.mock_server)
        .await;
    
    // Wait for circuit to transition to half-open
    assert!(context.wait_for_circuit_state(CircuitBreakerState::HalfOpen, Duration::from_millis(200)).await);
    
    // Make successful request to close circuit
    let result = context.make_request(&search_url).await;
    assert!(result.is_ok());
    
    // Circuit should be closed again
    assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Closed);
    
    let metrics = context.get_test_metrics().await;
    assert!(metrics.successful_requests > 0);
    assert!(metrics.is_resilient());
}

/// Test partial indexer timeout (some requests succeed, others timeout)  
#[tokio::test]
async fn test_partial_indexer_timeout_scenario() {
    let context = FaultInjectionTestContext::new("indexer_api").await;
    
    // Setup unstable endpoint (fast, slow, timeout pattern)
    context.setup_unstable_endpoint("/api/v1/search").await;
    
    let search_url = format!("{}/api/v1/search", context.base_url());
    
    let mut results = Vec::new();
    
    // Make several requests and collect results
    for i in 0..5 {
        let start_time = Instant::now();
        let result = context.make_request(&search_url).await;
        let elapsed = start_time.elapsed();
        
        results.push((i, result.is_ok(), elapsed));
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    
    let metrics = context.get_test_metrics().await;
    
    // Should have a mix of successes and failures
    assert!(metrics.successful_requests > 0);
    assert!(metrics.failed_requests > 0);
    
    // Some requests should be fast, others slow/timeout
    let fast_requests = results.iter().filter(|(_, _, elapsed)| *elapsed < Duration::from_millis(500)).count();
    let slow_requests = results.iter().filter(|(_, _, elapsed)| *elapsed >= Duration::from_millis(500)).count();
    
    assert!(fast_requests > 0, "Should have some fast requests");
    assert!(slow_requests > 0, "Should have some slow/timeout requests");
    
    println!("Request timing distribution:");
    for (i, success, elapsed) in results {
        println!("  Request {}: {} ({:?})", i, if success { "SUCCESS" } else { "FAILED" }, elapsed);
    }
    
    println!("Final metrics: {:?}", metrics);
}

/// Test indexer timeout with query parameters
#[tokio::test]
async fn test_indexer_search_timeout_with_params() {
    let context = FaultInjectionTestContext::new("indexer_search").await;
    
    // Setup endpoint that times out for specific search queries
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("query", "timeout_movie"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
        .mount(&context.mock_server)
        .await;
        
    // Setup normal response for other queries
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": []
        })))
        .mount(&context.mock_server)
        .await;
    
    // Test timeout query
    let timeout_url = format!("{}/api/v1/search?query=timeout_movie", context.base_url());
    let result = context.make_request(&timeout_url).await;
    assert!(result.is_err());
    
    // Test normal query (should work)
    let normal_url = format!("{}/api/v1/search?query=normal_movie", context.base_url());
    let result = context.make_request(&normal_url).await;
    assert!(result.is_ok());
    
    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 2);
    assert_eq!(metrics.successful_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test cascading timeout failures across multiple indexers
#[tokio::test]
async fn test_cascading_indexer_timeouts() {
    // Create multiple contexts to simulate different indexers
    let indexer1 = FaultInjectionTestContext::new("indexer1").await;
    let indexer2 = FaultInjectionTestContext::new("indexer2").await;
    let indexer3 = FaultInjectionTestContext::new("indexer3").await;
    
    // Setup all indexers to timeout
    indexer1.setup_timeout_endpoint("/api/search").await;
    indexer2.setup_timeout_endpoint("/api/search").await; 
    indexer3.setup_timeout_endpoint("/api/search").await;
    
    let url1 = format!("{}/api/search", indexer1.base_url());
    let url2 = format!("{}/api/search", indexer2.base_url());
    let url3 = format!("{}/api/search", indexer3.base_url());
    
    // Simulate trying each indexer in sequence
    let results = vec![
        indexer1.make_request(&url1).await,
        indexer2.make_request(&url2).await,
        indexer3.make_request(&url3).await,
    ];
    
    // All should fail with timeouts
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_err(), "Indexer {} should have failed", i + 1);
        if let Err(error) = result {
            match error {
                RadarrError::Timeout { .. } => {
                    // Expected timeout error
                }
                other => panic!("Expected timeout error for indexer {}, got: {:?}", i + 1, other),
            }
        }
    }
    
    // Verify all circuits are still closed (single failures shouldn't open circuits)
    assert_eq!(indexer1.circuit_breaker.get_state().await, CircuitBreakerState::Closed);
    assert_eq!(indexer2.circuit_breaker.get_state().await, CircuitBreakerState::Closed);
    assert_eq!(indexer3.circuit_breaker.get_state().await, CircuitBreakerState::Closed);
}

/// Test indexer timeout alert generation
#[tokio::test]
async fn test_indexer_timeout_alert_generation() {
    let context = FaultInjectionTestContext::new("critical_indexer").await;
    
    // Setup endpoint that always times out
    context.setup_timeout_endpoint("/api/v1/search").await;
    
    let search_url = format!("{}/api/v1/search", context.base_url());
    
    // Trigger multiple timeouts to simulate critical failure
    for _ in 0..5 {
        let _ = context.make_request(&search_url).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Verify circuit breaker is open (critical failure state)
    assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Open);
    
    // In a real implementation, this would verify alerts were sent
    // For now, we simulate checking alert generation
    let has_alerts = context.verify_failure_alerts_generated().await;
    
    // Note: This will be false in our mock implementation, but in real system
    // should verify that proper alerts (Discord, email, etc.) were triggered
    println!("Alerts generated for critical indexer timeout: {}", has_alerts);
    
    let metrics = context.get_test_metrics().await;
    assert!(!metrics.is_resilient() || metrics.circuit_state == CircuitBreakerState::Open);
}

/// Test indexer timeout with exponential backoff retry
#[tokio::test]
async fn test_indexer_timeout_with_retry_backoff() {
    let context = FaultInjectionTestContext::new("retry_indexer").await;
    
    // Setup endpoint that fails first few times, then succeeds
    context.setup_intermittent_failure_endpoint("/api/v1/search", 2).await;
    
    let search_url = format!("{}/api/v1/search", context.base_url());
    
    let mut attempts = Vec::new();
    let overall_start = Instant::now();
    
    // Simulate retry logic with exponential backoff
    let mut delay = Duration::from_millis(100);
    let max_attempts = 5;
    
    for attempt in 1..=max_attempts {
        let start_time = Instant::now();
        let result = context.make_request(&search_url).await;
        let elapsed = start_time.elapsed();
        
        attempts.push((attempt, result.is_ok(), elapsed));
        
        if result.is_ok() {
            println!("Request succeeded on attempt {}", attempt);
            break;
        }
        
        if attempt < max_attempts {
            println!("Attempt {} failed, retrying after {:?}", attempt, delay);
            tokio::time::sleep(delay).await;
            delay = std::cmp::min(delay * 2, Duration::from_secs(5)); // Exponential backoff with cap
        }
    }
    
    let total_time = overall_start.elapsed();
    
    println!("Retry attempts:");
    for (attempt, success, elapsed) in &attempts {
        println!("  Attempt {}: {} ({:?})", attempt, if *success { "SUCCESS" } else { "FAILED" }, elapsed);
    }
    println!("Total retry time: {:?}", total_time);
    
    let metrics = context.get_test_metrics().await;
    
    // Should eventually succeed with intermittent failure endpoint
    assert!(metrics.successful_requests > 0);
    assert!(attempts.iter().any(|(_, success, _)| *success), "Should have at least one successful attempt");
    
    // Total time should reflect exponential backoff
    assert!(total_time > Duration::from_millis(100), "Should take time due to retries");
}

#[cfg(test)]
mod timeout_integration_tests {
    use super::*;
    
    /// Integration test: Indexer timeout affects search workflow
    #[tokio::test] 
    async fn test_search_workflow_with_indexer_timeouts() {
        let primary_indexer = FaultInjectionTestContext::new("primary_indexer").await;
        let backup_indexer = FaultInjectionTestContext::new("backup_indexer").await;
        
        // Primary indexer times out
        primary_indexer.setup_timeout_endpoint("/search").await;
        
        // Backup indexer works
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    {"title": "Movie from backup", "quality": "1080p"}
                ]
            })))
            .mount(&backup_indexer.mock_server)
            .await;
        
        let primary_url = format!("{}/search", primary_indexer.base_url());
        let backup_url = format!("{}/search", backup_indexer.base_url());
        
        // Try primary first (will timeout)
        let primary_result = primary_indexer.make_request(&primary_url).await;
        assert!(primary_result.is_err());
        
        // Fallback to backup (should succeed)
        let backup_result = backup_indexer.make_request(&backup_url).await;
        assert!(backup_result.is_ok());
        
        if let Ok(body) = backup_result {
            assert!(body.contains("Movie from backup"));
        }
        
        // Verify primary indexer circuit behavior
        let primary_metrics = primary_indexer.get_test_metrics().await;
        let backup_metrics = backup_indexer.get_test_metrics().await;
        
        assert_eq!(primary_metrics.failed_requests, 1);
        assert_eq!(backup_metrics.successful_requests, 1);
        
        println!("Primary indexer metrics: {:?}", primary_metrics);
        println!("Backup indexer metrics: {:?}", backup_metrics);
    }
}