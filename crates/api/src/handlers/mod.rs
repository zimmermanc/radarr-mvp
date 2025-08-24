//! API request handlers
//!
//! This module contains the HTTP request handlers for all API endpoints,
//! implementing the business logic for each route.

pub mod movies;
pub mod search;
pub mod downloads;
pub mod health;
pub mod commands;
pub mod calendar;
pub mod queue;
pub mod quality;
pub mod streaming;
pub mod monitoring;

// Re-export handler functions
pub use movies::*;
pub use search::*;
pub use downloads::*;
pub use health::*;
pub use commands::*;
pub use calendar::*;
pub use queue::*;
pub use quality::*;
pub use monitoring::*;