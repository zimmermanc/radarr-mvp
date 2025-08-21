//! Radarr indexers module
//!
//! This crate provides integration with torrent and NZB indexers
//! through the Prowlarr API and direct indexer implementations.
//! Includes rate limiting, error handling, circuit breaker pattern,
//! and production-ready client implementations.

pub mod hdbits;
pub mod models;
pub mod prowlarr;
pub mod service_health;

#[cfg(test)]
pub mod tests;

// Re-export common types
pub use models::*;
pub use prowlarr::{IndexerClient, ProwlarrClient, ProwlarrConfig, ProwlarrConfigBuilder};
pub use hdbits::{HDBitsClient, HDBitsConfig, MovieSearchRequest};
pub use service_health::{ServiceHealth, HealthStatus, ServiceMetrics};

#[cfg(test)]
pub use tests::MockIndexerClient;

/// Create a Prowlarr client from environment variables
pub fn client_from_env() -> radarr_core::Result<ProwlarrClient> {
    prowlarr::from_env()
}

/// Create an HDBits client from environment variables
pub fn hdbits_client_from_env() -> radarr_core::Result<HDBitsClient> {
    HDBitsClient::from_env()
}
