//! HDBits Scene Group Analysis and Reputation Scoring System
//! 
//! This crate provides comprehensive analysis tools for HDBits scene groups,
//! including reputation scoring, quality metrics, and evidence-based assessments.

pub mod hdbits;
pub mod hdbits_browse_analyzer;
pub mod hdbits_comprehensive_analyzer;
pub mod hdbits_session_analyzer;

pub use hdbits::*;

// Re-export key types for external use
pub use hdbits_comprehensive_analyzer::{HDBitsComprehensiveAnalyzer, HDBitsComprehensiveConfig};
pub use hdbits_session_analyzer::{HDBitsSessionAnalyzer, HDBitsSessionConfig};
pub use hdbits_browse_analyzer::{HDBitsBrowseAnalyzer, HDBitsBrowseConfig};

#[cfg(test)]
mod tests {
    #[test]
    fn analysis_crate_compiles() {
        // Basic compilation test
        assert!(true);
    }
}