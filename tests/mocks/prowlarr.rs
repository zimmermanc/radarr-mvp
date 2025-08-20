//! Mock Prowlarr client for testing

use async_trait::async_trait;
use radarr_core::{Result, RadarrError};
use radarr_indexers::{
    models::{SearchRequest, SearchResponse, ProwlarrIndexer, IndexerStats, ProwlarrSearchResult, Category, IndexerCapabilities, IndexerStatus},
    prowlarr::IndexerClient,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Mock Prowlarr client that simulates API responses
#[derive(Debug, Clone)]
pub struct MockProwlarrClient {
    /// Whether to simulate failures
    pub should_fail: Arc<Mutex<bool>>,
    /// Mock search responses by query
    pub search_responses: Arc<Mutex<HashMap<String, SearchResponse>>>,
    /// Mock indexers
    pub indexers: Arc<Mutex<Vec<ProwlarrIndexer>>>,
    /// Request counter for testing
    pub request_count: Arc<Mutex<u32>>,
}

impl Default for MockProwlarrClient {
    fn default() -> Self {
        let mut search_responses = HashMap::new();
        
        // Default search response for any movie
        let default_response = SearchResponse {
            total: 2,
            results: vec![
                ProwlarrSearchResult {
                    title: "Fight Club 1999 1080p BluRay x264-SPARKS".to_string(),
                    download_url: "magnet:?xt=urn:btih:fightclub1999".to_string(),
                    info_url: Some("http://test-indexer.com/details/123".to_string()),
                    indexer_id: 1,
                    indexer: "Test Indexer".to_string(),
                    size: Some(1_500_000_000), // 1.5 GB
                    seeders: Some(100),
                    leechers: Some(5),
                    download_factor: Some(1.0),
                    upload_factor: Some(1.0),
                    publish_date: Some(chrono::Utc::now()),
                    categories: vec![Category { id: 2000, name: "Movies".to_string(), description: None }],
                    attributes: std::collections::HashMap::new(),
                    imdb_id: Some("tt0137523".to_string()),
                    tmdb_id: Some(550),
                    freeleech: Some(false),
                },
                ProwlarrSearchResult {
                    title: "Fight Club 1999 720p BluRay x264-SPARKS".to_string(),
                    download_url: "magnet:?xt=urn:btih:fightclub1999_720p".to_string(),
                    info_url: Some("http://test-indexer.com/details/124".to_string()),
                    indexer_id: 1,
                    indexer: "Test Indexer".to_string(),
                    size: Some(800_000_000), // 800 MB
                    seeders: Some(50),
                    leechers: Some(2),
                    download_factor: Some(1.0),
                    upload_factor: Some(1.0),
                    publish_date: Some(chrono::Utc::now()),
                    categories: vec![Category { id: 2000, name: "Movies".to_string(), description: None }],
                    attributes: std::collections::HashMap::new(),
                    imdb_id: Some("tt0137523".to_string()),
                    tmdb_id: Some(550),
                    freeleech: Some(false),
                },
            ],
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: vec![],
        };
        
        search_responses.insert("default".to_string(), default_response);
        
        let indexers = vec![
            ProwlarrIndexer {
                id: 1,
                name: "Test Indexer".to_string(),
                implementation: "Torznab".to_string(),
                base_url: "http://test-indexer.com".to_string(),
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
                    Category { id: 2030, name: "Movies/SD".to_string(), description: None },
                    Category { id: 2040, name: "Movies/HD".to_string(), description: None },
                ],
                capabilities: IndexerCapabilities {
                    search_params: vec!["q".to_string(), "imdbid".to_string(), "tmdbid".to_string()],
                    tv_search: false,
                    movie_search: true,
                    music_search: false,
                    book_search: false,
                    limits: None,
                },
                priority: 25,
                supports_rss: true,
                supports_search: true,
                last_sync: Some(chrono::Utc::now()),
            },
        ];
        
        Self {
            should_fail: Arc::new(Mutex::new(false)),
            search_responses: Arc::new(Mutex::new(search_responses)),
            indexers: Arc::new(Mutex::new(indexers)),
            request_count: Arc::new(Mutex::new(0)),
        }
    }
}

impl MockProwlarrClient {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set whether the client should fail requests
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }
    
    /// Add a custom search response for a specific query
    pub fn add_search_response(&self, query: String, response: SearchResponse) {
        self.search_responses.lock().unwrap().insert(query, response);
    }
    
    /// Get the number of requests made
    pub fn get_request_count(&self) -> u32 {
        *self.request_count.lock().unwrap()
    }
    
    /// Reset request count
    pub fn reset_request_count(&self) {
        *self.request_count.lock().unwrap() = 0;
    }
    
    /// Simulate network delay
    async fn simulate_delay(&self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    
    /// Increment request counter
    fn increment_requests(&self) {
        *self.request_count.lock().unwrap() += 1;
    }
}

#[async_trait]
impl IndexerClient for MockProwlarrClient {
    async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: "Mock Prowlarr client configured to fail".to_string(),
            });
        }
        
        // Determine which response to return based on request
        let search_key = if let Some(ref query) = request.query {
            query.clone()
        } else if let Some(ref imdb_id) = request.imdb_id {
            format!("imdb:{}", imdb_id)
        } else if let Some(tmdb_id) = request.tmdb_id {
            format!("tmdb:{}", tmdb_id)
        } else {
            "default".to_string()
        };
        
        let responses = self.search_responses.lock().unwrap();
        let response = responses.get(&search_key)
            .or_else(|| responses.get("default"))
            .cloned()
            .unwrap_or_else(|| SearchResponse {
                total: 0,
                results: vec![],
                indexers_searched: 0,
                indexers_with_errors: 0,
                errors: vec![],
            });
        
        Ok(response)
    }
    
    async fn get_indexers(&self) -> Result<Vec<ProwlarrIndexer>> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: "Mock Prowlarr client configured to fail".to_string(),
            });
        }
        
        Ok(self.indexers.lock().unwrap().clone())
    }
    
    async fn test_indexer(&self, indexer_id: i32) -> Result<bool> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: "Mock Prowlarr client configured to fail".to_string(),
            });
        }
        
        // Simulate that indexer 1 works, others don't
        Ok(indexer_id == 1)
    }
    
    async fn health_check(&self) -> Result<bool> {
        self.simulate_delay().await;
        self.increment_requests();
        
        Ok(!*self.should_fail.lock().unwrap())
    }
}

/// Helper to create search responses for testing
pub mod search_helpers {
    use super::*;
    
    pub fn create_movie_releases(movie_title: &str, count: usize) -> SearchResponse {
        let mut results = Vec::new();
        
        for i in 0..count {
            let quality = if i % 2 == 0 { "1080p" } else { "720p" };
            let size = if quality == "1080p" { 1_500_000_000 } else { 800_000_000 };
            
            results.push(ProwlarrSearchResult {
                title: format!("{} {} BluRay x264-GROUP{}", movie_title, quality, i),
                download_url: format!("magnet:?xt=urn:btih:test{}", i),
                info_url: Some(format!("http://test-indexer.com/details/{}", i)),
                indexer_id: 1,
                indexer: "Test Indexer".to_string(),
                size: Some(size),
                seeders: Some(100 - i as i32 * 10),
                leechers: Some(5 + i as i32),
                download_factor: Some(1.0),
                upload_factor: Some(1.0),
                publish_date: Some(chrono::Utc::now()),
                categories: vec![Category { 
                    id: if quality == "1080p" { 2040 } else { 2030 }, 
                    name: format!("Movies/{}", quality), 
                    description: None 
                }],
                attributes: std::collections::HashMap::new(),
                imdb_id: None,
                tmdb_id: None,
                freeleech: Some(false),
            });
        }
        
        SearchResponse {
            total: count as i32,
            results,
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: vec![],
        }
    }
    
    pub fn create_empty_response() -> SearchResponse {
        SearchResponse {
            total: 0,
            results: vec![],
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: vec![],
        }
    }
    
    pub fn create_error_response() -> Result<SearchResponse> {
        Err(RadarrError::ExternalServiceError {
            service: "prowlarr".to_string(),
            error: "Test error response".to_string(),
        })
    }
}