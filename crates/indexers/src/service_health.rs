//! Service health monitoring and circuit breaker implementation
//!
//! This module provides health monitoring, metrics collection, and circuit breaker
//! functionality for external service integrations to improve reliability.

use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use radarr_core::{Result, RadarrError};

/// Service health status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Service is healthy and responding normally
    Healthy,
    /// Service is experiencing issues but still functional
    Degraded,
    /// Service is down or unresponsive
    Down,
    /// Service is temporarily disabled due to circuit breaker
    CircuitOpen,
}

/// Service performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetrics {
    /// Total number of requests made
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Last request timestamp (Unix timestamp in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_request_time: Option<u64>,
    /// Current error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Service uptime percentage over the monitoring window
    pub uptime_percentage: f64,
}

impl Default for ServiceMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_ms: 0.0,
            last_request_time: None,
            error_rate: 0.0,
            uptime_percentage: 100.0,
        }
    }
}

impl ServiceMetrics {
    /// Record a successful request with response time
    pub fn record_success(&mut self, response_time: Duration) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.last_request_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
        
        // Update average response time using weighted average
        let response_time_ms = response_time.as_millis() as f64;
        if self.total_requests == 1 {
            self.avg_response_time_ms = response_time_ms;
        } else {
            // Weighted average favoring recent measurements
            let weight = 0.1;
            self.avg_response_time_ms = 
                self.avg_response_time_ms * (1.0 - weight) + response_time_ms * weight;
        }
        
        self.update_error_rate();
    }
    
    /// Record a failed request
    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_request_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
        self.update_error_rate();
    }
    
    /// Update error rate calculation
    fn update_error_rate(&mut self) {
        if self.total_requests > 0 {
            self.error_rate = self.failed_requests as f64 / self.total_requests as f64;
        }
    }
    
    /// Get success rate (inverse of error rate)
    pub fn success_rate(&self) -> f64 {
        1.0 - self.error_rate
    }
    
    /// Check if service is performing well based on metrics
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if error rate < 10% and avg response time < 5 seconds
        self.error_rate < 0.1 && self.avg_response_time_ms < 5000.0
    }
    
    /// Reset metrics (useful for testing or periodic cleanup)
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
enum CircuitBreakerState {
    /// Normal operation, requests pass through
    Closed,
    /// Circuit is open, requests are rejected
    Open { opened_at: Instant },
    /// Testing if service has recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open the circuit (e.g., 5 failures)
    pub failure_threshold: u32,
    /// Timeout before attempting to close the circuit (e.g., 60 seconds)
    pub timeout: Duration,
    /// Success threshold to close the circuit from half-open (e.g., 3 successes)
    pub success_threshold: u32,
    /// Time window for counting failures (e.g., 300 seconds)
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(60),
            success_threshold: 3,
            failure_window: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Circuit breaker implementation for service reliability
#[derive(Debug)]
struct CircuitBreaker {
    state: CircuitBreakerState,
    config: CircuitBreakerConfig,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitBreakerState::Closed,
            config,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
        }
    }
    
    /// Check if a request should be allowed through
    fn should_allow_request(&mut self) -> bool {
        let now = Instant::now();
        
        match &self.state {
            CircuitBreakerState::Closed => {
                // Clean up old failures outside the window
                if let Some(last_failure) = self.last_failure_time {
                    if now.duration_since(last_failure) > self.config.failure_window {
                        self.failure_count = 0;
                    }
                }
                true
            },
            CircuitBreakerState::Open { opened_at } => {
                // Check if timeout has passed
                if now.duration_since(*opened_at) > self.config.timeout {
                    self.state = CircuitBreakerState::HalfOpen;
                    self.success_count = 0;
                    true
                } else {
                    false
                }
            },
            CircuitBreakerState::HalfOpen => true,
        }
    }
    
    /// Record a successful operation
    fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                self.failure_count = 0;
            },
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                }
            },
            CircuitBreakerState::Open { .. } => {
                // Shouldn't happen, but reset to closed if we get here
                self.state = CircuitBreakerState::Closed;
                self.failure_count = 0;
                self.success_count = 0;
            }
        }
    }
    
    /// Record a failed operation
    fn record_failure(&mut self) {
        let now = Instant::now();
        self.last_failure_time = Some(now);
        
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.config.failure_threshold {
                    self.state = CircuitBreakerState::Open { opened_at: now };
                }
            },
            CircuitBreakerState::HalfOpen => {
                // Immediately open on any failure in half-open state
                self.state = CircuitBreakerState::Open { opened_at: now };
                self.failure_count = 1;
                self.success_count = 0;
            },
            CircuitBreakerState::Open { .. } => {
                // Already open, no action needed
            }
        }
    }
    
    /// Get current state for monitoring
    fn get_state(&self) -> &CircuitBreakerState {
        &self.state
    }
}

/// Service health monitor with circuit breaker and metrics
#[derive(Debug)]
pub struct ServiceHealth {
    service_name: String,
    metrics: Arc<RwLock<ServiceMetrics>>,
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
}

impl ServiceHealth {
    /// Create a new service health monitor
    pub fn new(service_name: String) -> Self {
        Self::with_config(service_name, CircuitBreakerConfig::default())
    }
    
    /// Create a service health monitor with custom circuit breaker configuration
    pub fn with_config(service_name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            service_name,
            metrics: Arc::new(RwLock::new(ServiceMetrics::default())),
            circuit_breaker: Arc::new(RwLock::new(CircuitBreaker::new(config))),
        }
    }
    
    /// Check if a request should be allowed (circuit breaker check)
    pub async fn should_allow_request(&self) -> bool {
        let mut circuit_breaker = self.circuit_breaker.write().await;
        circuit_breaker.should_allow_request()
    }
    
    /// Execute a request with automatic health monitoring
    pub async fn execute_request<F, T>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        // Check circuit breaker
        if !self.should_allow_request().await {
            return Err(RadarrError::ExternalServiceError {
                service: self.service_name.clone(),
                error: "Circuit breaker is open - service temporarily unavailable".to_string(),
            });
        }
        
        let start_time = Instant::now();
        let result = operation.await;
        let response_time = start_time.elapsed();
        
        // Record metrics based on result
        match &result {
            Ok(_) => {
                self.record_success(response_time).await;
            },
            Err(_) => {
                self.record_failure().await;
            }
        }
        
        result
    }
    
    /// Record a successful operation
    async fn record_success(&self, response_time: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.record_success(response_time);
        
        let mut circuit_breaker = self.circuit_breaker.write().await;
        circuit_breaker.record_success();
    }
    
    /// Record a failed operation
    async fn record_failure(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.record_failure();
        
        let mut circuit_breaker = self.circuit_breaker.write().await;
        circuit_breaker.record_failure();
    }
    
    /// Get current health status
    pub async fn get_health_status(&self) -> HealthStatus {
        let metrics = self.metrics.read().await;
        let circuit_breaker = self.circuit_breaker.read().await;
        
        match circuit_breaker.get_state() {
            CircuitBreakerState::Open { .. } => HealthStatus::CircuitOpen,
            CircuitBreakerState::HalfOpen => HealthStatus::Degraded,
            CircuitBreakerState::Closed => {
                if metrics.is_healthy() {
                    HealthStatus::Healthy
                } else if metrics.error_rate < 0.5 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Down
                }
            }
        }
    }
    
    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> ServiceMetrics {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }
    
    /// Reset all metrics and circuit breaker state
    pub async fn reset(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.reset();
        
        let mut circuit_breaker = self.circuit_breaker.write().await;
        *circuit_breaker = CircuitBreaker::new(circuit_breaker.config.clone());
    }
    
    /// Get service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[test]
    fn test_service_metrics_success_recording() {
        let mut metrics = ServiceMetrics::default();
        
        metrics.record_success(Duration::from_millis(100));
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.avg_response_time_ms, 100.0);
        assert_eq!(metrics.error_rate, 0.0);
        assert!(metrics.is_healthy());
    }
    
    #[test]
    fn test_service_metrics_failure_recording() {
        let mut metrics = ServiceMetrics::default();
        
        metrics.record_failure();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.error_rate, 1.0);
        assert!(!metrics.is_healthy());
    }
    
    #[test]
    fn test_service_metrics_mixed_requests() {
        let mut metrics = ServiceMetrics::default();
        
        // Record some successes and failures  
        metrics.record_success(Duration::from_millis(100));
        metrics.record_success(Duration::from_millis(200));
        metrics.record_failure();
        metrics.record_success(Duration::from_millis(150));
        
        assert_eq!(metrics.total_requests, 4);
        assert_eq!(metrics.successful_requests, 3);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.error_rate, 0.25);
        assert!(!metrics.is_healthy()); // Error rate of 25% > 10% threshold, so not healthy
        
        // Add more successes to get below 10% error rate
        for _ in 0..6 {
            metrics.record_success(Duration::from_millis(100));
        }
        
        assert_eq!(metrics.total_requests, 10);
        assert_eq!(metrics.error_rate, 0.1); // 1 failure out of 10 = 10%
        assert!(!metrics.is_healthy()); // Exactly 10% is not < 10%
        
        // Add one more success to get below 10%
        metrics.record_success(Duration::from_millis(100));
        assert_eq!(metrics.total_requests, 11);
        assert!(metrics.error_rate < 0.1); // Should be 1/11 ≈ 0.09
        assert!(metrics.is_healthy()); // Now < 10% error rate
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_basic_operation() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            timeout: Duration::from_millis(100),
            success_threshold: 2,
            failure_window: Duration::from_secs(60),
        };
        
        let health = ServiceHealth::with_config("test-service".to_string(), config);
        
        // Should allow requests initially
        assert!(health.should_allow_request().await);
        
        // Simulate failures to open circuit
        for _ in 0..3 {
            health.record_failure().await;
        }
        
        // Circuit should be open now
        assert!(!health.should_allow_request().await);
        assert_eq!(health.get_health_status().await, HealthStatus::CircuitOpen);
        
        // Wait for timeout
        sleep(Duration::from_millis(150)).await;
        
        // Should allow one request (half-open state)
        assert!(health.should_allow_request().await);
        assert_eq!(health.get_health_status().await, HealthStatus::Degraded);
    }
    
    #[tokio::test]
    async fn test_execute_request_success() {
        let health = ServiceHealth::new("test-service".to_string());
        
        let result = health.execute_request(async {
            Ok::<i32, RadarrError>(42)
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        
        let metrics = health.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(health.get_health_status().await, HealthStatus::Healthy);
    }
    
    #[tokio::test]
    async fn test_execute_request_failure() {
        let health = ServiceHealth::new("test-service".to_string());
        
        let result = health.execute_request(async {
            Err::<i32, RadarrError>(RadarrError::ExternalServiceError {
                service: "test".to_string(),
                error: "test error".to_string(),
            })
        }).await;
        
        assert!(result.is_err());
        
        let metrics = health.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_prevents_requests() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            timeout: Duration::from_secs(1),
            success_threshold: 1,
            failure_window: Duration::from_secs(60),
        };
        
        let health = ServiceHealth::with_config("test-service".to_string(), config);
        
        // Execute failing requests to open circuit
        for _ in 0..2 {
            let _ = health.execute_request(async {
                Err::<i32, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test".to_string(),
                    error: "test error".to_string(),
                })
            }).await;
        }
        
        // Next request should be rejected by circuit breaker
        let result = health.execute_request(async {
            Ok::<i32, RadarrError>(42)
        }).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circuit breaker is open"));
    }
}