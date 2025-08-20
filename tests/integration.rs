//! Integration tests for Radarr MVP
//!
//! This module contains comprehensive integration tests that test the complete movie workflow:
//! 1. Add movie via API
//! 2. Search for releases via Prowlarr integration 
//! 3. Start download via qBittorrent integration
//! 4. Track download progress
//! 5. Import completed downloads
//!
//! Tests use mock external services for reliable testing.

mod common;
mod mocks;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::{TestContext, create_test_movie, create_test_app};
use radarr_api::models::*;
use radarr_core::models::{MinimumAvailability};
use serde_json::json;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Test the complete movie workflow from addition to import
#[tokio::test]
async fn test_complete_movie_workflow() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Step 1: Add a movie via API
    let movie_request = CreateMovieRequest {
        tmdb_id: 550, // Fight Club
        title: Some("Fight Club".to_string()),
        monitored: true,
        quality_profile_id: Some(1),
        minimum_availability: Some(MinimumAvailability::Released),
        metadata: Some(json!({
            "tmdb": {
                "overview": "An insomniac office worker and a devil-may-care soapmaker form an underground fight club.",
                "vote_average": 8.4,
                "release_date": "1999-10-15"
            }
        })),
    };

    let response = server
        .post("/api/v3/movie")
        .json(&movie_request)
        .await;

    assert_eq!(response.status_code(), StatusCode::CREATED);
    
    let movie_response: MovieResponse = response.json();
    let movie_id = movie_response.id;
    assert_eq!(movie_response.tmdb_id, 550);
    assert_eq!(movie_response.title, "Fight Club");
    assert!(movie_response.monitored);

    // Step 2: Search for releases
    let search_response = server
        .get(&format!("/api/v3/release?movieId={}", movie_id))
        .await;

    assert_eq!(search_response.status_code(), StatusCode::OK);
    
    let releases: Vec<ReleaseResponse> = search_response.json();
    assert!(!releases.is_empty(), "Should find at least one release");
    
    let best_release = &releases[0];
    assert!(best_release.title.contains("Fight Club"));
    assert!(best_release.download_url.starts_with("magnet:") || best_release.download_url.starts_with("http"));

    // Step 3: Start download
    let download_request = DownloadRequest {
        guid: best_release.guid.clone(),
        indexer_id: best_release.indexer_id,
    };

    let download_response = server
        .post("/api/v3/command/download")
        .json(&download_request)
        .await;

    assert_eq!(download_response.status_code(), StatusCode::ACCEPTED);
    
    let download_result: DownloadResponse = download_response.json();
    let download_id = download_result.id;

    // Step 4: Track download progress
    let mut download_completed = false;
    for attempt in 0..10 {
        sleep(Duration::from_millis(100)).await;
        
        let status_response = server
            .get(&format!("/api/v3/download/{}", download_id))
            .await;
            
        assert_eq!(status_response.status_code(), StatusCode::OK);
        
        let download_status: DownloadResponse = status_response.json();
        
        if download_status.status == "completed" {
            download_completed = true;
            assert_eq!(download_status.progress, 100.0);
            break;
        }
        
        // Progress should be increasing or stable
        assert!(download_status.progress >= 0.0 && download_status.progress <= 100.0);
    }

    assert!(download_completed, "Download should complete within test timeout");

    // Step 5: Import completed download
    let import_response = server
        .post(&format!("/api/v3/command/import/{}", download_id))
        .await;

    assert_eq!(import_response.status_code(), StatusCode::ACCEPTED);

    // Verify movie now has file
    let updated_movie_response = server
        .get(&format!("/api/v3/movie/{}", movie_id))
        .await;

    assert_eq!(updated_movie_response.status_code(), StatusCode::OK);
    
    let updated_movie: MovieResponse = updated_movie_response.json();
    assert!(updated_movie.has_file, "Movie should have file after import");
    assert!(updated_movie.movie_file_id.is_some(), "Movie should have file ID");

    // Cleanup
    test_ctx.cleanup().await;
}

/// Test error handling for invalid movie IDs
#[tokio::test]
async fn test_invalid_movie_id_error_handling() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    let invalid_id = Uuid::new_v4();
    
    // Test getting non-existent movie
    let response = server
        .get(&format!("/api/v3/movie/{}", invalid_id))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    // Test updating non-existent movie
    let update_request = UpdateMovieRequest {
        monitored: Some(false),
        quality_profile_id: None,
        minimum_availability: None,
        metadata: None,
    };
    
    let response = server
        .put(&format!("/api/v3/movie/{}", invalid_id))
        .json(&update_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    // Test deleting non-existent movie
    let response = server
        .delete(&format!("/api/v3/movie/{}", invalid_id))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);

    test_ctx.cleanup().await;
}

/// Test external service failures
#[tokio::test]
async fn test_external_service_failures() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Create a movie first
    let movie = create_test_movie(&test_ctx.pool).await;

    // Test search with failing Prowlarr
    let search_response = server
        .get(&format!("/api/v3/release?movieId={}&fail_prowlarr=true", movie.id))
        .await;

    assert_eq!(search_response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);

    // Test download with failing qBittorrent  
    let download_request = DownloadRequest {
        guid: "test-guid".to_string(),
        indexer_id: 1,
    };

    let download_response = server
        .post("/api/v3/command/download?fail_qbittorrent=true")
        .json(&download_request)
        .await;

    assert_eq!(download_response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);

    test_ctx.cleanup().await;
}

/// Test database error conditions
#[tokio::test]
async fn test_database_error_handling() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Test creating duplicate TMDB ID
    let movie_request = CreateMovieRequest {
        tmdb_id: 123456,
        title: Some("Test Movie".to_string()),
        monitored: true,
        quality_profile_id: None,
        minimum_availability: None,
        metadata: None,
    };

    // First creation should succeed
    let response1 = server
        .post("/api/v3/movie")
        .json(&movie_request)
        .await;
    assert_eq!(response1.status_code(), StatusCode::CREATED);

    // Second creation should fail with conflict
    let response2 = server
        .post("/api/v3/movie")
        .json(&movie_request)
        .await;
    assert_eq!(response2.status_code(), StatusCode::CONFLICT);

    test_ctx.cleanup().await;
}

/// Test API validation and bad requests
#[tokio::test]
async fn test_api_validation() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Test invalid TMDB ID (negative)
    let invalid_request = json!({
        "tmdb_id": -1,
        "title": "Invalid Movie",
        "monitored": true
    });

    let response = server
        .post("/api/v3/movie")
        .json(&invalid_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    // Test invalid TMDB ID (zero)
    let zero_request = json!({
        "tmdb_id": 0,
        "title": "Zero Movie", 
        "monitored": true
    });

    let response = server
        .post("/api/v3/movie")
        .json(&zero_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    // Test missing required fields
    let incomplete_request = json!({
        "title": "Incomplete Movie"
        // Missing tmdb_id
    });

    let response = server
        .post("/api/v3/movie")
        .json(&incomplete_request)
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    // Test invalid UUID in path
    let response = server
        .get("/api/v3/movie/invalid-uuid")
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);

    test_ctx.cleanup().await;
}

/// Test concurrent operations and race conditions
#[tokio::test]
async fn test_concurrent_operations() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Create multiple movies concurrently
    let movie_requests: Vec<_> = (1..=10).map(|i| CreateMovieRequest {
        tmdb_id: 1000 + i,
        title: Some(format!("Concurrent Movie {}", i)),
        monitored: true,
        quality_profile_id: Some(1),
        minimum_availability: None,
        metadata: None,
    }).collect();

    let mut handles = Vec::new();
    
    for request in movie_requests {
        let server_clone = server.clone();
        let handle = tokio::spawn(async move {
            server_clone
                .post("/api/v3/movie")
                .json(&request)
                .await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let responses = futures::future::join_all(handles).await;
    
    // All should succeed
    for response_result in responses {
        let response = response_result.unwrap();
        assert_eq!(response.status_code(), StatusCode::CREATED);
    }

    // Verify all movies were created
    let list_response = server
        .get("/api/v3/movie?page=1&pageSize=20")
        .await;
    
    assert_eq!(list_response.status_code(), StatusCode::OK);
    
    let paginated_response: PaginatedResponse<MovieResponse> = list_response.json();
    assert!(paginated_response.records.len() >= 10);

    test_ctx.cleanup().await;
}

/// Test memory usage and connection pooling under load
#[tokio::test]
async fn test_performance_under_load() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Create a movie for testing
    let movie = create_test_movie(&test_ctx.pool).await;

    // Perform many concurrent read operations
    let mut handles = Vec::new();
    
    for _i in 0..50 {
        let server_clone = server.clone();
        let movie_id = movie.id;
        let handle = tokio::spawn(async move {
            let start = std::time::Instant::now();
            
            let response = server_clone
                .get(&format!("/api/v3/movie/{}", movie_id))
                .await;
                
            let duration = start.elapsed();
            (response.status_code(), duration)
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    
    // All requests should succeed
    for result in &results {
        let (status, duration) = result.as_ref().unwrap();
        assert_eq!(*status, StatusCode::OK);
        
        // Individual requests should be reasonably fast (< 1 second)
        assert!(duration.as_secs() < 1, "Request took too long: {:?}", duration);
    }

    // Calculate average response time
    let total_duration: std::time::Duration = results
        .iter()
        .map(|r| r.as_ref().unwrap().1)
        .sum();
    let avg_duration = total_duration / results.len() as u32;
    
    // Average should be under 100ms for simple reads
    assert!(avg_duration.as_millis() < 100, "Average response time too slow: {:?}", avg_duration);

    test_ctx.cleanup().await;
}

/// Test pagination and large result sets
#[tokio::test]
async fn test_pagination() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Create many movies
    for i in 1..=25 {
        let request = CreateMovieRequest {
            tmdb_id: 2000 + i,
            title: Some(format!("Pagination Test Movie {}", i)),
            monitored: true,
            quality_profile_id: None,
            minimum_availability: None,
            metadata: None,
        };

        let response = server
            .post("/api/v3/movie")
            .json(&request)
            .await;
        assert_eq!(response.status_code(), StatusCode::CREATED);
    }

    // Test first page
    let page1_response = server
        .get("/api/v3/movie?page=1&pageSize=10")
        .await;
    
    assert_eq!(page1_response.status_code(), StatusCode::OK);
    
    let page1: PaginatedResponse<MovieResponse> = page1_response.json();
    assert_eq!(page1.page, 1);
    assert_eq!(page1.page_size, 10);
    assert_eq!(page1.records.len(), 10);
    assert!(page1.total_records >= 25);
    assert!(page1.total_pages >= 3);

    // Test second page
    let page2_response = server
        .get("/api/v3/movie?page=2&pageSize=10")
        .await;
    
    assert_eq!(page2_response.status_code(), StatusCode::OK);
    
    let page2: PaginatedResponse<MovieResponse> = page2_response.json();
    assert_eq!(page2.page, 2);
    assert_eq!(page2.page_size, 10);
    assert_eq!(page2.records.len(), 10);

    // Ensure no duplicate records between pages
    let page1_ids: std::collections::HashSet<_> = page1.records.iter().map(|m| m.id).collect();
    let page2_ids: std::collections::HashSet<_> = page2.records.iter().map(|m| m.id).collect();
    
    assert!(page1_ids.is_disjoint(&page2_ids), "Pages should not have overlapping records");

    // Test invalid page numbers
    let invalid_page_response = server
        .get("/api/v3/movie?page=0&pageSize=10")
        .await;
    assert_eq!(invalid_page_response.status_code(), StatusCode::BAD_REQUEST);

    let large_page_response = server
        .get("/api/v3/movie?page=1000&pageSize=10")
        .await;
    assert_eq!(large_page_response.status_code(), StatusCode::OK);
    
    let large_page: PaginatedResponse<MovieResponse> = large_page_response.json();
    assert!(large_page.records.is_empty());

    test_ctx.cleanup().await;
}

/// Test health check endpoints
#[tokio::test]
async fn test_health_checks() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Test basic health check
    let health_response = server
        .get("/health")
        .await;
    
    assert_eq!(health_response.status_code(), StatusCode::OK);
    
    let health_status: serde_json::Value = health_response.json();
    assert_eq!(health_status["status"], "healthy");
    assert!(health_status["timestamp"].is_string());

    // Test readiness check
    let ready_response = server
        .get("/ready")
        .await;
    
    assert_eq!(ready_response.status_code(), StatusCode::OK);

    test_ctx.cleanup().await;
}

/// Test search functionality with various parameters
#[tokio::test]
async fn test_search_functionality() {
    let test_ctx = TestContext::new().await;
    let app = create_test_app(test_ctx.pool.clone()).await;
    let server = TestServer::new(app).unwrap();

    // Create test movie
    let movie = create_test_movie(&test_ctx.pool).await;

    // Test search by movie ID
    let search_response = server
        .get(&format!("/api/v3/release?movieId={}", movie.id))
        .await;
    
    assert_eq!(search_response.status_code(), StatusCode::OK);
    
    let releases: Vec<ReleaseResponse> = search_response.json();
    assert!(!releases.is_empty());

    // Test search with quality filters
    let quality_search_response = server
        .get(&format!("/api/v3/release?movieId={}&quality=1080p", movie.id))
        .await;
    
    assert_eq!(quality_search_response.status_code(), StatusCode::OK);

    // Test search with category filters
    let category_search_response = server
        .get(&format!("/api/v3/release?movieId={}&categories=2000,2010", movie.id))
        .await;
    
    assert_eq!(category_search_response.status_code(), StatusCode::OK);

    // Test invalid search parameters
    let invalid_search_response = server
        .get(&format!("/api/v3/release?movieId={}&quality=invalid", movie.id))
        .await;
    
    assert_eq!(invalid_search_response.status_code(), StatusCode::BAD_REQUEST);

    test_ctx.cleanup().await;
}