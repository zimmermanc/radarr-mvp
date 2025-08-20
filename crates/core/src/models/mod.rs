//! Core domain models for Radarr
//! 
//! This module contains the fundamental entities and value objects
//! that represent the core concepts in the Radarr domain.

pub mod movie;
pub mod quality;
pub mod release;
pub mod indexer;
pub mod download;

// Re-export all models for easier access
pub use movie::*;
pub use quality::*;
pub use release::*;
pub use indexer::*;
pub use download::*;

