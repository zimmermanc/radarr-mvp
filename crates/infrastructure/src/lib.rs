//! Radarr infrastructure module
//!
//! This module provides concrete implementations of repository traits
//! defined in the core domain layer, using PostgreSQL as the data store.

pub mod repositories;
pub mod database;
pub mod error;

// Re-export for easy access
pub use database::*;
pub use error::*;
pub use repositories::*;
