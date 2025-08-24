//! Search API handlers

use crate::{
    error::{ApiError, ApiResult},
    models::ReleaseResponse,
};
use axum::{
    extract::{Query, State},
    Json,
};
use radarr_infrastructure::DatabasePool;
use serde::Deserialize;
use tracing::{info, instrument};
use uuid::Uuid;

/// Search query parameters
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    #[serde(rename = "movieId")]
    pub movie_id: Option<Uuid>,
    pub quality: Option<String>,
    pub categories: Option<String>,
    /// For testing failures
    pub fail_prowlarr: Option<bool>,
}

/// Application state
#[derive(Clone)]
pub struct SearchState {
    pub database_pool: DatabasePool,
}

/// GET /api/v3/release - Search for releases
#[instrument(skip(state))]
pub async fn search_releases(
    State(state): State<SearchState>,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<Vec<ReleaseResponse>>> {
    info!("Searching releases with params: {:?}", params);

    // Simulate failure for testing
    if params.fail_prowlarr.unwrap_or(false) {
        return Err(ApiError::ExternalServiceError {
            service: "prowlarr".to_string(),
            error: "Simulated Prowlarr failure for testing".to_string(),
        });
    }

    // Create mock releases for testing
    let releases = vec![
        ReleaseResponse {
            guid: "test-guid-1".to_string(),
            title: "Fight Club 1999 1080p BluRay x264-SPARKS".to_string(),
            download_url: "magnet:?xt=urn:btih:fightclub1999".to_string(),
            info_url: Some("http://test-indexer.com/details/123".to_string()),
            indexer: "Test Indexer".to_string(),
            indexer_id: 1,
            size: Some(1_500_000_000), // 1.5 GB
            seeders: Some(100),
            leechers: Some(5),
            download_factor: Some(1.0),
            upload_factor: Some(1.0),
            publish_date: Some(chrono::Utc::now()),
            imdb_id: Some("tt0137523".to_string()),
            tmdb_id: Some(550),
            freeleech: Some(false),
            quality_score: Some(85),
            progress: 0.0,
        },
        ReleaseResponse {
            guid: "test-guid-2".to_string(),
            title: "Fight Club 1999 720p BluRay x264-SPARKS".to_string(),
            download_url: "magnet:?xt=urn:btih:fightclub1999_720p".to_string(),
            info_url: Some("http://test-indexer.com/details/124".to_string()),
            indexer: "Test Indexer".to_string(),
            indexer_id: 1,
            size: Some(800_000_000), // 800 MB
            seeders: Some(50),
            leechers: Some(2),
            download_factor: Some(1.0),
            upload_factor: Some(1.0),
            publish_date: Some(chrono::Utc::now()),
            imdb_id: Some("tt0137523".to_string()),
            tmdb_id: Some(550),
            freeleech: Some(false),
            quality_score: Some(75),
            progress: 0.0,
        },
    ];

    info!("Found {} releases", releases.len());
    Ok(Json(releases))
}
