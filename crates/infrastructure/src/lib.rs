//! Radarr infrastructure module
//!
//! This module provides concrete implementations of repository traits
//! defined in the core domain layer, using PostgreSQL as the data store.

pub mod database;
pub mod download_clients;
pub mod error;
pub mod lists;
pub mod monitoring;
pub mod repositories;
pub mod streaming;
pub mod tmdb;
pub mod trakt;
pub mod watchmode;

// Re-export for easy access
pub use database::*;
pub use download_clients::*;
pub use error::*;
pub use lists::*;
pub use monitoring::*;
pub use repositories::*;
pub use tmdb::*;
