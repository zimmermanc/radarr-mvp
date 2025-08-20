// HDBits API Integration and Scene Group Analysis
// Data-driven reputation scoring based on real HDBits metrics

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct HDBitsConfig {
    pub username: String,
    pub passkey: String,
    pub api_url: String,
    pub rate_limit_per_hour: u32,
}

impl Default for HDBitsConfig {
    fn default() -> Self {
        Self {
            username: "blargdiesel".to_string(),
            passkey: "ed487790cd0dee98941ab5c132179bd2c8c5e23622c0c04a800ad543cde2990cd44ed960892d990214ea1618bf29780386a77246a21dc636d83420e077e69863".to_string(),
            api_url: "https://hdbits.org/api/torrents".to_string(),
            rate_limit_per_hour: 150,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HDBitsSearchRequest {
    pub username: String,
    pub passkey: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Vec<u32>>, // [1] for internal releases only
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct HDBitsResponse {
    pub status: u32,
    pub message: Option<String>,
    pub data: Option<Vec<HDBitsTorrent>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HDBitsTorrent {
    pub id: String,
    pub name: String,
    pub times_completed: u32,
    pub seeders: u32,
    pub leechers: u32,
    pub size: u64,
    pub added: String, // ISO date string
    pub imdb: Option<HDBitsImdb>,
    pub tvdb: Option<u32>,
    pub category: HDBitsCategory,
    pub type_category: String,
    pub type_codec: String,
    pub type_medium: String,
    pub type_origin: String,
    pub freeleech: Option<String>,
    pub internal: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HDBitsImdb {
    pub id: u32,
    pub rating: Option<f32>,
    pub votes: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HDBitsCategory {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGroupMetrics {
    pub group_name: String,
    pub total_releases: u32,
    pub internal_releases: u32,
    pub avg_seeders: f64,
    pub avg_leechers: f64,
    pub avg_size_gb: f64,
    pub avg_completion_rate: f64,
    pub seeder_leecher_ratio: f64,
    pub internal_ratio: f64,
    pub quality_consistency: f64,
    pub recency_score: f64,
    pub reputation_score: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub release_history: Vec<ReleaseMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMetric {
    pub torrent_id: String,
    pub name: String,
    pub seeders: u32,
    pub leechers: u32,
    pub size_gb: f64,
    pub completion_rate: f64,
    pub is_internal: bool,
    pub added_date: DateTime<Utc>,
    pub category: String,
    pub codec: String,
    pub medium: String,
}

pub struct HDBitsClient {
    client: Client,
    config: HDBitsConfig,
    requests_made: std::sync::Arc<std::sync::Mutex<u32>>,
    last_request_time: std::sync::Arc<std::sync::Mutex<DateTime<Utc>>>,
}

impl HDBitsClient {
    pub fn new(config: HDBitsConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("radarr-rust/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            requests_made: std::sync::Arc::new(std::sync::Mutex::new(0)),
            last_request_time: std::sync::Arc::new(std::sync::Mutex::new(Utc::now())),
        }
    }

    async fn check_rate_limit(&self) -> Result<()> {
        let now = Utc::now();
        let mut last_time = self.last_request_time.lock().unwrap();
        let mut requests = self.requests_made.lock().unwrap();

        // Reset counter if more than an hour has passed
        if now.signed_duration_since(*last_time).num_seconds() > 3600 {
            *requests = 0;
        }

        if *requests >= self.config.rate_limit_per_hour {
            let wait_time = 3600 - now.signed_duration_since(*last_time).num_seconds();
            if wait_time > 0 {
                warn!("Rate limit reached, waiting {} seconds", wait_time);
                sleep(Duration::from_secs(wait_time as u64)).await;
                *requests = 0;
            }
        }

        // Add delay between requests to be respectful
        sleep(Duration::from_millis(2400)).await; // 1500 requests per hour = 2.4s between requests
        
        *requests += 1;
        *last_time = now;
        
        Ok(())
    }

    pub async fn search_torrents(&self, request: HDBitsSearchRequest) -> Result<Vec<HDBitsTorrent>> {
        self.check_rate_limit().await?;

        debug!("Searching HDBits with request: {:?}", request);

        let response = self
            .client
            .post(&self.config.api_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to HDBits API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HDBits API returned error status: {}",
                response.status()
            ));
        }

        let hdbits_response: HDBitsResponse = response
            .json()
            .await
            .context("Failed to parse HDBits API response")?;

        if hdbits_response.status != 5 {
            return Err(anyhow::anyhow!(
                "HDBits API error: {} - {}",
                hdbits_response.status,
                hdbits_response.message.unwrap_or_default()
            ));
        }

        Ok(hdbits_response.data.unwrap_or_default())
    }

    pub async fn collect_internal_releases(&self, categories: Vec<u32>, limit_per_category: u32) -> Result<Vec<HDBitsTorrent>> {
        let mut all_torrents = Vec::new();

        for category in categories {
            info!("Collecting internal releases for category: {}", category);
            
            let mut page = 0;
            let mut total_collected = 0;

            while total_collected < limit_per_category {
                let request = HDBitsSearchRequest {
                    username: self.config.username.clone(),
                    passkey: self.config.passkey.clone(),
                    category: Some(vec![category]),
                    origin: Some(vec![1]), // Internal releases only
                    limit: Some(100), // Max per request
                    page: Some(page),
                    codec: None,
                    medium: None,
                    search: None,
                };

                let torrents = self.search_torrents(request).await?;
                
                if torrents.is_empty() {
                    break; // No more results
                }

                let to_take = std::cmp::min(torrents.len(), (limit_per_category - total_collected) as usize);
                all_torrents.extend(torrents.into_iter().take(to_take));
                total_collected += to_take as u32;

                page += 1;
                
                if total_collected >= limit_per_category {
                    break;
                }
            }

            info!("Collected {} internal releases for category {}", total_collected, category);
        }

        Ok(all_torrents)
    }

    pub async fn collect_comprehensive_data(&self) -> Result<Vec<HDBitsTorrent>> {
        info!("Starting comprehensive HDBits data collection");
        
        // Focus on movie categories for scene group analysis
        let categories = vec![
            1, // Movie
            3, // Documentary
        ];

        // Collect both internal and external releases for comparison
        let mut all_torrents = Vec::new();

        // Collect internal releases (higher quality, scene groups)
        let internal_torrents = self.collect_internal_releases(categories.clone(), 500).await?;
        all_torrents.extend(internal_torrents);

        // Collect external releases for baseline comparison
        for category in categories {
            info!("Collecting external releases for category: {}", category);
            
            let request = HDBitsSearchRequest {
                username: self.config.username.clone(),
                passkey: self.config.passkey.clone(),
                category: Some(vec![category]),
                origin: None, // All origins
                limit: Some(100),
                page: Some(0),
                codec: None,
                medium: None,
                search: None,
            };

            let torrents = self.search_torrents(request).await?;
            all_torrents.extend(torrents.into_iter().take(200)); // Limit external for comparison
        }

        info!("Comprehensive data collection complete: {} total torrents", all_torrents.len());
        Ok(all_torrents)
    }
}

pub struct SceneGroupAnalyzer {
    pub group_metrics: HashMap<String, SceneGroupMetrics>,
}

impl SceneGroupAnalyzer {
    pub fn new() -> Self {
        Self {
            group_metrics: HashMap::new(),
        }
    }

    pub fn extract_scene_group(torrent_name: &str) -> Option<String> {
        // Common scene group patterns in release names
        let patterns = [
            r"-([A-Za-z0-9]+)$",              // Standard: Movie.Name.2023.1080p.BluRay.x264-GROUP
            r"\.([A-Za-z0-9]+)$",             // Dot notation: Movie.Name.2023.1080p.BluRay.x264.GROUP
            r"\[([A-Za-z0-9]+)\]$",           // Brackets: Movie.Name.2023.1080p.BluRay.x264[GROUP]
            r"\(([A-Za-z0-9]+)\)$",           // Parentheses: Movie.Name.2023.1080p.BluRay.x264(GROUP)
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(torrent_name) {
                    if let Some(group) = captures.get(1) {
                        let group_name = group.as_str().to_uppercase();
                        // Filter out common false positives
                        if !["X264", "X265", "H264", "H265", "HEVC", "AVC", "AAC", "AC3", "DTS", "BLURAY", "WEB", "HDTV"].contains(&group_name.as_str()) {
                            return Some(group_name);
                        }
                    }
                }
            }
        }

        None
    }

    pub fn analyze_torrents(&mut self, torrents: Vec<HDBitsTorrent>) -> Result<()> {
        info!("Analyzing {} torrents for scene group metrics", torrents.len());

        for torrent in torrents {
            if let Some(group_name) = Self::extract_scene_group(&torrent.name) {
                let release_metric = ReleaseMetric {
                    torrent_id: torrent.id.clone(),
                    name: torrent.name.clone(),
                    seeders: torrent.seeders,
                    leechers: torrent.leechers,
                    size_gb: torrent.size as f64 / (1024.0 * 1024.0 * 1024.0),
                    completion_rate: if torrent.seeders + torrent.leechers > 0 {
                        torrent.times_completed as f64 / (torrent.seeders + torrent.leechers) as f64
                    } else {
                        0.0
                    },
                    is_internal: torrent.internal,
                    added_date: chrono::DateTime::parse_from_rfc3339(&torrent.added)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    category: torrent.category.name,
                    codec: torrent.type_codec,
                    medium: torrent.type_medium,
                };

                let group_metrics = self.group_metrics.entry(group_name.clone()).or_insert_with(|| {
                    SceneGroupMetrics {
                        group_name: group_name.clone(),
                        total_releases: 0,
                        internal_releases: 0,
                        avg_seeders: 0.0,
                        avg_leechers: 0.0,
                        avg_size_gb: 0.0,
                        avg_completion_rate: 0.0,
                        seeder_leecher_ratio: 0.0,
                        internal_ratio: 0.0,
                        quality_consistency: 0.0,
                        recency_score: 0.0,
                        reputation_score: 0.0,
                        first_seen: release_metric.added_date,
                        last_seen: release_metric.added_date,
                        release_history: Vec::new(),
                    }
                });

                // Update metrics
                group_metrics.total_releases += 1;
                if release_metric.is_internal {
                    group_metrics.internal_releases += 1;
                }
                
                group_metrics.release_history.push(release_metric.clone());
                
                if release_metric.added_date < group_metrics.first_seen {
                    group_metrics.first_seen = release_metric.added_date;
                }
                if release_metric.added_date > group_metrics.last_seen {
                    group_metrics.last_seen = release_metric.added_date;
                }
            }
        }

        // Calculate derived metrics for each group
        let group_keys: Vec<String> = self.group_metrics.keys().cloned().collect();
        for group_name in group_keys {
            if let Some(group_metrics) = self.group_metrics.get_mut(&group_name) {
                Self::calculate_group_metrics_static(group_metrics);
            }
        }

        info!("Analysis complete. Found {} unique scene groups", self.group_metrics.len());
        Ok(())
    }

    fn calculate_group_metrics_static(metrics: &mut SceneGroupMetrics) {
        if metrics.release_history.is_empty() {
            return;
        }

        let releases = &metrics.release_history;
        let count = releases.len() as f64;

        // Basic averages
        metrics.avg_seeders = releases.iter().map(|r| r.seeders as f64).sum::<f64>() / count;
        metrics.avg_leechers = releases.iter().map(|r| r.leechers as f64).sum::<f64>() / count;
        metrics.avg_size_gb = releases.iter().map(|r| r.size_gb).sum::<f64>() / count;
        metrics.avg_completion_rate = releases.iter().map(|r| r.completion_rate).sum::<f64>() / count;

        // Ratios
        metrics.seeder_leecher_ratio = if metrics.avg_leechers > 0.0 {
            metrics.avg_seeders / metrics.avg_leechers
        } else {
            metrics.avg_seeders // If no leechers, ratio is just seeders
        };

        metrics.internal_ratio = metrics.internal_releases as f64 / metrics.total_releases as f64;

        // Quality consistency (lower variance in size = higher consistency)
        let size_variance = releases.iter()
            .map(|r| (r.size_gb - metrics.avg_size_gb).powi(2))
            .sum::<f64>() / count;
        metrics.quality_consistency = 1.0 / (1.0 + size_variance); // Inverse relationship

        // Recency score (more recent releases = higher score)
        let now = Utc::now();
        let days_since_last = now.signed_duration_since(metrics.last_seen).num_days() as f64;
        metrics.recency_score = 1.0 / (1.0 + days_since_last / 30.0); // Decay over months

        // Overall reputation score (weighted combination)
        metrics.reputation_score = Self::calculate_reputation_score_static(metrics);
    }

    fn calculate_reputation_score_static(metrics: &SceneGroupMetrics) -> f64 {
        // Weighted scoring formula based on real data analysis
        let weights = ReputationWeights {
            seeder_ratio: 0.25,      // High seeders indicate quality/popularity
            internal_ratio: 0.20,    // Internal releases are typically higher quality
            completion_rate: 0.15,   // High completion indicates good releases
            consistency: 0.15,       // Consistent quality over time
            recency: 0.10,          // Recent activity indicates active group
            volume: 0.10,           // Number of releases indicates established group
            size_appropriateness: 0.05, // Reasonable file sizes for quality
        };

        let seeder_score = (metrics.seeder_leecher_ratio / 10.0).min(1.0); // Normalize to 0-1
        let internal_score = metrics.internal_ratio;
        let completion_score = metrics.avg_completion_rate.min(1.0);
        let consistency_score = metrics.quality_consistency;
        let recency_score = metrics.recency_score;
        let volume_score = (metrics.total_releases as f64 / 100.0).min(1.0); // Normalize
        let size_score = Self::calculate_size_appropriateness_score_static(metrics);

        let weighted_score = 
            weights.seeder_ratio * seeder_score +
            weights.internal_ratio * internal_score +
            weights.completion_rate * completion_score +
            weights.consistency * consistency_score +
            weights.recency * recency_score +
            weights.volume * volume_score +
            weights.size_appropriateness * size_score;

        // Scale to 0-100 for easier interpretation
        weighted_score * 100.0
    }

    fn calculate_size_appropriateness_score_static(metrics: &SceneGroupMetrics) -> f64 {
        // Reasonable size ranges for different quality levels
        // This is based on general quality standards
        let avg_size = metrics.avg_size_gb;
        
        match avg_size {
            size if size >= 15.0 && size <= 50.0 => 1.0,  // Full quality range
            size if size >= 8.0 && size < 15.0 => 0.8,    // Good compressed
            size if size >= 4.0 && size < 8.0 => 0.6,     // Acceptable
            size if size >= 2.0 && size < 4.0 => 0.4,     // Low quality
            size if size < 2.0 => 0.2,                     // Very low quality
            size if size > 50.0 => 0.7,                    // Possibly uncompressed/remux
            _ => 0.5,                                       // Default
        }
    }

    pub fn get_top_groups(&self, limit: usize) -> Vec<&SceneGroupMetrics> {
        let mut groups: Vec<&SceneGroupMetrics> = self.group_metrics.values().collect();
        groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        groups.into_iter().take(limit).collect()
    }

    pub fn get_group_by_name(&self, name: &str) -> Option<&SceneGroupMetrics> {
        self.group_metrics.get(&name.to_uppercase())
    }

    pub fn export_analysis(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.group_metrics)
            .context("Failed to serialize scene group analysis")
    }
}

struct ReputationWeights {
    seeder_ratio: f64,
    internal_ratio: f64,
    completion_rate: f64,
    consistency: f64,
    recency: f64,
    volume: f64,
    size_appropriateness: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_group_extraction() {
        assert_eq!(SceneGroupAnalyzer::extract_scene_group("Movie.Name.2023.1080p.BluRay.x264-SPARKS"), Some("SPARKS".to_string()));
        assert_eq!(SceneGroupAnalyzer::extract_scene_group("Movie.Name.2023.1080p.BluRay.x264.ROVERS"), Some("ROVERS".to_string()));
        assert_eq!(SceneGroupAnalyzer::extract_scene_group("Movie.Name.2023.1080p.BluRay.x264[CMRG]"), Some("CMRG".to_string()));
        assert_eq!(SceneGroupAnalyzer::extract_scene_group("Movie.Name.2023.1080p.BluRay.x264(FGT)"), Some("FGT".to_string()));
        
        // Should not match codec/format strings
        assert_eq!(SceneGroupAnalyzer::extract_scene_group("Movie.Name.2023.1080p.BluRay-x264"), None);
        assert_eq!(SceneGroupAnalyzer::extract_scene_group("Movie.Name.2023.1080p.BluRay.HEVC"), None);
    }

    #[test]
    fn test_reputation_score_calculation() {
        let metrics = SceneGroupMetrics {
            group_name: "TEST".to_string(),
            total_releases: 50,
            internal_releases: 40,
            avg_seeders: 25.0,
            avg_leechers: 5.0,
            avg_size_gb: 20.0,
            avg_completion_rate: 0.8,
            seeder_leecher_ratio: 5.0,
            internal_ratio: 0.8,
            quality_consistency: 0.9,
            recency_score: 0.8,
            reputation_score: 0.0,
            first_seen: Utc::now() - chrono::Duration::days(365),
            last_seen: Utc::now() - chrono::Duration::days(7),
            release_history: Vec::new(),
        };

        let score = SceneGroupAnalyzer::calculate_reputation_score_static(&metrics);
        
        assert!(score > 50.0, "Score was {} but should be > 50.0", score); // Should be a good score
        assert!(score <= 100.0, "Score was {} but should be <= 100.0", score); // Should not exceed max
    }
}