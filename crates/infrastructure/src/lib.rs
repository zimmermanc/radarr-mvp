//! Radarr infrastructure module
//!
//! This module provides concrete implementations of repository traits
//! defined in the core domain layer, using PostgreSQL as the data store.

pub mod repositories;
pub mod database;
pub mod error;
pub mod download_clients;
pub mod tmdb;
pub mod trakt;
pub mod watchmode;
pub mod streaming;
pub mod lists;
pub mod monitoring;

// Re-export for easy access
pub use database::*;
pub use error::*;
pub use repositories::*;
pub use download_clients::*;
pub use tmdb::*;
pub use lists::*;
pub use monitoring::*;
