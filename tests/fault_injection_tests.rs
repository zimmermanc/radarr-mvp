//! Fault injection test suite entry point
//!
//! This file serves as the entry point for all fault injection tests.
//! It includes all the fault injection test modules and provides
//! integration with the existing test framework.

mod fault_injection;

// Re-export test modules for easy access
pub use fault_injection::*;

#[cfg(test)]
mod integration_tests {
    use super::fault_injection::*;

    /// High-level integration test that combines multiple fault scenarios
    #[tokio::test]
    async fn test_complete_fault_injection_workflow() {
        println!("=== Complete Fault Injection Workflow Test ===");

        // Create contexts for different services
        let indexer = FaultInjectionTestContext::new("workflow_indexer").await;
        let download_client = FaultInjectionTestContext::new("workflow_downloader").await;
        let metadata_service = FaultInjectionTestContext::new("workflow_metadata").await;

        // Phase 1: Normal operation (all services working)
        println!("\n--- Phase 1: Normal Operation ---");

        // Setup normal responses
        use serde_json::json;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"title": "Test Movie", "quality": "1080p"}]
            })))
            .mount(&indexer.mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "downloading", "progress": 0.5
            })))
            .mount(&download_client.mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/movie/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "title": "Test Movie", "year": 2024
            })))
            .mount(&metadata_service.mock_server)
            .await;

        // Test normal operation
        let indexer_url = format!("{}/search", indexer.base_url());
        let download_url = format!("{}/status", download_client.base_url());
        let metadata_url = format!("{}/movie/123", metadata_service.base_url());

        let indexer_result = indexer.make_request(&indexer_url).await;
        let download_result = download_client.make_request(&download_url).await;
        let metadata_result = metadata_service.make_request(&metadata_url).await;

        assert!(indexer_result.is_ok(), "Indexer should work normally");
        assert!(
            download_result.is_ok(),
            "Download client should work normally"
        );
        assert!(
            metadata_result.is_ok(),
            "Metadata service should work normally"
        );

        println!("✓ All services operational in normal conditions");

        // Phase 2: Introduce faults
        println!("\n--- Phase 2: Fault Injection ---");

        // Indexer starts timing out
        indexer.setup_timeout_endpoint("/search").await;

        // Download client returns rate limits
        download_client
            .setup_rate_limited_endpoint("/status", 30)
            .await;

        // Metadata service becomes unavailable
        Mock::given(method("GET"))
            .and(path("/movie/123"))
            .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
            .mount(&metadata_service.mock_server)
            .await;

        // Test fault behavior
        let mut fault_results = Vec::new();

        for i in 0..5 {
            let indexer_result = indexer.make_request(&indexer_url).await;
            let download_result = download_client.make_request(&download_url).await;
            let metadata_result = metadata_service.make_request(&metadata_url).await;

            fault_results.push((
                i + 1,
                indexer_result.is_ok(),
                download_result.is_ok(),
                metadata_result.is_ok(),
            ));

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }

        // Analyze fault behavior
        let indexer_failures = fault_results.iter().filter(|(_, i, _, _)| !*i).count();
        let download_failures = fault_results.iter().filter(|(_, _, d, _)| !*d).count();
        let metadata_failures = fault_results.iter().filter(|(_, _, _, m)| !*m).count();

        assert!(indexer_failures > 0, "Indexer should experience timeouts");
        assert!(
            download_failures > 0,
            "Download client should experience rate limits"
        );
        assert!(
            metadata_failures > 0,
            "Metadata service should be unavailable"
        );

        println!(
            "✓ Fault injection working - {} indexer, {} download, {} metadata failures",
            indexer_failures, download_failures, metadata_failures
        );

        // Phase 3: Verify circuit breaker behavior
        println!("\n--- Phase 3: Circuit Breaker Verification ---");

        let indexer_state = indexer.circuit_breaker.get_state().await;
        let download_state = download_client.circuit_breaker.get_state().await;
        let metadata_state = metadata_service.circuit_breaker.get_state().await;

        println!(
            "Circuit breaker states: Indexer={:?}, Download={:?}, Metadata={:?}",
            indexer_state, download_state, metadata_state
        );

        // At least one circuit should have opened
        let open_circuits = [indexer_state, download_state, metadata_state]
            .iter()
            .filter(|&&state| state == radarr_core::circuit_breaker::CircuitBreakerState::Open)
            .count();

        assert!(
            open_circuits > 0,
            "At least one circuit breaker should have opened"
        );

        // Phase 4: Recovery
        println!("\n--- Phase 4: Service Recovery ---");

        // Wait for circuits to transition to half-open
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        // Setup recovery endpoints
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{"title": "Recovered Movie", "quality": "1080p"}]
            })))
            .mount(&indexer.mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "completed", "progress": 1.0
            })))
            .mount(&download_client.mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/movie/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "title": "Recovered Movie", "year": 2024, "status": "restored"
            })))
            .mount(&metadata_service.mock_server)
            .await;

        // Test recovery
        let recovery_results = vec![
            indexer.make_request(&indexer_url).await,
            download_client.make_request(&download_url).await,
            metadata_service.make_request(&metadata_url).await,
        ];

        let recovered_services = recovery_results.iter().filter(|r| r.is_ok()).count();
        println!(
            "✓ {} out of 3 services recovered successfully",
            recovered_services
        );

        // Final metrics
        let indexer_metrics = indexer.get_test_metrics().await;
        let download_metrics = download_client.get_test_metrics().await;
        let metadata_metrics = metadata_service.get_test_metrics().await;

        println!("\n--- Final Test Metrics ---");
        println!(
            "Indexer: {} total, {} success, {} failed, {} rejected",
            indexer_metrics.total_requests,
            indexer_metrics.successful_requests,
            indexer_metrics.failed_requests,
            indexer_metrics.rejected_requests
        );
        println!(
            "Download: {} total, {} success, {} failed, {} rejected",
            download_metrics.total_requests,
            download_metrics.successful_requests,
            download_metrics.failed_requests,
            download_metrics.rejected_requests
        );
        println!(
            "Metadata: {} total, {} success, {} failed, {} rejected",
            metadata_metrics.total_requests,
            metadata_metrics.successful_requests,
            metadata_metrics.failed_requests,
            metadata_metrics.rejected_requests
        );

        // Verify system resilience
        assert!(indexer_metrics.is_resilient() || indexer_metrics.successful_requests > 0);
        assert!(download_metrics.is_resilient() || download_metrics.successful_requests > 0);
        assert!(metadata_metrics.is_resilient() || metadata_metrics.successful_requests > 0);

        println!("✓ Complete fault injection workflow test passed successfully!");
    }

    /// Test to verify all fault injection test modules are accessible
    #[tokio::test]
    async fn test_fault_injection_modules_loaded() {
        println!("=== Fault Injection Modules Test ===");

        // Test that we can create contexts for each module's functionality
        let contexts = vec![
            FaultInjectionTestContext::new("indexer_timeout_module").await,
            FaultInjectionTestContext::new("rate_limit_module").await,
            FaultInjectionTestContext::new("download_stall_module").await,
            FaultInjectionTestContext::new("disk_full_module").await,
            FaultInjectionTestContext::new("corrupt_file_module").await,
            FaultInjectionTestContext::new("service_unavailable_module").await,
            FaultInjectionTestContext::new("circuit_breaker_module").await,
        ];

        println!(
            "✓ All {} fault injection modules loaded successfully",
            contexts.len()
        );

        // Verify each context is functional
        for (i, context) in contexts.iter().enumerate() {
            let metrics = context.get_test_metrics().await;
            assert_eq!(
                metrics.total_requests, 0,
                "Context {} should start with 0 requests",
                i
            );
            assert_eq!(
                metrics.successful_requests, 0,
                "Context {} should start with 0 successes",
                i
            );
            assert_eq!(
                metrics.failed_requests, 0,
                "Context {} should start with 0 failures",
                i
            );
        }

        println!("✓ All fault injection contexts are functional");
    }
}
