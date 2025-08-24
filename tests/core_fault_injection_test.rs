//! Core fault injection test - tests only circuit breaker functionality

use radarr_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use radarr_core::{RadarrError, Result};
use std::time::Duration;

#[tokio::test]
async fn test_circuit_breaker_basic_functionality() {
    println!("=== Circuit Breaker Basic Functionality Test ===");

    let config = CircuitBreakerConfig::new("test_service")
        .with_failure_threshold(3)
        .with_timeout(Duration::from_millis(100));
    let circuit_breaker = CircuitBreaker::new(config);

    // Initial state should be closed
    assert_eq!(
        circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );
    println!("✓ Circuit breaker starts in Closed state");

    // Simulate successful requests
    for i in 0..3 {
        let result = circuit_breaker
            .call(async { Ok::<String, RadarrError>(format!("Success {}", i + 1)) })
            .await;

        assert!(result.is_ok());
        println!("Request {}: Success", i + 1);
    }

    // Circuit should still be closed
    assert_eq!(
        circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );
    println!("✓ Circuit remains closed after successful requests");

    // Simulate failing requests
    for i in 0..4 {
        let result = circuit_breaker
            .call(async {
                Err::<String, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test_service".to_string(),
                    error: format!("Simulated error {}", i + 1),
                })
            })
            .await;

        match result {
            Err(RadarrError::ExternalServiceError { .. }) => {
                println!("Request {}: Failed as expected", i + 4);
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                println!(
                    "Request {}: Circuit breaker opened (service: {})",
                    i + 4,
                    service
                );
                break;
            }
            other => {
                println!("Request {}: Unexpected result: {:?}", i + 4, other);
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Circuit should be open now
    assert_eq!(circuit_breaker.get_state().await, CircuitBreakerState::Open);
    println!("✓ Circuit breaker opened after failures");

    // Additional requests should be rejected
    let rejected_result = circuit_breaker
        .call(async { Ok::<String, RadarrError>("Should not execute".to_string()) })
        .await;

    assert!(rejected_result.is_err());
    if let Err(RadarrError::CircuitBreakerOpen { service }) = rejected_result {
        assert_eq!(service, "test_service");
        println!("✓ Additional requests correctly rejected");
    }

    let metrics = circuit_breaker.get_metrics().await;
    println!(
        "Final metrics: {} total, {} successful, {} failed, {} rejected",
        metrics.total_requests,
        metrics.successful_requests,
        metrics.failed_requests,
        metrics.rejected_requests
    );

    assert_eq!(metrics.successful_requests, 3);
    assert!(metrics.failed_requests >= 3);
    assert!(metrics.rejected_requests >= 1);

    println!("✓ Circuit breaker basic functionality test passed!");
}

#[tokio::test]
async fn test_circuit_breaker_recovery() {
    println!("=== Circuit Breaker Recovery Test ===");

    let config = CircuitBreakerConfig::new("recovery_service")
        .with_failure_threshold(2)
        .with_timeout(Duration::from_millis(50))
        .with_success_threshold(1);
    let circuit_breaker = CircuitBreaker::new(config);

    // Cause failures to open circuit
    for i in 0..3 {
        let result = circuit_breaker
            .call(async {
                Err::<String, RadarrError>(RadarrError::NetworkError {
                    message: format!("Network error {}", i + 1),
                })
            })
            .await;

        println!("Failure {}: {:?}", i + 1, result.is_err());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert_eq!(circuit_breaker.get_state().await, CircuitBreakerState::Open);
    println!("✓ Circuit opened due to failures");

    // Wait for timeout to transition to half-open
    tokio::time::sleep(Duration::from_millis(75)).await;

    // Make successful request to close circuit
    let recovery_result = circuit_breaker
        .call(async { Ok::<String, RadarrError>("Recovery successful".to_string()) })
        .await;

    assert!(recovery_result.is_ok());
    if let Ok(message) = recovery_result {
        assert_eq!(message, "Recovery successful");
        println!("✓ Recovery request succeeded: {}", message);
    }

    // Circuit should be closed again
    assert_eq!(
        circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );
    println!("✓ Circuit closed after successful recovery");

    // Verify further requests work
    let post_recovery_result = circuit_breaker
        .call(async { Ok::<String, RadarrError>("Post recovery request".to_string()) })
        .await;

    assert!(post_recovery_result.is_ok());
    println!("✓ Post-recovery requests working normally");

    let final_metrics = circuit_breaker.get_metrics().await;
    println!(
        "Recovery metrics: {} total, {} successful, {} failed",
        final_metrics.total_requests,
        final_metrics.successful_requests,
        final_metrics.failed_requests
    );

    assert!(final_metrics.successful_requests >= 2); // Recovery + post-recovery
    assert!(final_metrics.failed_requests >= 2); // Initial failures

    println!("✓ Circuit breaker recovery test passed!");
}

#[tokio::test]
async fn test_circuit_breaker_timeout_functionality() {
    println!("=== Circuit Breaker Timeout Test ===");

    let config = CircuitBreakerConfig::new("timeout_service")
        .with_failure_threshold(2)
        .with_request_timeout(Duration::from_millis(50));
    let circuit_breaker = CircuitBreaker::new(config);

    // Simulate request that times out
    let start_time = std::time::Instant::now();
    let timeout_result = circuit_breaker
        .call(async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<String, RadarrError>("Should timeout".to_string())
        })
        .await;
    let elapsed = start_time.elapsed();

    assert!(timeout_result.is_err());
    assert!(
        elapsed < Duration::from_millis(100),
        "Should timeout quickly: {:?}",
        elapsed
    );

    if let Err(RadarrError::Timeout { operation }) = timeout_result {
        assert!(operation.contains("timeout_service"));
        println!("✓ Timeout detected correctly: {}", operation);
    } else {
        panic!("Expected timeout error, got: {:?}", timeout_result);
    }

    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);

    println!("✓ Circuit breaker timeout functionality test passed!");
}

#[tokio::test]
async fn test_circuit_breaker_manual_control() {
    println!("=== Circuit Breaker Manual Control Test ===");

    let config = CircuitBreakerConfig::new("manual_service");
    let circuit_breaker = CircuitBreaker::new(config);

    // Test manual open
    circuit_breaker.force_open().await;
    assert_eq!(circuit_breaker.get_state().await, CircuitBreakerState::Open);
    println!("✓ Circuit manually opened");

    // Request should be rejected
    let rejected_result = circuit_breaker
        .call(async { Ok::<String, RadarrError>("Should be rejected".to_string()) })
        .await;

    assert!(rejected_result.is_err());
    if let Err(RadarrError::CircuitBreakerOpen { service }) = rejected_result {
        assert_eq!(service, "manual_service");
        println!("✓ Request correctly rejected when manually opened");
    }

    // Test manual close
    circuit_breaker.force_close().await;
    assert_eq!(
        circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );
    println!("✓ Circuit manually closed");

    // Request should now succeed
    let success_result = circuit_breaker
        .call(async { Ok::<String, RadarrError>("Now should work".to_string()) })
        .await;

    assert!(success_result.is_ok());
    if let Ok(message) = success_result {
        println!("✓ Request succeeded after manual close: {}", message);
    }

    // Test metrics reset
    let metrics_before = circuit_breaker.get_metrics().await;
    assert!(metrics_before.total_requests > 0);

    circuit_breaker.reset_metrics().await;
    let metrics_after = circuit_breaker.get_metrics().await;

    assert_eq!(metrics_after.total_requests, 0);
    assert_eq!(metrics_after.successful_requests, 0);
    assert_eq!(metrics_after.failed_requests, 0);
    assert_eq!(metrics_after.rejected_requests, 0);
    println!("✓ Metrics reset successfully");

    println!("✓ Circuit breaker manual control test passed!");
}

#[tokio::test]
async fn test_circuit_breaker_health_check() {
    println!("=== Circuit Breaker Health Check Test ===");

    let config = CircuitBreakerConfig::new("health_service").with_failure_threshold(3);
    let circuit_breaker = CircuitBreaker::new(config);

    // Initial health should be good (no requests)
    assert!(circuit_breaker.is_healthy().await);
    println!("✓ Circuit breaker healthy with no requests");

    // Make successful requests
    for i in 0..5 {
        let result = circuit_breaker
            .call(async { Ok::<String, RadarrError>(format!("Success {}", i + 1)) })
            .await;
        assert!(result.is_ok());
    }

    assert!(circuit_breaker.is_healthy().await);
    println!("✓ Circuit breaker healthy with all successes");

    // Make some failures (but not enough to open circuit)
    let _ = circuit_breaker
        .call(async {
            Err::<String, RadarrError>(RadarrError::NetworkError {
                message: "Single failure".to_string(),
            })
        })
        .await;

    assert!(circuit_breaker.is_healthy().await);
    println!("✓ Circuit breaker still healthy with single failure");

    // Make enough failures to open circuit
    for _ in 0..3 {
        let _ = circuit_breaker
            .call(async {
                Err::<String, RadarrError>(RadarrError::ExternalServiceError {
                    service: "test".to_string(),
                    error: "Multiple failures".to_string(),
                })
            })
            .await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert!(!circuit_breaker.is_healthy().await);
    println!("✓ Circuit breaker unhealthy after multiple failures");

    let final_metrics = circuit_breaker.get_metrics().await;
    println!(
        "Health check metrics: success_rate={:.1}%, circuit_state={:?}",
        final_metrics.successful_requests as f64 / final_metrics.total_requests as f64 * 100.0,
        final_metrics.state
    );

    println!("✓ Circuit breaker health check test passed!");
}

#[tokio::test]
async fn test_different_error_types() {
    println!("=== Different Error Types Test ===");

    let config = CircuitBreakerConfig::new("error_types_service").with_failure_threshold(2);
    let circuit_breaker = CircuitBreaker::new(config);

    let error_types = vec![
        RadarrError::NetworkError {
            message: "Network failure".to_string(),
        },
        RadarrError::ExternalServiceError {
            service: "test_service".to_string(),
            error: "Service error".to_string(),
        },
        RadarrError::TemporaryError {
            message: "Temporary issue".to_string(),
        },
        RadarrError::IoError("IO failure".to_string()),
        RadarrError::SerializationError("JSON parse error".to_string()),
    ];

    // Test each error type
    for (i, error) in error_types.iter().enumerate() {
        let result = circuit_breaker
            .call(async { Err::<String, RadarrError>(error.clone()) })
            .await;

        match result {
            Err(ref e) if std::mem::discriminant(e) == std::mem::discriminant(error) => {
                println!("Error type {}: Handled correctly - {:?}", i + 1, e);
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                println!(
                    "Error type {}: Circuit breaker opened (service: {})",
                    i + 1,
                    service
                );
                break;
            }
            other => {
                println!("Error type {}: Unexpected result: {:?}", i + 1, other);
            }
        }

        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let metrics = circuit_breaker.get_metrics().await;
    println!(
        "Error types metrics: {} total, {} failed",
        metrics.total_requests, metrics.failed_requests
    );

    assert!(metrics.failed_requests >= 2);

    println!("✓ Different error types test passed!");
}

#[tokio::test]
async fn test_concurrent_circuit_breaker_access() {
    println!("=== Concurrent Circuit Breaker Access Test ===");

    let config = CircuitBreakerConfig::new("concurrent_service").with_failure_threshold(5);
    let circuit_breaker = std::sync::Arc::new(CircuitBreaker::new(config));

    let mut handles = Vec::new();

    // Create concurrent tasks
    for i in 0..10 {
        let cb = circuit_breaker.clone();
        let handle = tokio::spawn(async move {
            let result = cb
                .call(async {
                    if i % 3 == 0 {
                        // Some requests fail
                        Err::<String, RadarrError>(RadarrError::NetworkError {
                            message: format!("Network error from task {}", i),
                        })
                    } else {
                        // Most requests succeed
                        Ok::<String, RadarrError>(format!("Success from task {}", i))
                    }
                })
                .await;

            (i, result)
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;

    let mut successes = 0;
    let mut failures = 0;
    let mut rejections = 0;

    for result in results {
        let (task_id, response) = result.unwrap();

        match response {
            Ok(_) => {
                successes += 1;
                println!("Task {}: Success", task_id);
            }
            Err(RadarrError::NetworkError { .. }) => {
                failures += 1;
                println!("Task {}: Network error", task_id);
            }
            Err(RadarrError::CircuitBreakerOpen { .. }) => {
                rejections += 1;
                println!("Task {}: Circuit breaker rejection", task_id);
            }
            Err(other) => {
                println!("Task {}: Other error: {:?}", task_id, other);
            }
        }
    }

    println!(
        "Concurrent results: {} successes, {} failures, {} rejections",
        successes, failures, rejections
    );

    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.total_requests, 10);
    assert_eq!(
        metrics.successful_requests as usize
            + metrics.failed_requests as usize
            + metrics.rejected_requests as usize,
        10
    );

    println!(
        "Final concurrent metrics: {} total, {} successful, {} failed, {} rejected",
        metrics.total_requests,
        metrics.successful_requests,
        metrics.failed_requests,
        metrics.rejected_requests
    );

    println!("✓ Concurrent circuit breaker access test passed!");
}

#[tokio::test]
async fn test_fault_injection_complete_workflow() {
    println!("=== Complete Fault Injection Workflow Test ===");

    // Simulate different service components
    let indexer_config = CircuitBreakerConfig::new("indexer_service")
        .with_failure_threshold(3)
        .with_timeout(Duration::from_millis(50));
    let indexer_cb = CircuitBreaker::new(indexer_config);

    let downloader_config = CircuitBreakerConfig::new("downloader_service")
        .with_failure_threshold(2)
        .with_timeout(Duration::from_millis(75));
    let downloader_cb = CircuitBreaker::new(downloader_config);

    // Phase 1: Normal operation
    println!("\n--- Phase 1: Normal Operation ---");

    let indexer_result = indexer_cb
        .call(async { Ok::<String, RadarrError>("Indexer search results".to_string()) })
        .await;

    let downloader_result = downloader_cb
        .call(async { Ok::<String, RadarrError>("Download started".to_string()) })
        .await;

    assert!(indexer_result.is_ok());
    assert!(downloader_result.is_ok());
    println!("✓ Both services operational");

    // Phase 2: Introduce faults
    println!("\n--- Phase 2: Fault Injection ---");

    // Indexer starts failing
    for i in 0..4 {
        let result = indexer_cb
            .call(async {
                Err::<String, RadarrError>(RadarrError::ExternalServiceError {
                    service: "indexer".to_string(),
                    error: "Indexer timeout".to_string(),
                })
            })
            .await;

        if let Err(RadarrError::CircuitBreakerOpen { .. }) = result {
            println!("Indexer circuit opened on attempt {}", i + 1);
            break;
        } else {
            println!("Indexer failure {}: {:?}", i + 1, result.is_err());
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Downloader gets rate limited
    for i in 0..3 {
        let result = downloader_cb
            .call(async {
                Err::<String, RadarrError>(RadarrError::RateLimited {
                    service: "downloader".to_string(),
                    retry_after: Some(30),
                })
            })
            .await;

        if let Err(RadarrError::CircuitBreakerOpen { .. }) = result {
            println!("Downloader circuit opened on attempt {}", i + 1);
            break;
        } else {
            println!("Downloader rate limit {}: {:?}", i + 1, result.is_err());
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Phase 3: Check circuit states
    println!("\n--- Phase 3: Circuit Breaker States ---");

    let indexer_state = indexer_cb.get_state().await;
    let downloader_state = downloader_cb.get_state().await;

    println!("Indexer circuit: {:?}", indexer_state);
    println!("Downloader circuit: {:?}", downloader_state);

    // At least one should be open
    assert!(
        indexer_state == CircuitBreakerState::Open || downloader_state == CircuitBreakerState::Open
    );

    // Phase 4: Recovery simulation
    println!("\n--- Phase 4: Service Recovery ---");

    // Wait for circuits to potentially transition to half-open
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try recovery requests
    let indexer_recovery = indexer_cb
        .call(async { Ok::<String, RadarrError>("Indexer recovered".to_string()) })
        .await;

    let downloader_recovery = downloader_cb
        .call(async { Ok::<String, RadarrError>("Downloader recovered".to_string()) })
        .await;

    println!("Indexer recovery: {:?}", indexer_recovery.is_ok());
    println!("Downloader recovery: {:?}", downloader_recovery.is_ok());

    // Final metrics
    let indexer_metrics = indexer_cb.get_metrics().await;
    let downloader_metrics = downloader_cb.get_metrics().await;

    println!("\n--- Final Metrics ---");
    println!(
        "Indexer: {} total, {} success, {} failed, {} rejected",
        indexer_metrics.total_requests,
        indexer_metrics.successful_requests,
        indexer_metrics.failed_requests,
        indexer_metrics.rejected_requests
    );
    println!(
        "Downloader: {} total, {} success, {} failed, {} rejected",
        downloader_metrics.total_requests,
        downloader_metrics.successful_requests,
        downloader_metrics.failed_requests,
        downloader_metrics.rejected_requests
    );

    // Verify system demonstrated resilience
    assert!(indexer_metrics.total_requests > 0);
    assert!(downloader_metrics.total_requests > 0);

    println!("✓ Complete fault injection workflow test passed!");
}
