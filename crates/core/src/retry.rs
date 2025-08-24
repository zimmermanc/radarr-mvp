//! Retry logic with exponential backoff and circuit breaker patterns

use crate::{RadarrError, Result};
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, error, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a configuration for quick retries (API calls)
    pub fn quick() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create a configuration for slow retries (downloads, imports)
    pub fn slow() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(300),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Retry policy determines which errors should be retried
#[derive(Debug, Clone, Copy)]
pub enum RetryPolicy {
    /// Retry all errors
    All,
    /// Retry only transient errors (network, timeout, etc)
    Transient,
    /// Never retry
    Never,
}

/// Execute an async operation with retry logic
pub async fn retry_with_backoff<F, Fut, T>(
    config: RetryConfig,
    policy: RetryPolicy,
    operation_name: &str,
    mut operation: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = config.initial_delay;

    loop {
        attempt += 1;
        debug!(
            "Attempting {} (attempt {}/{})",
            operation_name, attempt, config.max_attempts
        );

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!("{} succeeded on attempt {}", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(err) => {
                // Check if we should retry this error
                if !should_retry(&err, policy) {
                    debug!(
                        "{} failed with non-retryable error: {}",
                        operation_name, err
                    );
                    return Err(err);
                }

                // Check if we've exhausted attempts
                if attempt >= config.max_attempts {
                    error!(
                        "{} failed after {} attempts: {}",
                        operation_name, config.max_attempts, err
                    );
                    return Err(RadarrError::RetryExhausted {
                        operation: operation_name.to_string(),
                        attempts: config.max_attempts,
                        last_error: Box::new(err),
                    });
                }

                // Calculate next delay with exponential backoff
                warn!(
                    "{} failed on attempt {}/{}: {}. Retrying in {:?}",
                    operation_name, attempt, config.max_attempts, err, delay
                );

                sleep(delay).await;

                // Calculate next delay
                delay = calculate_next_delay(delay, &config);
            }
        }
    }
}

/// Determine if an error should be retried based on policy
fn should_retry(error: &RadarrError, policy: RetryPolicy) -> bool {
    match policy {
        RetryPolicy::Never => false,
        RetryPolicy::All => true,
        RetryPolicy::Transient => matches!(
            error,
            RadarrError::NetworkError { .. }
                | RadarrError::Timeout { .. }
                | RadarrError::ExternalServiceError { .. }
                | RadarrError::TemporaryError { .. }
        ),
    }
}

/// Calculate the next retry delay with exponential backoff and jitter
fn calculate_next_delay(current: Duration, config: &RetryConfig) -> Duration {
    let mut next = current.mul_f64(config.backoff_multiplier);

    // Apply maximum delay cap
    if next > config.max_delay {
        next = config.max_delay;
    }

    // Add jitter if configured
    if config.jitter {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.5..1.5);
        next = next.mul_f64(jitter_factor);
    }

    next
}

/// Circuit breaker for protecting against cascading failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Name of the protected service
    name: String,
    /// Current state of the circuit
    state: CircuitState,
    /// Number of failures before opening circuit
    failure_threshold: u32,
    /// Current failure count
    failure_count: u32,
    /// Duration to wait before attempting to close circuit
    reset_timeout: Duration,
    /// Time when circuit was opened
    opened_at: Option<std::time::Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(name: impl Into<String>, failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            name: name.into(),
            state: CircuitState::Closed,
            failure_threshold,
            failure_count: 0,
            reset_timeout,
            opened_at: None,
        }
    }

    /// Check if the circuit allows requests
    pub fn can_proceed(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if we should transition to half-open
                if let Some(opened_at) = self.opened_at {
                    if opened_at.elapsed() >= self.reset_timeout {
                        debug!("Circuit breaker {} transitioning to half-open", self.name);
                        self.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                debug!(
                    "Circuit breaker {} closing after successful test",
                    self.name
                );
                self.state = CircuitState::Closed;
                self.failure_count = 0;
                self.opened_at = None;
            }
            CircuitState::Closed => {
                // Reset failure count on success
                if self.failure_count > 0 {
                    self.failure_count = 0;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                warn!("Circuit breaker {} received success while open", self.name);
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                // Failed test, reopen circuit
                warn!("Circuit breaker {} reopening after failed test", self.name);
                self.state = CircuitState::Open;
                self.opened_at = Some(std::time::Instant::now());
                self.failure_count = 0;
            }
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    error!(
                        "Circuit breaker {} opening after {} failures",
                        self.name, self.failure_count
                    );
                    self.state = CircuitState::Open;
                    self.opened_at = Some(std::time::Instant::now());
                }
            }
            CircuitState::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Execute an operation with circuit breaker protection
    pub async fn execute<F, Fut, T>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        if !self.can_proceed() {
            return Err(RadarrError::CircuitBreakerOpen {
                service: self.name.clone(),
            });
        }

        match operation().await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(err) => {
                self.record_failure();
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let attempt = Arc::new(AtomicU32::new(0));
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };

        let attempt_clone = attempt.clone();
        let result = retry_with_backoff(config, RetryPolicy::All, "test_operation", move || {
            let attempt = attempt_clone.clone();
            async move {
                let current_attempt = attempt.fetch_add(1, Ordering::SeqCst) + 1;
                if current_attempt == 2 {
                    Ok(42)
                } else {
                    Err(RadarrError::TemporaryError {
                        message: "simulated failure".to_string(),
                    })
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let config = RetryConfig {
            max_attempts: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };

        let result: Result<()> =
            retry_with_backoff(config, RetryPolicy::All, "test_operation", || async {
                Err(RadarrError::TemporaryError {
                    message: "always fails".to_string(),
                })
            })
            .await;

        assert!(matches!(result, Err(RadarrError::RetryExhausted { .. })));
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let mut cb = CircuitBreaker::new("test", 2, Duration::from_millis(100));

        // Initially closed
        assert!(cb.can_proceed());

        // First failure
        cb.record_failure();
        assert!(cb.can_proceed());

        // Second failure opens circuit
        cb.record_failure();
        assert!(!cb.can_proceed());

        // Wait for reset timeout
        std::thread::sleep(Duration::from_millis(100));

        // Should transition to half-open
        assert!(cb.can_proceed());

        // Success closes circuit
        cb.record_success();
        assert!(cb.can_proceed());
    }
}
