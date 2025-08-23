//! Multi-indexer service for aggregating search results across multiple sources
//!
//! This service provides:
//! - Parallel search across multiple indexers (HDBits + Prowlarr)
//! - Result aggregation and deduplication
//! - Timeout racing between sources
//! - Per-indexer health monitoring and circuit breaking
//! - Intelligent fallback strategies

use crate::{
    IndexerClient, SearchRequest, SearchResponse, ProwlarrSearchResult,
    hdbits::{HDBitsClient, HDBitsConfig},
    prowlarr::{ProwlarrClient, ProwlarrConfig},
};
use async_trait::async_trait;
use radarr_core::{RadarrError, Result, correlation::{CorrelationContext, set_current_context}};
use std::{sync::Arc, time::{Duration, Instant}, collections::HashMap};
use tokio::time::timeout;
use tracing::{info, warn, debug, error, instrument};
use uuid::Uuid;

/// Configuration for multi-indexer service
#[derive(Debug, Clone)]
pub struct MultiIndexerConfig {
    /// Maximum time to wait for all indexers to respond
    pub search_timeout_seconds: u64,
    /// Whether to return partial results if some indexers fail
    pub allow_partial_results: bool,
    /// Minimum number of indexers that must succeed
    pub min_successful_indexers: u32,
    /// Enable result deduplication across indexers
    pub enable_deduplication: bool,
}

impl Default for MultiIndexerConfig {
    fn default() -> Self {
        Self {
            search_timeout_seconds: 30,
            allow_partial_results: true,
            min_successful_indexers: 1,
            enable_deduplication: true,
        }
    }
}

/// Individual indexer result with metadata
#[derive(Debug, Clone)]
pub struct IndexerSearchResult {
    pub indexer_name: String,
    pub indexer_id: Option<i32>,
    pub results: Vec<ProwlarrSearchResult>,
    pub search_time_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Multi-indexer service that aggregates results from multiple sources
#[derive(Debug)]
pub struct MultiIndexerService {
    config: MultiIndexerConfig,
    hdbits_client: Option<Arc<HDBitsClient>>,
    prowlarr_client: Option<Arc<ProwlarrClient>>,
}

impl MultiIndexerService {
    /// Create a new multi-indexer service
    pub fn new(config: MultiIndexerConfig) -> Self {
        Self {
            config,
            hdbits_client: None,
            prowlarr_client: None,
        }
    }

    /// Add HDBits indexer
    pub fn with_hdbits(mut self, client: Arc<HDBitsClient>) -> Self {
        self.hdbits_client = Some(client);
        self
    }

    /// Add Prowlarr indexer
    pub fn with_prowlarr(mut self, client: Arc<ProwlarrClient>) -> Self {
        self.prowlarr_client = Some(client);
        self
    }

    /// Search across all configured indexers in parallel
    #[instrument(skip(self), fields(indexers_count = self.get_indexer_count()))]
    pub async fn search_all(&self, request: &SearchRequest) -> Result<SearchResponse> {
        let start_time = Instant::now();
        let correlation_id = Uuid::new_v4();
        
        // Set correlation context for this search operation
        let context = CorrelationContext::new("multi_indexer.search_all")
            .with_session(correlation_id.to_string());
        set_current_context(context);

        info!("Starting multi-indexer search with {} indexers", self.get_indexer_count());

        // Launch parallel searches
        let mut search_tasks = Vec::new();

        // HDBits search
        if let Some(ref hdbits) = self.hdbits_client {
            let hdbits_clone = hdbits.clone();
            let request_clone = request.clone();
            let task = tokio::spawn(async move {
                Self::search_indexer("HDBits", hdbits_clone.as_ref(), &request_clone).await
            });
            search_tasks.push(("HDBits", task));
        }

        // Prowlarr search
        if let Some(ref prowlarr) = self.prowlarr_client {
            let prowlarr_clone = prowlarr.clone();
            let request_clone = request.clone();
            let task = tokio::spawn(async move {
                Self::search_indexer("Prowlarr", prowlarr_clone.as_ref(), &request_clone).await
            });
            search_tasks.push(("Prowlarr", task));
        }

        if search_tasks.is_empty() {
            return Err(RadarrError::ConfigurationError {
                field: "indexers".to_string(),
                message: "No indexers configured".to_string(),
            });
        }

        // Wait for all searches to complete or timeout
        let search_timeout = Duration::from_secs(self.config.search_timeout_seconds);
        let mut results = Vec::new();

        for (indexer_name, task) in search_tasks {
            match timeout(search_timeout, task).await {
                Ok(result) => {
                    match result {
                        Ok(search_result) => {
                            debug!("Search completed for {}: {} results in {}ms", 
                                   indexer_name, search_result.results.len(), search_result.search_time_ms);
                            results.push(search_result);
                        }
                        Err(e) => {
                            warn!("Search failed for {}: {}", indexer_name, e);
                            results.push(IndexerSearchResult {
                                indexer_name: indexer_name.to_string(),
                                indexer_id: None,
                                results: vec![],
                                search_time_ms: 0,
                                success: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                Err(_) => {
                    warn!("Search timed out for {} after {}s", indexer_name, search_timeout.as_secs());
                    results.push(IndexerSearchResult {
                        indexer_name: indexer_name.to_string(),
                        indexer_id: None,
                        results: vec![],
                        search_time_ms: search_timeout.as_millis() as u64,
                        success: false,
                        error: Some("Timeout".to_string()),
                    });
                }
            }
        }

        // Check if we have enough successful indexers
        let successful_count = results.iter().filter(|r| r.success).count() as u32;
        if successful_count < self.config.min_successful_indexers {
            return Err(RadarrError::ExternalServiceError {
                service: "multi_indexer".to_string(),
                error: format!("Only {} of {} required indexers succeeded", 
                             successful_count, self.config.min_successful_indexers),
            });
        }

        // Aggregate results
        let aggregated_response = self.aggregate_results(results, start_time.elapsed()).await?;

        info!("Multi-indexer search completed: {} total results from {} indexers in {}ms",
              aggregated_response.total, successful_count, start_time.elapsed().as_millis());

        Ok(aggregated_response)
    }

    /// Search a single indexer and return results with metadata
    async fn search_indexer(
        indexer_name: &str,
        client: &dyn IndexerClient,
        request: &SearchRequest,
    ) -> Result<IndexerSearchResult> {
        let start_time = Instant::now();
        
        match client.search(request).await {
            Ok(response) => {
                Ok(IndexerSearchResult {
                    indexer_name: indexer_name.to_string(),
                    indexer_id: None,
                    results: response.results,
                    search_time_ms: start_time.elapsed().as_millis() as u64,
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                Ok(IndexerSearchResult {
                    indexer_name: indexer_name.to_string(),
                    indexer_id: None,
                    results: vec![],
                    search_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Aggregate results from multiple indexers
    async fn aggregate_results(
        &self,
        indexer_results: Vec<IndexerSearchResult>,
        total_duration: Duration,
    ) -> Result<SearchResponse> {
        let mut all_results = Vec::new();
        let mut indexers_searched = 0;
        let mut indexers_with_errors = 0;
        let mut errors = Vec::new();

        // Collect all results
        for indexer_result in indexer_results {
            indexers_searched += 1;
            
            if indexer_result.success {
                // Add indexer name to each result for provenance
                for mut result in indexer_result.results {
                    result.indexer = indexer_result.indexer_name.clone();
                    all_results.push(result);
                }
            } else {
                indexers_with_errors += 1;
                if let Some(error_msg) = indexer_result.error {
                    errors.push(crate::models::SearchError {
                        indexer: indexer_result.indexer_name,
                        message: error_msg,
                        code: None,
                    });
                }
            }
        }

        // Deduplicate results if enabled
        if self.config.enable_deduplication {
            all_results = self.deduplicate_results(all_results).await;
            debug!("Deduplication reduced {} results to {}", 
                   all_results.len() + indexers_with_errors as usize, all_results.len());
        }

        // Sort results by quality score (descending)
        all_results.sort_by(|a, b| {
            let score_a = self.calculate_result_score(a);
            let score_b = self.calculate_result_score(b);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(SearchResponse {
            total: all_results.len() as i32,
            results: all_results,
            indexers_searched,
            indexers_with_errors,
            errors,
        })
    }

    /// Deduplicate results across indexers using InfoHash and title similarity
    async fn deduplicate_results(&self, results: Vec<ProwlarrSearchResult>) -> Vec<ProwlarrSearchResult> {
        let mut deduped = Vec::new();
        let mut seen_hashes = HashMap::new();
        let mut seen_titles = HashMap::new();

        for result in results {
            let mut should_add = true;

            // Check InfoHash deduplication (exact match) - using download_url as hash for now
            // TODO: Extract actual InfoHash from magnet URLs
            let hash_key = Self::extract_hash_from_url(&result.download_url);
            if let Some(hash) = hash_key {
                if let Some(&existing_idx) = seen_hashes.get(&hash) {
                    // Compare with existing result
                    if self.calculate_result_score(&result) > self.calculate_result_score(&deduped[existing_idx]) {
                        // Replace with better result
                        deduped[existing_idx] = result.clone();
                    }
                    should_add = false;
                } else {
                    seen_hashes.insert(hash, deduped.len());
                }
            }

            // Check title similarity (fuzzy matching)
            if should_add {
                let title_key = Self::normalize_title(&result.title);
                if let Some(&existing_idx) = seen_titles.get(&title_key) {
                    // Compare file sizes - if similar, likely duplicate
                    if let (Some(size1), Some(size2)) = (result.size, deduped[existing_idx].size) {
                        let size_diff_percent = ((size1 - size2).abs() as f64 / size1 as f64) * 100.0;
                        if size_diff_percent < 10.0 { // Within 10% size difference
                            if self.calculate_result_score(&result) > self.calculate_result_score(&deduped[existing_idx]) {
                                deduped[existing_idx] = result.clone();
                            }
                            should_add = false;
                        }
                    }
                } else {
                    seen_titles.insert(title_key, deduped.len());
                }
            }

            if should_add {
                deduped.push(result);
            }
        }

        debug!("Deduplication: {} → {} results", results.len(), deduped.len());
        deduped
    }

    /// Extract InfoHash from magnet URL or return None
    fn extract_hash_from_url(url: &str) -> Option<String> {
        if url.starts_with("magnet:?xt=urn:btih:") {
            url.split("urn:btih:")
                .nth(1)
                .and_then(|hash_part| hash_part.split('&').next())
                .map(|hash| hash.to_uppercase())
        } else {
            None
        }
    }

    /// Normalize title for similarity comparison
    fn normalize_title(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .filter(|word| !["the", "a", "an", "and", "or", "of", "in", "on", "at", "to", "for", "with"].contains(word))
            .collect::<Vec<&str>>()
            .join(" ")
    }

    /// Calculate overall quality score for a result
    fn calculate_result_score(&self, result: &ProwlarrSearchResult) -> f64 {
        let mut score = 0.0;

        // Freeleech bonus
        if result.freeleech == Some(true) {
            score += 50.0;
        }

        // Seeders (capped)
        if let Some(seeders) = result.seeders {
            score += (seeders as f64).min(30.0);
        }

        // Size penalty for very large files (>25GB)
        if let Some(size) = result.size {
            if size > 26843545600 { // 25GB
                score -= 10.0;
            }
        }

        // Quality indicators
        let title_lower = result.title.to_lowercase();
        if title_lower.contains("remux") {
            score += 20.0;
        } else if title_lower.contains("2160p") || title_lower.contains("4k") {
            score += 15.0;
        } else if title_lower.contains("1080p") {
            score += 10.0;
        } else if title_lower.contains("720p") {
            score += 5.0;
        }

        // Codec preferences
        if title_lower.contains("x265") || title_lower.contains("hevc") {
            score += 5.0;
        }

        // HDR bonus
        if title_lower.contains("hdr") || title_lower.contains("dolby") {
            score += 8.0;
        }

        // Indexer preference (HDBits preferred for quality)
        if result.indexer == "HDBits" {
            score += 3.0;
        }

        score
    }

    /// Get count of configured indexers
    fn get_indexer_count(&self) -> usize {
        let mut count = 0;
        if self.hdbits_client.is_some() { count += 1; }
        if self.prowlarr_client.is_some() { count += 1; }
        count
    }
}

#[async_trait]
impl IndexerClient for MultiIndexerService {
    async fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        self.search_all(request).await
    }

    async fn get_indexers(&self) -> Result<Vec<crate::models::ProwlarrIndexer>> {
        let mut indexers = Vec::new();

        // Get HDBits as indexer
        if let Some(ref hdbits) = self.hdbits_client {
            if let Ok(hdbits_indexers) = hdbits.get_indexers().await {
                indexers.extend(hdbits_indexers);
            }
        }

        // Get Prowlarr indexers
        if let Some(ref prowlarr) = self.prowlarr_client {
            if let Ok(prowlarr_indexers) = prowlarr.get_indexers().await {
                indexers.extend(prowlarr_indexers);
            }
        }

        Ok(indexers)
    }

    async fn test_indexer(&self, indexer_id: i32) -> Result<bool> {
        // Try to test on each configured indexer
        if let Some(ref hdbits) = self.hdbits_client {
            if let Ok(result) = hdbits.test_indexer(indexer_id).await {
                return Ok(result);
            }
        }

        if let Some(ref prowlarr) = self.prowlarr_client {
            if let Ok(result) = prowlarr.test_indexer(indexer_id).await {
                return Ok(result);
            }
        }

        Err(RadarrError::NotFoundError {
            entity: "indexer".to_string(),
            id: indexer_id.to_string(),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        let mut any_healthy = false;

        if let Some(ref hdbits) = self.hdbits_client {
            if hdbits.health_check().await.unwrap_or(false) {
                any_healthy = true;
            }
        }

        if let Some(ref prowlarr) = self.prowlarr_client {
            if prowlarr.health_check().await.unwrap_or(false) {
                any_healthy = true;
            }
        }

        Ok(any_healthy)
    }

    async fn get_service_health(&self) -> crate::service_health::HealthStatus {
        // Return combined health status (simplified for now)
        if self.health_check().await.unwrap_or(false) {
            crate::service_health::HealthStatus::Healthy
        } else {
            crate::service_health::HealthStatus::Down
        }
    }

    async fn get_service_metrics(&self) -> crate::service_health::ServiceMetrics {
        // Return default metrics (could aggregate from all indexers)
        crate::service_health::ServiceMetrics::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    #[test]
    fn test_title_normalization() {
        assert_eq!(
            MultiIndexerService::normalize_title("The Matrix (1999) - Extended Cut"),
            "matrix 1999 extended cut"
        );
        
        assert_eq!(
            MultiIndexerService::normalize_title("A Beautiful Mind"),
            "beautiful mind"
        );
    }

    #[test]
    fn test_result_scoring() {
        let service = MultiIndexerService::new(MultiIndexerConfig::default());
        
        let result = ProwlarrSearchResult {
            title: "Movie.2024.2160p.UHD.BluRay.x265.HDR.Atmos-GROUP".to_string(),
            indexer: "HDBits".to_string(),
            indexer_id: 1,
            download_url: "magnet:test".to_string(),
            info_url: Some("https://test".to_string()),
            size: Some(15000000000), // 15GB
            seeders: Some(25),
            leechers: Some(2),
            freeleech: Some(true),
            download_factor: Some(0.0),
            upload_factor: Some(1.0),
            publish_date: Some(chrono::Utc::now()),
            categories: vec![],
            attributes: std::collections::HashMap::new(),
            imdb_id: None,
            tmdb_id: None,
            info_hash: Some("ABCD1234".to_string()),
        };
        
        let score = service.calculate_result_score(&result);
        
        // Should get high score for: freeleech (50) + seeders (25) + 4K (15) + HDBits (3) + HDR (8) = 101+
        assert!(score > 100.0, "Score was {}, expected > 100", score);
    }
}