//! HDBits API client implementation

use super::{models::*, parser::parse_quality, HDBitsConfig, RateLimiter, map_hdbits_error};
use crate::{IndexerClient, models::*};
use async_trait::async_trait;
use radarr_core::{RadarrError, models::release::{Release, ReleaseProtocol}};
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use tracing::{debug, info, warn, error};

// Re-export Result type for convenience
type Result<T> = radarr_core::Result<T>;

/// HDBits indexer client
#[derive(Debug)]
pub struct HDBitsClient {
    config: HDBitsConfig,
    client: Client,
    rate_limiter: RateLimiter,
}

impl HDBitsClient {
    /// Create a new HDBits client with configuration
    pub fn new(config: HDBitsConfig) -> Result<Self> {
        config.validate()?;
        
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("RadarrMVP/1.0")
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "HDBits HTTP Client".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;
        
        let rate_limiter = RateLimiter::new(config.rate_limit_per_hour);
        
        Ok(Self {
            config,
            client,
            rate_limiter,
        })
    }
    
    /// Create HDBits client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = HDBitsConfig::from_env()?;
        Self::new(config)
    }
    
    /// Search for movies
    pub async fn search_movies(&self, request: &MovieSearchRequest) -> Result<Vec<Release>> {
        // Build HDBits API request
        let api_request = self.build_search_request(request)?;
        
        // Execute search with rate limiting
        self.rate_limiter.acquire().await?;
        
        info!("Searching HDBits for movies: {:?}", request);
        
        let response = self.client
            .post(&self.config.api_url)
            .json(&api_request)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: format!("Request failed: {}", e),
            })?;
        
        if !response.status().is_success() {
            return Err(RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: format!("HTTP error: {}", response.status()),
            });
        }
        
        let hdbits_response: HDBitsResponse = response
            .json()
            .await
            .map_err(|e| RadarrError::SerializationError(e.to_string()))?;
        
        // Check API response status
        if hdbits_response.status != 0 {
            let error_msg = hdbits_response.message
                .unwrap_or_else(|| format!("Unknown error (status: {})", hdbits_response.status));
            return Err(map_hdbits_error(&error_msg));
        }
        
        let torrents = hdbits_response.data.unwrap_or_default();
        debug!("HDBits returned {} torrents", torrents.len());
        
        // Filter by minimum seeders if specified
        let filtered_torrents: Vec<_> = if let Some(min_seeders) = request.min_seeders {
            torrents.into_iter()
                .filter(|t| t.seeders >= min_seeders)
                .collect()
        } else {
            torrents
        };
        
        debug!("After filtering: {} torrents", filtered_torrents.len());
        
        // Convert to Release structs
        let releases = filtered_torrents
            .into_iter()
            .map(|torrent| self.torrent_to_release(torrent))
            .collect();
        
        Ok(releases)
    }
    
    /// Convert HDBits torrent to Release struct
    fn torrent_to_release(&self, torrent: HDBitsTorrent) -> Release {
        let mut release = Release::new(
            1, // HDBits indexer ID - should be configurable
            torrent.name.clone(),
            torrent.download_url(&self.config.passkey),
            format!("hdbits-{}", torrent.id),
            ReleaseProtocol::Torrent,
        );
        
        // Set optional fields
        release.info_url = Some(torrent.info_url());
        release.size_bytes = Some(torrent.size_bytes());
        release.seeders = Some(torrent.seeders as i32);
        release.leechers = Some(torrent.leechers as i32);
        release.published_date = torrent.parsed_date();
        release.age_hours = torrent.age_hours();
        
        // Parse and set quality information
        release.quality = parse_quality(&torrent.name);
        
        // Add HDBits-specific metadata
        let mut quality_map = serde_json::Map::new();
        if let Some(existing) = release.quality.as_object() {
            quality_map = existing.clone();
        }
        
        // Add HDBits metadata
        quality_map.insert("indexer".to_string(), serde_json::json!("HDBits"));
        quality_map.insert("times_completed".to_string(), serde_json::json!(torrent.times_completed));
        quality_map.insert("internal".to_string(), serde_json::json!(torrent.is_internal()));
        quality_map.insert("freeleech".to_string(), serde_json::json!(torrent.is_freeleech()));
        quality_map.insert("type_category".to_string(), serde_json::json!(torrent.type_category));
        quality_map.insert("type_codec".to_string(), serde_json::json!(torrent.type_codec));
        quality_map.insert("type_medium".to_string(), serde_json::json!(torrent.type_medium));
        quality_map.insert("type_origin".to_string(), serde_json::json!(torrent.type_origin));
        
        if let Some(scene_group) = torrent.scene_group() {
            quality_map.insert("scene_group".to_string(), serde_json::json!(scene_group));
        }
        
        if let Some(imdb_id) = torrent.imdb_id() {
            quality_map.insert("imdb_id".to_string(), serde_json::json!(imdb_id));
        }
        
        if let Some(imdb_info) = &torrent.imdb {
            if let Some(rating) = imdb_info.rating {
                quality_map.insert("imdb_rating".to_string(), serde_json::json!(rating));
            }
            if let Some(votes) = imdb_info.votes {
                quality_map.insert("imdb_votes".to_string(), serde_json::json!(votes));
            }
        }
        
        release.quality = serde_json::Value::Object(quality_map);
        
        release
    }
    
    /// Build HDBits API search request
    pub(crate) fn build_search_request(&self, request: &MovieSearchRequest) -> Result<HDBitsSearchRequest> {
        let mut api_request = HDBitsSearchRequest {
            username: self.config.username.clone(),
            passkey: self.config.passkey.clone(),
            category: Some(categories::ALL_MOVIES.to_vec()),
            codec: None,
            medium: None,
            origin: None,
            search: None,
            limit: request.limit,
            page: None,
            imdb: None,
        };
        
        // Add search term if provided
        if let Some(title) = &request.title {
            let search_term = if let Some(year) = request.year {
                format!("{} {}", title, year)
            } else {
                title.clone()
            };
            api_request.search = Some(search_term);
        }
        
        // Add IMDB ID if provided
        if let Some(imdb_id) = &request.imdb_id {
            if let Ok(imdb_numeric) = imdb_id.parse::<u32>() {
                api_request.imdb = Some(HDBitsImdbSearch { id: imdb_numeric });
            } else {
                warn!("Invalid IMDB ID format: {}", imdb_id);
            }
        }
        
        Ok(api_request)
    }
    
    /// Test HDBits connectivity and authentication
    pub async fn test_connection(&self) -> Result<bool> {
        info!("Testing HDBits connection");
        
        let test_request = MovieSearchRequest::new()
            .with_title("test")
            .with_limit(1);
        
        match self.search_movies(&test_request).await {
            Ok(_) => {
                info!("HDBits connection test successful");
                Ok(true)
            }
            Err(e) => {
                error!("HDBits connection test failed: {}", e);
                Err(e)
            }
        }
    }
}

#[async_trait]
impl IndexerClient for HDBitsClient {
    async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        // Convert generic SearchRequest to HDBits-specific MovieSearchRequest
        let movie_request = self.convert_search_request(request)?;
        
        // Search using HDBits API
        let releases = self.search_movies(&movie_request).await?;
        
        // Convert to Prowlarr-style search results for compatibility
        let results: Vec<ProwlarrSearchResult> = releases
            .into_iter()
            .map(|release| self.release_to_prowlarr_result(release))
            .collect();
        
        Ok(SearchResponse {
            total: results.len() as i32,
            results,
            indexers_searched: 1,
            indexers_with_errors: 0,
            errors: vec![],
        })
    }
    
    async fn get_indexers(&self) -> Result<Vec<ProwlarrIndexer>> {
        // Return HDBits as a single indexer
        let indexer = ProwlarrIndexer {
            id: 1,
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
                Category { id: 1, name: "Movies".to_string(), description: Some("All movie categories".to_string()) },
            ],
            capabilities: IndexerCapabilities {
                search_params: vec!["q".to_string(), "imdbid".to_string()],
                tv_search: false,
                movie_search: true,
                music_search: false,
                book_search: false,
                limits: Some(SearchLimits { max: Some(100), default: Some(50) }),
            },
            priority: 1,
            supports_rss: false,
            supports_search: true,
            last_sync: Some(chrono::Utc::now()),
        };
        
        Ok(vec![indexer])
    }
    
    async fn test_indexer(&self, _indexer_id: i32) -> Result<bool> {
        self.test_connection().await
    }
    
    async fn health_check(&self) -> Result<bool> {
        self.test_connection().await
    }
}

impl HDBitsClient {
    /// Convert generic SearchRequest to HDBits MovieSearchRequest
    pub(crate) fn convert_search_request(&self, request: &SearchRequest) -> Result<MovieSearchRequest> {
        let mut movie_request = MovieSearchRequest::new();
        
        if let Some(query) = &request.query {
            movie_request = movie_request.with_title(query);
        }
        
        if let Some(imdb_id) = &request.imdb_id {
            movie_request = movie_request.with_imdb_id(imdb_id);
        }
        
        if let Some(limit) = request.limit {
            movie_request = movie_request.with_limit(limit as u32);
        }
        
        if let Some(min_seeders) = request.min_seeders {
            movie_request = movie_request.with_min_seeders(min_seeders as u32);
        }
        
        Ok(movie_request)
    }
    
    /// Convert Release back to ProwlarrSearchResult for compatibility
    fn release_to_prowlarr_result(&self, release: Release) -> ProwlarrSearchResult {
        ProwlarrSearchResult {
            title: release.title,
            download_url: release.download_url,
            info_url: release.info_url,
            indexer_id: release.indexer_id,
            indexer: "HDBits".to_string(),
            size: release.size_bytes,
            seeders: release.seeders,
            leechers: release.leechers,
            download_factor: Some(1.0), // HDBits is 1:1 ratio
            upload_factor: Some(1.0),
            publish_date: release.published_date,
            categories: vec![Category {
                id: 2000,
                name: "Movies".to_string(),
                description: Some("Movie releases".to_string()),
            }],
            attributes: std::collections::HashMap::new(),
            imdb_id: release.quality.get("imdb_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            tmdb_id: None, // HDBits doesn't provide TMDB IDs
            freeleech: release.quality.get("freeleech").and_then(|v| v.as_bool()),
        }
    }
}

// Re-export for convenience
pub use super::models::MovieSearchRequest;