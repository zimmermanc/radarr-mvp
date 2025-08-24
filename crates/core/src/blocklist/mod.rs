//! Blocklist System for Failed Releases
//!
//! The blocklist system tracks releases that have failed to download or import,
//! preventing unnecessary retry attempts and improving system efficiency.
//!
//! Features:
//! - Automatic blocking on various failure types
//! - Time-based TTL expiration with configurable retry delays
//! - Manual unblock capabilities for administrative overrides
//! - Comprehensive failure taxonomy for targeted handling
//! - Integration with circuit breaker patterns

pub mod models;
pub mod repository;
pub mod service;

#[cfg(test)]
pub mod tests;

// Re-export public types
pub use models::*;
pub use repository::*;
pub use service::*;
