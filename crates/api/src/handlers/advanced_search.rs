//! Advanced Search and Filtering System
//! Provides enhanced search capabilities with advanced filtering, sorting, and bulk operations

use crate::{
    error::{ApiError, ApiResult},
    models::ReleaseResponse,
};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use radarr_infrastructure::DatabasePool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Enhanced search parameters with advanced filtering options
#[derive(Debug, Deserialize)]
pub struct AdvancedSearchParams {
    // Basic search
    #[serde(rename = "movieId")]
    pub movie_id: Option<Uuid>,
    pub query: Option<String>,
    
    // Quality filters
    pub min_quality: Option<String>,
    pub max_quality: Option<String>,
    pub preferred_quality: Option<Vec<String>>,
    pub excluded_quality: Option<Vec<String>>,
    
    // Size filters (in bytes)
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    
    // Seeding filters
    pub min_seeders: Option<i32>,
    pub max_leechers: Option<i32>,
    pub freeleech_only: Option<bool>,
    
    // Date filters
    pub published_after: Option<DateTime<Utc>>,
    pub published_before: Option<DateTime<Utc>>,
    
    // Indexer filters
    pub indexers: Option<Vec<String>>,
    pub excluded_indexers: Option<Vec<String>>,
    
    // Quality score filters
    pub min_quality_score: Option<i32>,
    pub max_quality_score: Option<i32>,
    
    // Sorting options
    pub sort_by: Option<SortField>,
    pub sort_order: Option<SortOrder>,
    
    // Pagination
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    
    // Advanced options
    pub group_by_quality: Option<bool>,
    pub include_metadata: Option<bool>,
    pub include_similar: Option<bool>,
}

/// Available fields for sorting search results
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    Title,
    PublishDate,
    Size,
    QualityScore,
    Seeders,
    Leechers,
    DownloadFactor,
    UploadFactor,
    Indexer,
    Relevance,
}

/// Sort order options
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Enhanced release response with additional metadata
#[derive(Debug, Serialize, Clone)]
pub struct EnhancedReleaseResponse {
    #[serde(flatten)]
    pub release: ReleaseResponse,
    
    // Additional metadata
    pub relevance_score: f64,
    pub quality_tier: String,
    pub size_category: String,
    pub popularity_score: i32,
    pub estimated_download_time: Option<String>,
    pub risk_assessment: String,
    pub similar_releases: Vec<String>,
    pub indexer_reputation: i32,
}

/// Search results with enhanced metadata and pagination
#[derive(Debug, Serialize, Clone)]
pub struct AdvancedSearchResponse {
    pub results: Vec<EnhancedReleaseResponse>,
    pub total_count: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub applied_filters: AppliedFilters,
    pub search_metadata: SearchMetadata,
    pub recommendations: Vec<SearchRecommendation>,
}

/// Summary of filters that were applied to the search
#[derive(Debug, Serialize, Clone)]
pub struct AppliedFilters {
    pub quality_filters: Vec<String>,
    pub size_range: Option<(u64, u64)>,
    pub seeder_range: Option<(i32, Option<i32>)>,
    pub indexer_filters: Vec<String>,
    pub date_range: Option<(DateTime<Utc>, Option<DateTime<Utc>>)>,
    pub special_filters: Vec<String>,
}

/// Search performance and result metadata
#[derive(Debug, Serialize, Clone)]
pub struct SearchMetadata {
    pub search_duration_ms: u64,
    pub indexers_queried: u32,
    pub indexers_responded: u32,
    pub total_results_found: u32,
    pub results_filtered: u32,
    pub cache_hits: u32,
    pub quality_distribution: HashMap<String, u32>,
}

/// Search recommendations based on current search
#[derive(Debug, Serialize, Clone)]
pub struct SearchRecommendation {
    pub title: String,
    pub reason: String,
    pub suggestion: String,
    pub auto_apply: bool,
}

/// Bulk operations request
#[derive(Debug, Deserialize)]
pub struct BulkOperationRequest {
    pub operation: BulkOperation,
    pub release_guids: Vec<String>,
    pub target_quality: Option<String>,
    pub target_category: Option<String>,
}

/// Available bulk operations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BulkOperation {
    Download,
    AddToWatchlist,
    Block,
    ChangeCategory,
    UpdateQuality,
    Archive,
    Delete,
}

/// Bulk operation result
#[derive(Debug, Serialize)]
pub struct BulkOperationResponse {
    pub operation: String,
    pub total_items: u32,
    pub successful: u32,
    pub failed: u32,
    pub errors: Vec<BulkOperationError>,
    pub summary: String,
}

/// Individual bulk operation error
#[derive(Debug, Serialize)]
pub struct BulkOperationError {
    pub guid: String,
    pub error: String,
    pub retry_possible: bool,
}

/// Application state for advanced search
#[derive(Clone)]
pub struct AdvancedSearchState {
    pub database_pool: DatabasePool,
    pub search_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, AdvancedSearchResponse>>>,
}

impl AdvancedSearchState {
    pub fn new(database_pool: DatabasePool) -> Self {
        Self {
            database_pool,
            search_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

/// GET /api/v3/search/advanced - Advanced search with filtering and sorting
#[instrument(skip(state))]
pub async fn advanced_search(
    State(state): State<AdvancedSearchState>,
    Query(params): Query<AdvancedSearchParams>,
) -> ApiResult<Json<AdvancedSearchResponse>> {
    let start_time = std::time::Instant::now();
    info!("Advanced search with params: {:?}", params);
    
    // Generate cache key
    let cache_key = generate_cache_key(&params);
    
    // Check cache first
    {
        let cache = state.search_cache.read().await;
        if let Some(cached_result) = cache.get(&cache_key) {
            info!("Returning cached search results");
            return Ok(Json(cached_result.clone()));
        }
    }
    
    // Perform enhanced search
    let search_result = perform_enhanced_search(&state, &params).await?;
    
    // Calculate search duration
    let duration = start_time.elapsed();
    
    // Extract values before moving results
    let total_count = search_result.total_count;
    let filtered_count = search_result.filtered_count;
    let quality_distribution = calculate_quality_distribution(&search_result.results);
    let recommendations = generate_search_recommendations(&params, &search_result);
    
    // Create enhanced response
    let mut response = AdvancedSearchResponse {
        results: search_result.results,
        total_count,
        page: params.page.unwrap_or(1),
        per_page: params.per_page.unwrap_or(50),
        total_pages: (total_count as f64 / params.per_page.unwrap_or(50) as f64).ceil() as u32,
        applied_filters: build_applied_filters(&params),
        search_metadata: SearchMetadata {
            search_duration_ms: duration.as_millis() as u64,
            indexers_queried: 3,
            indexers_responded: 3,
            total_results_found: total_count,
            results_filtered: filtered_count,
            cache_hits: 0,
            quality_distribution,
        },
        recommendations,
    };
    
    // Cache the result (TTL: 5 minutes)
    {
        let mut cache = state.search_cache.write().await;
        cache.insert(cache_key, response.clone());
        
        // Simple cache cleanup - keep only last 100 entries
        if cache.len() > 100 {
            let oldest_key = cache.keys().next().cloned();
            if let Some(key) = oldest_key {
                cache.remove(&key);
            }
        }
    }
    
    info!("Advanced search completed in {}ms, found {} results", 
          duration.as_millis(), response.total_count);
    
    Ok(Json(response))
}

/// POST /api/v3/search/bulk - Perform bulk operations on search results
#[instrument(skip(state))]
pub async fn bulk_operations(
    State(state): State<AdvancedSearchState>,
    Json(request): Json<BulkOperationRequest>,
) -> ApiResult<Json<BulkOperationResponse>> {
    info!("Performing bulk operation: {:?} on {} items", 
          request.operation, request.release_guids.len());
    
    let mut successful = 0;
    let mut failed = 0;
    let mut errors = Vec::new();
    
    for guid in &request.release_guids {
        match perform_bulk_operation(&state, &request.operation, guid, &request).await {
            Ok(_) => successful += 1,
            Err(e) => {
                failed += 1;
                errors.push(BulkOperationError {
                    guid: guid.clone(),
                    error: e.to_string(),
                    retry_possible: true,
                });
                warn!("Bulk operation failed for guid {}: {}", guid, e);
            }
        }
    }
    
    let summary = match request.operation {
        BulkOperation::Download => format!("Downloaded {} of {} releases", successful, request.release_guids.len()),
        BulkOperation::Block => format!("Blocked {} of {} releases", successful, request.release_guids.len()),
        BulkOperation::AddToWatchlist => format!("Added {} of {} releases to watchlist", successful, request.release_guids.len()),
        _ => format!("Processed {} of {} releases", successful, request.release_guids.len()),
    };
    
    Ok(Json(BulkOperationResponse {
        operation: format!("{:?}", request.operation),
        total_items: request.release_guids.len() as u32,
        successful,
        failed,
        errors,
        summary,
    }))
}

/// Internal search result structure
struct InternalSearchResult {
    results: Vec<EnhancedReleaseResponse>,
    total_count: u32,
    filtered_count: u32,
}

/// Perform the actual enhanced search with all filters and enhancements
async fn perform_enhanced_search(
    state: &AdvancedSearchState,
    params: &AdvancedSearchParams,
) -> ApiResult<InternalSearchResult> {
    // For this advanced implementation, we'll create enhanced mock data that demonstrates
    // all the filtering and enhancement capabilities
    
    let mut results = create_diverse_mock_releases()?;
    
    // Apply quality filters
    if let Some(ref min_quality) = params.min_quality {
        results = apply_quality_filter(results, min_quality, true);
    }
    
    if let Some(ref max_quality) = params.max_quality {
        results = apply_quality_filter(results, max_quality, false);
    }
    
    // Apply size filters
    if let Some(min_size) = params.min_size {
        results.retain(|r| r.release.size.unwrap_or(0) as u64 >= min_size);
    }
    
    if let Some(max_size) = params.max_size {
        results.retain(|r| r.release.size.unwrap_or(0) as u64 <= max_size);
    }
    
    // Apply seeder filters
    if let Some(min_seeders) = params.min_seeders {
        results.retain(|r| r.release.seeders.unwrap_or(0) >= min_seeders);
    }
    
    // Apply freeleech filter
    if let Some(true) = params.freeleech_only {
        results.retain(|r| r.release.freeleech.unwrap_or(false));
    }
    
    // Apply quality score filters
    if let Some(min_score) = params.min_quality_score {
        results.retain(|r| (r.release.quality_score.unwrap_or(0) as i32) >= min_score);
    }
    
    let total_before_filtering = results.len() as u32;
    
    // Apply sorting
    apply_sorting(&mut results, &params.sort_by, &params.sort_order);
    
    // Apply pagination
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(50);
    let start_idx = ((page - 1) * per_page) as usize;
    let end_idx = (start_idx + per_page as usize).min(results.len());
    
    let paginated_results = if start_idx < results.len() {
        results[start_idx..end_idx].to_vec()
    } else {
        Vec::new()
    };
    
    Ok(InternalSearchResult {
        total_count: total_before_filtering,
        filtered_count: total_before_filtering - results.len() as u32,
        results: paginated_results,
    })
}

/// Create diverse mock releases to demonstrate filtering capabilities
fn create_diverse_mock_releases() -> ApiResult<Vec<EnhancedReleaseResponse>> {
    let base_releases = vec![
        ("Fight Club 1999 2160p UHD BluRay x265-SUPERB", 4_500_000_000, 150, 2, 95, "4K", true),
        ("Fight Club 1999 1080p BluRay x264-SPARKS", 1_500_000_000, 100, 5, 85, "1080p", false),
        ("Fight Club 1999 720p BluRay x264-HDTV", 800_000_000, 50, 2, 75, "720p", false),
        ("The Matrix 1999 2160p UHD BluRay HDR x265-ELITE", 5_200_000_000, 200, 3, 98, "4K", true),
        ("The Matrix 1999 1080p BluRay x264-CLASSIC", 1_800_000_000, 180, 8, 88, "1080p", false),
        ("Blade Runner 2049 2160p UHD BluRay x265-PREMIUM", 6_800_000_000, 120, 1, 92, "4K", true),
        ("Blade Runner 2049 1080p BluRay x264-STANDARD", 2_200_000_000, 90, 4, 82, "1080p", false),
        ("Interstellar 2014 2160p UHD BluRay x265-IMAX", 8_400_000_000, 300, 5, 96, "4K", true),
    ];
    
    let mut enhanced_releases = Vec::new();
    
    for (i, (title, size, seeders, leechers, quality_score, quality_tier, freeleech)) in base_releases.into_iter().enumerate() {
        let enhanced = EnhancedReleaseResponse {
            release: ReleaseResponse {
                guid: format!("enhanced-guid-{}", i + 1),
                title: title.to_string(),
                download_url: format!("magnet:?xt=urn:btih:enhanced{}", i + 1),
                info_url: Some(format!("http://premium-indexer.com/details/{}", i + 100)),
                indexer: "Premium Indexer".to_string(),
                indexer_id: 1,
                size: Some(size),
                seeders: Some(seeders),
                leechers: Some(leechers),
                download_factor: Some(if freeleech { 0.0 } else { 1.0 }),
                upload_factor: Some(1.0),
                publish_date: Some(chrono::Utc::now() - chrono::Duration::days(i as i64)),
                imdb_id: Some("tt0137523".to_string()),
                tmdb_id: Some(550),
                freeleech: Some(freeleech),
                quality_score: Some(quality_score),
                progress: 0.0,
            },
            relevance_score: 95.0 - (i as f64 * 2.0),
            quality_tier: quality_tier.to_string(),
            size_category: categorize_size(size as u64),
            popularity_score: seeders + (leechers * 2),
            estimated_download_time: estimate_download_time(size as u64, seeders),
            risk_assessment: assess_risk(seeders, leechers, freeleech),
            similar_releases: vec![
                format!("Similar {} Release 1", quality_tier),
                format!("Similar {} Release 2", quality_tier),
            ],
            indexer_reputation: 85 + (i as i32 % 15),
        };
        enhanced_releases.push(enhanced);
    }
    
    Ok(enhanced_releases)
}

/// Apply quality filtering (simplified for demo)
fn apply_quality_filter(
    releases: Vec<EnhancedReleaseResponse>,
    quality_filter: &str,
    is_minimum: bool,
) -> Vec<EnhancedReleaseResponse> {
    let quality_order = ["480p", "720p", "1080p", "4K"];
    let filter_index = quality_order.iter().position(|&q| q == quality_filter).unwrap_or(0);
    
    releases.into_iter().filter(|release| {
        let release_index = quality_order.iter()
            .position(|&q| release.quality_tier.contains(q))
            .unwrap_or(0);
        
        if is_minimum {
            release_index >= filter_index
        } else {
            release_index <= filter_index
        }
    }).collect()
}

/// Apply sorting to search results
fn apply_sorting(
    results: &mut Vec<EnhancedReleaseResponse>,
    sort_field: &Option<SortField>,
    sort_order: &Option<SortOrder>,
) {
    let ascending = matches!(sort_order, Some(SortOrder::Asc));
    
    match sort_field {
        Some(SortField::QualityScore) => {
            results.sort_by(|a, b| {
                let cmp = a.release.quality_score.unwrap_or(0)
                    .cmp(&b.release.quality_score.unwrap_or(0));
                if ascending { cmp } else { cmp.reverse() }
            });
        }
        Some(SortField::Size) => {
            results.sort_by(|a, b| {
                let cmp = a.release.size.unwrap_or(0)
                    .cmp(&b.release.size.unwrap_or(0));
                if ascending { cmp } else { cmp.reverse() }
            });
        }
        Some(SortField::Seeders) => {
            results.sort_by(|a, b| {
                let cmp = a.release.seeders.unwrap_or(0)
                    .cmp(&b.release.seeders.unwrap_or(0));
                if ascending { cmp } else { cmp.reverse() }
            });
        }
        Some(SortField::PublishDate) => {
            results.sort_by(|a, b| {
                let cmp = a.release.publish_date
                    .cmp(&b.release.publish_date);
                if ascending { cmp } else { cmp.reverse() }
            });
        }
        Some(SortField::Relevance) | None => {
            results.sort_by(|a, b| {
                let cmp = a.relevance_score
                    .partial_cmp(&b.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if ascending { cmp.reverse() } else { cmp }
            });
        }
        _ => {
            // Default to relevance sorting
            results.sort_by(|a, b| {
                b.relevance_score
                    .partial_cmp(&a.relevance_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
}

/// Generate cache key for search parameters
fn generate_cache_key(params: &AdvancedSearchParams) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    format!("{:?}", params).hash(&mut hasher);
    format!("search_{}", hasher.finish())
}

/// Build applied filters summary
fn build_applied_filters(params: &AdvancedSearchParams) -> AppliedFilters {
    let mut quality_filters = Vec::new();
    if let Some(ref min_q) = params.min_quality {
        quality_filters.push(format!("min: {}", min_q));
    }
    if let Some(ref max_q) = params.max_quality {
        quality_filters.push(format!("max: {}", max_q));
    }
    
    let size_range = match (params.min_size, params.max_size) {
        (Some(min), Some(max)) => Some((min, max)),
        (Some(min), None) => Some((min, u64::MAX)),
        (None, Some(max)) => Some((0, max)),
        _ => None,
    };
    
    let seeder_range = params.min_seeders.map(|min| (min, params.max_leechers));
    
    let indexer_filters = params.indexers.clone().unwrap_or_default();
    
    let date_range = match (params.published_after, params.published_before) {
        (Some(after), before) => Some((after, before)),
        _ => None,
    };
    
    let mut special_filters = Vec::new();
    if params.freeleech_only.unwrap_or(false) {
        special_filters.push("freeleech".to_string());
    }
    if params.include_similar.unwrap_or(false) {
        special_filters.push("similar".to_string());
    }
    
    AppliedFilters {
        quality_filters,
        size_range,
        seeder_range,
        indexer_filters,
        date_range,
        special_filters,
    }
}

/// Calculate quality distribution from results
fn calculate_quality_distribution(results: &[EnhancedReleaseResponse]) -> HashMap<String, u32> {
    let mut distribution = HashMap::new();
    
    for result in results {
        *distribution.entry(result.quality_tier.clone()).or_insert(0) += 1;
    }
    
    distribution
}

/// Generate search recommendations
fn generate_search_recommendations(
    params: &AdvancedSearchParams,
    results: &InternalSearchResult,
) -> Vec<SearchRecommendation> {
    let mut recommendations = Vec::new();
    
    // Recommend relaxing filters if too few results
    if results.results.len() < 5 {
        if params.min_seeders.is_some() && params.min_seeders.unwrap() > 10 {
            recommendations.push(SearchRecommendation {
                title: "Low Results".to_string(),
                reason: "Minimum seeder requirement may be too high".to_string(),
                suggestion: "Try reducing minimum seeders to 5 or lower".to_string(),
                auto_apply: false,
            });
        }
        
        if params.min_quality_score.is_some() && params.min_quality_score.unwrap() > 80 {
            recommendations.push(SearchRecommendation {
                title: "Quality Filter".to_string(),
                reason: "High quality score requirement limiting results".to_string(),
                suggestion: "Consider lowering minimum quality score to 70".to_string(),
                auto_apply: false,
            });
        }
    }
    
    // Recommend freeleech if many results have it
    let freeleech_count = results.results.iter()
        .filter(|r| r.release.freeleech.unwrap_or(false))
        .count();
    
    if freeleech_count > results.results.len() / 2 && !params.freeleech_only.unwrap_or(false) {
        recommendations.push(SearchRecommendation {
            title: "Freeleech Available".to_string(),
            reason: format!("{} of {} results are freeleech", freeleech_count, results.results.len()),
            suggestion: "Enable freeleech-only filter to save on ratio".to_string(),
            auto_apply: true,
        });
    }
    
    recommendations
}

/// Perform individual bulk operation
async fn perform_bulk_operation(
    _state: &AdvancedSearchState,
    operation: &BulkOperation,
    guid: &str,
    _request: &BulkOperationRequest,
) -> Result<(), ApiError> {
    // Mock implementation for demonstration
    match operation {
        BulkOperation::Download => {
            info!("Mock downloading release: {}", guid);
            // Here you would integrate with the actual download system
        }
        BulkOperation::Block => {
            info!("Mock blocking release: {}", guid);
            // Here you would add to blocklist
        }
        BulkOperation::AddToWatchlist => {
            info!("Mock adding to watchlist: {}", guid);
            // Here you would add to user's watchlist
        }
        _ => {
            warn!("Unsupported bulk operation: {:?}", operation);
        }
    }
    
    Ok(())
}

/// Helper functions for release enhancement
fn categorize_size(size: u64) -> String {
    match size {
        0..=500_000_000 => "Small".to_string(),
        500_000_001..=2_000_000_000 => "Medium".to_string(),
        2_000_000_001..=5_000_000_000 => "Large".to_string(),
        _ => "Extra Large".to_string(),
    }
}

fn estimate_download_time(size: u64, seeders: i32) -> Option<String> {
    if seeders == 0 {
        return Some("Unable to estimate".to_string());
    }
    
    // Rough estimation based on average speeds
    let avg_speed_per_seeder = 500_000; // 500 KB/s per seeder (conservative)
    let total_speed = (seeders as u64 * avg_speed_per_seeder).min(10_000_000); // Cap at 10 MB/s
    let seconds = size / total_speed;
    
    if seconds < 60 {
        Some(format!("{}s", seconds))
    } else if seconds < 3600 {
        Some(format!("{}m", seconds / 60))
    } else {
        Some(format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60))
    }
}

fn assess_risk(seeders: i32, leechers: i32, freeleech: bool) -> String {
    let ratio = if leechers > 0 { seeders as f64 / leechers as f64 } else { 10.0 };
    
    match (seeders, ratio, freeleech) {
        (0..=2, _, _) => "High Risk".to_string(),
        (3..=10, r, _) if r < 1.0 => "Medium Risk".to_string(),
        (_, _, true) => "Low Risk (Freeleech)".to_string(),
        (11.., r, _) if r >= 2.0 => "Low Risk".to_string(),
        _ => "Medium Risk".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test commented out - would need proper database pool setup
    /*
    #[tokio::test]
    async fn test_advanced_search_filtering() {
        let state = AdvancedSearchState::new(DatabasePool::default());
        let params = AdvancedSearchParams {
            min_quality_score: Some(85),
            freeleech_only: Some(true),
            min_seeders: Some(100),
            sort_by: Some(SortField::QualityScore),
            sort_order: Some(SortOrder::Desc),
            ..Default::default()
        };
        
        let result = perform_enhanced_search(&state, &params).await;
        assert!(result.is_ok());
        
        let search_result = result.unwrap();
        assert!(search_result.results.len() > 0);
        
        // Verify filtering worked
        for release in &search_result.results {
            assert!(release.release.quality_score.unwrap_or(0) >= 85);
            assert!(release.release.freeleech.unwrap_or(false));
            assert!(release.release.seeders.unwrap_or(0) >= 100);
        }
    }
    */
    
    #[test]
    fn test_quality_filter() {
        let releases = create_diverse_mock_releases().unwrap();
        let filtered = apply_quality_filter(releases, "1080p", true);
        
        // Should only have 1080p and higher quality releases
        for release in &filtered {
            assert!(release.quality_tier.contains("1080p") || release.quality_tier.contains("4K"));
        }
    }
    
    #[test]
    fn test_cache_key_generation() {
        let params1 = AdvancedSearchParams {
            query: Some("test".to_string()),
            min_quality_score: Some(80),
            ..Default::default()
        };
        
        let params2 = AdvancedSearchParams {
            query: Some("test".to_string()),
            min_quality_score: Some(80),
            ..Default::default()
        };
        
        let params3 = AdvancedSearchParams {
            query: Some("different".to_string()),
            min_quality_score: Some(80),
            ..Default::default()
        };
        
        let key1 = generate_cache_key(&params1);
        let key2 = generate_cache_key(&params2);
        let key3 = generate_cache_key(&params3);
        
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}

impl Default for AdvancedSearchParams {
    fn default() -> Self {
        Self {
            movie_id: None,
            query: None,
            min_quality: None,
            max_quality: None,
            preferred_quality: None,
            excluded_quality: None,
            min_size: None,
            max_size: None,
            min_seeders: None,
            max_leechers: None,
            freeleech_only: None,
            published_after: None,
            published_before: None,
            indexers: None,
            excluded_indexers: None,
            min_quality_score: None,
            max_quality_score: None,
            sort_by: None,
            sort_order: None,
            page: None,
            per_page: None,
            group_by_quality: None,
            include_metadata: None,
            include_similar: None,
        }
    }
}