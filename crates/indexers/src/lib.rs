//! Radarr indexers module
//!
//! This crate provides integration with torrent and NZB indexers
//! through the Prowlarr API. It includes rate limiting, error handling,
//! circuit breaker pattern, and production-ready client implementations.

pub mod models;
pub mod prowlarr;
pub mod service_health;

#[cfg(test)]
pub mod tests;

// Re-export common types
pub use models::*;
pub use prowlarr::{IndexerClient, ProwlarrClient, ProwlarrConfig, ProwlarrConfigBuilder};
pub use service_health::{ServiceHealth, HealthStatus, ServiceMetrics};

#[cfg(test)]
pub use tests::MockIndexerClient;

/// Create a Prowlarr client from environment variables
pub fn client_from_env() -> radarr_core::Result<ProwlarrClient> {
    prowlarr::from_env()
}
