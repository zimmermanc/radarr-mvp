//! Quality profiles and scoring system
//!
//! This module implements quality profiles that define user preferences
//! for movie releases, including resolution, source, and format preferences.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use uuid::Uuid;

/// Video quality levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Quality {
    /// Standard definition (480p and below)
    SD,
    /// High definition 720p
    HD720p,
    /// Full high definition 1080p
    HD1080p,
    /// Ultra high definition 4K/2160p
    UHD4K,
    /// Unknown or unspecified quality
    Unknown,
}

impl Quality {
    /// Get quality score for comparison (higher is better)
    pub fn score(&self) -> i32 {
        match self {
            Quality::SD => 1,
            Quality::HD720p => 2,
            Quality::HD1080p => 3,
            Quality::UHD4K => 4,
            Quality::Unknown => 0,
        }
    }

    /// Parse quality from resolution string
    pub fn from_resolution(resolution: &str) -> Self {
        let res = resolution.to_lowercase();
        if res.contains("2160p") || res.contains("4k") {
            Quality::UHD4K
        } else if res.contains("1080p") {
            Quality::HD1080p
        } else if res.contains("720p") {
            Quality::HD720p
        } else if res.contains("480p") || res.contains("sd") {
            Quality::SD
        } else {
            Quality::Unknown
        }
    }
}

/// Source type for releases
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    /// BluRay source (highest quality)
    BluRay,
    /// Web download (high quality)
    WebDL,
    /// TV broadcast recording
    HDTV,
    /// DVD source
    DVD,
    /// Camera recording (lowest quality)
    CAM,
    /// Unknown source
    Unknown,
}

impl Source {
    /// Get source quality score (higher is better)
    pub fn score(&self) -> i32 {
        match self {
            Source::BluRay => 5,
            Source::WebDL => 4,
            Source::HDTV => 3,
            Source::DVD => 2,
            Source::CAM => 1,
            Source::Unknown => 0,
        }
    }

    /// Parse source from release name
    pub fn from_release_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();
        if name_lower.contains("bluray") || name_lower.contains("blu-ray") {
            Source::BluRay
        } else if name_lower.contains("web-dl") || name_lower.contains("webdl") {
            Source::WebDL
        } else if name_lower.contains("hdtv") {
            Source::HDTV
        } else if name_lower.contains("dvd") {
            Source::DVD
        } else if name_lower.contains("cam") || name_lower.contains("camrip") {
            Source::CAM
        } else {
            Source::Unknown
        }
    }
}

/// Individual quality item in a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityItem {
    /// The quality level
    pub quality: Quality,
    /// Whether this quality is allowed
    pub allowed: bool,
    /// Whether this quality is preferred
    pub preferred: bool,
}

impl QualityItem {
    pub fn new(quality: Quality, allowed: bool, preferred: bool) -> Self {
        Self {
            quality,
            allowed,
            preferred,
        }
    }
}

/// Complete quality profile definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProfile {
    /// Unique identifier
    pub id: Uuid,
    /// Profile name
    pub name: String,
    /// Quality cutoff (minimum acceptable quality)
    pub cutoff: Quality,
    /// List of quality preferences
    pub items: Vec<QualityItem>,
    /// Minimum format score required
    pub min_format_score: i32,
    /// Whether upgrades are allowed
    pub upgrade_allowed: bool,
}

impl QualityProfile {
    /// Create a new quality profile
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            cutoff: Quality::HD1080p,
            items: Self::default_quality_items(),
            min_format_score: 0,
            upgrade_allowed: true,
        }
    }

    /// Default quality items for new profiles
    fn default_quality_items() -> Vec<QualityItem> {
        vec![
            QualityItem::new(Quality::UHD4K, true, true),
            QualityItem::new(Quality::HD1080p, true, false),
            QualityItem::new(Quality::HD720p, true, false),
            QualityItem::new(Quality::SD, false, false),
        ]
    }

    /// Check if a quality is allowed by this profile
    pub fn is_quality_allowed(&self, quality: &Quality) -> bool {
        self.items
            .iter()
            .find(|item| item.quality == *quality)
            .map(|item| item.allowed)
            .unwrap_or(false)
    }

    /// Check if a quality is preferred by this profile
    pub fn is_quality_preferred(&self, quality: &Quality) -> bool {
        self.items
            .iter()
            .find(|item| item.quality == *quality)
            .map(|item| item.preferred)
            .unwrap_or(false)
    }

    /// Calculate quality score for a release
    pub fn calculate_quality_score(&self, quality: &Quality, source: &Source) -> i32 {
        if !self.is_quality_allowed(quality) {
            return -1; // Not allowed
        }

        let mut score = quality.score() * 10 + source.score();

        // Bonus for preferred quality
        if self.is_quality_preferred(quality) {
            score += 50;
        }

        score
    }

    /// Check if an upgrade is warranted
    pub fn should_upgrade(&self, current_quality: &Quality, new_quality: &Quality) -> bool {
        if !self.upgrade_allowed {
            return false;
        }

        // Only upgrade if new quality is better and allowed
        self.is_quality_allowed(new_quality) && new_quality.score() > current_quality.score()
    }
}

/// Default quality profiles
impl Default for QualityProfile {
    fn default() -> Self {
        Self::new("Default".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_scoring() {
        assert_eq!(Quality::UHD4K.score(), 4);
        assert_eq!(Quality::HD1080p.score(), 3);
        assert_eq!(Quality::HD720p.score(), 2);
        assert_eq!(Quality::SD.score(), 1);
        assert_eq!(Quality::Unknown.score(), 0);
    }

    #[test]
    fn test_source_scoring() {
        assert_eq!(Source::BluRay.score(), 5);
        assert_eq!(Source::WebDL.score(), 4);
        assert_eq!(Source::HDTV.score(), 3);
        assert_eq!(Source::DVD.score(), 2);
        assert_eq!(Source::CAM.score(), 1);
        assert_eq!(Source::Unknown.score(), 0);
    }

    #[test]
    fn test_quality_parsing() {
        assert_eq!(Quality::from_resolution("1080p"), Quality::HD1080p);
        assert_eq!(Quality::from_resolution("2160p"), Quality::UHD4K);
        assert_eq!(Quality::from_resolution("720p"), Quality::HD720p);
        assert_eq!(Quality::from_resolution("4K"), Quality::UHD4K);
    }

    #[test]
    fn test_source_parsing() {
        assert_eq!(
            Source::from_release_name("Movie.2023.1080p.BluRay.x264"),
            Source::BluRay
        );
        assert_eq!(
            Source::from_release_name("Movie.2023.1080p.WEB-DL.x264"),
            Source::WebDL
        );
        assert_eq!(
            Source::from_release_name("Movie.2023.720p.HDTV.x264"),
            Source::HDTV
        );
    }

    #[test]
    fn test_quality_profile_defaults() {
        let profile = QualityProfile::default();
        assert_eq!(profile.name, "Default");
        assert_eq!(profile.cutoff, Quality::HD1080p);
        assert!(profile.upgrade_allowed);

        // Should allow 4K, 1080p, 720p but not SD
        assert!(profile.is_quality_allowed(&Quality::UHD4K));
        assert!(profile.is_quality_allowed(&Quality::HD1080p));
        assert!(profile.is_quality_allowed(&Quality::HD720p));
        assert!(!profile.is_quality_allowed(&Quality::SD));

        // Should prefer 4K only
        assert!(profile.is_quality_preferred(&Quality::UHD4K));
        assert!(!profile.is_quality_preferred(&Quality::HD1080p));
    }

    #[test]
    fn test_profile_quality_scoring() {
        let profile = QualityProfile::default();

        // 4K BluRay should score highest (4*10 + 5 + 50 = 95)
        let score_4k_bluray = profile.calculate_quality_score(&Quality::UHD4K, &Source::BluRay);
        assert_eq!(score_4k_bluray, 95);

        // 1080p BluRay should score lower (3*10 + 5 = 35)
        let score_1080p_bluray =
            profile.calculate_quality_score(&Quality::HD1080p, &Source::BluRay);
        assert_eq!(score_1080p_bluray, 35);

        // SD should not be allowed (-1)
        let score_sd = profile.calculate_quality_score(&Quality::SD, &Source::BluRay);
        assert_eq!(score_sd, -1);
    }

    #[test]
    fn test_upgrade_logic() {
        let profile = QualityProfile::default();

        // Should upgrade from 720p to 1080p
        assert!(profile.should_upgrade(&Quality::HD720p, &Quality::HD1080p));

        // Should upgrade from 1080p to 4K
        assert!(profile.should_upgrade(&Quality::HD1080p, &Quality::UHD4K));

        // Should not downgrade from 1080p to 720p
        assert!(!profile.should_upgrade(&Quality::HD1080p, &Quality::HD720p));

        // Should not upgrade to disallowed quality
        assert!(!profile.should_upgrade(&Quality::HD720p, &Quality::SD));
    }
}
