//! Basic route configuration

pub mod monitoring;
pub mod streaming;

use crate::handlers::health::health_check;
use axum::{routing::get, Router};

/// Create basic routes for testing
pub fn create_routes() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(health_check))
}

// Re-export route creation functions
pub use monitoring::create_monitoring_routes;
