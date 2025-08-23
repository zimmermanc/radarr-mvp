//! Fault injection tests for resilience testing
//! 
//! This module provides comprehensive fault injection testing to validate
//! the system's resilience and recovery capabilities under various failure scenarios.
//! 
//! Tests simulate:
//! - Network failures (timeouts, connection drops)  
//! - Service failures (500/503 errors, unavailable services)
//! - Rate limiting (429 responses with Retry-After headers)
//! - Resource exhaustion (disk full, memory pressure)
//! - Data corruption (corrupt files, invalid responses)
//! - Circuit breaker behavior under various fault conditions
//! 
//! Key focus areas:
//! - Graceful degradation under failure conditions
//! - Proper error handling and recovery mechanisms
//! - Circuit breaker activation and recovery
//! - Resource cleanup after failures
//! - Alert generation for critical failures
//! - System stability under cascading failures

use std::time::Duration;
use tokio::time::Instant;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path, header};
use serde_json::json;
use radarr_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use radarr_core::{RadarrError, Result};
use radarr_infrastructure::monitoring::{AlertManager, HealthChecker, PrometheusMetrics};

pub mod indexer_timeout;
pub mod rate_limit_429;  
pub mod download_stall;
pub mod disk_full;
pub mod corrupt_file;
pub mod service_unavailable;
pub mod circuit_breaker_test;

/// Common test utilities and setup
pub struct FaultInjectionTestContext {
    pub mock_server: MockServer,
    pub circuit_breaker: CircuitBreaker,
    pub alert_manager: AlertManager,
    pub health_checker: HealthChecker,
    pub metrics: PrometheusMetrics,
    pub start_time: Instant,
}

impl FaultInjectionTestContext {
    /// Create a new test context with all required components
    pub async fn new(service_name: &str) -> Self {
        let mock_server = MockServer::start().await;
        
        let circuit_breaker_config = CircuitBreakerConfig::new(service_name)
            .with_failure_threshold(3)
            .with_timeout(Duration::from_millis(100))
            .with_success_threshold(1)
            .with_request_timeout(Duration::from_secs(5));
            
        let circuit_breaker = CircuitBreaker::new(circuit_breaker_config);
        let alert_manager = AlertManager::new().await;
        let health_checker = HealthChecker::new();
        let metrics = PrometheusMetrics::new();
        
        Self {
            mock_server,
            circuit_breaker, 
            alert_manager,
            health_checker,
            metrics,
            start_time: Instant::now(),
        }
    }
    
    /// Get the base URL for the mock server
    pub fn base_url(&self) -> String {
        self.mock_server.uri()
    }
    
    /// Setup a mock endpoint that always returns 500 errors
    pub async fn setup_always_failing_endpoint(&self, path: &str) {
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&self.mock_server)
            .await;
    }
    
    /// Setup a mock endpoint that times out (never responds)
    pub async fn setup_timeout_endpoint(&self, path: &str) {
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(30)))
            .mount(&self.mock_server)
            .await;
    }
    
    /// Setup a mock endpoint that returns 429 rate limit with Retry-After header
    pub async fn setup_rate_limited_endpoint(&self, path: &str, retry_after_seconds: u64) {
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string("Rate limit exceeded")
                    .insert_header("Retry-After", retry_after_seconds.to_string())
            )
            .mount(&self.mock_server)
            .await;
    }
    
    /// Setup a mock endpoint that intermittently fails (fails first N requests, then succeeds)
    pub async fn setup_intermittent_failure_endpoint(&self, path: &str, failure_count: usize) {
        for i in 0..failure_count {
            Mock::given(method("GET"))
                .and(path(path))
                .respond_with(ResponseTemplate::new(500).set_body_string("Temporary failure"))
                .up_to_n_times(1)
                .mount(&self.mock_server)
                .await;
        }
        
        // After failures, return success
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "data": "recovered"
            })))
            .mount(&self.mock_server)
            .await;
    }
    
    /// Setup a mock endpoint that returns invalid JSON
    pub async fn setup_corrupt_data_endpoint(&self, path: &str) {
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{ invalid json syntax }")
                    .insert_header("content-type", "application/json")
            )
            .mount(&self.mock_server)
            .await;
    }
    
    /// Setup a mock endpoint that returns partial responses (incomplete data)
    pub async fn setup_partial_response_endpoint(&self, path: &str) {
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{ \"incomplete\": tru")
                    .insert_header("content-type", "application/json")
            )
            .mount(&self.mock_server)
            .await;
    }
    
    /// Setup a mock endpoint that simulates network instability (random delays)
    pub async fn setup_unstable_endpoint(&self, path: &str) {
        // Fast response
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": "fast"})))
            .up_to_n_times(1)
            .mount(&self.mock_server)
            .await;
            
        // Slow response
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(2000))
                    .set_body_json(json!({"result": "slow"}))
            )
            .up_to_n_times(1)
            .mount(&self.mock_server)
            .await;
            
        // Timeout
        Mock::given(method("GET"))
            .and(path(path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(10))
                    .set_body_json(json!({"result": "timeout"}))
            )
            .mount(&self.mock_server)
            .await;
    }
    
    /// Simulate making HTTP requests with the circuit breaker protection
    pub async fn make_request(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::new();
        
        self.circuit_breaker.call(async {
            let response = client.get(url).send().await
                .map_err(|e| RadarrError::NetworkError { 
                    message: format!("HTTP request failed: {}", e) 
                })?;
                
            let status = response.status();
            if status.is_server_error() {
                return Err(RadarrError::ExternalServiceError {
                    service: "test_service".to_string(),
                    error: format!("Server error: {}", status)
                });
            }
            
            if status == 429 {
                let retry_after = response.headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok());
                    
                return Err(RadarrError::RateLimited {
                    service: "test_service".to_string(),
                    retry_after
                });
            }
            
            let body = response.text().await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Failed to read response body: {}", e)
                })?;
                
            Ok::<String, RadarrError>(body)
        }).await
    }
    
    /// Wait for circuit breaker to reach expected state with timeout
    pub async fn wait_for_circuit_state(&self, expected_state: CircuitBreakerState, timeout: Duration) -> bool {
        let start = Instant::now();
        
        while start.elapsed() < timeout {
            if self.circuit_breaker.get_state().await == expected_state {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        false
    }
    
    /// Verify that proper alerts are generated for failures
    pub async fn verify_failure_alerts_generated(&self) -> bool {
        // In a real implementation, this would check the alert manager
        // For now, we'll simulate checking that alerts were generated
        let alerts = self.alert_manager.get_active_alerts().await;
        !alerts.is_empty()
    }
    
    /// Verify that system can recover after failures
    pub async fn verify_recovery_after_failure(&self, recovery_endpoint_url: &str) -> Result<bool> {
        // Wait for circuit breaker to allow requests again
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Try making a successful request
        match self.make_request(recovery_endpoint_url).await {
            Ok(_) => {
                // Verify circuit breaker transitions back to closed
                let state = self.circuit_breaker.get_state().await;
                Ok(state == CircuitBreakerState::Closed)
            }
            Err(_) => Ok(false)
        }
    }
    
    /// Get test execution metrics
    pub async fn get_test_metrics(&self) -> TestMetrics {
        let cb_metrics = self.circuit_breaker.get_metrics().await;
        let elapsed = self.start_time.elapsed();
        
        TestMetrics {
            elapsed_time: elapsed,
            total_requests: cb_metrics.total_requests,
            successful_requests: cb_metrics.successful_requests,
            failed_requests: cb_metrics.failed_requests,
            rejected_requests: cb_metrics.rejected_requests,
            circuit_state: cb_metrics.state,
            consecutive_failures: cb_metrics.consecutive_failures,
        }
    }
}

/// Test execution metrics
#[derive(Debug)]
pub struct TestMetrics {
    pub elapsed_time: Duration,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub rejected_requests: u64,
    pub circuit_state: CircuitBreakerState,
    pub consecutive_failures: u32,
}

impl TestMetrics {
    /// Calculate success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 100.0;
        }
        (self.successful_requests as f64 / self.total_requests as f64) * 100.0
    }
    
    /// Calculate failure rate as percentage
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.failed_requests as f64 / self.total_requests as f64) * 100.0
    }
    
    /// Calculate rejection rate as percentage
    pub fn rejection_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.rejected_requests as f64 / self.total_requests as f64) * 100.0
    }
    
    /// Check if metrics indicate system resilience
    pub fn is_resilient(&self) -> bool {
        // System is considered resilient if:
        // 1. Circuit breaker eventually stabilizes (not stuck open)
        // 2. Some requests succeed eventually (system can recover)
        // 3. Failures don't cascade indefinitely
        
        self.circuit_state != CircuitBreakerState::Open || 
        (self.successful_requests > 0 && self.consecutive_failures < 10)
    }
}

/// Helper macro for running fault injection tests with common setup
#[macro_export]
macro_rules! fault_injection_test {
    ($test_name:ident, $service_name:expr, $test_body:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let context = FaultInjectionTestContext::new($service_name).await;
            $test_body(context).await;
        }
    };
}

/// Helper function to assert circuit breaker behaves correctly under load
pub async fn assert_circuit_breaker_behavior(
    context: &FaultInjectionTestContext,
    endpoint_url: &str,
    expected_failures: u32,
) -> TestMetrics {
    let mut request_count = 0;
    let max_requests = expected_failures + 5; // Try a few extra requests
    
    // Keep making requests until circuit opens or we hit max
    while request_count < max_requests {
        match context.make_request(endpoint_url).await {
            Ok(_) => {
                // Success - continue
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                // Circuit opened as expected
                break;
            }
            Err(_) => {
                // Other failures are expected
            }
        }
        request_count += 1;
        
        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Get final metrics
    let metrics = context.get_test_metrics().await;
    
    // Circuit should be open after enough failures
    if metrics.failed_requests >= expected_failures as u64 {
        assert_eq!(metrics.circuit_state, CircuitBreakerState::Open);
    }
    
    metrics
}

/// Helper function to test recovery behavior
pub async fn test_recovery_scenario(
    context: &FaultInjectionTestContext,
    failing_endpoint: &str,
    recovery_endpoint: &str,
) -> bool {
    // First, trigger failures to open circuit
    for _ in 0..5 {
        let _ = context.make_request(failing_endpoint).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Verify circuit is open
    assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Open);
    
    // Wait for circuit to transition to half-open
    assert!(context.wait_for_circuit_state(CircuitBreakerState::HalfOpen, Duration::from_secs(1)).await);
    
    // Test recovery with successful endpoint
    match context.verify_recovery_after_failure(recovery_endpoint).await {
        Ok(recovered) => recovered,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fault_injection_context_creation() {
        let context = FaultInjectionTestContext::new("test_service").await;
        
        // Verify initial state
        assert_eq!(context.circuit_breaker.get_state().await, CircuitBreakerState::Closed);
        assert!(!context.base_url().is_empty());
        
        let metrics = context.get_test_metrics().await;
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
    }
    
    #[tokio::test]
    async fn test_mock_server_setup() {
        let context = FaultInjectionTestContext::new("test_service").await;
        
        // Setup failing endpoint
        context.setup_always_failing_endpoint("/test").await;
        
        // Test that it fails as expected
        let url = format!("{}/test", context.base_url());
        let result = context.make_request(&url).await;
        assert!(result.is_err());
        
        if let Err(RadarrError::ExternalServiceError { service, error }) = result {
            assert_eq!(service, "test_service");
            assert!(error.contains("500"));
        } else {
            panic!("Expected ExternalServiceError");
        }
    }
    
    #[tokio::test] 
    async fn test_circuit_breaker_integration() {
        let context = FaultInjectionTestContext::new("test_service").await;
        
        // Setup failing endpoint
        context.setup_always_failing_endpoint("/fail").await;
        
        let url = format!("{}/fail", context.base_url());
        
        // Test circuit breaker behavior
        let metrics = assert_circuit_breaker_behavior(&context, &url, 3).await;
        
        // Verify circuit opened
        assert_eq!(metrics.circuit_state, CircuitBreakerState::Open);
        assert!(metrics.failed_requests >= 3);
        assert!(metrics.rejected_requests > 0);
    }
    
    #[tokio::test]
    async fn test_recovery_mechanism() {
        let context = FaultInjectionTestContext::new("test_service").await;
        
        // Setup failing and recovery endpoints
        context.setup_always_failing_endpoint("/fail").await;
        
        Mock::given(method("GET"))
            .and(path("/recover"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "healthy"
            })))
            .mount(&context.mock_server)
            .await;
        
        let failing_url = format!("{}/fail", context.base_url());
        let recovery_url = format!("{}/recover", context.base_url());
        
        // Test recovery scenario
        let recovered = test_recovery_scenario(&context, &failing_url, &recovery_url).await;
        assert!(recovered);
    }
}