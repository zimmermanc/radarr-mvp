//! Health check and monitoring routes

use crate::handlers::health;
use axum::{
    routing::get,
    Router,
};

/// Create health check and monitoring routes
pub fn create_health_routes() -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/health/live", get(health::liveness))
        .route("/health/ready", get(health::readiness))
        .route("/metrics", get(health::metrics))
}