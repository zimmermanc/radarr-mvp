//! Search integration service for connecting indexer results with queue management

use crate::models::{Movie, Release, QueuePriority};
use crate::services::{QueueService, QueueRepository, DownloadClientService};
use crate::{Result, RadarrError};
use uuid::Uuid;

/// Service for integrating search results with download queue
pub struct SearchIntegrationService<Q: QueueRepository, D: DownloadClientService> {
    queue_service: QueueService<Q, D>,
}

impl<Q: QueueRepository, D: DownloadClientService> SearchIntegrationService<Q, D> {
    /// Create a new search integration service
    pub fn new(queue_service: QueueService<Q, D>) -> Self {
        Self { queue_service }
    }
    
    /// Automatically download the best release for a movie
    pub async fn auto_download_for_movie(
        &self,
        movie: &Movie,
        releases: Vec<Release>,
        quality_preferences: &QualityPreferences,
    ) -> Result<Option<Uuid>> {
        if releases.is_empty() {
            return Ok(None);
        }
        
        // Score and rank releases
        let mut scored_releases: Vec<_> = releases
            .into_iter()
            .map(|release| {
                let score = self.calculate_release_score(&release, quality_preferences);
                (release, score)
            })
            .collect();
        
        // Sort by score (highest first)
        scored_releases.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take the best release
        if let Some((best_release, score)) = scored_releases.into_iter().next() {
            if score >= quality_preferences.minimum_score_threshold {
                tracing::info!("Auto-downloading release '{}' for movie '{}' (score: {:.2})", 
                             best_release.title, movie.title, score);
                
                let queue_item = self.queue_service.grab_release(
                    movie,
                    &best_release,
                    Some(QueuePriority::Normal),
                    Some("movies".to_string()),
                ).await?;
                
                return Ok(Some(queue_item.id));
            } else {
                tracing::debug!("Best release '{}' for movie '{}' scored {:.2}, below threshold {:.2}", 
                              best_release.title, movie.title, score, quality_preferences.minimum_score_threshold);
            }
        }
        
        Ok(None)
    }
    
    /// Grab a specific release manually
    pub async fn grab_release_manual(
        &self,
        movie: &Movie,
        release: &Release,
        priority: Option<QueuePriority>,
    ) -> Result<Uuid> {
        tracing::info!("Manually grabbing release '{}' for movie '{}'", 
                     release.title, movie.title);
        
        let queue_item = self.queue_service.grab_release(
            movie,
            release,
            Some(priority.unwrap_or(QueuePriority::High)),
            Some("movies".to_string()),
        ).await?;
        
        Ok(queue_item.id)
    }
    
    /// Calculate release score based on quality preferences
    fn calculate_release_score(&self, release: &Release, prefs: &QualityPreferences) -> f32 {
        let mut score = 0.0;
        
        // Parse release name for quality indicators
        let title_lower = release.title.to_lowercase();
        
        // Resolution scoring
        if title_lower.contains("2160p") || title_lower.contains("4k") {
            score += prefs.resolution_scores.uhd_4k;
        } else if title_lower.contains("1080p") {
            score += prefs.resolution_scores.full_hd;
        } else if title_lower.contains("720p") {
            score += prefs.resolution_scores.hd;
        } else {
            score += prefs.resolution_scores.sd;
        }
        
        // Source scoring
        if title_lower.contains("bluray") || title_lower.contains("blu-ray") {
            score += prefs.source_scores.bluray;
        } else if title_lower.contains("webrip") {
            score += prefs.source_scores.webrip;
        } else if title_lower.contains("webdl") || title_lower.contains("web-dl") {
            score += prefs.source_scores.webdl;
        } else if title_lower.contains("hdtv") {
            score += prefs.source_scores.hdtv;
        } else if title_lower.contains("dvdrip") {
            score += prefs.source_scores.dvdrip;
        }
        
        // Codec scoring
        if title_lower.contains("x265") || title_lower.contains("hevc") {
            score += prefs.codec_scores.hevc;
        } else if title_lower.contains("x264") || title_lower.contains("avc") {
            score += prefs.codec_scores.avc;
        }
        
        // Size scoring (prefer reasonable sizes)
        if let Some(size_bytes) = release.size_bytes {
            let size_gb = size_bytes as f32 / (1024.0 * 1024.0 * 1024.0);
            
            // Reasonable size ranges by resolution
            let size_score = if title_lower.contains("2160p") || title_lower.contains("4k") {
                // 4K: prefer 15-50GB
                if size_gb >= 15.0 && size_gb <= 50.0 {
                    prefs.size_preference_bonus
                } else if size_gb < 8.0 {
                    -10.0 // Probably low quality
                } else {
                    0.0
                }
            } else if title_lower.contains("1080p") {
                // 1080p: prefer 5-15GB  
                if size_gb >= 5.0 && size_gb <= 15.0 {
                    prefs.size_preference_bonus
                } else if size_gb < 2.0 {
                    -5.0 // Probably low quality
                } else {
                    0.0
                }
            } else {
                // 720p and below: prefer 2-8GB
                if size_gb >= 2.0 && size_gb <= 8.0 {
                    prefs.size_preference_bonus
                } else if size_gb < 1.0 {
                    -3.0 // Probably low quality
                } else {
                    0.0
                }
            };
            
            score += size_score;
        }
        
        // Seeders bonus (for torrents)
        if let Some(seeders) = release.seeders {
            if seeders >= 50 {
                score += prefs.seeder_bonus.high;
            } else if seeders >= 10 {
                score += prefs.seeder_bonus.medium;
            } else if seeders >= 5 {
                score += prefs.seeder_bonus.low;
            } else if seeders < 2 {
                score -= 5.0; // Very few seeders
            }
        }
        
        // Age penalty (prefer newer releases)
        if let Some(age_hours) = release.age_hours {
            let age_days = age_hours as f32 / 24.0;
            if age_days > 30.0 {
                score -= (age_days - 30.0) * 0.1; // 0.1 point penalty per day after 30 days
            }
        }
        
        // Scene group bonus/penalty
        for preferred_group in &prefs.preferred_groups {
            if title_lower.contains(&preferred_group.to_lowercase()) {
                score += prefs.preferred_group_bonus;
                break;
            }
        }
        
        for ignored_group in &prefs.ignored_groups {
            if title_lower.contains(&ignored_group.to_lowercase()) {
                score -= prefs.ignored_group_penalty;
                break;
            }
        }
        
        // Must-have keywords
        let has_required = prefs.required_keywords.iter()
            .all(|keyword| title_lower.contains(&keyword.to_lowercase()));
        if !has_required {
            score -= 50.0; // Heavy penalty for missing required keywords
        }
        
        // Forbidden keywords
        let has_forbidden = prefs.forbidden_keywords.iter()
            .any(|keyword| title_lower.contains(&keyword.to_lowercase()));
        if has_forbidden {
            score -= 25.0; // Heavy penalty for forbidden keywords
        }
        
        score.max(0.0) // Don't allow negative scores
    }
}

/// Quality preferences for release scoring
#[derive(Debug, Clone)]
pub struct QualityPreferences {
    /// Minimum score threshold for auto-download
    pub minimum_score_threshold: f32,
    /// Resolution scoring weights
    pub resolution_scores: ResolutionScores,
    /// Source scoring weights  
    pub source_scores: SourceScores,
    /// Codec scoring weights
    pub codec_scores: CodecScores,
    /// Bonus for preferred file sizes
    pub size_preference_bonus: f32,
    /// Seeder bonus configuration
    pub seeder_bonus: SeederBonus,
    /// Preferred scene groups
    pub preferred_groups: Vec<String>,
    /// Ignored scene groups
    pub ignored_groups: Vec<String>,
    /// Bonus points for preferred groups
    pub preferred_group_bonus: f32,
    /// Penalty points for ignored groups
    pub ignored_group_penalty: f32,
    /// Required keywords (all must be present)
    pub required_keywords: Vec<String>,
    /// Forbidden keywords (none can be present)
    pub forbidden_keywords: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolutionScores {
    pub uhd_4k: f32,
    pub full_hd: f32,
    pub hd: f32,
    pub sd: f32,
}

#[derive(Debug, Clone)]
pub struct SourceScores {
    pub bluray: f32,
    pub webdl: f32,
    pub webrip: f32,
    pub hdtv: f32,
    pub dvdrip: f32,
}

#[derive(Debug, Clone)]
pub struct CodecScores {
    pub hevc: f32,
    pub avc: f32,
}

#[derive(Debug, Clone)]
pub struct SeederBonus {
    pub high: f32,   // 50+ seeders
    pub medium: f32, // 10-49 seeders
    pub low: f32,    // 5-9 seeders
}

impl Default for QualityPreferences {
    fn default() -> Self {
        Self {
            minimum_score_threshold: 50.0,
            resolution_scores: ResolutionScores {
                uhd_4k: 40.0,
                full_hd: 35.0,
                hd: 25.0,
                sd: 5.0,
            },
            source_scores: SourceScores {
                bluray: 30.0,
                webdl: 25.0,
                webrip: 20.0,
                hdtv: 10.0,
                dvdrip: 5.0,
            },
            codec_scores: CodecScores {
                hevc: 10.0,
                avc: 5.0,
            },
            size_preference_bonus: 5.0,
            seeder_bonus: SeederBonus {
                high: 10.0,
                medium: 5.0,
                low: 2.0,
            },
            preferred_groups: vec![
                "IMAX".to_string(),
                "FraMeSToR".to_string(),
                "KRaLiMaRKo".to_string(),
            ],
            ignored_groups: vec![
                "YIFY".to_string(),
                "YTS".to_string(),
            ],
            preferred_group_bonus: 15.0,
            ignored_group_penalty: 20.0,
            required_keywords: vec![],
            forbidden_keywords: vec![
                "CAM".to_string(),
                "TS".to_string(),
                "HDTS".to_string(),
                "SCREENER".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Release, ReleaseProtocol, Movie};
    
    #[test]
    fn test_release_scoring() {
        let prefs = QualityPreferences::default();
        
        // High quality release
        let mut high_quality = Release::new(
            1,
            "Movie.Name.2023.2160p.BluRay.x265.HDR-IMAX".to_string(),
            "magnet:test".to_string(),
            "guid1".to_string(),
            ReleaseProtocol::Torrent,
        );
        high_quality.size_bytes = Some(25 * 1024 * 1024 * 1024); // 25GB
        high_quality.seeders = Some(100);
        
        // Test would need actual implementations - this is a placeholder test
        // In a real test, you would create a mock queue service
        // and test the scoring algorithm
        
        // For now, just test that the quality preferences have sensible defaults
        assert!(prefs.resolution_scores.uhd_4k > prefs.resolution_scores.full_hd);
        assert!(prefs.source_scores.bluray > prefs.source_scores.webrip);
    }
    
    #[test]  
    fn test_quality_preferences_default() {
        let prefs = QualityPreferences::default();
        
        assert_eq!(prefs.minimum_score_threshold, 50.0);
        assert!(prefs.resolution_scores.uhd_4k > prefs.resolution_scores.full_hd);
        assert!(prefs.source_scores.bluray > prefs.source_scores.webrip);
        assert!(prefs.codec_scores.hevc > prefs.codec_scores.avc);
        assert!(!prefs.preferred_groups.is_empty());
        assert!(!prefs.ignored_groups.is_empty());
        assert!(!prefs.forbidden_keywords.is_empty());
    }
}