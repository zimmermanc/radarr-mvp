//! Core domain models and business logic for Radarr
//!
//! This crate contains the fundamental domain models, value objects,
//! and business rules that define the Radarr application.

pub mod blocklist;
pub mod circuit_breaker;
pub mod correlation;
pub mod domain;
pub mod error;
pub mod events;
pub mod jobs;
pub mod models;
pub mod notifications;
pub mod progress;
pub mod retry;
pub mod rss;
pub mod services;
pub mod streaming;
pub mod tracing;

// Re-export core types
pub use domain::*;
pub use error::*;
pub use events::*;
pub use models::*;
pub use notifications::*;
pub use services::*;
// Selective re-exports to avoid naming conflicts
pub use blocklist::*;
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerMetrics};
pub use retry::{retry_with_backoff, RetryConfig, RetryPolicy};
