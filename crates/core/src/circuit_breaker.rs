//! Circuit breaker implementation for external service resilience
//!
//! This module provides a generic circuit breaker that can be used to protect
//! against cascading failures when external services become unavailable.
//!
//! Circuit breaker states:
//! - Closed: Normal operation, requests pass through
//! - Open: Service is failing, requests are rejected immediately  
//! - Half-Open: Testing recovery, single request allowed through

use crate::{RadarrError, Result};
use std::sync::atomic::{AtomicU64, Ordering};
// use std::sync::atomic::AtomicUsize; // Currently unused
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    /// Normal operation - requests pass through
    Closed,
    /// Service is failing - requests are rejected immediately
    Open,
    /// Testing recovery - single request allowed through
    HalfOpen,
}

impl CircuitBreakerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::HalfOpen => "half_open",
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration to wait before transitioning from Open to Half-Open
    pub timeout: Duration,
    /// Success threshold for closing the circuit from Half-Open
    pub success_threshold: u32,
    /// Request timeout for individual operations
    pub request_timeout: Duration,
    /// Service name for logging and error reporting
    pub service_name: String,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 1,
            request_timeout: Duration::from_secs(10),
            service_name: "unknown".to_string(),
        }
    }
}

impl CircuitBreakerConfig {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_success_threshold(mut self, threshold: u32) -> Self {
        self.success_threshold = threshold;
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

/// Circuit breaker metrics for monitoring and health checks
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    /// Current state of the circuit breaker
    pub state: CircuitBreakerState,
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Number of requests rejected due to open circuit
    pub rejected_requests: u64,
    /// Current consecutive failure count
    pub consecutive_failures: u32,
    /// Current consecutive success count (in half-open state)
    pub consecutive_successes: u32,
    /// Last failure time
    pub last_failure_time: Option<Instant>,
    /// Last success time
    pub last_success_time: Option<Instant>,
    /// Time when circuit was last opened
    pub circuit_opened_time: Option<Instant>,
    /// Service name
    pub service_name: String,
}

/// Internal state for the circuit breaker
#[derive(Debug)]
struct CircuitBreakerInternalState {
    state: CircuitBreakerState,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_failure_time: Option<Instant>,
    last_success_time: Option<Instant>,
    circuit_opened_time: Option<Instant>,
}

/// Circuit breaker implementation for protecting external service calls
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitBreakerInternalState>>,
    metrics: CircuitBreakerMetrics,
    // Atomic counters for lock-free metrics updates
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    rejected_requests: AtomicU64,
}

impl std::fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("config", &self.config)
            .field("metrics", &self.metrics)
            .field(
                "total_requests",
                &self.total_requests.load(Ordering::Relaxed),
            )
            .field(
                "successful_requests",
                &self.successful_requests.load(Ordering::Relaxed),
            )
            .field(
                "failed_requests",
                &self.failed_requests.load(Ordering::Relaxed),
            )
            .field(
                "rejected_requests",
                &self.rejected_requests.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let service_name = config.service_name.clone();

        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerInternalState {
                state: CircuitBreakerState::Closed,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_failure_time: None,
                last_success_time: None,
                circuit_opened_time: None,
            })),
            metrics: CircuitBreakerMetrics {
                state: CircuitBreakerState::Closed,
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                rejected_requests: 0,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_failure_time: None,
                last_success_time: None,
                circuit_opened_time: None,
                service_name,
            },
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            rejected_requests: AtomicU64::new(0),
        }
    }

    /// Execute an operation protected by the circuit breaker
    pub async fn call<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
        E: Into<RadarrError>,
    {
        // Check if we can proceed with the request
        let can_proceed = self.can_proceed().await?;
        if !can_proceed {
            self.rejected_requests.fetch_add(1, Ordering::Relaxed);
            return Err(RadarrError::CircuitBreakerOpen {
                service: self.config.service_name.clone(),
            });
        }

        self.total_requests.fetch_add(1, Ordering::Relaxed);

        // Execute the operation with timeout
        let start_time = Instant::now();
        let operation_result = tokio::time::timeout(self.config.request_timeout, operation).await;

        match operation_result {
            Ok(Ok(result)) => {
                // Operation succeeded
                self.on_success().await;
                debug!(
                    service = %self.config.service_name,
                    duration_ms = start_time.elapsed().as_millis(),
                    "Circuit breaker: operation succeeded"
                );
                Ok(result)
            }
            Ok(Err(error)) => {
                // Operation failed
                let radarr_error = error.into();
                self.on_failure(&radarr_error).await;
                error!(
                    service = %self.config.service_name,
                    error = %radarr_error,
                    duration_ms = start_time.elapsed().as_millis(),
                    "Circuit breaker: operation failed"
                );
                Err(radarr_error)
            }
            Err(_) => {
                // Operation timed out
                let timeout_error = RadarrError::Timeout {
                    operation: format!("{} request", self.config.service_name),
                };
                self.on_failure(&timeout_error).await;
                error!(
                    service = %self.config.service_name,
                    timeout_ms = self.config.request_timeout.as_millis(),
                    "Circuit breaker: operation timed out"
                );
                Err(timeout_error)
            }
        }
    }

    /// Check if a request can proceed based on current circuit breaker state
    async fn can_proceed(&self) -> Result<bool> {
        let state = self.state.read().await;

        match state.state {
            CircuitBreakerState::Closed => Ok(true),
            CircuitBreakerState::HalfOpen => {
                // In half-open state, allow one request through to test recovery
                Ok(true)
            }
            CircuitBreakerState::Open => {
                // Check if timeout period has elapsed
                if let Some(opened_time) = state.circuit_opened_time {
                    if opened_time.elapsed() >= self.config.timeout {
                        // Transition to half-open state
                        drop(state);
                        self.transition_to_half_open().await;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    // No opened time recorded, something is wrong - allow request
                    warn!(
                        service = %self.config.service_name,
                        "Circuit breaker is open but no opened time recorded"
                    );
                    Ok(true)
                }
            }
        }
    }

    /// Handle successful operation
    async fn on_success(&self) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write().await;
        state.last_success_time = Some(Instant::now());
        state.consecutive_failures = 0;

        match state.state {
            CircuitBreakerState::Closed => {
                // Already closed, nothing to do
            }
            CircuitBreakerState::HalfOpen => {
                state.consecutive_successes += 1;
                if state.consecutive_successes >= self.config.success_threshold {
                    // Transition back to closed state
                    info!(
                        service = %self.config.service_name,
                        successes = state.consecutive_successes,
                        "Circuit breaker: transitioning to CLOSED state"
                    );
                    state.state = CircuitBreakerState::Closed;
                    state.consecutive_successes = 0;
                    state.circuit_opened_time = None;
                }
            }
            CircuitBreakerState::Open => {
                // This shouldn't happen as we should have transitioned to half-open first
                warn!(
                    service = %self.config.service_name,
                    "Unexpected success while circuit is open"
                );
            }
        }
    }

    /// Handle failed operation
    async fn on_failure(&self, error: &RadarrError) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);

        let mut state = self.state.write().await;
        state.last_failure_time = Some(Instant::now());
        state.consecutive_successes = 0;

        match state.state {
            CircuitBreakerState::Closed => {
                state.consecutive_failures += 1;
                if state.consecutive_failures >= self.config.failure_threshold {
                    // Transition to open state
                    warn!(
                        service = %self.config.service_name,
                        failures = state.consecutive_failures,
                        error = %error,
                        "Circuit breaker: transitioning to OPEN state"
                    );
                    state.state = CircuitBreakerState::Open;
                    state.circuit_opened_time = Some(Instant::now());
                }
            }
            CircuitBreakerState::HalfOpen => {
                // Test failed, go back to open state
                warn!(
                    service = %self.config.service_name,
                    error = %error,
                    "Circuit breaker: test request failed, transitioning back to OPEN state"
                );
                state.state = CircuitBreakerState::Open;
                state.circuit_opened_time = Some(Instant::now());
                state.consecutive_failures += 1;
            }
            CircuitBreakerState::Open => {
                // Already open, just increment failure count
                state.consecutive_failures += 1;
            }
        }
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        if state.state == CircuitBreakerState::Open {
            info!(
                service = %self.config.service_name,
                "Circuit breaker: transitioning to HALF_OPEN state"
            );
            state.state = CircuitBreakerState::HalfOpen;
            state.consecutive_successes = 0;
        }
    }

    /// Get current circuit breaker metrics
    pub async fn get_metrics(&self) -> CircuitBreakerMetrics {
        let state = self.state.read().await;

        CircuitBreakerMetrics {
            state: state.state,
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            rejected_requests: self.rejected_requests.load(Ordering::Relaxed),
            consecutive_failures: state.consecutive_failures,
            consecutive_successes: state.consecutive_successes,
            last_failure_time: state.last_failure_time,
            last_success_time: state.last_success_time,
            circuit_opened_time: state.circuit_opened_time,
            service_name: self.config.service_name.clone(),
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitBreakerState {
        let state = self.state.read().await;
        state.state
    }

    /// Force the circuit breaker to open (for testing or manual intervention)
    pub async fn force_open(&self) {
        let mut state = self.state.write().await;
        warn!(
            service = %self.config.service_name,
            "Circuit breaker: manually forced to OPEN state"
        );
        state.state = CircuitBreakerState::Open;
        state.circuit_opened_time = Some(Instant::now());
    }

    /// Force the circuit breaker to close (for testing or manual intervention)
    pub async fn force_close(&self) {
        let mut state = self.state.write().await;
        info!(
            service = %self.config.service_name,
            "Circuit breaker: manually forced to CLOSED state"
        );
        state.state = CircuitBreakerState::Closed;
        state.consecutive_failures = 0;
        state.consecutive_successes = 0;
        state.circuit_opened_time = None;
    }

    /// Reset circuit breaker statistics
    pub async fn reset_metrics(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.rejected_requests.store(0, Ordering::Relaxed);

        let mut state = self.state.write().await;
        state.consecutive_failures = 0;
        state.consecutive_successes = 0;
        state.last_failure_time = None;
        state.last_success_time = None;

        info!(
            service = %self.config.service_name,
            "Circuit breaker: metrics reset"
        );
    }

    /// Check if the service is healthy based on success rate
    pub async fn is_healthy(&self) -> bool {
        let metrics = self.get_metrics().await;

        // If no requests have been made, consider it healthy
        if metrics.total_requests == 0 {
            return true;
        }

        // If circuit is open, it's not healthy
        if metrics.state == CircuitBreakerState::Open {
            return false;
        }

        // Calculate success rate
        let success_rate = metrics.successful_requests as f64 / metrics.total_requests as f64;

        // Consider healthy if success rate is above 80% and not too many consecutive failures
        success_rate >= 0.8 && metrics.consecutive_failures < self.config.failure_threshold / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let config = CircuitBreakerConfig::new("test_service").with_failure_threshold(3);
        let cb = CircuitBreaker::new(config);

        // Should start in closed state
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);

        // Successful operation should work
        let result = cb.call(async { Ok::<_, RadarrError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let metrics = cb.get_metrics().await;
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_circuit_breaker_transitions_to_open() {
        let config = CircuitBreakerConfig::new("test_service").with_failure_threshold(2);
        let cb = CircuitBreaker::new(config);

        // First failure
        let result = cb
            .call(async {
                Err::<i32, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test".to_string(),
                    error: "test error".to_string(),
                })
            })
            .await;
        assert!(result.is_err());
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);

        // Second failure should open the circuit
        let result = cb
            .call(async {
                Err::<i32, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test".to_string(),
                    error: "test error".to_string(),
                })
            })
            .await;
        assert!(result.is_err());
        assert_eq!(cb.get_state().await, CircuitBreakerState::Open);

        // Subsequent requests should be rejected
        let result = cb.call(async { Ok::<_, RadarrError>(42) }).await;
        assert!(result.is_err());
        if let RadarrError::CircuitBreakerOpen { service } = result.unwrap_err() {
            assert_eq!(service, "test_service");
        } else {
            panic!("Expected CircuitBreakerOpen error");
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_recovery() {
        let config = CircuitBreakerConfig::new("test_service")
            .with_failure_threshold(1)
            .with_timeout(Duration::from_millis(50));
        let cb = CircuitBreaker::new(config);

        // Fail to open circuit
        let result = cb
            .call(async {
                Err::<i32, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test".to_string(),
                    error: "test error".to_string(),
                })
            })
            .await;
        assert!(result.is_err());
        assert_eq!(cb.get_state().await, CircuitBreakerState::Open);

        // Wait for timeout
        sleep(Duration::from_millis(60)).await;

        // Next request should succeed and close the circuit
        let result = cb.call(async { Ok::<_, RadarrError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_timeout() {
        let config = CircuitBreakerConfig::new("test_service")
            .with_request_timeout(Duration::from_millis(50));
        let cb = CircuitBreaker::new(config);

        // Operation that times out
        let result = cb
            .call(async {
                sleep(Duration::from_millis(100)).await;
                Ok::<_, RadarrError>(42)
            })
            .await;

        assert!(result.is_err());
        if let RadarrError::Timeout { operation } = result.unwrap_err() {
            assert!(operation.contains("test_service"));
        } else {
            panic!("Expected Timeout error");
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_metrics() {
        let config = CircuitBreakerConfig::new("test_service");
        let cb = CircuitBreaker::new(config);

        // Successful request
        let _ = cb.call(async { Ok::<_, RadarrError>(1) }).await;

        // Failed request
        let _ = cb
            .call(async {
                Err::<i32, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test".to_string(),
                    error: "test error".to_string(),
                })
            })
            .await;

        let metrics = cb.get_metrics().await;
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.service_name, "test_service");
        assert!(metrics.last_success_time.is_some());
        assert!(metrics.last_failure_time.is_some());
    }

    #[tokio::test]
    async fn test_manual_circuit_control() {
        let config = CircuitBreakerConfig::new("test_service");
        let cb = CircuitBreaker::new(config);

        // Force open
        cb.force_open().await;
        assert_eq!(cb.get_state().await, CircuitBreakerState::Open);

        // Request should be rejected
        let result = cb.call(async { Ok::<_, RadarrError>(42) }).await;
        assert!(result.is_err());

        // Force close
        cb.force_close().await;
        assert_eq!(cb.get_state().await, CircuitBreakerState::Closed);

        // Request should work
        let result = cb.call(async { Ok::<_, RadarrError>(42) }).await;
        assert!(result.is_ok());
    }
}
