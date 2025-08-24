//! HDBits HTML scraping client implementation
//!
//! Since HDBits doesn't provide a public API, this implementation
//! scrapes the browse pages using session authentication.

use super::{map_hdbits_error, models::*, parser::parse_quality, HDBitsConfig, RateLimiter};
use crate::{models::*, IndexerClient};
use async_trait::async_trait;
use radarr_core::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    models::release::{Release, ReleaseProtocol},
    RadarrError,
};
use reqwest::{cookie::Jar, Client, ClientBuilder};
use scraper::{Html, Selector};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use url::Url;

// Re-export Result type for convenience
type Result<T> = radarr_core::Result<T>;

/// HDBits indexer client with HTML scraping support
#[derive(Debug)]
pub struct HDBitsClient {
    config: HDBitsConfig,
    client: Client,
    rate_limiter: RateLimiter,
    base_url: String,
    circuit_breaker: CircuitBreaker,
}

impl HDBitsClient {
    /// Create a new HDBits client with configuration
    pub fn new(config: HDBitsConfig) -> Result<Self> {
        config.validate()?;

        // Create cookie jar for session management
        let cookie_jar = Arc::new(Jar::default());

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .cookie_provider(cookie_jar)
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "HDBits HTTP Client".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;

        let rate_limiter = RateLimiter::new(config.rate_limit_per_hour);

        // Configure circuit breaker for HDBits with appropriate settings
        let circuit_breaker_config = CircuitBreakerConfig::new("HDBits")
            .with_failure_threshold(3) // Lower threshold due to scraping sensitivity
            .with_timeout(Duration::from_secs(60)) // Longer timeout for recovery
            .with_request_timeout(Duration::from_secs(config.timeout_seconds))
            .with_success_threshold(1);

        Ok(Self {
            config,
            client,
            rate_limiter,
            base_url: "https://hdbits.org".to_string(),
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        })
    }

    /// Create HDBits client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = HDBitsConfig::from_env()?;
        Self::new(config)
    }

    /// Search for movies using HTML scraping
    pub async fn search_movies(&self, request: &MovieSearchRequest) -> Result<Vec<Release>> {
        let request_clone = request.clone();
        let base_url_clone = self.base_url.clone();
        let client_clone = self.client.clone();
        let config_clone = self.config.clone();

        // Wrap the entire search operation in circuit breaker
        let html_result: Result<String> = self
            .circuit_breaker
            .call(async move {
                // Authenticate if needed (clone for inner closure)
                if config_clone.passkey.is_empty() || config_clone.passkey == "your_passkey_here" {
                    return Err(RadarrError::ConfigurationError {
                        field: "passkey".to_string(),
                        message: "Please set a valid HDBits passkey".to_string(),
                    });
                }

                info!("Searching HDBits for movies: {:?}", request_clone);

                // Build search URL
                let search_url = Self::build_search_url_static(&base_url_clone, &request_clone)?;

                let response = client_clone.get(&search_url).send().await.map_err(|e| {
                    RadarrError::ExternalServiceError {
                        service: "HDBits".to_string(),
                        error: format!("Request failed: {}", e),
                    }
                })?;

                if !response.status().is_success() {
                    return Err(RadarrError::ExternalServiceError {
                        service: "HDBits".to_string(),
                        error: format!("HTTP error: {}", response.status()),
                    });
                }

                let html =
                    response
                        .text()
                        .await
                        .map_err(|e| RadarrError::ExternalServiceError {
                            service: "HDBits".to_string(),
                            error: format!("Failed to read response: {}", e),
                        })?;

                Ok(html)
            })
            .await;

        let html = html_result?;

        // Rate limiting and parsing outside circuit breaker (they don't involve external calls)
        self.rate_limiter.acquire().await?;

        // Parse HTML and extract torrents
        let torrents = self.parse_browse_page(&html)?;
        debug!("HDBits returned {} torrents", torrents.len());

        // Filter by minimum seeders if specified
        let filtered_torrents: Vec<_> = if let Some(min_seeders) = request.min_seeders {
            torrents
                .into_iter()
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
        quality_map.insert(
            "times_completed".to_string(),
            serde_json::json!(torrent.times_completed),
        );
        quality_map.insert(
            "internal".to_string(),
            serde_json::json!(torrent.is_internal()),
        );
        quality_map.insert(
            "freeleech".to_string(),
            serde_json::json!(torrent.is_freeleech()),
        );
        quality_map.insert(
            "type_category".to_string(),
            serde_json::json!(torrent.type_category),
        );
        quality_map.insert(
            "type_codec".to_string(),
            serde_json::json!(torrent.type_codec),
        );
        quality_map.insert(
            "type_medium".to_string(),
            serde_json::json!(torrent.type_medium),
        );
        quality_map.insert(
            "type_origin".to_string(),
            serde_json::json!(torrent.type_origin),
        );

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

    /// Build HDBits browse search URL (static version for use in circuit breaker)
    fn build_search_url_static(base_url: &str, request: &MovieSearchRequest) -> Result<String> {
        let mut url = Url::parse(&format!("{}/browse", base_url)).map_err(|e| {
            RadarrError::ConfigurationError {
                field: "base_url".to_string(),
                message: format!("Invalid base URL: {}", e),
            }
        })?;

        // Add category filter for movies
        url.query_pairs_mut().append_pair("category", "1"); // Movies category

        // Add search term if provided
        if let Some(title) = &request.title {
            let search_term = if let Some(year) = request.year {
                format!("{} {}", title, year)
            } else {
                title.clone()
            };
            url.query_pairs_mut().append_pair("search", &search_term);
        }

        // Add IMDB ID if provided
        if let Some(imdb_id) = &request.imdb_id {
            url.query_pairs_mut().append_pair("imdb", imdb_id);
        }

        // Set items per page
        if let Some(limit) = request.limit {
            url.query_pairs_mut()
                .append_pair("limit", &limit.to_string());
        }

        Ok(url.to_string())
    }

    /// Build HDBits browse search URL
    fn build_search_url(&self, request: &MovieSearchRequest) -> Result<String> {
        Self::build_search_url_static(&self.base_url, request)
    }

    /// Ensure we have a valid session by setting cookies
    async fn ensure_authenticated(&self) -> Result<()> {
        // Set the session cookie
        let cookie_url =
            Url::parse(&self.base_url).map_err(|e| RadarrError::ConfigurationError {
                field: "base_url".to_string(),
                message: format!("Invalid base URL: {}", e),
            })?;

        // The session cookie should be set by the HTTP client cookie jar
        // For now, we'll assume it's valid if provided
        if self.config.passkey.is_empty() || self.config.passkey == "your_passkey_here" {
            return Err(RadarrError::ConfigurationError {
                field: "passkey".to_string(),
                message: "Please set a valid HDBits passkey".to_string(),
            });
        }

        Ok(())
    }

    /// Parse HDBits browse page HTML
    fn parse_browse_page(&self, html: &str) -> Result<Vec<HDBitsTorrent>> {
        let document = Html::parse_document(html);

        // Check if we're logged in
        if html.contains("login") && html.contains("password") {
            return Err(RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: "Authentication failed - redirected to login page".to_string(),
            });
        }

        // Select torrent rows from the browse table
        let row_selector = Selector::parse("table.browse tr").unwrap();
        let mut torrents = Vec::new();

        for row in document.select(&row_selector) {
            if let Ok(torrent) = self.parse_torrent_row(row) {
                torrents.push(torrent);
            }
        }

        if torrents.is_empty() {
            // Try alternative selector patterns
            let alt_selector = Selector::parse("tr[class*='browse']").unwrap();
            for row in document.select(&alt_selector) {
                if let Ok(torrent) = self.parse_torrent_row(row) {
                    torrents.push(torrent);
                }
            }
        }

        // Filter to movies only, then deduplicate by InfoHash
        let movie_torrents = self.filter_movies_only(torrents);
        Ok(self.deduplicate_by_infohash(movie_torrents))
    }

    /// Filter torrents to include only movies (exclude TV shows, documentaries, etc.)
    pub fn filter_movies_only(&self, torrents: Vec<HDBitsTorrent>) -> Vec<HDBitsTorrent> {
        torrents
            .into_iter()
            .filter(|torrent| {
                // Check if torrent is in a movie category
                matches!(
                    torrent.type_category,
                    super::models::categories::MOVIE
                        | super::models::categories::MOVIE_BD
                        | super::models::categories::MOVIE_UHD
                )
            })
            .collect()
    }

    /// Deduplicate torrents by InfoHash, keeping the best quality/seeded version
    pub fn deduplicate_by_infohash(&self, torrents: Vec<HDBitsTorrent>) -> Vec<HDBitsTorrent> {
        use std::collections::HashMap;

        let mut hash_map: HashMap<String, Vec<HDBitsTorrent>> = HashMap::new();

        // Group torrents by InfoHash
        for torrent in torrents {
            hash_map
                .entry(torrent.hash.clone())
                .or_default()
                .push(torrent);
        }

        // For each InfoHash group, select the best torrent
        let mut deduplicated = Vec::new();
        for (hash, mut group) in hash_map {
            if group.is_empty() {
                continue;
            }

            if group.len() == 1 {
                deduplicated.push(group.into_iter().next().unwrap());
                continue;
            }

            // Sort by quality score (prefer freeleech, higher seeders, better quality)
            group.sort_by(|a, b| {
                let score_a = self.calculate_dedup_score(a);
                let score_b = self.calculate_dedup_score(b);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            debug!(
                "Deduplicated {} torrents with hash {} - selected: {}",
                group.len(),
                hash,
                group[0].name
            );

            deduplicated.push(group.into_iter().next().unwrap());
        }

        deduplicated
    }

    /// Calculate score for deduplication (higher is better)
    fn calculate_dedup_score(&self, torrent: &HDBitsTorrent) -> f64 {
        let mut score = 0.0;

        // Strong preference for freeleech
        if torrent.is_freeleech() {
            score += 50.0;
        }

        // Seeders count (capped to avoid skewing too much)
        score += (torrent.seeders as f64).min(20.0);

        // Prefer internal releases
        if torrent.is_internal() {
            score += 10.0;
        }

        // Quality indicators in title
        let title_lower = torrent.name.to_lowercase();
        if title_lower.contains("remux") {
            score += 15.0;
        } else if title_lower.contains("2160p") || title_lower.contains("4k") {
            score += 12.0;
        } else if title_lower.contains("1080p") {
            score += 8.0;
        } else if title_lower.contains("720p") {
            score += 5.0;
        }

        // Codec preferences
        if title_lower.contains("x265") || title_lower.contains("hevc") {
            score += 3.0;
        }

        // HDR bonus
        if title_lower.contains("hdr") || title_lower.contains("dolby") {
            score += 5.0;
        }

        score
    }

    /// Parse individual torrent row from HTML
    fn parse_torrent_row(&self, row: scraper::ElementRef) -> Result<HDBitsTorrent> {
        let name_selector = Selector::parse("a[href*='details.php']").unwrap();
        let size_selector = Selector::parse("td:nth-child(6)").unwrap(); // Adjust as needed
        let seeders_selector = Selector::parse("td:nth-child(8)").unwrap();
        let leechers_selector = Selector::parse("td:nth-child(9)").unwrap();

        // Extract torrent name and ID
        let name_element = row.select(&name_selector).next().ok_or_else(|| {
            RadarrError::SerializationError("Could not find torrent name".to_string())
        })?;

        let name = name_element.inner_html();
        let href = name_element.value().attr("href").ok_or_else(|| {
            RadarrError::SerializationError("Could not find torrent link".to_string())
        })?;

        // Extract ID from href like "details.php?id=123456"
        let id = href
            .split("id=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| {
                RadarrError::SerializationError("Could not parse torrent ID".to_string())
            })?;

        // Extract size (convert from text like "1.2 GB")
        let size_text = row
            .select(&size_selector)
            .next()
            .map(|e| e.inner_html())
            .unwrap_or_default();
        let size = self.parse_size(&size_text).unwrap_or(0);

        // Extract seeders and leechers
        let seeders = row
            .select(&seeders_selector)
            .next()
            .and_then(|e| e.inner_html().parse::<u32>().ok())
            .unwrap_or(0);

        let leechers = row
            .select(&leechers_selector)
            .next()
            .and_then(|e| e.inner_html().parse::<u32>().ok())
            .unwrap_or(0);

        Ok(HDBitsTorrent {
            id,
            hash: format!("{:x}", md5::compute(&name)), // Generate a fake hash
            name,
            times_completed: 0, // Not available from HTML
            seeders,
            leechers,
            size,
            added: chrono::Utc::now().to_rfc3339(),
            utadded: Some(chrono::Utc::now().timestamp() as u64),
            descr: None,
            comments: None,
            numfiles: None,
            filename: None,
            type_category: 1, // Movies
            type_codec: 1,    // Default
            type_medium: 1,   // Default
            type_origin: 0,   // Default
            type_exclusive: None,
            freeleech: "no".to_string(), // Default to not freeleech
            torrent_status: None,
            bookmarked: None,
            wishlisted: None,
            tags: None,
            username: None,
            owner: None,
            imdb: None,
            tvdb: None,
        })
    }

    /// Parse size string like "1.2 GB" to bytes
    fn parse_size(&self, size_str: &str) -> Option<u64> {
        let parts: Vec<&str> = size_str.trim().split_whitespace().collect();
        if parts.len() != 2 {
            return None;
        }

        let size_val: f64 = parts[0].parse().ok()?;
        let unit = parts[1].to_uppercase();

        let multiplier = match unit.as_str() {
            "B" | "BYTES" => 1,
            "KB" => 1024,
            "MB" => 1024 * 1024,
            "GB" => 1024 * 1024 * 1024,
            "TB" => 1024u64 * 1024 * 1024 * 1024,
            _ => return None,
        };

        Some((size_val * multiplier as f64) as u64)
    }

    /// Test HDBits connectivity and authentication
    pub async fn test_connection(&self) -> Result<bool> {
        info!("Testing HDBits connection");

        // Test by trying to access the browse page
        let browse_url = format!("{}/browse", self.base_url);

        self.rate_limiter.acquire().await?;

        let response = self.client.get(&browse_url).send().await.map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: format!("Connection test failed: {}", e),
            }
        })?;

        if !response.status().is_success() {
            return Err(RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: format!("HTTP error: {}", response.status()),
            });
        }

        let html = response
            .text()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: format!("Failed to read response: {}", e),
            })?;

        // Check if we're logged in
        if html.contains("login") && html.contains("password") {
            return Err(RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: "Authentication failed - session cookie invalid".to_string(),
            });
        }

        info!("HDBits connection test successful");
        Ok(true)
    }

    /// Get circuit breaker metrics for monitoring
    pub async fn get_circuit_breaker_metrics(&self) -> radarr_core::CircuitBreakerMetrics {
        self.circuit_breaker.get_metrics().await
    }

    /// Check if HDBits service is healthy
    pub async fn is_healthy(&self) -> bool {
        self.circuit_breaker.is_healthy().await
    }
}

#[async_trait]
impl IndexerClient for HDBitsClient {
    async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        // Check if we should skip due to recent failures
        if self.rate_limiter.should_skip_due_to_failures().await {
            return Err(RadarrError::ExternalServiceError {
                service: "HDBits".to_string(),
                error: "Skipping request due to recent failures".to_string(),
            });
        }

        // Convert generic SearchRequest to HDBits-specific MovieSearchRequest
        let movie_request = self.convert_search_request(request)?;

        // Search using HDBits API
        let releases = match self.search_movies(&movie_request).await {
            Ok(releases) => {
                self.rate_limiter.record_success().await;
                releases
            }
            Err(e) => {
                self.rate_limiter.record_failure().await?;
                return Err(e);
            }
        };

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
            categories: vec![Category {
                id: 1,
                name: "Movies".to_string(),
                description: Some("All movie categories".to_string()),
            }],
            capabilities: IndexerCapabilities {
                search_params: vec!["q".to_string(), "imdbid".to_string()],
                tv_search: false,
                movie_search: true,
                music_search: false,
                book_search: false,
                limits: Some(SearchLimits {
                    max: Some(100),
                    default: Some(50),
                }),
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
    pub(crate) fn convert_search_request(
        &self,
        request: &SearchRequest,
    ) -> Result<MovieSearchRequest> {
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
        // Extract info hash before moving the release
        let info_hash = Self::extract_info_hash_from_release(&release);

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
            imdb_id: release
                .quality
                .get("imdb_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tmdb_id: None, // HDBits doesn't provide TMDB IDs
            freeleech: release.quality.get("freeleech").and_then(|v| v.as_bool()),
            info_hash,
        }
    }

    /// Extract info hash from release data
    fn extract_info_hash_from_release(release: &Release) -> Option<String> {
        // First try to extract from download URL if it's a magnet link
        if let Some(hash) = Self::extract_hash_from_magnet(&release.download_url) {
            return Some(hash);
        }

        // Try to extract from quality metadata
        if let Some(hash) = release
            .quality
            .get("info_hash")
            .and_then(|h| h.as_str())
            .filter(|h| h.len() == 40 || h.len() == 32)
        {
            return Some(hash.to_uppercase());
        }

        if let Some(hash) = release
            .quality
            .get("hash")
            .and_then(|h| h.as_str())
            .filter(|h| h.len() == 40 || h.len() == 32)
        {
            return Some(hash.to_uppercase());
        }

        // Try to extract from other common metadata fields
        if let Some(hash) = release
            .quality
            .get("torrent_hash")
            .and_then(|h| h.as_str())
            .filter(|h| h.len() == 40 || h.len() == 32)
        {
            return Some(hash.to_uppercase());
        }

        None
    }

    /// Extract info hash from magnet URL
    fn extract_hash_from_magnet(url: &str) -> Option<String> {
        if url.starts_with("magnet:") {
            // Parse magnet URL for info hash (xt parameter)
            if let Some(xt_start) = url.find("xt=") {
                let xt_part = &url[xt_start + 3..]; // Skip "xt="

                // Look for btih hash format
                if xt_part.starts_with("urn:btih:") {
                    let hash_part = &xt_part[9..]; // Skip "urn:btih:"
                    return hash_part
                        .split('&')
                        .next()
                        .filter(|h| h.len() == 40 || h.len() == 32)
                        .map(|hash| hash.to_uppercase());
                }

                // Direct hash format
                if let Some(hash) = xt_part.split('&').next() {
                    if hash.len() == 40 || hash.len() == 32 {
                        return Some(hash.to_uppercase());
                    }
                }
            }
        }
        None
    }
}

// Re-export for convenience
pub use super::models::MovieSearchRequest;
