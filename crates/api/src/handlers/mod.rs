//! API request handlers
//!
//! This module contains the HTTP request handlers for all API endpoints,
//! implementing the business logic for each route.

pub mod advanced_search;
pub mod calendar;
pub mod commands;
pub mod downloads;
pub mod health;
pub mod monitoring;
pub mod movies;
pub mod quality;
pub mod queue;
pub mod search;
pub mod streaming;

// Re-export handler functions
pub use advanced_search::*;
pub use calendar::*;
pub use commands::*;
pub use downloads::*;
pub use health::*;
pub use monitoring::*;
pub use movies::*;
pub use quality::*;
pub use queue::*;
pub use search::*;
