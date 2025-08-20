//! Search API routes

use crate::handlers::search;
use axum::{
    routing::post,
    Router,
};

/// Create search-related routes
pub fn create_search_routes() -> Router {
    Router::new()
        .route("/indexer/search", post(search::search_movies))
}