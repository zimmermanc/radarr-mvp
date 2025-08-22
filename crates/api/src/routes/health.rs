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
        .route("/health/detailed", get(health::detailed_health_check))
        .route("/health/services/:service", get(health::service_health_check))
}