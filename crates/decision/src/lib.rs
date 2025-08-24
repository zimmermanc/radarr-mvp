//! Radarr decision module
//!
//! This crate handles decision-making logic for release selection,
//! quality profiles, and automated download decisions.

pub mod custom_formats;
pub mod engine;
pub mod quality;

// Re-export main types
pub use custom_formats::{CustomFormat, CustomFormatEngine, FormatSpecification, ReleaseData};
pub use engine::{DecisionEngine, Release, ReleaseScore};
pub use quality::{Quality, QualityItem, QualityProfile, Source};
