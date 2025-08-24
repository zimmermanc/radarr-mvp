//! Core domain models for Radarr
//!
//! This module contains the fundamental entities and value objects
//! that represent the core concepts in the Radarr domain.

pub mod download;
pub mod indexer;
pub mod movie;
pub mod quality;
pub mod queue;
pub mod release;

// Re-export all models for easier access
pub use download::*;
pub use indexer::*;
pub use movie::*;
pub use quality::*;
pub use queue::*;
pub use release::*;
