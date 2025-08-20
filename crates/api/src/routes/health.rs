//! Health check routes

use crate::handlers::health;
use axum::{
    routing::get,
    Router,
};

/// Create health check routes
pub fn create_health_routes() -> Router {
    Router::new()
        .route("/health", get(health::health_check))
}