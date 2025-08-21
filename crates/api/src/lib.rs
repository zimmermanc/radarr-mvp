//! Radarr REST API implementation
//!
//! This crate provides HTTP REST API endpoints for the Radarr application,
//! implementing the Radarr v3 API specification with proper error handling,
//! pagination, and integration with the domain services.

pub mod routes;
pub mod handlers;
pub mod models;
pub mod extractors;
pub mod middleware;
pub mod error;
pub mod simple_api;
pub mod telemetry;
pub mod metrics;
pub mod tracing;
pub mod security;
pub mod validation;

// Re-export main types
pub use error::{ApiError, ApiResult};
pub use models::*;
pub use simple_api::{SimpleApiState, create_simple_api_router};
pub use telemetry::{TelemetryConfig, ServiceInfo, init_telemetry, shutdown_telemetry};
pub use metrics::{MetricsCollector};
pub use tracing::{DistributedTracing, simple_tracing_middleware, instrument_business_operation};
pub use security::{SecurityConfig, apply_security, configure_cors, security_headers};
pub use validation::{validate_json, ValidationErrorResponse};

// use axum::Router;
// use radarr_infrastructure::DatabasePool;
// use radarr_indexers::IndexerClient;
// use radarr_downloaders::QBittorrentClient;
// use std::sync::Arc;

// Disabled for MVP - use simple_api instead
// /// API service configuration
// #[derive(Debug, Clone)]
// pub struct ApiConfig {
//     /// Database connection pool
//     pub database_pool: DatabasePool,
//     /// Optional CORS origins
//     pub cors_origins: Option<Vec<String>>,
//     /// API rate limit (requests per minute)
//     pub rate_limit: Option<u32>,
// }
// 
// /// Complete API dependencies for creating the router
// pub struct ApiDependencies {
//     pub database_pool: DatabasePool,
//     pub indexer_client: Arc<dyn IndexerClient + Send + Sync>,
//     pub download_client: Arc<QBittorrentClient>,
//     pub cors_origins: Option<Vec<String>>,
// }

// Disabled for MVP - use simple_api instead
// /// Create a new API router with all endpoints configured
// pub fn create_api_router(config: ApiConfig) -> Router {
//     create_router(config)
// }
// 
// /// Create API router with all dependencies properly injected
// pub fn create_api_router_with_deps(deps: ApiDependencies) -> Router {
//     routes::create_router_with_state(deps)
// }
