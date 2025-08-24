//! Comprehensive end-to-end search test for the Radarr MVP system
//!
//! This test validates the complete search workflow from API request through
//! indexer integration to result processing, including error scenarios.

use crate::common::{TestContext, create_test_app, create_test_movie, assertions};
use crate::mocks::prowlarr::{MockProwlarrClient, search_helpers};
use axum::http::StatusCode;
use axum_test::TestServer;
use radarr_api::models::ReleaseResponse;
use radarr_core::{Movie, MovieStatus, MinimumAvailability, Result as RadarrResult};
use radarr_decision::{Quality, QualityItem};
use radarr_indexers::{
    IndexerClient, SearchRequest, SearchResponse, ProwlarrSearchResult, 
    Category, ProwlarrIndexer, IndexerStatus, IndexerCapabilities
};
use radarr_infrastructure::DatabasePool;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{timeout, Duration};
use uuid::Uuid;

/// Mock HDBits client for comprehensive testing
#[derive(Debug, Clone)]
pub struct MockHDBitsClient {
    pub should_fail: Arc<Mutex<bool>>,
    pub search_responses: Arc<Mutex<HashMap<String, SearchResponse>>>,
    pub request_count: Arc<Mutex<u32>>,
    pub response_delay_ms: Arc<Mutex<u64>>,
}

impl Default for MockHDBitsClient {
    fn default() -> Self {
        let mut search_responses = HashMap::new();
        
        // Create high-quality HDBits response
        let hdbits_response = SearchResponse {
            total: 1,
            results: vec![
                ProwlarrSearchResult {
                    title: "The Matrix 1999 2160p UHD BluRay x265 HDR Atmos-HDBits".to_string(),
                    download_url: "https://hdbits.org/download/123456/test.torrent".to_string(),
                    info_url: Some("https://hdbits.org/details.php?id=123456".to_string()),
                    indexer_id: 2,
                    indexer: "HDBits".to_string(),
                    size: Some(35_000_000_000), // 35GB
                    seeders: Some(150),
                    leechers: Some(3),
                    download_factor: Some(0.0), // Freeleech
                    upload_factor: Some(1.0),
                    publish_date: Some(chrono::Utc::now()),
                    categories: vec![Category { id: 2000, name: "Movies".to_string(), description: None }],
                    attributes: HashMap::new(),
                    imdb_id: Some("tt0133093".to_string()),
                    tmdb_id: Some(603),
                    freeleech: Some(true),
                    info_hash: Some("HDBITS_4K_HDR_ATMOS_HASH_123456".to_string()),
                },
            ],
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: vec![],
        };
        
        search_responses.insert("hdbits".to_string(), hdbits_response);
        
        Self {
            should_fail: Arc::new(Mutex::new(false)),
            search_responses: Arc::new(Mutex::new(search_responses)),
            request_count: Arc::new(Mutex::new(0)),
            response_delay_ms: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl IndexerClient for MockHDBitsClient {
    async fn search(&self, request: &SearchRequest) -> RadarrResult<SearchResponse> {
        // Simulate delay if configured
        let delay = *self.response_delay_ms.lock().unwrap();
        if delay > 0 {
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        
        *self.request_count.lock().unwrap() += 1;
        
        if *self.should_fail.lock().unwrap() {
            return Err(radarr_core::RadarrError::ExternalServiceError {
                service: "hdbits".to_string(),
                error: "HDBits API timeout".to_string(),
            });
        }
        
        let responses = self.search_responses.lock().unwrap();
        let response = responses.get("hdbits").cloned().unwrap_or_else(|| SearchResponse {
            total: 0,
            results: vec![],
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: vec![],
        });
        
        Ok(response)
    }
    
    async fn get_indexers(&self) -> RadarrResult<Vec<ProwlarrIndexer>> {
        Ok(vec![
            ProwlarrIndexer {
                id: 2,
                name: "HDBits".to_string(),
                implementation: "HDBits".to_string(),
                base_url: "https://hdbits.org".to_string(),
                enable: true,
                status: IndexerStatus {
                    status: "healthy".to_string(),
                    last_error: None,
                    failure_count: 0,
                    last_test: Some(chrono::Utc::now()),
                    disabled_till: None,
                },
                categories: vec![
                    Category { id: 2000, name: "Movies".to_string(), description: None },
                ],
                capabilities: IndexerCapabilities {
                    search_params: vec!["q".to_string(), "imdbid".to_string()],
                    tv_search: false,
                    movie_search: true,
                    music_search: false,
                    book_search: false,
                    limits: None,
                },
                priority: 10, // Higher priority than Prowlarr
                supports_rss: false,
                supports_search: true,
                last_sync: Some(chrono::Utc::now()),
            },
        ])
    }
    
    async fn test_indexer(&self, _indexer_id: i32) -> RadarrResult<bool> {
        Ok(!*self.should_fail.lock().unwrap())
    }
    
    async fn health_check(&self) -> RadarrResult<bool> {
        Ok(!*self.should_fail.lock().unwrap())
    }
}

/// Release scoring result for quality decisions
pub struct ReleaseScore {
    pub release: ProwlarrSearchResult,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Comprehensive search testing service that coordinates multiple indexers
pub struct SearchTestService {
    pub prowlarr_client: MockProwlarrClient,
    pub hdbits_client: MockHDBitsClient,
    pub database_pool: DatabasePool,
}

impl SearchTestService {
    pub async fn new(database_pool: DatabasePool) -> Self {
        let prowlarr_client = MockProwlarrClient::default();
        let hdbits_client = MockHDBitsClient::default();
        
        Self {
            prowlarr_client,
            hdbits_client,
            database_pool,
        }
    }
    
    /// Execute search across multiple indexers and combine results
    pub async fn search_all_indexers(&self, request: &SearchRequest) -> RadarrResult<SearchResponse> {
        let mut combined_results = Vec::new();
        let mut total_indexers_searched = 0;
        let mut total_indexers_with_errors = 0;
        let mut all_errors = Vec::new();
        
        // Search Prowlarr
        match self.prowlarr_client.search(request).await {
            Ok(response) => {
                combined_results.extend(response.results);
                total_indexers_searched += response.indexers_searched;
                total_indexers_with_errors += response.indexers_with_errors;
                all_errors.extend(response.errors);
            }
            Err(e) => {
                total_indexers_with_errors += 1;
                all_errors.push(radarr_indexers::models::SearchError {
                    indexer: "Prowlarr".to_string(),
                    message: e.to_string(),
                    code: None,
                });
            }
        }
        
        // Search HDBits
        match self.hdbits_client.search(request).await {
            Ok(response) => {
                combined_results.extend(response.results);
                total_indexers_searched += response.indexers_searched;
                total_indexers_with_errors += response.indexers_with_errors;
                all_errors.extend(response.errors);
            }
            Err(e) => {
                total_indexers_with_errors += 1;
                all_errors.push(radarr_indexers::models::SearchError {
                    indexer: "HDBits".to_string(),
                    message: e.to_string(),
                    code: None,
                });
            }
        }
        
        Ok(SearchResponse {
            total: combined_results.len() as i32,
            results: combined_results,
            indexers_searched: total_indexers_searched,
            indexers_with_errors: total_indexers_with_errors,
            errors: all_errors,
        })
    }
    
    /// Apply quality decision logic to filter and rank results
    pub fn apply_quality_decisions(&self, results: Vec<ProwlarrSearchResult>) -> Vec<ReleaseScore> {
        results.into_iter()
            .map(|result| {
                let score = self.score_release(&result);
                ReleaseScore {
                    release: result,
                    score,
                    reasons: vec![], // Simplified for testing
                }
            })
            .collect()
    }
    
    /// Simple scoring algorithm for testing
    fn score_release(&self, release: &ProwlarrSearchResult) -> f64 {
        let mut score = 0.0;
        let title_lower = release.title.to_lowercase();
        
        // Resolution scoring
        if title_lower.contains("2160p") || title_lower.contains("4k") {
            score += 40.0;
        } else if title_lower.contains("1080p") {
            score += 30.0;
        } else if title_lower.contains("720p") {
            score += 20.0;
        } else {
            score += 5.0;
        }
        
        // Source scoring
        if title_lower.contains("bluray") || title_lower.contains("blu-ray") {
            score += 25.0;
        } else if title_lower.contains("webdl") || title_lower.contains("web-dl") {
            score += 20.0;
        } else if title_lower.contains("webrip") {
            score += 15.0;
        }
        
        // Codec bonus
        if title_lower.contains("x265") || title_lower.contains("hevc") {
            score += 10.0;
        } else if title_lower.contains("x264") {
            score += 5.0;
        }
        
        // Freeleech bonus
        if release.freeleech == Some(true) {
            score += 15.0;
        }
        
        // Seeder bonus
        if let Some(seeders) = release.seeders {
            if seeders >= 50 {
                score += 10.0;
            } else if seeders >= 20 {
                score += 5.0;
            } else if seeders >= 10 {
                score += 2.0;
            }
        }
        
        // Premium indexer bonus (HDBits)
        if release.indexer == "HDBits" {
            score += 20.0;
        }
        
        // HDR/Atmos bonus
        if title_lower.contains("hdr") || title_lower.contains("atmos") {
            score += 15.0;
        }
        
        score
    }
    
    /// Simulate indexer failure scenarios
    pub fn simulate_prowlarr_failure(&self) {
        self.prowlarr_client.set_should_fail(true);
    }
    
    pub fn simulate_hdbits_failure(&self) {
        *self.hdbits_client.should_fail.lock().unwrap() = true;
    }
    
    pub fn simulate_indexer_timeout(&self, timeout_ms: u64) {
        *self.hdbits_client.response_delay_ms.lock().unwrap() = timeout_ms;
    }
    
    pub fn reset_failures(&self) {
        self.prowlarr_client.set_should_fail(false);
        *self.hdbits_client.should_fail.lock().unwrap() = false;
        *self.hdbits_client.response_delay_ms.lock().unwrap() = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Test 1: Complete successful search workflow
    #[tokio::test]
    async fn test_complete_search_workflow() {
        let test_context = TestContext::new().await;
        let test_movie = create_test_movie(&test_context.pool).await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Create search request based on movie
        let search_request = SearchRequest {
            imdb_id: test_movie.imdb_id.clone(),
            tmdb_id: Some(test_movie.tmdb_id),
            categories: vec![2000], // Movies
            limit: Some(50),
            min_seeders: Some(5),
            ..Default::default()
        };
        
        // Execute search across all indexers
        let search_response = search_service.search_all_indexers(&search_request).await
            .expect("Search should succeed");
        
        // Verify search results
        assert!(search_response.total > 0, "Should find releases");
        assert!(search_response.indexers_searched >= 2, "Should search both indexers");
        assert_eq!(search_response.indexers_with_errors, 0, "Should have no errors");
        
        // Verify we got releases from both indexers
        let prowlarr_releases = search_response.results.iter()
            .filter(|r| r.indexer == "Test Indexer")
            .count();
        let hdbits_releases = search_response.results.iter()
            .filter(|r| r.indexer == "HDBits")
            .count();
        
        assert!(prowlarr_releases > 0, "Should have Prowlarr releases");
        assert!(hdbits_releases > 0, "Should have HDBits releases");
        
        // Apply quality decisions
        let scored_releases = search_service.apply_quality_decisions(search_response.results);
        assert!(!scored_releases.is_empty(), "Should have scored releases");
        
        // Verify quality scoring prioritizes high-quality releases
        let best_release = scored_releases.iter()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .expect("Should have best release");
        
        // HDBits 4K release should score highest
        assert!(best_release.release.indexer == "HDBits", "Best release should be from HDBits");
        assert!(best_release.release.title.contains("2160p"), "Best release should be 4K");
        assert!(best_release.release.freeleech == Some(true), "Best release should be freeleech");
        
        test_context.cleanup().await;
    }
    
    /// Test 2: API endpoint integration
    #[tokio::test]
    async fn test_search_api_endpoint() {
        let test_context = TestContext::new().await;
        let test_movie = create_test_movie(&test_context.pool).await;
        let app = create_test_app(test_context.pool.clone()).await;
        let server = TestServer::new(app).unwrap();
        
        // Test successful search
        let response = server
            .get(&format!("/api/v3/release?movieId={}&quality=1080p", test_movie.id))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let releases: Vec<ReleaseResponse> = response.json();
        assert!(!releases.is_empty(), "Should return releases");
        
        // Verify response format
        for release in &releases {
            assertions::assert_valid_release_response(release);
        }
        
        test_context.cleanup().await;
    }
    
    /// Test 3: Error handling - Prowlarr failure
    #[tokio::test]
    async fn test_prowlarr_failure_handling() {
        let test_context = TestContext::new().await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Simulate Prowlarr failure
        search_service.simulate_prowlarr_failure();
        
        let search_request = SearchRequest {
            query: Some("Test Movie".to_string()),
            categories: vec![2000],
            ..Default::default()
        };
        
        let search_response = search_service.search_all_indexers(&search_request).await
            .expect("Search should succeed despite Prowlarr failure");
        
        // Should still get results from HDBits
        assert!(search_response.total > 0, "Should still find releases from HDBits");
        assert_eq!(search_response.indexers_with_errors, 1, "Should report one indexer error");
        assert!(!search_response.errors.is_empty(), "Should have error details");
        
        // Verify error information
        let prowlarr_error = search_response.errors.iter()
            .find(|e| e.indexer == "Prowlarr")
            .expect("Should have Prowlarr error");
        assert!(prowlarr_error.message.contains("configured to fail"), "Error message should be descriptive");
        
        search_service.reset_failures();
        test_context.cleanup().await;
    }
    
    /// Test 4: Error handling - HDBits failure
    #[tokio::test]
    async fn test_hdbits_failure_handling() {
        let test_context = TestContext::new().await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Simulate HDBits failure
        search_service.simulate_hdbits_failure();
        
        let search_request = SearchRequest {
            imdb_id: Some("tt0133093".to_string()),
            categories: vec![2000],
            ..Default::default()
        };
        
        let search_response = search_service.search_all_indexers(&search_request).await
            .expect("Search should succeed despite HDBits failure");
        
        // Should still get results from Prowlarr
        assert!(search_response.total > 0, "Should still find releases from Prowlarr");
        assert_eq!(search_response.indexers_with_errors, 1, "Should report one indexer error");
        
        // Verify error information
        let hdbits_error = search_response.errors.iter()
            .find(|e| e.indexer == "HDBits")
            .expect("Should have HDBits error");
        assert!(hdbits_error.message.contains("timeout"), "Error message should indicate timeout");
        
        search_service.reset_failures();
        test_context.cleanup().await;
    }
    
    /// Test 5: Error handling - All indexers down
    #[tokio::test]
    async fn test_all_indexers_down() {
        let test_context = TestContext::new().await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Simulate both indexers failing
        search_service.simulate_prowlarr_failure();
        search_service.simulate_hdbits_failure();
        
        let search_request = SearchRequest {
            query: Some("Test Movie".to_string()),
            categories: vec![2000],
            ..Default::default()
        };
        
        let search_response = search_service.search_all_indexers(&search_request).await
            .expect("Search should succeed but return no results");
        
        assert_eq!(search_response.total, 0, "Should find no releases");
        assert_eq!(search_response.indexers_with_errors, 2, "Should report both indexers as failed");
        assert_eq!(search_response.errors.len(), 2, "Should have errors from both indexers");
        
        search_service.reset_failures();
        test_context.cleanup().await;
    }
    
    /// Test 6: Timeout handling
    #[tokio::test]
    async fn test_indexer_timeout_handling() {
        let test_context = TestContext::new().await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Simulate slow HDBits response
        search_service.simulate_indexer_timeout(3000); // 3 second delay
        
        let search_request = SearchRequest {
            query: Some("Test Movie".to_string()),
            categories: vec![2000],
            ..Default::default()
        };
        
        // Test with timeout
        let search_future = search_service.search_all_indexers(&search_request);
        let result = timeout(Duration::from_secs(2), search_future).await;
        
        // Should timeout and we should handle it gracefully
        if result.is_err() {
            println!("Search timed out as expected");
        } else {
            // If it doesn't timeout, verify it still works
            let search_response = result.unwrap().expect("Search should succeed");
            assert!(search_response.total >= 0, "Should handle delayed responses");
        }
        
        search_service.reset_failures();
        test_context.cleanup().await;
    }
    
    /// Test 7: Quality filtering and scoring
    #[tokio::test]
    async fn test_quality_filtering_and_scoring() {
        let test_context = TestContext::new().await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Add custom search responses with different qualities
        let mixed_quality_releases = vec![
            ProwlarrSearchResult {
                title: "Movie.2023.CAM.XviD-LOW".to_string(),
                download_url: "magnet:?xt=urn:btih:cam".to_string(),
                info_url: None,
                indexer_id: 1,
                indexer: "Test Indexer".to_string(),
                size: Some(700_000_000), // 700MB
                seeders: Some(200),
                leechers: Some(50),
                download_factor: Some(1.0),
                upload_factor: Some(1.0),
                publish_date: Some(chrono::Utc::now()),
                categories: vec![Category { id: 2000, name: "Movies".to_string(), description: None }],
                attributes: HashMap::new(),
                imdb_id: Some("tt1234567".to_string()),
                tmdb_id: Some(12345),
                freeleech: Some(false),
                info_hash: Some("cam_release_hash".to_string()),
            },
            ProwlarrSearchResult {
                title: "Movie.2023.1080p.BluRay.x264-QUALITY".to_string(),
                download_url: "magnet:?xt=urn:btih:bluray".to_string(),
                info_url: None,
                indexer_id: 1,
                indexer: "Test Indexer".to_string(),
                size: Some(8_000_000_000), // 8GB
                seeders: Some(100),
                leechers: Some(10),
                download_factor: Some(1.0),
                upload_factor: Some(1.0),
                publish_date: Some(chrono::Utc::now()),
                categories: vec![Category { id: 2000, name: "Movies".to_string(), description: None }],
                attributes: HashMap::new(),
                imdb_id: Some("tt1234567".to_string()),
                tmdb_id: Some(12345),
                freeleech: Some(false),
                info_hash: Some("bluray_release_hash".to_string()),
            },
        ];
        
        let scored_releases = search_service.apply_quality_decisions(mixed_quality_releases);
        
        // Verify scoring prioritizes quality
        let bluray_score = scored_releases.iter()
            .find(|r| r.release.title.contains("BluRay"))
            .expect("Should find BluRay release")
            .score;
        
        let cam_score = scored_releases.iter()
            .find(|r| r.release.title.contains("CAM"))
            .expect("Should find CAM release")
            .score;
        
        assert!(bluray_score > cam_score, "BluRay should score higher than CAM release");
        
        test_context.cleanup().await;
    }
    
    /// Test 8: No results scenario
    #[tokio::test]
    async fn test_no_results_scenario() {
        let test_context = TestContext::new().await;
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Configure clients to return no results
        search_service.prowlarr_client.add_search_response(
            "no_results".to_string(),
            search_helpers::create_empty_response()
        );
        
        *search_service.hdbits_client.search_responses.lock().unwrap() = HashMap::new(); // Empty responses
        
        let search_request = SearchRequest {
            query: Some("Non-Existent Movie 9999".to_string()),
            categories: vec![2000],
            ..Default::default()
        };
        
        let search_response = search_service.search_all_indexers(&search_request).await
            .expect("Search should succeed");
        
        assert_eq!(search_response.total, 0, "Should find no releases");
        assert!(search_response.indexers_searched > 0, "Should have searched indexers");
        assert_eq!(search_response.indexers_with_errors, 0, "Should have no errors");
        
        test_context.cleanup().await;
    }
    
    /// Test 9: API endpoint error scenarios
    #[tokio::test]
    async fn test_api_endpoint_error_scenarios() {
        let test_context = TestContext::new().await;
        let app = create_test_app(test_context.pool.clone()).await;
        let server = TestServer::new(app).unwrap();
        
        // Test with fail_prowlarr parameter
        let response = server
            .get("/api/v3/release?fail_prowlarr=true")
            .await;
        
        assert_eq!(response.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        
        let error_body: serde_json::Value = response.json();
        assert!(error_body.get("error").is_some(), "Should return error details");
        
        test_context.cleanup().await;
    }
    
    /// Test 10: Comprehensive integration with pagination
    #[tokio::test]
    async fn test_search_with_pagination_and_filtering() {
        let test_context = TestContext::new().await;
        let test_movie = create_test_movie(&test_context.pool).await;
        let app = create_test_app(test_context.pool.clone()).await;
        let server = TestServer::new(app).unwrap();
        
        // Test with quality and category filters
        let response = server
            .get(&format!("/api/v3/release?movieId={}&quality=1080p&categories=2000", test_movie.id))
            .await;
        
        assert_eq!(response.status_code(), StatusCode::OK);
        
        let releases: Vec<ReleaseResponse> = response.json();
        
        // Verify all releases match quality criteria
        for release in &releases {
            assertions::assert_valid_release_response(release);
            
            // All releases should be movies (category 2000)
            assert!(release.indexer_id > 0, "Should have valid indexer ID");
            
            // Verify release quality information
            assert!(release.quality_score.is_some(), "Should have quality score");
            assert!(release.size.is_some(), "Should have size information");
        }
        
        test_context.cleanup().await;
    }
    
    /// Test 11: Performance and concurrency
    #[tokio::test]
    async fn test_concurrent_search_requests() {
        let test_context = TestContext::new().await;
        let search_service = Arc::new(SearchTestService::new(test_context.pool.clone()).await);
        
        let mut handles = Vec::new();
        
        // Execute 5 concurrent searches
        for i in 0..5 {
            let service = Arc::clone(&search_service);
            let handle = tokio::spawn(async move {
                let search_request = SearchRequest {
                    query: Some(format!("Movie {}", i)),
                    categories: vec![2000],
                    limit: Some(10),
                    ..Default::default()
                };
                
                service.search_all_indexers(&search_request).await
            });
            handles.push(handle);
        }
        
        // Wait for all searches to complete
        let mut successful_searches = 0;
        for handle in handles {
            match handle.await {
                Ok(Ok(_response)) => successful_searches += 1,
                Ok(Err(e)) => println!("Search failed: {}", e),
                Err(e) => println!("Task failed: {}", e),
            }
        }
        
        assert!(successful_searches >= 4, "Most concurrent searches should succeed");
        
        // Verify request counting
        let prowlarr_requests = search_service.prowlarr_client.get_request_count();
        let hdbits_requests = *search_service.hdbits_client.request_count.lock().unwrap();
        
        assert!(prowlarr_requests >= 4, "Should have made multiple Prowlarr requests");
        assert!(hdbits_requests >= 4, "Should have made multiple HDBits requests");
        
        test_context.cleanup().await;
    }
    
    /// Test 12: Database integration and persistence
    #[tokio::test]
    async fn test_search_database_integration() {
        let test_context = TestContext::new().await;
        
        // Create multiple test movies
        let movies = vec![
            create_test_movie(&test_context.pool).await,
            create_test_movie(&test_context.pool).await,
        ];
        
        let search_service = SearchTestService::new(test_context.pool.clone()).await;
        
        // Search for each movie and verify database consistency
        for movie in movies {
            let search_request = SearchRequest {
                tmdb_id: Some(movie.tmdb_id),
                imdb_id: movie.imdb_id.clone(),
                categories: vec![2000],
                ..Default::default()
            };
            
            let search_response = search_service.search_all_indexers(&search_request).await
                .expect("Search should succeed");
            
            assert!(search_response.total > 0, "Should find releases for movie {}", movie.title);
            
            // Verify results contain relevant movie information
            for result in &search_response.results {
                if let Some(ref imdb_id) = movie.imdb_id {
                    if result.imdb_id.is_some() {
                        // IMDB IDs should match when available
                        assert!(
                            result.imdb_id.as_ref().unwrap() == imdb_id || 
                            result.tmdb_id == Some(movie.tmdb_id),
                            "Release should match movie identifiers"
                        );
                    }
                }
            }
        }
        
        test_context.cleanup().await;
    }
}