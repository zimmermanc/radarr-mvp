//! Core domain models and business logic for Radarr
//! 
//! This crate contains the fundamental domain models, value objects,
//! and business rules that define the Radarr application.

pub mod models;
pub mod domain;
pub mod error;
pub mod notifications;
pub mod services;
pub mod events;
pub mod retry;
pub mod progress;
pub mod rss;
pub mod circuit_breaker;
pub mod correlation;
pub mod tracing;
pub mod streaming;
pub mod jobs;
pub mod blocklist;

// Re-export core types
pub use models::*;
pub use domain::*;
pub use error::*;
pub use notifications::*;
pub use services::*;
pub use events::*;
// Selective re-exports to avoid naming conflicts
pub use retry::{retry_with_backoff, RetryConfig, RetryPolicy};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerMetrics};
pub use blocklist::*;