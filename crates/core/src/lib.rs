//! Core domain models and business logic for Radarr
//! 
//! This crate contains the fundamental domain models, value objects,
//! and business rules that define the Radarr application.

pub mod models;
pub mod domain;
pub mod error;

// Re-export core types
pub use models::*;
pub use domain::*;
pub use error::*;