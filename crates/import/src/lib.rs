//! Radarr import pipeline
//!
//! This crate provides comprehensive functionality for importing media files
//! into the Radarr media management system. It includes file scanning,
//! analysis, hardlink management, and renaming capabilities.
//!
//! # Key Components
//!
//! - **File Scanner**: Recursively discovers media files in directories
//! - **File Analyzer**: Extracts metadata and quality information from filenames
//! - **Hardlink Manager**: Creates hardlinks or copies files while preserving originals
//! - **Rename Engine**: Generates organized filenames based on configurable templates
//! - **Import Pipeline**: Orchestrates the complete import workflow
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use radarr_import::{ImportPipeline, ImportConfig};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ImportConfig::default();
//!     let pipeline = ImportPipeline::new(config);
//!     
//!     let source_dir = Path::new("/downloads/movies");
//!     let destination_dir = Path::new("/movies");
//!     
//!     let results = pipeline.import_directory(source_dir, destination_dir).await?;
//!     println!("Imported {} files", results.successful_imports);
//!     
//!     Ok(())
//! }
//! ```

pub mod file_analyzer;
pub mod file_scanner;
pub mod hardlink_manager;
pub mod integration;
pub mod pipeline;
pub mod rename_engine;

// Re-export main types for convenience
pub use file_analyzer::{AnalyzedFile, FileAnalyzer, QualityInfo};
pub use file_scanner::{DetectedFile, FileScanner, MediaType, ScanConfig};
pub use hardlink_manager::{HardlinkConfig, HardlinkManager, HardlinkResult, HardlinkStats};
pub use integration::{ImportService, IntegratedImportConfig, IntegratedImportResult};
pub use pipeline::{ImportConfig, ImportPipeline, ImportResult, ImportStats};
pub use rename_engine::{RenameConfig, RenameEngine, RenameResult};

// Re-export core error types
pub use radarr_core::{RadarrError, Result};
