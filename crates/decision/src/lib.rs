//! Radarr decision module
//!
//! This crate handles decision-making logic for release selection,
//! quality profiles, and automated download decisions.

pub mod quality;
pub mod engine;
pub mod custom_formats;

// Re-export main types
pub use quality::{Quality, Source, QualityProfile, QualityItem};
pub use engine::{DecisionEngine, Release, ReleaseScore};
pub use custom_formats::{CustomFormat, CustomFormatEngine, FormatSpecification, ReleaseData};
