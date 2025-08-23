use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::handlers::streaming;
use radarr_core::streaming::traits::StreamingAggregator;

/// Create streaming service routes
pub fn streaming_routes(aggregator: Arc<dyn StreamingAggregator>) -> Router {
    Router::new()
        // Trending endpoints
        .route("/trending/:media_type", get(streaming::get_trending))
        
        // Availability endpoint
        .route("/availability/:tmdb_id", get(streaming::get_availability))
        
        // Coming soon endpoint
        .route("/coming-soon/:media_type", get(streaming::get_coming_soon))
        
        // Providers list
        .route("/providers", get(streaming::get_providers))
        
        // Admin endpoints
        .route("/cache/refresh", post(streaming::refresh_cache))
        
        // Trakt authentication
        .route("/trakt/auth/init", post(streaming::init_trakt_auth))
        
        // Add the aggregator as a layer extension
        .layer(axum::Extension(aggregator))
}