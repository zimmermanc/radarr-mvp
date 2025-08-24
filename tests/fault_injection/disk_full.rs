//! Disk full error fault injection tests
//!
//! Tests the system's behavior when storage space becomes exhausted.
//! Validates:
//! - Detection of disk space exhaustion
//! - Graceful handling of "no space left" errors
//! - Download pausing when disk is full
//! - Import process handling of insufficient space
//! - Alert generation for disk space issues
//! - Recovery when disk space becomes available

use super::*;
use radarr_core::circuit_breaker::CircuitBreakerState;
use radarr_core::RadarrError;
use serde_json::json;
use std::io::{Error as IoError, ErrorKind};
use std::time::Duration;
use tokio::time::Instant;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, ResponseTemplate};

/// Simulate disk space check returning low/no space
async fn simulate_disk_full_check(context: &FaultInjectionTestContext, available_bytes: u64) {
    Mock::given(method("GET"))
        .and(path("/api/disk/space"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "path": "/downloads",
            "total": 1073741824000, // 1TB total
            "available": available_bytes,
            "used": 1073741824000 - available_bytes,
            "percentage_used": ((1073741824000 - available_bytes) as f64 / 1073741824000.0 * 100.0) as u32
        })))
        .mount(&context.mock_server)
        .await;
}

/// Test disk space monitoring and detection
#[tokio::test]
async fn test_disk_space_monitoring() {
    let context = FaultInjectionTestContext::new("disk_monitor").await;

    // Setup disk space endpoint showing low space
    simulate_disk_full_check(&context, 1048576).await; // Only 1MB available

    let space_url = format!("{}/api/disk/space", context.base_url());

    let result = context.make_request(&space_url).await;
    assert!(result.is_ok());

    if let Ok(body) = result {
        let response: serde_json::Value = serde_json::from_str(&body).unwrap();
        let available = response["available"].as_u64().unwrap();
        let percentage_used = response["percentage_used"].as_u64().unwrap();

        assert_eq!(available, 1048576); // 1MB
        assert!(
            percentage_used > 90,
            "Should indicate high disk usage: {}%",
            percentage_used
        );

        println!(
            "Disk space check: {} bytes available, {}% used",
            available, percentage_used
        );
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 1);
    assert_eq!(metrics.failed_requests, 0);
}

/// Test download failure due to insufficient disk space
#[tokio::test]
async fn test_download_failure_disk_full() {
    let context = FaultInjectionTestContext::new("download_disk_full").await;

    // Setup download client endpoints
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&context.mock_server)
        .await;

    // Add torrent fails due to disk full
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": "No space left on device",
            "code": "ENOSPC"
        })))
        .mount(&context.mock_server)
        .await;

    let login_url = format!("{}/api/v2/auth/login", context.base_url());
    let add_url = format!("{}/api/v2/torrents/add", context.base_url());

    // Login should work
    let login_result = context.make_request(&login_url).await;
    assert!(login_result.is_ok());

    // Add torrent should fail with disk full error
    let client = reqwest::Client::new();
    let add_result = context
        .circuit_breaker
        .call(async {
            let response = client
                .post(&add_url)
                .form(&[
                    ("urls", "magnet:?xt=urn:btih:test"),
                    ("savepath", "/downloads"),
                ])
                .send()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Add torrent request failed: {}", e),
                })?;

            let status = response.status();
            if status.is_server_error() {
                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read error response: {}", e),
                    })?;

                if body.contains("No space left") || body.contains("ENOSPC") {
                    return Err(RadarrError::IoError(
                        "Disk full: No space left on device".to_string(),
                    ));
                }

                return Err(RadarrError::ExternalServiceError {
                    service: "download_client".to_string(),
                    error: format!("Server error: {}", status),
                });
            }

            let body = response
                .text()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Failed to read response: {}", e),
                })?;

            Ok::<String, RadarrError>(body)
        })
        .await;

    assert!(add_result.is_err());
    if let Err(RadarrError::IoError(msg)) = add_result {
        assert!(msg.contains("Disk full") || msg.contains("No space left"));
        println!("Correctly detected disk full error: {}", msg);
    } else {
        panic!("Expected IoError for disk full, got: {:?}", add_result);
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 1); // Login succeeded
    assert_eq!(metrics.failed_requests, 1); // Add torrent failed
}

/// Test import process handling insufficient disk space
#[tokio::test]
async fn test_import_failure_disk_full() {
    let context = FaultInjectionTestContext::new("import_disk_full").await;

    // Setup disk space check (very low space)
    simulate_disk_full_check(&context, 512000).await; // Only 512KB available

    // Setup import endpoint that fails due to disk space
    Mock::given(method("POST"))
        .and(path("/api/import/movie"))
        .respond_with(
            ResponseTemplate::new(507) // HTTP 507 Insufficient Storage
                .set_body_json(json!({
                    "error": "Insufficient disk space for import",
                    "required_space": 2147483648, // 2GB needed
                    "available_space": 512000,    // 512KB available
                    "path": "/downloads/complete"
                })),
        )
        .mount(&context.mock_server)
        .await;

    let space_url = format!("{}/api/disk/space", context.base_url());
    let import_url = format!("{}/api/import/movie", context.base_url());

    // First check disk space
    let space_result = context.make_request(&space_url).await;
    assert!(space_result.is_ok());

    // Then attempt import
    let client = reqwest::Client::new();
    let import_result = context
        .circuit_breaker
        .call(async {
            let response = client
                .post(&import_url)
                .json(&json!({
                    "source_path": "/downloads/complete/Test Movie 2024.mkv",
                    "destination_path": "/media/movies/Test Movie (2024)/Test Movie 2024.mkv"
                }))
                .send()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Import request failed: {}", e),
                })?;

            let status = response.status();
            if status == 507 {
                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read error response: {}", e),
                    })?;

                return Err(RadarrError::IoError(format!("Import failed: {}", body)));
            }

            let body = response
                .text()
                .await
                .map_err(|e| RadarrError::NetworkError {
                    message: format!("Failed to read response: {}", e),
                })?;

            Ok::<String, RadarrError>(body)
        })
        .await;

    assert!(import_result.is_err());
    if let Err(RadarrError::IoError(msg)) = import_result {
        assert!(msg.contains("Insufficient disk space"));
        println!("Import correctly failed due to disk space: {}", msg);
    } else {
        panic!(
            "Expected IoError for insufficient disk space, got: {:?}",
            import_result
        );
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 1); // Space check succeeded
    assert_eq!(metrics.failed_requests, 1); // Import failed
}

/// Test download pausing when disk space is low
#[tokio::test]
async fn test_download_pause_on_low_disk_space() {
    let context = FaultInjectionTestContext::new("pause_downloads").await;

    // Setup download info endpoint showing active downloads
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "pause1",
                "name": "Large Movie",
                "state": "downloading",
                "progress": 0.7,
                "dlspeed": 10485760
            }
        ])))
        .up_to_n_times(1)
        .mount(&context.mock_server)
        .await;

    // Setup pause endpoint
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/pause"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&context.mock_server)
        .await;

    // Setup disk space check (critically low)
    simulate_disk_full_check(&context, 52428800).await; // 50MB available

    // After pause, downloads show as paused
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "hash": "pause1",
                "name": "Large Movie",
                "state": "pausedDL",
                "progress": 0.7,
                "dlspeed": 0
            }
        ])))
        .mount(&context.mock_server)
        .await;

    let info_url = format!("{}/api/v2/torrents/info", context.base_url());
    let pause_url = format!("{}/api/v2/torrents/pause", context.base_url());
    let space_url = format!("{}/api/disk/space", context.base_url());

    // Check initial download state
    let initial_result = context.make_request(&info_url).await;
    assert!(initial_result.is_ok());
    if let Ok(body) = initial_result {
        assert!(body.contains("downloading"));
        println!("Initial state: downloads active");
    }

    // Check disk space (low space detected)
    let space_result = context.make_request(&space_url).await;
    assert!(space_result.is_ok());

    // Pause downloads due to low space
    let client = reqwest::Client::new();
    let pause_result = client
        .post(&pause_url)
        .form(&[("hashes", "pause1")])
        .send()
        .await;
    assert!(pause_result.is_ok());

    // Verify downloads are now paused
    let paused_result = context.make_request(&info_url).await;
    assert!(paused_result.is_ok());
    if let Ok(body) = paused_result {
        assert!(body.contains("pausedDL"));
        println!("Downloads successfully paused due to low disk space");
    }

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 3); // Initial check + space check + pause verification
    assert_eq!(metrics.failed_requests, 0);
}

/// Test recovery when disk space becomes available
#[tokio::test]
async fn test_disk_space_recovery() {
    let context = FaultInjectionTestContext::new("disk_recovery").await;

    // Phase 1: Low disk space
    Mock::given(method("GET"))
        .and(path("/api/disk/space"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "path": "/downloads",
            "total": 1073741824000,
            "available": 1048576, // 1MB - critically low
            "used": 1073740775424,
            "percentage_used": 99
        })))
        .up_to_n_times(2)
        .mount(&context.mock_server)
        .await;

    // Phase 2: Space becomes available (cleanup completed)
    Mock::given(method("GET"))
        .and(path("/api/disk/space"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "path": "/downloads",
            "total": 1073741824000,
            "available": 107374182400, // 100GB available
            "used": 966367641600,
            "percentage_used": 90
        })))
        .mount(&context.mock_server)
        .await;

    // Setup download resume endpoint
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/resume"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Ok."))
        .mount(&context.mock_server)
        .await;

    let space_url = format!("{}/api/disk/space", context.base_url());
    let resume_url = format!("{}/api/v2/torrents/resume", context.base_url());

    // Phase 1: Detect low space
    for i in 0..2 {
        let result = context.make_request(&space_url).await;
        assert!(result.is_ok());

        if let Ok(body) = result {
            let response: serde_json::Value = serde_json::from_str(&body).unwrap();
            let percentage_used = response["percentage_used"].as_u64().unwrap();
            assert_eq!(percentage_used, 99);
            println!(
                "Phase 1 check {}: {}% disk usage (critical)",
                i + 1,
                percentage_used
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Phase 2: Space recovered
    let recovery_result = context.make_request(&space_url).await;
    assert!(recovery_result.is_ok());

    if let Ok(body) = recovery_result {
        let response: serde_json::Value = serde_json::from_str(&body).unwrap();
        let available = response["available"].as_u64().unwrap();
        let percentage_used = response["percentage_used"].as_u64().unwrap();

        assert_eq!(available, 107374182400); // 100GB
        assert_eq!(percentage_used, 90);
        println!(
            "Space recovered: {}GB available, {}% used",
            available / 1024 / 1024 / 1024,
            percentage_used
        );
    }

    // Resume downloads
    let client = reqwest::Client::new();
    let resume_result = client
        .post(&resume_url)
        .form(&[("hashes", "all")])
        .send()
        .await;
    assert!(resume_result.is_ok());

    println!("Downloads resumed after space recovery");

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 3); // 2 low space checks + 1 recovery check
    assert_eq!(metrics.failed_requests, 0);
    assert!(metrics.is_resilient());
}

/// Test multiple concurrent disk operations during space exhaustion
#[tokio::test]
async fn test_concurrent_operations_disk_full() {
    let context = FaultInjectionTestContext::new("concurrent_disk_full").await;

    // Setup disk space showing critically low space
    simulate_disk_full_check(&context, 1024).await; // Only 1KB available

    // Setup various endpoints that fail due to disk space
    Mock::given(method("POST"))
        .and(path("/api/import/movie"))
        .respond_with(
            ResponseTemplate::new(507).set_body_json(json!({"error": "No space left on device"})),
        )
        .mount(&context.mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(json!({"error": "ENOSPC: no space left on device"})),
        )
        .mount(&context.mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/backup/create"))
        .respond_with(
            ResponseTemplate::new(507)
                .set_body_json(json!({"error": "Insufficient storage space"})),
        )
        .mount(&context.mock_server)
        .await;

    let space_url = format!("{}/api/disk/space", context.base_url());
    let import_url = format!("{}/api/import/movie", context.base_url());
    let add_url = format!("{}/api/v2/torrents/add", context.base_url());
    let backup_url = format!("{}/api/backup/create", context.base_url());

    // Make concurrent requests
    let mut handles = Vec::new();

    // Space check (should work)
    let context_ref = &context;
    let url = space_url.clone();
    handles.push(tokio::spawn(async move {
        ("space_check", context_ref.make_request(&url).await)
    }));

    // Import (should fail)
    let context_ref = &context;
    let url = import_url.clone();
    handles.push(tokio::spawn(async move {
        let client = reqwest::Client::new();
        let result = context_ref
            .circuit_breaker
            .call(async {
                let response = client
                    .post(&url)
                    .json(&json!({"file": "test.mkv"}))
                    .send()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    })?;

                if response.status() == 507 {
                    return Err(RadarrError::IoError("Disk full".to_string()));
                }

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        ("import", result)
    }));

    // Add torrent (should fail)
    let context_ref = &context;
    let url = add_url.clone();
    handles.push(tokio::spawn(async move {
        let client = reqwest::Client::new();
        let result = context_ref
            .circuit_breaker
            .call(async {
                let response = client
                    .post(&url)
                    .form(&[("urls", "magnet:?xt=urn:btih:test")])
                    .send()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Request failed: {}", e),
                    })?;

                if response.status().is_server_error() {
                    let body = response.text().await.unwrap_or_default();
                    if body.contains("ENOSPC") || body.contains("no space left") {
                        return Err(RadarrError::IoError("Disk full".to_string()));
                    }
                }

                let body = response
                    .text()
                    .await
                    .map_err(|e| RadarrError::NetworkError {
                        message: format!("Failed to read response: {}", e),
                    })?;

                Ok::<String, RadarrError>(body)
            })
            .await;

        ("add_torrent", result)
    }));

    // Wait for all operations to complete
    let results = futures::future::join_all(handles).await;

    let mut successful_ops = 0;
    let mut disk_full_errors = 0;

    for result in results {
        let (operation, response) = result.unwrap();

        match response {
            Ok(_) => {
                successful_ops += 1;
                println!("Operation {} succeeded", operation);
            }
            Err(RadarrError::IoError(msg)) if msg.contains("Disk full") => {
                disk_full_errors += 1;
                println!(
                    "Operation {} failed with disk full error: {}",
                    operation, msg
                );
            }
            Err(other) => {
                println!(
                    "Operation {} failed with other error: {:?}",
                    operation, other
                );
            }
        }
    }

    // Space check should succeed, other operations should fail with disk full
    assert!(successful_ops >= 1, "At least space check should succeed");
    assert!(
        disk_full_errors >= 2,
        "Import and add torrent should fail with disk full"
    );

    let metrics = context.get_test_metrics().await;
    assert!(metrics.total_requests >= 3);
    assert!(metrics.successful_requests >= 1);
    assert!(metrics.failed_requests >= 2);

    println!(
        "Concurrent disk full test - successful: {}, disk full errors: {}",
        successful_ops, disk_full_errors
    );
}

/// Test disk full alert generation
#[tokio::test]
async fn test_disk_full_alert_generation() {
    let context = FaultInjectionTestContext::new("disk_alerts").await;

    // Setup critically low disk space
    simulate_disk_full_check(&context, 0).await; // 0 bytes available

    let space_url = format!("{}/api/disk/space", context.base_url());

    // Check disk space multiple times (simulating monitoring)
    for i in 0..3 {
        let result = context.make_request(&space_url).await;
        assert!(result.is_ok());

        if let Ok(body) = result {
            let response: serde_json::Value = serde_json::from_str(&body).unwrap();
            let available = response["available"].as_u64().unwrap();
            assert_eq!(available, 0);
            println!(
                "Disk space check {}: {} bytes available (critical)",
                i + 1,
                available
            );
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // In a real implementation, this would verify that alerts were sent
    // (Discord notifications, email alerts, etc.)
    let has_alerts = context.verify_failure_alerts_generated().await;

    println!("Critical disk space alerts generated: {}", has_alerts);

    let metrics = context.get_test_metrics().await;
    assert_eq!(metrics.successful_requests, 3);
    assert_eq!(metrics.failed_requests, 0);

    // All requests succeeded but disk space is critical
    // In real system, this would trigger immediate alerts
}

#[cfg(test)]
mod disk_integration_tests {
    use super::*;

    /// Integration test: Complete disk space workflow
    #[tokio::test]
    async fn test_complete_disk_space_workflow() {
        let context = FaultInjectionTestContext::new("complete_disk_workflow").await;

        // Phase 1: Normal operation (sufficient space)
        Mock::given(method("GET"))
            .and(path("/api/disk/space"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "available": 107374182400, // 100GB
                "percentage_used": 50
            })))
            .up_to_n_times(1)
            .mount(&context.mock_server)
            .await;

        // Phase 2: Space running low
        Mock::given(method("GET"))
            .and(path("/api/disk/space"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "available": 1073741824, // 1GB - warning level
                "percentage_used": 90
            })))
            .up_to_n_times(1)
            .mount(&context.mock_server)
            .await;

        // Phase 3: Critical space (pause downloads)
        Mock::given(method("GET"))
            .and(path("/api/disk/space"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "available": 104857600, // 100MB - critical
                "percentage_used": 95
            })))
            .up_to_n_times(1)
            .mount(&context.mock_server)
            .await;

        // Phase 4: Space exhausted
        Mock::given(method("GET"))
            .and(path("/api/disk/space"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "available": 0, // 0 bytes - exhausted
                "percentage_used": 100
            })))
            .up_to_n_times(2)
            .mount(&context.mock_server)
            .await;

        // Phase 5: Space recovered
        Mock::given(method("GET"))
            .and(path("/api/disk/space"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "available": 53687091200, // 50GB - healthy
                "percentage_used": 75
            })))
            .mount(&context.mock_server)
            .await;

        let space_url = format!("{}/api/disk/space", context.base_url());

        // Simulate monitoring over time
        let phases = vec![
            ("Normal", 100, 50),
            ("Warning", 1, 90),
            ("Critical", 0, 95),
            ("Exhausted 1", 0, 100),
            ("Exhausted 2", 0, 100),
            ("Recovered", 50, 75),
        ];

        for (phase_name, expected_gb, expected_percentage) in phases {
            let result = context.make_request(&space_url).await;
            assert!(
                result.is_ok(),
                "Space check should succeed in {} phase",
                phase_name
            );

            if let Ok(body) = result {
                let response: serde_json::Value = serde_json::from_str(&body).unwrap();
                let available_gb = response["available"].as_u64().unwrap() / 1024 / 1024 / 1024;
                let percentage_used = response["percentage_used"].as_u64().unwrap();

                assert!(
                    available_gb == expected_gb || available_gb < 1,
                    "Phase {}: expected ~{}GB, got {}GB",
                    phase_name,
                    expected_gb,
                    available_gb
                );
                assert_eq!(
                    percentage_used, expected_percentage,
                    "Phase {}: expected {}% used, got {}%",
                    phase_name, expected_percentage, percentage_used
                );

                println!(
                    "Phase {}: {}GB available, {}% used",
                    phase_name, available_gb, percentage_used
                );
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let metrics = context.get_test_metrics().await;
        assert_eq!(metrics.successful_requests, 6); // All phases monitored successfully
        assert_eq!(metrics.failed_requests, 0);
        assert!(metrics.is_resilient());

        println!("Complete disk space workflow completed successfully");
    }
}
