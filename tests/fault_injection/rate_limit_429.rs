//! Rate limiting (429 responses) fault injection tests
//!
//! Tests the system's behavior when external services enforce rate limits.
//! Validates:
//! - Proper handling of 429 responses with Retry-After headers
//! - Exponential backoff and retry logic for rate limits
//! - Circuit breaker behavior under rate limiting
//! - Graceful degradation when rate limited
//! - Recovery after rate limit periods expire

use super::*;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::{Mock, ResponseTemplate, Times};
use wiremock::matchers::{method, path, header};
use serde_json::json;
use radarr_core::circuit_breaker::CircuitBreakerState;
use radarr_core::RadarrError;

/// Test basic rate limiting with Retry-After header
#[tokio::test]
async fn test_basic_rate_limit_handling() {
    let context = FaultInjectionTestContext::new("rate_limited_service").await;
    
    // Setup endpoint that returns 429 with Retry-After
    context.setup_rate_limited_endpoint("/api/data", 2).await;
    
    let api_url = format!("{}/api/data", context.base_url());
    
    let start_time = Instant::now();
    let result = context.make_request(&api_url).await;
    let elapsed = start_time.elapsed();
    
    // Should fail with rate limit error
    assert!(result.is_err());
    if let Err(RadarrError::RateLimited { service, retry_after }) = result {
        assert_eq!(service, "test_service");
        assert_eq!(retry_after, Some(2));
    } else {
        panic!("Expected RateLimited error, got: {:?}", result);
    }
    
    // Should respond quickly (not wait for retry period)
    assert!(elapsed < Duration::from_secs(1));
    
    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test rate limiting with no Retry-After header
#[tokio::test]
async fn test_rate_limit_without_retry_after() {
    let context = FaultInjectionTestContext::new("no_retry_after_service").await;
    
    // Setup endpoint that returns 429 without Retry-After header
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded - no retry info")
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    let result = context.make_request(&api_url).await;
    
    assert!(result.is_err());
    if let Err(RadarrError::RateLimited { service, retry_after }) = result {
        assert_eq!(service, "test_service");
        assert_eq!(retry_after, None);
    } else {
        panic!("Expected RateLimited error, got: {:?}", result);
    }
}

/// Test multiple rate limit responses leading to circuit breaker activation
#[tokio::test]
async fn test_repeated_rate_limits_open_circuit() {
    let context = FaultInjectionTestContext::new("persistent_rate_limit").await;
    
    // Setup endpoint that consistently returns 429
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded")
                .insert_header("Retry-After", "1")
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    
    // Make multiple requests to trigger circuit breaker
    let mut rate_limit_count = 0;
    for i in 0..6 {
        println!("Making request {}", i + 1);
        let result = context.make_request(&api_url).await;
        
        match result {
            Err(RadarrError::RateLimited { .. }) => {
                rate_limit_count += 1;
                println!("Request {} rate limited", i + 1);
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                println!("Circuit breaker opened after {} rate limits", rate_limit_count);
                assert_eq!(service, "persistent_rate_limit");
                break;
            }
            other => {
                println!("Unexpected result for request {}: {:?}", i + 1, other);
            }
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Circuit should be open due to repeated rate limits
    let state = context.circuit_breaker.get_state().await;
    assert_eq!(state, CircuitBreakerState::Open);
    
    let metrics = context.get_test_metrics().await;
    assert!(metrics.failed_requests >= 3);
    assert!(rate_limit_count >= 3);
    assert!(metrics.rejected_requests > 0);
}

/// Test rate limit recovery after retry period
#[tokio::test]
async fn test_rate_limit_recovery() {
    let context = FaultInjectionTestContext::new("recovery_service").await;
    
    // First, return rate limit
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string("Rate limit exceeded")
                .insert_header("Retry-After", "1")
        )
        .up_to_n_times(3)
        .mount(&context.mock_server)
        .await;
    
    // Then allow successful requests
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "data": "recovered from rate limit",
                    "timestamp": chrono::Utc::now().timestamp()
                }))
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    
    // First requests should be rate limited
    for i in 0..3 {
        let result = context.make_request(&api_url).await;
        assert!(result.is_err());
        
        if let Err(RadarrError::RateLimited { .. }) = result {
            println!("Request {} rate limited as expected", i + 1);
        } else {
            panic!("Expected rate limit error on request {}", i + 1);
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Circuit should be open now
    assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Open);
    
    // Wait for circuit breaker timeout to transition to half-open
    assert!(context.wait_for_circuit_state(CircuitBreakerState::HalfOpen, Duration::from_millis(200)).await);
    
    // Next request should succeed (simulating rate limit period expired)
    let result = context.make_request(&api_url).await;
    assert!(result.is_ok());
    
    if let Ok(body) = result {
        assert!(body.contains("recovered from rate limit"));
    }
    
    // Circuit should be closed again
    assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Closed);
    
    let metrics = context.get_test_metrics().await;
    assert!(metrics.successful_requests > 0);
    assert!(metrics.is_resilient());
}

/// Test rate limiting with varying Retry-After values
#[tokio::test]
async fn test_variable_retry_after_periods() {
    let context = FaultInjectionTestContext::new("variable_rate_limit").await;
    
    // Setup different rate limit responses with different retry periods
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "5")
                .set_body_string("Short rate limit")
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "30")
                .set_body_string("Medium rate limit")
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;
    
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "300")
                .set_body_string("Long rate limit")
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    let mut retry_periods = Vec::new();
    
    // Make requests and collect retry periods
    for i in 0..3 {
        let result = context.make_request(&api_url).await;
        assert!(result.is_err());
        
        if let Err(RadarrError::RateLimited { retry_after, .. }) = result {
            retry_periods.push(retry_after);
            println!("Request {} rate limited, retry after: {:?}", i + 1, retry_after);
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Should have collected different retry periods
    assert_eq!(retry_periods.len(), 3);
    assert_eq!(retry_periods[0], Some(5));
    assert_eq!(retry_periods[1], Some(30)); 
    assert_eq!(retry_periods[2], Some(300));
    
    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 3);
    assert_eq!(metrics.failed_requests, 3);
}

/// Test concurrent requests under rate limiting
#[tokio::test]
async fn test_concurrent_requests_rate_limited() {
    let context = FaultInjectionTestContext::new("concurrent_rate_limit").await;
    
    // Setup rate limited endpoint
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "2")
                .set_body_string("Too many concurrent requests")
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    
    // Make concurrent requests
    let mut handles = Vec::new();
    for i in 0..5 {
        let context_ref = &context;
        let url = api_url.clone();
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
    
    let mut rate_limit_count = 0;
    let mut circuit_open_count = 0;
    
    for result in results {
        let (request_id, response, elapsed) = result.unwrap();
        
        match response {
            Err(RadarrError::RateLimited { .. }) => {
                rate_limit_count += 1;
                println!("Concurrent request {} rate limited in {:?}", request_id, elapsed);
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                circuit_open_count += 1;
                println!("Concurrent request {} rejected by circuit breaker in {:?}", request_id, elapsed);
            }
            other => {
                println!("Concurrent request {} result: {:?} in {:?}", request_id, other, elapsed);
            }
        }
    }
    
    // Should have some rate limits and potentially circuit breaker rejections
    assert!(rate_limit_count > 0);
    println!("Rate limited requests: {}, Circuit breaker rejections: {}", rate_limit_count, circuit_open_count);
    
    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 5);
    assert!(metrics.failed_requests > 0 || metrics.rejected_requests > 0);
}

/// Test burst rate limiting scenario
#[tokio::test]
async fn test_burst_rate_limiting() {
    let context = FaultInjectionTestContext::new("burst_rate_limit").await;
    
    // Allow first few requests, then rate limit
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"message": "success"}))
        )
        .up_to_n_times(2)
        .mount(&context.mock_server)
        .await;
    
    // Then rate limit subsequent requests
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "3")
                .set_body_string("Rate limit after burst")
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    let mut results = Vec::new();
    
    // Make rapid sequential requests (simulating burst)
    for i in 0..8 {
        let start = Instant::now();
        let result = context.make_request(&api_url).await;
        let elapsed = start.elapsed();
        
        results.push((i, result.is_ok(), elapsed));
        
        // Very small delay to simulate burst
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    
    let metrics = context.get_test_metrics().await;
    
    // Should have some successful requests followed by rate limits
    assert!(metrics.successful_requests >= 2);
    assert!(metrics.failed_requests > 0);
    
    let successful_count = results.iter().filter(|(_, success, _)| *success).count();
    let failed_count = results.iter().filter(|(_, success, _)| !*success).count();
    
    println!("Burst results: {} successful, {} failed", successful_count, failed_count);
    
    for (i, success, elapsed) in results.iter().take(5) {
        println!("  Request {}: {} ({:?})", i, if *success { "SUCCESS" } else { "FAILED" }, elapsed);
    }
    
    // Should have initial successes followed by rate limits
    assert!(results[0].1, "First request should succeed");
    assert!(results[1].1, "Second request should succeed");
    assert!(!results.last().unwrap().1, "Later requests should be rate limited");
}

/// Test rate limiting with different HTTP methods
#[tokio::test]
async fn test_rate_limiting_different_methods() {
    let context = FaultInjectionTestContext::new("method_rate_limit").await;
    
    // GET requests are rate limited
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_string("GET rate limited")
        )
        .mount(&context.mock_server)
        .await;
    
    // POST requests work normally
    Mock::given(method("POST"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"created": true}))
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    
    // Test GET request (should be rate limited)
    let get_result = context.make_request(&api_url).await;
    assert!(get_result.is_err());
    
    if let Err(RadarrError::RateLimited { .. }) = get_result {
        println!("GET request rate limited as expected");
    } else {
        panic!("Expected rate limit error for GET request");
    }
    
    // Test POST request (should succeed)
    let client = reqwest::Client::new();
    let post_result = context.circuit_breaker.call(async {
        let response = client.post(&api_url)
            .json(&json!({"test": "data"}))
            .send()
            .await
            .map_err(|e| RadarrError::NetworkError { 
                message: format!("POST request failed: {}", e) 
            })?;
            
        if response.status() == 429 {
            return Err(RadarrError::RateLimited {
                service: "test_service".to_string(),
                retry_after: None
            });
        }
        
        let body = response.text().await
            .map_err(|e| RadarrError::NetworkError {
                message: format!("Failed to read response: {}", e)
            })?;
            
        Ok::<String, RadarrError>(body)
    }).await;
    
    assert!(post_result.is_ok());
    if let Ok(body) = post_result {
        assert!(body.contains("created"));
    }
    
    let metrics = context.get_test_metrics().await;
    // GET counted as failed, POST counted as successful
    assert_eq!(metrics.failed_requests, 1);
    assert_eq!(metrics.successful_requests, 1);
}

/// Test rate limit error message parsing
#[tokio::test]
async fn test_rate_limit_error_message_handling() {
    let context = FaultInjectionTestContext::new("message_parsing").await;
    
    // Setup rate limit with detailed error message
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
                .insert_header("X-RateLimit-Limit", "100")
                .insert_header("X-RateLimit-Remaining", "0")
                .insert_header("X-RateLimit-Reset", "1640995200")
                .set_body_json(json!({
                    "error": "Rate limit exceeded",
                    "message": "API rate limit of 100 requests per hour exceeded",
                    "retry_after": 60,
                    "limit": 100,
                    "remaining": 0
                }))
        )
        .mount(&context.mock_server)
        .await;
    
    let api_url = format!("{}/api/data", context.base_url());
    let result = context.make_request(&api_url).await;
    
    assert!(result.is_err());
    if let Err(RadarrError::RateLimited { service, retry_after }) = result {
        assert_eq!(service, "test_service");
        assert_eq!(retry_after, Some(60));
        
        println!("Parsed rate limit error - service: {}, retry_after: {:?}", service, retry_after);
    } else {
        panic!("Expected RateLimited error with parsed details");
    }
}

#[cfg(test)]
mod rate_limit_integration_tests {
    use super::*;
    
    /// Integration test: Multiple services with different rate limits
    #[tokio::test]
    async fn test_multiple_services_different_rate_limits() {
        let fast_service = FaultInjectionTestContext::new("fast_api").await;
        let slow_service = FaultInjectionTestContext::new("slow_api").await;
        
        // Fast service: low rate limit (retry after 1 second)
        fast_service.setup_rate_limited_endpoint("/api/fast", 1).await;
        
        // Slow service: high rate limit (retry after 10 seconds)
        slow_service.setup_rate_limited_endpoint("/api/slow", 10).await;
        
        let fast_url = format!("{}/api/fast", fast_service.base_url());
        let slow_url = format!("{}/api/slow", slow_service.base_url());
        
        // Test both services
        let fast_result = fast_service.make_request(&fast_url).await;
        let slow_result = slow_service.make_request(&slow_url).await;
        
        // Both should be rate limited but with different retry periods
        assert!(fast_result.is_err());
        assert!(slow_result.is_err());
        
        if let Err(RadarrError::RateLimited { retry_after: Some(fast_retry), .. }) = fast_result {
            if let Err(RadarrError::RateLimited { retry_after: Some(slow_retry), .. }) = slow_result {
                assert_eq!(fast_retry, 1);
                assert_eq!(slow_retry, 10);
                assert!(slow_retry > fast_retry, "Slow service should have longer retry period");
                
                println!("Fast service retry: {}s, Slow service retry: {}s", fast_retry, slow_retry);
            }
        }
        
        let fast_metrics = fast_service.get_test_metrics().await;
        let slow_metrics = slow_service.get_test_metrics().await;
        
        assert_eq!(fast_metrics.failed_requests, 1);
        assert_eq!(slow_metrics.failed_requests, 1);
    }
}