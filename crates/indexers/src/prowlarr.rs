//! Prowlarr API client for indexer integration
//!
//! This module provides a production-ready client for interacting with Prowlarr,
//! including search functionality, indexer status checking, and rate limiting.

use crate::models::{
    IndexerStats, ProwlarrIndexer, SearchRequest, SearchResponse,
};
use async_trait::async_trait;
use radarr_core::{RadarrError, Result};
use reqwest::{Client, Response, StatusCode};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};
use url::Url;

/// Configuration for the Prowlarr client
#[derive(Debug, Clone)]
pub struct ProwlarrConfig {
    /// Base URL of the Prowlarr instance (e.g., "http://localhost:9696")
    pub base_url: String,
    
    /// API key for authentication
    pub api_key: String,
    
    /// Request timeout in seconds
    pub timeout: u64,
    
    /// Rate limiting: maximum requests per minute
    pub max_requests_per_minute: u32,
    
    /// User agent string to send with requests
    pub user_agent: String,
    
    /// Whether to verify SSL certificates
    pub verify_ssl: bool,
}

impl Default for ProwlarrConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:9696".to_string(),
            api_key: String::new(),
            timeout: 30,
            max_requests_per_minute: 60,
            user_agent: "Radarr-Rust/1.0".to_string(),
            verify_ssl: true,
        }
    }
}

/// Rate limiter for API requests
#[derive(Debug)]
struct RateLimiter {
    max_requests: u32,
    window_duration: Duration,
    requests: Mutex<Vec<Instant>>,
}

impl RateLimiter {
    fn new(max_requests_per_minute: u32) -> Self {
        Self {
            max_requests: max_requests_per_minute,
            window_duration: Duration::from_secs(60),
            requests: Mutex::new(Vec::new()),
        }
    }
    
    async fn wait_if_needed(&self) -> Result<()> {
        let mut requests = self.requests.lock().await;
        let now = Instant::now();
        
        // Remove old requests outside the window
        requests.retain(|&time| now.duration_since(time) < self.window_duration);
        
        // Check if we're at the limit
        if requests.len() >= self.max_requests as usize {
            let oldest = requests[0];
            let wait_time = self.window_duration - now.duration_since(oldest);
            
            if wait_time > Duration::from_secs(0) {
                debug!("Rate limit reached, waiting {:?}", wait_time);
                drop(requests); // Release the lock before sleeping
                tokio::time::sleep(wait_time).await;
                
                // Re-acquire lock and clean up again
                requests = self.requests.lock().await;
                let now = Instant::now();
                requests.retain(|&time| now.duration_since(time) < self.window_duration);
            }
        }
        
        // Record this request
        requests.push(now);
        Ok(())
    }
}

/// Main Prowlarr API client
#[derive(Debug)]
pub struct ProwlarrClient {
    config: ProwlarrConfig,
    client: Client,
    rate_limiter: RateLimiter,
    base_url: Url,
}

impl ProwlarrClient {
    /// Create a new Prowlarr client with the given configuration
    pub fn new(config: ProwlarrConfig) -> Result<Self> {
        let base_url = Url::parse(&config.base_url)
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Invalid base URL: {}", e),
            })?;
            
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .user_agent(&config.user_agent)
            .danger_accept_invalid_certs(!config.verify_ssl)
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;
            
        let rate_limiter = RateLimiter::new(config.max_requests_per_minute);
        
        Ok(Self {
            config,
            client,
            rate_limiter,
            base_url,
        })
    }
    
    /// Search for releases using the given search request
    pub async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        self.rate_limiter.wait_if_needed().await?;
        
        let mut url = self.base_url.join("/api/v1/search")
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to build search URL: {}", e),
            })?;
        
        // Build query parameters using the URL's query builder to avoid lifetime issues
        {
            let mut query_pairs = url.query_pairs_mut();
            
            if let Some(ref query) = request.query {
                query_pairs.append_pair("query", query);
            }
            
            if let Some(ref imdb_id) = request.imdb_id {
                query_pairs.append_pair("imdbId", imdb_id);
            }
            
            if let Some(tmdb_id) = request.tmdb_id {
                query_pairs.append_pair("tmdbId", &tmdb_id.to_string());
            }
            
            if !request.categories.is_empty() {
                let categories_str = request.categories
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                query_pairs.append_pair("categories", &categories_str);
            }
            
            if !request.indexer_ids.is_empty() {
                let indexers_str = request.indexer_ids
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                query_pairs.append_pair("indexerIds", &indexers_str);
            }
            
            if let Some(limit) = request.limit {
                query_pairs.append_pair("limit", &limit.to_string());
            }
            
            if let Some(offset) = request.offset {
                query_pairs.append_pair("offset", &offset.to_string());
            }
            
            if let Some(min_seeders) = request.min_seeders {
                query_pairs.append_pair("minSeeders", &min_seeders.to_string());
            }
        }
        
        debug!("Searching Prowlarr: {}", url);
        
        let response = self.client
            .get(url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Request failed: {}", e),
            })?;
            
        self.handle_response(response).await
    }
    
    /// Get information about all configured indexers
    pub async fn get_indexers(&self) -> Result<Vec<ProwlarrIndexer>> {
        self.rate_limiter.wait_if_needed().await?;
        
        let url = self.base_url.join("/api/v1/indexer")
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to build indexer URL: {}", e),
            })?;
        
        debug!("Getting indexers from Prowlarr: {}", url);
        
        let response = self.client
            .get(url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Request failed: {}", e),
            })?;
            
        self.handle_response(response).await
    }
    
    /// Get information about a specific indexer
    pub async fn get_indexer(&self, indexer_id: i32) -> Result<ProwlarrIndexer> {
        self.rate_limiter.wait_if_needed().await?;
        
        let url = self.base_url.join(&format!("/api/v1/indexer/{}", indexer_id))
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to build indexer URL: {}", e),
            })?;
        
        debug!("Getting indexer {} from Prowlarr: {}", indexer_id, url);
        
        let response = self.client
            .get(url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Request failed: {}", e),
            })?;
            
        self.handle_response(response).await
    }
    
    /// Test connectivity to a specific indexer
    pub async fn test_indexer(&self, indexer_id: i32) -> Result<bool> {
        self.rate_limiter.wait_if_needed().await?;
        
        let url = self.base_url.join(&format!("/api/v1/indexer/{}/test", indexer_id))
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to build test URL: {}", e),
            })?;
        
        debug!("Testing indexer {} connectivity: {}", indexer_id, url);
        
        let response = self.client
            .post(url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Request failed: {}", e),
            })?;
            
        match response.status() {
            StatusCode::OK => Ok(true),
            StatusCode::BAD_REQUEST | StatusCode::INTERNAL_SERVER_ERROR => {
                let error_text = response.text().await.unwrap_or_default();
                warn!("Indexer {} test failed: {}", indexer_id, error_text);
                Ok(false)
            }
            status => Err(RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Unexpected status code: {}", status),
            }),
        }
    }
    
    /// Get statistics for indexer performance
    pub async fn get_indexer_stats(&self, indexer_id: i32) -> Result<IndexerStats> {
        self.rate_limiter.wait_if_needed().await?;
        
        let url = self.base_url.join(&format!("/api/v1/indexer/{}/stats", indexer_id))
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to build stats URL: {}", e),
            })?;
        
        debug!("Getting stats for indexer {}: {}", indexer_id, url);
        
        let response = self.client
            .get(url)
            .header("X-Api-Key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Request failed: {}", e),
            })?;
            
        self.handle_response(response).await
    }
    
    /// Check if the Prowlarr service is healthy and accessible
    pub async fn health_check(&self) -> Result<bool> {
        let url = self.base_url.join("/api/v1/system/status")
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to build status URL: {}", e),
            })?;
        
        debug!("Checking Prowlarr health: {}", url);
        
        let response = self.client
            .get(url)
            .header("X-Api-Key", &self.config.api_key)
            .timeout(Duration::from_secs(5)) // Short timeout for health checks
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Health check failed: {}", e),
            })?;
            
        Ok(response.status().is_success())
    }
    
    /// Helper method to handle HTTP responses and convert to appropriate types
    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();
        
        if status.is_success() {
            let text = response.text().await.map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to read response: {}", e),
            })?;
            
            serde_json::from_str(&text).map_err(|e| RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("Failed to parse JSON response: {}", e),
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(RadarrError::ExternalServiceError {
                service: "prowlarr".to_string(),
                error: format!("HTTP {}: {}", status, error_text),
            })
        }
    }
}

/// Trait for indexer clients to allow for testing and different implementations
#[async_trait]
pub trait IndexerClient: Send + Sync {
    /// Search for releases
    async fn search(&self, request: &SearchRequest) -> Result<SearchResponse>;
    
    /// Get all configured indexers
    async fn get_indexers(&self) -> Result<Vec<ProwlarrIndexer>>;
    
    /// Test indexer connectivity
    async fn test_indexer(&self, indexer_id: i32) -> Result<bool>;
    
    /// Check service health
    async fn health_check(&self) -> Result<bool>;
}

#[async_trait]
impl IndexerClient for ProwlarrClient {
    async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        self.search(request).await
    }
    
    async fn get_indexers(&self) -> Result<Vec<ProwlarrIndexer>> {
        self.get_indexers().await
    }
    
    async fn test_indexer(&self, indexer_id: i32) -> Result<bool> {
        self.test_indexer(indexer_id).await
    }
    
    async fn health_check(&self) -> Result<bool> {
        self.health_check().await
    }
}

/// Builder for ProwlarrConfig to make configuration easier
pub struct ProwlarrConfigBuilder {
    config: ProwlarrConfig,
}

impl ProwlarrConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ProwlarrConfig::default(),
        }
    }
    
    pub fn base_url<S: Into<String>>(mut self, url: S) -> Self {
        self.config.base_url = url.into();
        self
    }
    
    pub fn api_key<S: Into<String>>(mut self, key: S) -> Self {
        self.config.api_key = key.into();
        self
    }
    
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.config.timeout = seconds;
        self
    }
    
    pub fn rate_limit(mut self, requests_per_minute: u32) -> Self {
        self.config.max_requests_per_minute = requests_per_minute;
        self
    }
    
    pub fn user_agent<S: Into<String>>(mut self, agent: S) -> Self {
        self.config.user_agent = agent.into();
        self
    }
    
    pub fn verify_ssl(mut self, verify: bool) -> Self {
        self.config.verify_ssl = verify;
        self
    }
    
    pub fn build(self) -> ProwlarrConfig {
        self.config
    }
}

impl Default for ProwlarrConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a client from environment variables
pub fn from_env() -> Result<ProwlarrClient> {
    let base_url = std::env::var("PROWLARR_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:9696".to_string());
    let api_key = std::env::var("PROWLARR_API_KEY")
        .map_err(|_| RadarrError::ExternalServiceError {
            service: "prowlarr".to_string(),
            error: "PROWLARR_API_KEY environment variable not set".to_string(),
        })?;
        
    let config = ProwlarrConfigBuilder::new()
        .base_url(base_url)
        .api_key(api_key)
        .build();
        
    ProwlarrClient::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    // tokio_test used for async testing utilities
    
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2); // 2 requests per minute
        
        // First two requests should go through immediately
        limiter.wait_if_needed().await.unwrap();
        limiter.wait_if_needed().await.unwrap();
        
        // Third request should be delayed
        let start = Instant::now();
        limiter.wait_if_needed().await.unwrap();
        let elapsed = start.elapsed();
        
        // Should have waited for rate limit window
        assert!(elapsed >= Duration::from_secs(59), "Expected delay for rate limiting");
    }
    
    #[test]
    fn test_search_request_builders() {
        let request = SearchRequest::for_movie_imdb("tt0111161")
            .with_min_seeders(5)
            .with_limit(50);
            
        assert_eq!(request.imdb_id, Some("tt0111161".to_string()));
        assert_eq!(request.min_seeders, Some(5));
        assert_eq!(request.limit, Some(50));
        assert_eq!(request.categories, vec![2000]);
    }
    
    #[test]
    fn test_config_builder() {
        let config = ProwlarrConfigBuilder::new()
            .base_url("http://test:8080")
            .api_key("test-key")
            .timeout(60)
            .rate_limit(120)
            .user_agent("Test-Agent/1.0")
            .verify_ssl(false)
            .build();
            
        assert_eq!(config.base_url, "http://test:8080");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.timeout, 60);
        assert_eq!(config.max_requests_per_minute, 120);
        assert_eq!(config.user_agent, "Test-Agent/1.0");
        assert!(!config.verify_ssl);
    }
}