use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use radarr_core::streaming::{
    traits::StreamingAggregator, AvailabilityResponse, ComingSoonResponse, MediaType,
    StreamingProvider, TimeWindow, TrendingResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::{error::ApiError, models::ApiResponse};

/// Query parameters for trending endpoint
#[derive(Debug, Deserialize)]
pub struct TrendingQuery {
    #[serde(default = "default_time_window")]
    pub window: String,
}

fn default_time_window() -> String {
    "day".to_string()
}

/// Query parameters for availability endpoint
#[derive(Debug, Deserialize)]
pub struct AvailabilityQuery {
    #[serde(default = "default_region")]
    pub region: String,
}

fn default_region() -> String {
    "US".to_string()
}

/// Query parameters for coming soon endpoint
#[derive(Debug, Deserialize)]
pub struct ComingSoonQuery {
    #[serde(default = "default_region")]
    pub region: String,
}

/// Get trending movies or TV shows
pub async fn get_trending(
    Path((media_type_str,)): Path<(String,)>,
    Query(params): Query<TrendingQuery>,
    Extension(aggregator): Extension<Arc<dyn StreamingAggregator>>,
) -> Result<Json<ApiResponse<TrendingResponse>>, ApiError> {
    info!(
        "Getting trending {} for window: {}",
        media_type_str, params.window
    );

    // Parse media type
    let media_type = match media_type_str.as_str() {
        "movie" | "movies" => MediaType::Movie,
        "tv" | "shows" => MediaType::Tv,
        _ => {
            return Err(ApiError::ValidationError {
                field: "media_type".to_string(),
                message: "Invalid media type. Use 'movie' or 'tv'".to_string(),
            });
        }
    };

    // Parse time window
    let window = match params.window.as_str() {
        "day" | "daily" => TimeWindow::Day,
        "week" | "weekly" => TimeWindow::Week,
        _ => {
            return Err(ApiError::ValidationError {
                field: "window".to_string(),
                message: "Invalid time window. Use 'day' or 'week'".to_string(),
            });
        }
    };

    // Get trending from aggregator
    match aggregator.get_trending(media_type, window).await {
        Ok(response) => {
            info!(
                "Successfully fetched {} trending entries",
                response.entries.len()
            );
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to get trending: {}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Get streaming availability for a specific title
pub async fn get_availability(
    Path((tmdb_id,)): Path<(i32,)>,
    Query(params): Query<AvailabilityQuery>,
    Extension(aggregator): Extension<Arc<dyn StreamingAggregator>>,
) -> Result<Json<ApiResponse<AvailabilityResponse>>, ApiError> {
    info!(
        "Getting availability for TMDB {} in region: {}",
        tmdb_id, params.region
    );

    // For now, assume movie type (could be enhanced with a query param)
    let media_type = MediaType::Movie;

    // Get availability from aggregator
    match aggregator
        .get_availability(tmdb_id, media_type, &params.region)
        .await
    {
        Ok(mut response) => {
            // Add attribution for JustWatch (TMDB providers)
            if !response.availability.is_empty() {
                // This would be added to the response headers in production
                info!("Streaming data provided by JustWatch");
            }

            info!(
                "Successfully fetched availability with {} services",
                response
                    .availability
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>()
            );
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to get availability: {}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Get coming soon releases
pub async fn get_coming_soon(
    Path((media_type_str,)): Path<(String,)>,
    Query(params): Query<ComingSoonQuery>,
    Extension(aggregator): Extension<Arc<dyn StreamingAggregator>>,
) -> Result<Json<ApiResponse<ComingSoonResponse>>, ApiError> {
    info!(
        "Getting coming soon {} for region: {}",
        media_type_str, params.region
    );

    // Parse media type
    let media_type = match media_type_str.as_str() {
        "movie" | "movies" => MediaType::Movie,
        "tv" | "shows" => MediaType::Tv,
        _ => {
            return Err(ApiError::ValidationError {
                field: "media_type".to_string(),
                message: "Invalid media type. Use 'movie' or 'tv'".to_string(),
            });
        }
    };

    // Get coming soon from aggregator
    match aggregator.get_coming_soon(media_type, &params.region).await {
        Ok(response) => {
            info!(
                "Successfully fetched {} coming soon entries",
                response.entries.len()
            );
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            error!("Failed to get coming soon: {}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Get list of supported streaming providers
pub async fn get_providers() -> Json<ApiResponse<Vec<StreamingProviderInfo>>> {
    // This would ideally be dynamic based on configuration
    let providers = vec![
        StreamingProviderInfo {
            id: "netflix".to_string(),
            name: "Netflix".to_string(),
            logo_url: Some(
                "https://image.tmdb.org/t/p/original/t2yyOv40HZeVlLjYsCsPHnWLk4W.jpg".to_string(),
            ),
            service_types: vec!["subscription".to_string()],
            supported_regions: vec!["US".to_string(), "CA".to_string(), "UK".to_string()],
        },
        StreamingProviderInfo {
            id: "prime".to_string(),
            name: "Amazon Prime Video".to_string(),
            logo_url: Some(
                "https://image.tmdb.org/t/p/original/emthp39XA2YScoYL1p0sdbAH2WA.jpg".to_string(),
            ),
            service_types: vec![
                "subscription".to_string(),
                "rent".to_string(),
                "buy".to_string(),
            ],
            supported_regions: vec!["US".to_string(), "CA".to_string(), "UK".to_string()],
        },
        StreamingProviderInfo {
            id: "disney".to_string(),
            name: "Disney+".to_string(),
            logo_url: Some(
                "https://image.tmdb.org/t/p/original/7rwgEs15tFwyR9NPQ5vpzxTj19Q.jpg".to_string(),
            ),
            service_types: vec!["subscription".to_string()],
            supported_regions: vec!["US".to_string(), "CA".to_string(), "UK".to_string()],
        },
        StreamingProviderInfo {
            id: "hulu".to_string(),
            name: "Hulu".to_string(),
            logo_url: Some(
                "https://image.tmdb.org/t/p/original/zxrVdFjIjLqkfnwyghnfywTn3Lh.jpg".to_string(),
            ),
            service_types: vec!["subscription".to_string()],
            supported_regions: vec!["US".to_string()],
        },
        StreamingProviderInfo {
            id: "hbo".to_string(),
            name: "Max".to_string(),
            logo_url: Some(
                "https://image.tmdb.org/t/p/original/6Q3ZYUNA9Hsgj6iWnVsw2gR5V6z.jpg".to_string(),
            ),
            service_types: vec!["subscription".to_string()],
            supported_regions: vec!["US".to_string()],
        },
        StreamingProviderInfo {
            id: "apple".to_string(),
            name: "Apple TV+".to_string(),
            logo_url: Some(
                "https://image.tmdb.org/t/p/original/6uhKBfmtzFqOcLousHwZuzcrScK.jpg".to_string(),
            ),
            service_types: vec!["subscription".to_string(), "buy".to_string()],
            supported_regions: vec!["US".to_string(), "CA".to_string(), "UK".to_string()],
        },
    ];

    Json(ApiResponse::success(providers))
}

/// Refresh streaming cache (admin endpoint)
pub async fn refresh_cache(
    Extension(aggregator): Extension<Arc<dyn StreamingAggregator>>,
) -> Result<Json<ApiResponse<RefreshResult>>, ApiError> {
    info!("Refreshing streaming cache");

    match aggregator.refresh_cache().await {
        Ok(()) => {
            info!("Successfully refreshed streaming cache");
            Ok(Json(ApiResponse::success(RefreshResult {
                success: true,
                message: "Cache refreshed successfully".to_string(),
            })))
        }
        Err(e) => {
            error!("Failed to refresh cache: {}", e);
            Err(ApiError::from(e))
        }
    }
}

/// Initialize Trakt authentication (returns device code)
pub async fn init_trakt_auth(
    Extension(aggregator): Extension<Arc<dyn StreamingAggregator>>,
) -> Result<Json<ApiResponse<TraktAuthInit>>, ApiError> {
    use radarr_core::streaming::traits::TraktAdapter;
    use radarr_infrastructure::trakt::TraktClient;

    // This is a simplified version - in production, you'd get the client from the aggregator
    Err(ApiError::NotImplemented {
        message: "Trakt authentication initialization not yet implemented in API".to_string(),
    })
}

// Response models
#[derive(Debug, Serialize)]
pub struct StreamingProviderInfo {
    pub id: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub service_types: Vec<String>,
    pub supported_regions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RefreshResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct TraktAuthInit {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: i32,
    pub interval: i32,
}
