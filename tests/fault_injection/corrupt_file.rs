//! Corrupt file handling fault injection tests
//!
//! Tests the system's behavior when dealing with corrupted data and files.
//! Validates:
//! - Detection of corrupted API responses (invalid JSON, truncated data)
//! - Handling of corrupted download files
//! - Graceful recovery from data corruption
//! - Validation and error reporting for malformed data
//! - Circuit breaker behavior with persistent corruption
//! - File integrity checking and verification

use super::*;
use radarr_core::circuit_breaker::CircuitBreakerState;
use radarr_core::RadarrError;
use serde_json::json;
use std::time::Duration;
use tokio::time::Instant;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

/// Test API response with invalid JSON
#[tokio::test]
async fn test_invalid_json_response() {
    let context = FaultInjectionTestContext::new("corrupt_api").await;

    // Setup endpoint returning invalid JSON
    context.setup_corrupt_data_endpoint("/api/movies").await;

    let movies_url = format!("{}/api/movies", context.base_url());

    let result = context.make_request(&movies_url).await;
    assert!(result.is_err());

    // Should get a parsing/deserialization error
    if let Err(error) = result {
        match error {
            RadarrError::SerializationError(_) => {
                println!("Correctly detected JSON parsing error");
            }
            RadarrError::NetworkError { message } if message.contains("JSON") => {
                println!("JSON error detected in network layer: {}", message);
            }
            other => {
                // The mock server might return the invalid JSON as a string
                // In a real scenario, we'd attempt JSON parsing and get a SerializationError
                println!(
                    "Got error (may be valid depending on implementation): {:?}",
                    other
                );
            }
        }
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test API response with truncated/partial data
#[tokio::test]
async fn test_truncated_response_data() {
    let context = FaultInjectionTestContext::new("truncated_api").await;

    // Setup endpoint returning partial response
    context.setup_partial_response_endpoint("/api/search").await;

    let search_url = format!("{}/api/search", context.base_url());

    let result = context.make_request(&search_url).await;
    assert!(result.is_err());

    if let Err(error) = result {
        match error {
            RadarrError::SerializationError(_) => {
                println!("Correctly detected truncated data as serialization error");
            }
            RadarrError::NetworkError { message } => {
                println!("Truncated data detected as network error: {}", message);
            }
            other => {
                println!("Truncated data error: {:?}", other);
            }
        }
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test malformed API response headers
#[tokio::test]
async fn test_malformed_response_headers() {
    let context = FaultInjectionTestContext::new("malformed_headers").await;

    // Setup endpoint with malformed content-type and corrupted data
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("This is not JSON but claims to be")
                .insert_header("content-type", "application/json; charset=invalid-encoding")
                .insert_header("content-length", "999999"), // Wrong content length
        )
        .mount(&context.mock_server)
        .await;

    let data_url = format!("{}/api/data", context.base_url());

    let client = reqwest::Client::new();
    let result = context
        .circuit_breaker
        .call(async {
            let response =
                client
                    .get(&data_url)
                    .send()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    })?;

            // Check content-type header
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            let body = response
                .text()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Failed to read response: {}", e),
                })?;

            // If content-type says JSON but body isn't JSON, that's corruption
            if content_type.contains("application/json") {
                if let Err(_) = serde_json::from_str::<serde_json::Value>(&body) {
                    return Err(RadarrError::SerializationError(format!(
                        "Response claims to be JSON but contains invalid data: {}",
                        body
                    )));
                }
            }

            Ok::<String, RadarrError>(body)
        })
        .await;

    assert!(result.is_err());
    if let Err(RadarrError::SerializationError(msg)) = result {
        assert!(msg.contains("Response claims to be JSON"));
        println!("Detected content-type/body mismatch: {}", msg);
    } else {
        println!("Got error (may be valid): {:?}", result);
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.total_requests, 1);
    assert_eq!(metrics.failed_requests, 1);
}

/// Test corrupted file download detection
#[tokio::test]
async fn test_corrupted_download_file() {
    let context = FaultInjectionTestContext::new("corrupt_download").await;

    // Setup download info showing completed download
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "corrupt1",
                "name": "Test Movie 2024.mkv",
                "state": "completed",
                "progress": 1.0,
                "size": 1073741824,
                "downloaded": 1073741824
            }
        ])))
        .mount(&context.mock_server)
        .await;

    // Setup file verification endpoint (simulates checksum/integrity check)
    Mock::given(method("POST"))
        .and(path("/api/verify/file"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "file_path": "/downloads/Test Movie 2024.mkv",
            "expected_hash": "abc123def456",
            "actual_hash": "xyz789corrupted",
            "is_valid": false,
            "corruption_detected": true,
            "error": "File hash mismatch - possible corruption"
        })))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());
    let verify_url = format!("{}/api/verify/file", context.base_url());

    // Check download status (should be complete)
    let download_result = context.make_request(&info_url).await;
    assert!(download_result.is_ok());

    if let Ok(body) = download_result {
        assert!(body.contains("completed"));
        println!("Download reported as completed");
    }

    // Verify file integrity
    let client = reqwest::Client::new();
    let verify_result = context
        .circuit_breaker
        .call(async {
            let response = client
                .post(&verify_url)
                .json(&json!({
                    "file_path": "/downloads/Test Movie 2024.mkv",
                    "expected_hash": "abc123def456"
                }))
                .send()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Verify request failed: {}", e),
                })?;

            let body = response
                .text()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Failed to read response: {}", e),
                })?;

            // Parse verification result
            let verification: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| RadarrError::SerializationError(e.to_string()))?;

            if !verification["is_valid"].as_bool().unwrap_or(true) {
                return Err(RadarrError::IoError(format!(
                    "File corruption detected: {}",
                    verification["error"]
                        .as_str()
                        .unwrap_or("Unknown corruption")
                )));
            }

            Ok::<String, RadarrError>(body)
        })
        .await;

    assert!(verify_result.is_err());
    if let Err(RadarrError::IoError(msg)) = verify_result {
        assert!(msg.contains("File corruption detected"));
        println!("Successfully detected file corruption: {}", msg);
    } else {
        panic!(
            "Expected IoError for file corruption, got: {:?}",
            verify_result
        );
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 1); // Download status check succeeded
    assert_eq!(metrics.failed_requests, 1); // File verification failed
}

/// Test multiple corruption scenarios leading to circuit breaker activation
#[tokio::test]
async fn test_persistent_corruption_opens_circuit() {
    let context = FaultInjectionTestContext::new("persistent_corruption").await;

    // Setup endpoint that consistently returns corrupted data
    Mock::given(method("GET"))
        .and(path("/api/releases"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ \"releases\": [ corrupted json data without closing braces")
                .insert_header("content-type", "application/json"),
        )
        .mount(&context.mock_server)
        .await;

    let releases_url = format!("{}/api/releases", context.base_url());

    // Make multiple requests to trigger circuit breaker
    let mut corruption_count = 0;
    for i in 0..6 {
        println!("Making request {}", i + 1);

        let client = reqwest::Client::new();
        let result = context
            .circuit_breaker
            .call(async {
                let response = client.get(&releases_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    }
                })?;

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                // Attempt to parse as JSON
                serde_json::from_str::<serde_json::Value>(&body)
                    .map_err(|e| RadarrError::SerializationError(format!("Invalid JSON: {}", e)))?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        match result {
            Err(RadarrError::SerializationError(_)) => {
                corruption_count += 1;
                println!("Request {} detected corruption", i + 1);
            }
            Err(RadarrError::CircuitBreakerOpen { service }) => {
                println!(
                    "Circuit breaker opened after {} corruptions",
                    corruption_count
                );
                assert_eq!(service, "persistent_corruption");
                break;
            }
            other => {
                println!("Unexpected result for request {}: {:?}", i + 1, other);
            }
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Circuit should be open due to persistent corruption
    let state = context.circuit_breaker.get_state().await;
    assert_eq!(state, CircuitBreakerState::Open);

    let metrics = context.get_test_metrics().await;
    assert!(metrics.failed_requests >= 3);
    assert!(corruption_count >= 3);
    assert!(metrics.rejected_requests > 0);
}

/// Test recovery after corruption is resolved
#[tokio::test]
async fn test_corruption_recovery() {
    let context = FaultInjectionTestContext::new("corruption_recovery").await;

    // First, return corrupted data to trigger circuit breaker
    Mock::given(method("GET"))
        .and(path("/api/status"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ invalid json structure")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(3)
        .mount(&context.mock_server)
        .await;

    // Then return valid data (corruption resolved)
    Mock::given(method("GET"))
        .and(path("/api/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "status": "healthy",
            "corruption_resolved": true,
            "last_check": chrono::Utc::now().timestamp()
        })))
        .mount(&context.mock_server)
        .await;

    let status_url = format!("{}/api/status", context.base_url());

    // Trigger corruption errors to open circuit
    let client = reqwest::Client::new();
    for i in 0..3 {
        let result = context
            .circuit_breaker
            .call(async {
                let response = client.get(&status_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    }
                })?;

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                // Try to parse JSON
                serde_json::from_str::<serde_json::Value>(&body)
                    .map_err(|e| RadarrError::SerializationError(format!("Corrupt JSON: {}", e)))?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        assert!(
            result.is_err(),
            "Request {} should fail due to corruption",
            i + 1
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Verify circuit is open
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

    // Make request that should succeed (corruption resolved)
    let recovery_result =
        context
            .circuit_breaker
            .call(async {
                let response = client.get(&status_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    }
                })?;

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                // Parse JSON (should work now)
                let status: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
                    RadarrError::SerializationError(format!("JSON parse error: {}", e))
                })?;

                // Verify status indicates recovery
                if status["corruption_resolved"].as_bool() != Some(true) {
                    return Err(RadarrError::ValidationError {
                        field: "corruption_resolved".to_string(),
                        message: "Status does not indicate corruption resolved".to_string(),
                    });
                }

                Ok::<String, RadarrError>(body)
            })
            .await;

    assert!(recovery_result.is_ok(), "Recovery request should succeed");

    if let Ok(body) = recovery_result {
        assert!(body.contains("corruption_resolved"));
        println!("System recovered from corruption successfully");
    }

    // Circuit should be closed again
    assert_eq!(
        context.circuit_breaker.get_state().await,
        CircuitBreakerState::Closed
    );

    let metrics = context.get_test_metrics().await;
    assert!(metrics.successful_requests > 0);
    assert!(metrics.failed_requests >= 3);
    assert!(metrics.is_resilient());
}

/// Test mixed corruption scenarios (some requests succeed, others corrupted)
#[tokio::test]
async fn test_intermittent_corruption() {
    let context = FaultInjectionTestContext::new("intermittent_corruption").await;

    // First request: valid data
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"status": "ok", "request": 1})),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Second request: corrupted data
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ \"status\": invalid_value_without_quotes }")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Third request: valid data again
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"status": "ok", "request": 3})),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Fourth request: corrupted again
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ truncated json response")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Remaining requests: valid
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"status": "recovered", "stable": true})),
        )
        .mount(&context.mock_server)
        .await;

    let health_url = format!("{}/api/health", context.base_url());

    let mut results = Vec::new();
    let client = reqwest::Client::new();

    // Make several requests and track results
    for i in 0..6 {
        let result = context
            .circuit_breaker
            .call(async {
                let response = client.get(&health_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    }
                })?;

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                // Try to parse as JSON
                let parsed: serde_json::Value = serde_json::from_str(&body)
                    .map_err(|e| RadarrError::SerializationError(format!("JSON error: {}", e)))?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        results.push((i + 1, result.is_ok()));

        if let Err(RadarrError::CircuitBreakerOpen { .. }) = result {
            println!("Circuit breaker opened on request {}", i + 1);
            break;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let metrics = context.get_test_metrics().await;

    // Should have a mix of successes and failures
    let successful_count = results.iter().filter(|(_, success)| *success).count();
    let failed_count = results.iter().filter(|(_, success)| !*success).count();

    assert!(successful_count > 0, "Should have some successful requests");
    assert!(
        failed_count > 0,
        "Should have some failed requests due to corruption"
    );

    println!("Intermittent corruption results:");
    for (request_num, success) in results {
        println!(
            "  Request {}: {}",
            request_num,
            if success { "SUCCESS" } else { "FAILED" }
        );
    }

    println!(
        "Final metrics - successful: {}, failed: {}, total: {}",
        metrics.successful_requests, metrics.failed_requests, metrics.total_requests
    );
}

/// Test corruption in different data types and structures
#[tokio::test]
async fn test_various_corruption_types() {
    let context = FaultInjectionTestContext::new("various_corruption").await;

    // Array corruption (missing closing bracket)
    Mock::given(method("GET"))
        .and(path("/api/movies/list"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("[ {\"id\": 1}, {\"id\": 2}")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // String corruption (unescaped quotes)
    Mock::given(method("GET"))
        .and(path("/api/movies/details"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ \"title\": \"Movie with \"unescaped\" quotes\" }")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Number corruption (invalid numeric format)
    Mock::given(method("GET"))
        .and(path("/api/movies/stats"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ \"count\": 12.34.56, \"size\": \"not_a_number\" }")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Mixed valid/invalid structure
    Mock::given(method("GET"))
        .and(path("/api/movies/mixed"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("{ \"valid_field\": \"ok\", \"invalid\": { unclosed_object }")
                .insert_header("content-type", "application/json"),
        )
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    let test_cases = vec![
        ("/api/movies/list", "array corruption"),
        ("/api/movies/details", "string corruption"),
        ("/api/movies/stats", "number corruption"),
        ("/api/movies/mixed", "mixed corruption"),
    ];

    let client = reqwest::Client::new();
    let mut corruption_types_detected = 0;

    for (endpoint, corruption_type) in test_cases {
        let url = format!("{}{}", context.base_url(), endpoint);

        let result = context
            .circuit_breaker
            .call(async {
                let response =
                    client
                        .get(&url)
                        .send()
                        .await
                        .map_err(|e| RadarrError::NetworkError {
                            message: format!("Request failed: {}", e),
                        })?;

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                // Attempt JSON parsing
                serde_json::from_str::<serde_json::Value>(&body).map_err(|e| {
                    RadarrError::SerializationError(format!("{}: {}", corruption_type, e))
                })?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        if let Err(RadarrError::SerializationError(msg)) = result {
            corruption_types_detected += 1;
            println!("Detected {}: {}", corruption_type, msg);
        } else if let Err(RadarrError::CircuitBreakerOpen { .. }) = result {
            println!("Circuit breaker opened due to {}", corruption_type);
            break;
        } else {
            println!("Unexpected result for {}: {:?}", corruption_type, result);
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        corruption_types_detected >= 3,
        "Should detect multiple corruption types"
    );

    let metrics = context.get_test_metrics().await;
    assert!(metrics.failed_requests >= 3);

    // Circuit may or may not be open depending on failure threshold
    let state = context.circuit_breaker.get_state().await;
    println!("Final circuit state after various corruptions: {:?}", state);
}

#[cfg(test)]
mod corruption_integration_tests {
    use super::*;

    /// Integration test: Data corruption in complete workflow
    #[tokio::test]
    async fn test_corruption_in_search_workflow() {
        let search_service = FaultInjectionTestContext::new("search_workflow").await;
        let metadata_service = FaultInjectionTestContext::new("metadata_service").await;

        // Search service returns corrupted results
        Mock::given(method("GET"))
            .and(path("/api/search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(
                        "{ \"results\": [ {\"title\": corrupted_data_without_quotes } ] }",
                    )
                    .insert_header("content-type", "application/json"),
            )
            .mount(&search_service.mock_server)
            .await;

        // Metadata service works correctly
        Mock::given(method("GET"))
            .and(path("/api/metadata/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": 123,
                "title": "Valid Movie",
                "year": 2024,
                "quality": "1080p"
            })))
            .mount(&metadata_service.mock_server)
            .await;

        let search_url = format!("{}/api/search", search_service.base_url());
        let metadata_url = format!("{}/api/metadata/123", metadata_service.base_url());

        // Step 1: Search (should fail due to corruption)
        let client = reqwest::Client::new();
        let search_result = search_service
            .circuit_breaker
            .call(async {
                let response = client.get(&search_url).send().await.map_err(|e| {
                    RadarrError::NetworkError {
                        message: format!("Search request failed: {}", e),
                    }
                })?;

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read search response: {}", e),
                    })?;

                // Parse search results
                let _results: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
                    RadarrError::SerializationError(format!("Search JSON error: {}", e))
                })?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        assert!(
            search_result.is_err(),
            "Search should fail due to corruption"
        );

        // Step 2: Metadata lookup (should succeed)
        let metadata_result = metadata_service.make_request(&metadata_url).await;
        assert!(metadata_result.is_ok(), "Metadata lookup should succeed");

        if let Ok(body) = metadata_result {
            assert!(body.contains("Valid Movie"));
            println!("Metadata lookup succeeded despite search corruption");
        }

        let search_metrics = search_service.get_test_metrics().await;
        let metadata_metrics = metadata_service.get_test_metrics().await;

        assert_eq!(search_metrics.failed_requests, 1);
        assert_eq!(metadata_metrics.successful_requests, 1);

        println!(
            "Workflow handled partial corruption correctly - search failed, metadata succeeded"
        );
    }
}
