//! Lists and Discovery module
//!
//! This module provides functionality for importing movies from external lists
//! like Trakt, IMDb, and TMDb, with automated discovery and sync capabilities.

pub mod models;
pub mod providers;
pub mod sync_service;

pub use models::*;
pub use providers::*;
pub use sync_service::*;