//! Decision engine for automated release selection
//!
//! This module implements the core decision-making logic that evaluates
//! multiple releases and selects the best one based on quality profiles
//! and various release characteristics.

use crate::quality::{Quality, Source, QualityProfile};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Release information for decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    /// Release title
    pub title: String,
    /// Download URL
    pub download_url: String,
    /// File size in bytes
    pub size: Option<u64>,
    /// Number of seeders
    pub seeders: Option<u32>,
    /// Number of leechers  
    pub leechers: Option<u32>,
    /// Release group name
    pub release_group: Option<String>,
    /// Age of release in hours
    pub age_hours: Option<u32>,
    /// Whether it's freeleech
    pub freeleech: Option<bool>,
    /// Quality detected from title
    pub quality: Quality,
    /// Source detected from title
    pub source: Source,
}

impl Release {
    /// Parse quality and source from release title
    pub fn from_title(title: String, download_url: String) -> Self {
        let quality = Quality::from_resolution(&title);
        let source = Source::from_release_name(&title);
        
        Self {
            title,
            download_url,
            size: None,
            seeders: None,
            leechers: None,
            release_group: None,
            age_hours: None,
            freeleech: None,
            quality,
            source,
        }
    }
    
    /// Builder methods for setting optional fields
    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }
    
    pub fn with_seeders(mut self, seeders: u32) -> Self {
        self.seeders = Some(seeders);
        self
    }
    
    pub fn with_leechers(mut self, leechers: u32) -> Self {
        self.leechers = Some(leechers);
        self
    }
    
    pub fn with_release_group(mut self, group: String) -> Self {
        self.release_group = Some(group);
        self
    }
    
    pub fn with_age_hours(mut self, hours: u32) -> Self {
        self.age_hours = Some(hours);
        self
    }
    
    pub fn with_freeleech(mut self, freeleech: bool) -> Self {
        self.freeleech = Some(freeleech);
        self
    }
}

/// Release evaluation score
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseScore {
    /// Total score (higher is better)
    pub total: i32,
    /// Quality component score
    pub quality_score: i32,
    /// Seeders component score
    pub seeders_score: i32,
    /// Size component score
    pub size_score: i32,
    /// Age component score
    pub age_score: i32,
    /// Special bonuses (freeleech, etc.)
    pub bonus_score: i32,
}

impl PartialOrd for ReleaseScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReleaseScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.total.cmp(&other.total)
    }
}

/// Main decision engine
#[derive(Debug)]
pub struct DecisionEngine {
    /// Quality profile to use for decisions
    pub quality_profile: QualityProfile,
    /// Maximum file size in GB (None = no limit)
    pub max_size_gb: Option<u32>,
    /// Minimum number of seeders required
    pub min_seeders: Option<u32>,
    /// Maximum age in hours (None = no limit)
    pub max_age_hours: Option<u32>,
}

impl DecisionEngine {
    /// Create a new decision engine with the given quality profile
    pub fn new(quality_profile: QualityProfile) -> Self {
        Self {
            quality_profile,
            max_size_gb: Some(50), // Default 50GB limit
            min_seeders: Some(1),  // At least 1 seeder
            max_age_hours: Some(24 * 7), // Max 1 week old
        }
    }
    
    /// Create decision engine with no constraints
    pub fn permissive(quality_profile: QualityProfile) -> Self {
        Self {
            quality_profile,
            max_size_gb: None,
            min_seeders: None,
            max_age_hours: None,
        }
    }
    
    /// Evaluate a release and return its score
    pub fn evaluate_release(&self, release: &Release) -> Option<ReleaseScore> {
        // Check hard constraints first
        if !self.meets_constraints(release) {
            return None;
        }
        
        // Calculate component scores
        let quality_score = self.quality_profile.calculate_quality_score(&release.quality, &release.source);
        if quality_score < 0 {
            return None; // Quality not allowed
        }
        
        let seeders_score = self.calculate_seeders_score(release);
        let size_score = self.calculate_size_score(release);
        let age_score = self.calculate_age_score(release);
        let bonus_score = self.calculate_bonus_score(release);
        
        let total = quality_score + seeders_score + size_score + age_score + bonus_score;
        
        Some(ReleaseScore {
            total,
            quality_score,
            seeders_score,
            size_score,
            age_score,
            bonus_score,
        })
    }
    
    /// Select the best release from a list of candidates
    pub fn select_best_release(&self, releases: Vec<Release>) -> Option<Release> {
        let mut scored_releases: Vec<(Release, ReleaseScore)> = releases
            .into_iter()
            .filter_map(|release| {
                self.evaluate_release(&release)
                    .map(|score| (release, score))
            })
            .collect();
        
        // Sort by score (highest first)
        scored_releases.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Return the best release
        scored_releases.into_iter().next().map(|(release, _)| release)
    }
    
    /// Check if release meets hard constraints
    fn meets_constraints(&self, release: &Release) -> bool {
        // Size constraint
        if let (Some(max_gb), Some(size)) = (self.max_size_gb, release.size) {
            let size_gb = size / (1024 * 1024 * 1024);
            if size_gb > max_gb as u64 {
                return false;
            }
        }
        
        // Seeders constraint
        if let (Some(min_seeders), Some(seeders)) = (self.min_seeders, release.seeders) {
            if seeders < min_seeders {
                return false;
            }
        }
        
        // Age constraint
        if let (Some(max_hours), Some(age)) = (self.max_age_hours, release.age_hours) {
            if age > max_hours {
                return false;
            }
        }
        
        true
    }
    
    /// Calculate seeders score (more seeders = better)
    fn calculate_seeders_score(&self, release: &Release) -> i32 {
        match release.seeders {
            Some(seeders) => {
                match seeders {
                    0 => -10,           // No seeders = bad
                    1..=5 => 0,         // Few seeders = neutral
                    6..=20 => 5,        // Good seeders = bonus
                    21..=50 => 10,      // Great seeders = bigger bonus
                    _ => 15,            // Excellent seeders = maximum bonus
                }
            }
            None => 0, // Unknown seeders = neutral
        }
    }
    
    /// Calculate size score (prefer reasonable sizes)
    fn calculate_size_score(&self, release: &Release) -> i32 {
        match release.size {
            Some(size) => {
                let size_gb = size / (1024 * 1024 * 1024);
                match size_gb {
                    0..=5 => 5,         // Good size for most movies
                    6..=15 => 10,       // Ideal size range
                    16..=30 => 5,       // Large but reasonable
                    31..=50 => 0,       // Very large = neutral
                    _ => -5,            // Excessively large = penalty
                }
            }
            None => 0, // Unknown size = neutral
        }
    }
    
    /// Calculate age score (newer is generally better)
    fn calculate_age_score(&self, release: &Release) -> i32 {
        match release.age_hours {
            Some(hours) => {
                match hours {
                    0..=24 => 10,       // Very fresh = bonus
                    25..=72 => 5,       // Recent = small bonus
                    73..=168 => 0,      // This week = neutral
                    169..=720 => -2,    // This month = small penalty
                    _ => -5,            // Old = penalty
                }
            }
            None => 0, // Unknown age = neutral
        }
    }
    
    /// Calculate bonus score for special features
    fn calculate_bonus_score(&self, release: &Release) -> i32 {
        let mut bonus = 0;
        
        // Freeleech bonus
        if release.freeleech == Some(true) {
            bonus += 20;
        }
        
        // Known good release groups
        if let Some(ref group) = release.release_group {
            let group_lower = group.to_lowercase();
            if ["yify", "rarbg", "sparks", "blow"].iter().any(|&g| group_lower.contains(g)) {
                bonus += 10;
            }
        }
        
        bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_release(title: &str) -> Release {
        Release::from_title(title.to_string(), "http://test.com/download".to_string())
    }

    #[test]
    fn test_release_parsing() {
        let release = create_test_release("Movie.2023.1080p.BluRay.x264-GROUP");
        assert_eq!(release.quality, Quality::HD1080p);
        assert_eq!(release.source, Source::BluRay);
    }

    #[test]
    fn test_constraint_checking() {
        let profile = QualityProfile::default();
        let mut engine = DecisionEngine::new(profile);
        engine.min_seeders = Some(5);
        engine.max_size_gb = Some(10);
        
        // Release with enough seeders and small size should pass
        let good_release = create_test_release("Movie.2023.1080p.BluRay.x264")
            .with_seeders(10)
            .with_size(5 * 1024 * 1024 * 1024); // 5GB
        assert!(engine.meets_constraints(&good_release));
        
        // Release with too few seeders should fail
        let bad_seeders = create_test_release("Movie.2023.1080p.BluRay.x264")
            .with_seeders(2);
        assert!(!engine.meets_constraints(&bad_seeders));
        
        // Release too large should fail
        let too_large = create_test_release("Movie.2023.1080p.BluRay.x264")
            .with_size(15 * 1024 * 1024 * 1024); // 15GB
        assert!(!engine.meets_constraints(&too_large));
    }

    #[test]
    fn test_release_scoring() {
        let profile = QualityProfile::default();
        let engine = DecisionEngine::permissive(profile);
        
        let release = create_test_release("Movie.2023.1080p.BluRay.x264-YIFY")
            .with_seeders(25)
            .with_size(8 * 1024 * 1024 * 1024) // 8GB
            .with_age_hours(12)
            .with_freeleech(true)
            .with_release_group("YIFY".to_string());
        
        let score = engine.evaluate_release(&release).unwrap();
        
        // Quality: 1080p BluRay = 35, Seeders: 25 = 10, Size: 8GB = 10, Age: 12h = 10, 
        // Bonus: freeleech(20) + good group(10) = 30
        // Total should be 35 + 10 + 10 + 10 + 30 = 95
        assert_eq!(score.total, 95);
        assert_eq!(score.quality_score, 35);
        assert_eq!(score.seeders_score, 10);
        assert_eq!(score.size_score, 10);
        assert_eq!(score.age_score, 10);
        assert_eq!(score.bonus_score, 30);
    }

    #[test]
    fn test_best_release_selection() {
        let profile = QualityProfile::default();
        let engine = DecisionEngine::permissive(profile);
        
        let releases = vec![
            // Lower quality but good stats
            create_test_release("Movie.2023.720p.BluRay.x264")
                .with_seeders(50)
                .with_freeleech(true),
            // Higher quality, fewer seeders
            create_test_release("Movie.2023.1080p.BluRay.x264")
                .with_seeders(10),
            // Highest quality (should win despite fewer seeders)
            create_test_release("Movie.2023.2160p.BluRay.x264")
                .with_seeders(5),
        ];
        
        let best = engine.select_best_release(releases).unwrap();
        assert!(best.title.contains("2160p")); // 4K should win
    }

    #[test]
    fn test_no_suitable_releases() {
        let profile = QualityProfile::default();
        let engine = DecisionEngine::new(profile);
        
        // All SD releases should be rejected by default profile
        let releases = vec![
            create_test_release("Movie.2023.480p.DVD.x264"),
            create_test_release("Movie.2023.SD.HDTV.x264"),
        ];
        
        let result = engine.select_best_release(releases);
        assert!(result.is_none()); // No suitable releases
    }

    #[test]
    fn test_constraint_filtering() {
        let profile = QualityProfile::default();
        let mut engine = DecisionEngine::new(profile);
        engine.min_seeders = Some(10);
        
        let releases = vec![
            // Good quality but no seeders - should be filtered out
            create_test_release("Movie.2023.1080p.BluRay.x264")
                .with_seeders(2),
            // Lower quality but good seeders - should be selected
            create_test_release("Movie.2023.720p.BluRay.x264")
                .with_seeders(25),
        ];
        
        let best = engine.select_best_release(releases).unwrap();
        assert!(best.title.contains("720p")); // Only viable option
    }
}