use radarr_core::models::{Quality, Release};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Quality scoring engine that integrates HDBits scene group analysis
#[derive(Debug, Clone)]
pub struct QualityScorer {
    scene_group_scores: HashMap<String, u32>,
    quality_weights: ScoringWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub scene_group: f32,      // 40% default
    pub technical_quality: f32, // 30% default
    pub release_metadata: f32,   // 20% default
    pub custom_format: f32,      // 10% default
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            scene_group: 0.40,
            technical_quality: 0.30,
            release_metadata: 0.20,
            custom_format: 0.10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseScore {
    pub total_score: u32,
    pub scene_group_score: u32,
    pub technical_score: u32,
    pub metadata_score: u32,
    pub custom_format_score: u32,
    pub details: ScoreDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDetails {
    pub scene_group: String,
    pub resolution_points: u32,
    pub source_points: u32,
    pub codec_points: u32,
    pub seeder_points: u32,
    pub size_efficiency_points: u32,
    pub language_bonus: u32,
    pub hdr_bonus: u32,
    pub preferred_words_bonus: u32,
}

impl QualityScorer {
    pub fn new() -> Self {
        Self {
            scene_group_scores: Self::load_scene_group_scores(),
            quality_weights: ScoringWeights::default(),
        }
    }

    /// Load scene group scores from HDBits analysis
    /// In production, this would load from the analysis results database
    fn load_scene_group_scores() -> HashMap<String, u32> {
        let mut scores = HashMap::new();
        
        // Top-tier scene groups (from HDBits analysis)
        scores.insert("SPARKS".to_string(), 100);
        scores.insert("FLUX".to_string(), 100);
        scores.insert("AMIABLE".to_string(), 95);
        scores.insert("GECKOS".to_string(), 95);
        scores.insert("ROVERS".to_string(), 90);
        scores.insert("BLOW".to_string(), 90);
        
        // High-quality P2P groups
        scores.insert("NTb".to_string(), 95);
        scores.insert("CiNEFiLE".to_string(), 95);
        scores.insert("DON".to_string(), 90);
        scores.insert("SbR".to_string(), 90);
        
        // Web groups
        scores.insert("NTG".to_string(), 85);
        scores.insert("CMRG".to_string(), 85);
        scores.insert("TOMMY".to_string(), 80);
        scores.insert("TEPES".to_string(), 80);
        
        // Mid-tier groups
        scores.insert("RARBG".to_string(), 60);
        scores.insert("YTS".to_string(), 50);
        scores.insert("YIFY".to_string(), 40);
        
        scores
    }

    pub fn score_release(&self, release: &Release) -> ReleaseScore {
        let scene_group_score = self.calculate_scene_group_score(release);
        let technical_score = self.calculate_technical_score(release);
        let metadata_score = self.calculate_metadata_score(release);
        let custom_format_score = self.calculate_custom_format_score(release);
        
        let total_score = (
            scene_group_score as f32 * self.quality_weights.scene_group +
            technical_score as f32 * self.quality_weights.technical_quality +
            metadata_score as f32 * self.quality_weights.release_metadata +
            custom_format_score as f32 * self.quality_weights.custom_format
        ) as u32;
        
        ReleaseScore {
            total_score,
            scene_group_score,
            technical_score,
            metadata_score,
            custom_format_score,
            details: self.create_score_details(release),
        }
    }

    fn calculate_scene_group_score(&self, release: &Release) -> u32 {
        // Extract scene group from release title
        let scene_group = self.extract_scene_group(&release.title);
        
        self.scene_group_scores
            .get(&scene_group)
            .copied()
            .unwrap_or(30) // Default score for unknown groups
    }

    fn calculate_technical_score(&self, release: &Release) -> u32 {
        let mut score = 0u32;
        
        // Resolution scoring
        score += match release.quality.resolution.as_deref() {
            Some("2160p") | Some("4K") => 100,
            Some("1080p") => 80,
            Some("720p") => 60,
            Some("480p") => 40,
            _ => 30,
        };
        
        // Source scoring
        score += match release.quality.source.as_deref() {
            Some("BluRay") | Some("Blu-Ray") | Some("BD") => 100,
            Some("WEB-DL") | Some("WEBDL") => 80,
            Some("WEBRip") | Some("WEB") => 60,
            Some("HDTV") => 40,
            Some("DVD") => 30,
            _ => 20,
        };
        
        // Codec scoring
        score += match release.quality.codec.as_deref() {
            Some("H.265") | Some("HEVC") | Some("x265") => 100,
            Some("H.264") | Some("AVC") | Some("x264") => 80,
            Some("VP9") => 70,
            Some("XviD") | Some("DivX") => 40,
            _ => 50,
        };
        
        // Average the scores
        score / 3
    }

    fn calculate_metadata_score(&self, release: &Release) -> u32 {
        let mut score = 0u32;
        
        // Seeder scoring (availability)
        if let Some(seeders) = release.seeders {
            score += match seeders {
                s if s >= 100 => 100,
                s if s >= 50 => 80,
                s if s >= 20 => 60,
                s if s >= 10 => 40,
                s if s >= 5 => 20,
                _ => 10,
            };
        }
        
        // Size efficiency scoring
        let size_gb = release.size as f64 / (1024.0 * 1024.0 * 1024.0);
        let expected_size = self.get_expected_size(&release.quality);
        let size_ratio = size_gb / expected_size;
        
        score += if size_ratio >= 0.7 && size_ratio <= 1.3 {
            100 // Optimal size range
        } else if size_ratio >= 0.5 && size_ratio <= 1.5 {
            70
        } else if size_ratio >= 0.3 && size_ratio <= 2.0 {
            40
        } else {
            10 // Too small or too large
        };
        
        // Language bonus
        if release.languages.len() > 1 {
            score += 20; // Multi-language bonus
        }
        
        // HDR bonus
        if release.title.contains("HDR") || release.title.contains("DV") {
            score += 30;
        }
        
        // Normalize to 100
        score.min(100)
    }

    fn calculate_custom_format_score(&self, release: &Release) -> u32 {
        let mut score = 0u32;
        
        // Preferred words bonus
        let preferred_words = vec!["REMUX", "PROPER", "REPACK", "INTERNAL"];
        for word in &preferred_words {
            if release.title.contains(word) {
                score += 25;
            }
        }
        
        // Avoid certain keywords
        let avoid_words = vec!["CAM", "TS", "TELECINE", "SCREENER"];
        for word in &avoid_words {
            if release.title.contains(word) {
                return 0; // Immediate disqualification
            }
        }
        
        score.min(100)
    }

    fn extract_scene_group(&self, title: &str) -> String {
        // Extract scene group from release title
        // Usually it's the last part after the last dash
        if let Some(last_dash_idx) = title.rfind('-') {
            let group = &title[last_dash_idx + 1..];
            // Remove file extension if present
            if let Some(dot_idx) = group.rfind('.') {
                return group[..dot_idx].to_string();
            }
            return group.to_string();
        }
        
        "Unknown".to_string()
    }

    fn get_expected_size(&self, quality: &Quality) -> f64 {
        // Expected size in GB based on quality
        match quality.resolution.as_deref() {
            Some("2160p") => 25.0,
            Some("1080p") => 10.0,
            Some("720p") => 5.0,
            Some("480p") => 2.0,
            _ => 8.0,
        }
    }

    fn create_score_details(&self, release: &Release) -> ScoreDetails {
        ScoreDetails {
            scene_group: self.extract_scene_group(&release.title),
            resolution_points: self.score_resolution(&release.quality),
            source_points: self.score_source(&release.quality),
            codec_points: self.score_codec(&release.quality),
            seeder_points: release.seeders.map(|s| self.score_seeders(s)).unwrap_or(0),
            size_efficiency_points: self.score_size_efficiency(release),
            language_bonus: if release.languages.len() > 1 { 20 } else { 0 },
            hdr_bonus: if release.title.contains("HDR") { 30 } else { 0 },
            preferred_words_bonus: self.score_preferred_words(&release.title),
        }
    }

    fn score_resolution(&self, quality: &Quality) -> u32 {
        match quality.resolution.as_deref() {
            Some("2160p") | Some("4K") => 100,
            Some("1080p") => 80,
            Some("720p") => 60,
            Some("480p") => 40,
            _ => 30,
        }
    }

    fn score_source(&self, quality: &Quality) -> u32 {
        match quality.source.as_deref() {
            Some("BluRay") | Some("Blu-Ray") => 100,
            Some("WEB-DL") => 80,
            Some("WEBRip") => 60,
            Some("HDTV") => 40,
            _ => 20,
        }
    }

    fn score_codec(&self, quality: &Quality) -> u32 {
        match quality.codec.as_deref() {
            Some("H.265") | Some("HEVC") => 100,
            Some("H.264") | Some("AVC") => 80,
            _ => 50,
        }
    }

    fn score_seeders(&self, seeders: u32) -> u32 {
        match seeders {
            s if s >= 100 => 100,
            s if s >= 50 => 80,
            s if s >= 20 => 60,
            _ => 40,
        }
    }

    fn score_size_efficiency(&self, release: &Release) -> u32 {
        let size_gb = release.size as f64 / (1024.0 * 1024.0 * 1024.0);
        let expected = self.get_expected_size(&release.quality);
        let ratio = size_gb / expected;
        
        if ratio >= 0.7 && ratio <= 1.3 {
            100
        } else if ratio >= 0.5 && ratio <= 1.5 {
            70
        } else {
            40
        }
    }

    fn score_preferred_words(&self, title: &str) -> u32 {
        let mut score = 0u32;
        let preferred = vec!["REMUX", "PROPER", "REPACK"];
        for word in preferred {
            if title.contains(word) {
                score += 25;
            }
        }
        score.min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_group_extraction() {
        let scorer = QualityScorer::new();
        
        assert_eq!(scorer.extract_scene_group("Movie.2024.1080p.BluRay.x264-SPARKS"), "SPARKS");
        assert_eq!(scorer.extract_scene_group("Movie.2024.2160p.WEB-DL.DDP5.1.H.265-FLUX"), "FLUX");
        assert_eq!(scorer.extract_scene_group("Movie.2024.720p.WEBRip.x264-YTS.mkv"), "YTS");
    }

    #[test]
    fn test_quality_scoring() {
        let scorer = QualityScorer::new();
        
        let high_quality_release = Release {
            id: 1,
            title: "Movie.2024.1080p.BluRay.x264-SPARKS".to_string(),
            indexer: "HDBits".to_string(),
            quality: Quality {
                id: 1,
                name: "1080p BluRay".to_string(),
                resolution: Some("1080p".to_string()),
                source: Some("BluRay".to_string()),
                codec: Some("H.264".to_string()),
            },
            size: 10 * 1024 * 1024 * 1024, // 10GB
            seeders: Some(150),
            leechers: Some(5),
            languages: vec!["English".to_string()],
            scene_group: Some("SPARKS".to_string()),
            score: None,
        };
        
        let score = scorer.score_release(&high_quality_release);
        
        // Should get high score due to SPARKS group and good quality
        assert!(score.total_score > 80);
        assert_eq!(score.scene_group_score, 100);
    }
}