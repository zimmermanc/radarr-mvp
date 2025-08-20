//! Tests for the indexers crate

use crate::models::*;
use crate::prowlarr::*;
use async_trait::async_trait;
use radarr_core::Result;
use std::collections::HashMap;
// tokio_test used for async testing utilities

/// Mock implementation for testing
pub struct MockIndexerClient {
    pub search_responses: Vec<SearchResponse>,
    pub indexers: Vec<ProwlarrIndexer>,
    pub health_status: bool,
    pub test_results: HashMap<i32, bool>,
}

impl MockIndexerClient {
    pub fn new() -> Self {
        Self {
            search_responses: Vec::new(),
            indexers: Vec::new(),
            health_status: true,
            test_results: HashMap::new(),
        }
    }
    
    pub fn with_search_response(mut self, response: SearchResponse) -> Self {
        self.search_responses.push(response);
        self
    }
    
    pub fn with_indexer(mut self, indexer: ProwlarrIndexer) -> Self {
        self.indexers.push(indexer);
        self
    }
    
    pub fn with_test_result(mut self, indexer_id: i32, result: bool) -> Self {
        self.test_results.insert(indexer_id, result);
        self
    }
}

#[async_trait]
impl IndexerClient for MockIndexerClient {
    async fn search(&self, _request: &SearchRequest) -> Result<SearchResponse> {
        if let Some(response) = self.search_responses.first() {
            Ok(response.clone())
        } else {
            Ok(SearchResponse {
                total: 0,
                results: Vec::new(),
                indexers_searched: 0,
                indexers_with_errors: 0,
                errors: Vec::new(),
            })
        }
    }
    
    async fn get_indexers(&self) -> Result<Vec<ProwlarrIndexer>> {
        Ok(self.indexers.clone())
    }
    
    async fn test_indexer(&self, indexer_id: i32) -> Result<bool> {
        Ok(self.test_results.get(&indexer_id).copied().unwrap_or(false))
    }
    
    async fn health_check(&self) -> Result<bool> {
        Ok(self.health_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    // std::time::Duration used for testing rate limiting
    
    fn create_test_indexer() -> ProwlarrIndexer {
        ProwlarrIndexer {
            id: 1,
            name: "Test Indexer".to_string(),
            implementation: "torznab".to_string(),
            base_url: "http://test.example.com".to_string(),
            enable: true,
            status: IndexerStatus {
                status: "healthy".to_string(),
                last_error: None,
                failure_count: 0,
                last_test: Some(Utc::now()),
                disabled_till: None,
            },
            categories: vec![Category {
                id: 2000,
                name: "Movies".to_string(),
                description: Some("Movie torrents".to_string()),
            }],
            capabilities: IndexerCapabilities {
                search_params: vec!["q".to_string(), "imdbid".to_string()],
                tv_search: false,
                movie_search: true,
                music_search: false,
                book_search: false,
                limits: Some(SearchLimits {
                    max: Some(100),
                    default: Some(25),
                }),
            },
            priority: 25,
            supports_rss: true,
            supports_search: true,
            last_sync: Some(Utc::now()),
        }
    }
    
    fn create_test_search_result() -> ProwlarrSearchResult {
        ProwlarrSearchResult {
            title: "Test Movie 2023 1080p BluRay x264-TEST".to_string(),
            download_url: "http://test.example.com/download/123".to_string(),
            info_url: Some("http://test.example.com/details/123".to_string()),
            indexer_id: 1,
            indexer: "Test Indexer".to_string(),
            size: Some(8589934592), // 8GB
            seeders: Some(50),
            leechers: Some(5),
            download_factor: Some(1.0),
            upload_factor: Some(1.0),
            publish_date: Some(Utc::now()),
            categories: vec![Category {
                id: 2000,
                name: "Movies".to_string(),
                description: None,
            }],
            attributes: HashMap::new(),
            imdb_id: Some("tt1234567".to_string()),
            tmdb_id: Some(12345),
            freeleech: Some(false),
        }
    }
    
    #[tokio::test]
    async fn test_mock_client_search() {
        let response = SearchResponse {
            total: 1,
            results: vec![create_test_search_result()],
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: Vec::new(),
        };
        
        let client = MockIndexerClient::new().with_search_response(response);
        
        let request = SearchRequest::for_movie_imdb("tt1234567");
        let result = client.search(&request).await.unwrap();
        
        assert_eq!(result.total, 1);
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].title, "Test Movie 2023 1080p BluRay x264-TEST");
    }
    
    #[tokio::test]
    async fn test_mock_client_get_indexers() {
        let indexer = create_test_indexer();
        let client = MockIndexerClient::new().with_indexer(indexer.clone());
        
        let result = client.get_indexers().await.unwrap();
        
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Test Indexer");
        assert_eq!(result[0].id, 1);
    }
    
    #[tokio::test]
    async fn test_mock_client_test_indexer() {
        let client = MockIndexerClient::new().with_test_result(1, true);
        
        let result = client.test_indexer(1).await.unwrap();
        assert!(result);
        
        let result = client.test_indexer(999).await.unwrap();
        assert!(!result); // Default for unknown indexers
    }
    
    #[tokio::test]
    async fn test_mock_client_health_check() {
        let client = MockIndexerClient::new();
        let result = client.health_check().await.unwrap();
        assert!(result);
    }
    
    #[test]
    fn test_search_request_builders() {
        let imdb_request = SearchRequest::for_movie_imdb("tt0111161")
            .with_min_seeders(10)
            .with_limit(25);
            
        assert_eq!(imdb_request.imdb_id, Some("tt0111161".to_string()));
        assert_eq!(imdb_request.min_seeders, Some(10));
        assert_eq!(imdb_request.limit, Some(25));
        assert_eq!(imdb_request.categories, vec![2000]);
        
        let tmdb_request = SearchRequest::for_movie_tmdb(12345)
            .with_indexers(vec![1, 2, 3]);
            
        assert_eq!(tmdb_request.tmdb_id, Some(12345));
        assert_eq!(tmdb_request.indexer_ids, vec![1, 2, 3]);
        
        let title_request = SearchRequest::for_title("The Shawshank Redemption");
        assert_eq!(title_request.query, Some("The Shawshank Redemption".to_string()));
    }
    
    #[test]
    fn test_prowlarr_config_builder() {
        let config = ProwlarrConfigBuilder::new()
            .base_url("http://prowlarr.local:9696")
            .api_key("test-api-key-12345")
            .timeout(45)
            .rate_limit(90)
            .user_agent("Radarr-Test/1.0")
            .verify_ssl(false)
            .build();
            
        assert_eq!(config.base_url, "http://prowlarr.local:9696");
        assert_eq!(config.api_key, "test-api-key-12345");
        assert_eq!(config.timeout, 45);
        assert_eq!(config.max_requests_per_minute, 90);
        assert_eq!(config.user_agent, "Radarr-Test/1.0");
        assert!(!config.verify_ssl);
    }
    
    #[test]
    fn test_prowlarr_config_defaults() {
        let config = ProwlarrConfig::default();
        
        assert_eq!(config.base_url, "http://localhost:9696");
        assert!(config.api_key.is_empty());
        assert_eq!(config.timeout, 30);
        assert_eq!(config.max_requests_per_minute, 60);
        assert_eq!(config.user_agent, "Radarr-Rust/1.0");
        assert!(config.verify_ssl);
    }
    
    #[tokio::test]
    async fn test_search_response_error_handling() {
        let response_with_errors = SearchResponse {
            total: 0,
            results: Vec::new(),
            indexers_searched: 2,
            indexers_with_errors: 2,
            errors: vec![
                SearchError {
                    indexer: "Broken Indexer".to_string(),
                    message: "Connection timeout".to_string(),
                    code: Some("TIMEOUT".to_string()),
                },
                SearchError {
                    indexer: "Down Indexer".to_string(),
                    message: "HTTP 502 Bad Gateway".to_string(),
                    code: Some("HTTP_502".to_string()),
                },
            ],
        };
        
        let client = MockIndexerClient::new().with_search_response(response_with_errors);
        let request = SearchRequest::for_title("Test Movie");
        let result = client.search(&request).await.unwrap();
        
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.indexers_with_errors, 2);
        assert_eq!(result.total, 0);
    }
    
    /// Integration test for the actual client (requires environment setup)
    /// This test is ignored by default and should be run manually with proper Prowlarr setup
    #[tokio::test]
    #[ignore]
    async fn test_real_prowlarr_integration() {
        // This test requires:
        // 1. PROWLARR_BASE_URL environment variable
        // 2. PROWLARR_API_KEY environment variable
        // 3. Running Prowlarr instance
        
        let client = crate::client_from_env();
        if client.is_err() {
            println!("Skipping integration test - environment not configured");
            return;
        }
        
        let client = client.unwrap();
        
        // Test health check
        let health = client.health_check().await;
        assert!(health.is_ok(), "Health check should succeed");
        
        // Test get indexers
        let indexers = client.get_indexers().await;
        assert!(indexers.is_ok(), "Get indexers should succeed");
        
        // Test search with a common movie
        let request = SearchRequest::for_movie_imdb("tt0111161"); // The Shawshank Redemption
        let search_result = client.search(&request).await;
        assert!(search_result.is_ok(), "Search should succeed");
        
        println!("Integration test passed!");
    }
}