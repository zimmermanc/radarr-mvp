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
            username: String::new(),
            passkey: String::new(),
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

// Re-export HDBitsTorrent from indexers crate
pub use radarr_indexers::hdbits::HDBitsTorrent;

// Re-export HDBits types from indexers crate
pub use radarr_indexers::hdbits::{HDBitsImdbInfo as HDBitsImdb};

// Category is not available in indexers crate, define locally
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
    pub comprehensive_reputation_score: f64,
    pub evidence_based_tier: String,
    pub quality_tier: String,
    pub categories_covered: Vec<String>,
    pub seeder_health_score: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub release_history: Vec<ReleaseMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseMetric {
    pub torrent_id: String,
    pub name: String,
    pub seeders: i32,
    pub leechers: i32,
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
    pub config: Option<HDBitsConfig>,
    pub output_dir: Option<String>,
}

impl SceneGroupAnalyzer {
    pub fn new() -> Self {
        Self {
            group_metrics: HashMap::new(),
            config: None,
            output_dir: None,
        }
    }
    
    pub fn with_config(config: HDBitsConfig, output_dir: String) -> Self {
        Self {
            group_metrics: HashMap::new(),
            config: Some(config),
            output_dir: Some(output_dir),
        }
    }
    
    pub async fn collect_and_analyze(&mut self) -> Result<AnalysisReport> {
        info!("Starting data collection and analysis");
        
        // Ensure we have a configuration
        let config = self.config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("HDBits configuration not set"))?
            .clone();
        
        // Create API client
        let api_client = HDBitsClient::new(config);
        
        // Collect comprehensive data from HDBits
        let torrents = api_client.collect_comprehensive_data().await
            .context("Failed to collect data from HDBits")?;
        
        info!("Collected {} torrents from HDBits", torrents.len());
        
        // Analyze the collected torrents
        self.analyze_torrents(torrents)?;
        
        // Generate the analysis report
        let report = self.generate_analysis_report();
        
        // Save report if output directory is specified
        if let Some(output_dir) = &self.output_dir {
            self.save_analysis_report(&report, output_dir).await?;
        }
        
        Ok(report)
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
                    torrent_id: torrent.id.to_string(),
                    name: torrent.name.clone(),
                    seeders: torrent.seeders as i32,
                    leechers: torrent.leechers as i32,
                    size_gb: torrent.size as f64 / (1024.0 * 1024.0 * 1024.0),
                    completion_rate: if torrent.seeders + torrent.leechers > 0 {
                        torrent.times_completed as f64 / (torrent.seeders + torrent.leechers) as f64
                    } else {
                        0.0
                    },
                    is_internal: torrent.is_internal(),
                    added_date: torrent.parsed_date().unwrap_or_else(|| Utc::now()),
                    category: get_category_name(torrent.type_category),
                    codec: get_codec_name(torrent.type_codec),
                    medium: get_medium_name(torrent.type_medium),
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
                        comprehensive_reputation_score: 0.0,
                        evidence_based_tier: "Unrated".to_string(),
                        quality_tier: "Unrated".to_string(),
                        categories_covered: Vec::new(),
                        seeder_health_score: 0.0,
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
    
    pub fn generate_analysis_report(&self) -> AnalysisReport {
        let mut report = AnalysisReport::default();
        
        report.total_torrents_analyzed = self.group_metrics.values()
            .map(|g| g.total_releases)
            .sum();
        report.unique_scene_groups = self.group_metrics.len() as u32;
        report.internal_releases = self.group_metrics.values()
            .map(|g| g.internal_releases)
            .sum();
        report.external_releases = report.total_torrents_analyzed - report.internal_releases;
        
        // Quality distribution
        for group in self.group_metrics.values() {
            match group.reputation_score {
                score if score >= 80.0 => report.quality_distribution.premium_groups += 1,
                score if score >= 70.0 => report.quality_distribution.high_quality_groups += 1,
                score if score >= 60.0 => report.quality_distribution.standard_groups += 1,
                score if score >= 40.0 => report.quality_distribution.low_quality_groups += 1,
                _ => report.quality_distribution.poor_groups += 1,
            }
        }
        
        // Top groups
        let mut top_groups: Vec<_> = self.group_metrics.values().collect();
        top_groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        report.top_groups_by_reputation = top_groups.iter()
            .take(10)
            .map(|g| SceneGroupSummary {
                group_name: g.group_name.clone(),
                reputation_score: g.reputation_score,
                quality_tier: g.quality_tier.clone(),
                total_releases: g.total_releases,
            })
            .collect();
        
        // Statistical summary
        if !self.group_metrics.is_empty() {
            let reputation_scores: Vec<f64> = self.group_metrics.values()
                .map(|g| g.reputation_score)
                .collect();
            let seeder_counts: Vec<f64> = self.group_metrics.values()
                .map(|g| g.avg_seeders)
                .collect();
            let file_sizes: Vec<f64> = self.group_metrics.values()
                .map(|g| g.avg_size_gb)
                .collect();
            
            report.statistical_summary.reputation_scores = Self::calculate_statistics(&reputation_scores);
            report.statistical_summary.seeder_counts = Self::calculate_statistics(&seeder_counts);
            report.statistical_summary.file_sizes_gb = Self::calculate_statistics(&file_sizes);
        }
        
        // Temporal analysis
        let now = Utc::now();
        for group in self.group_metrics.values() {
            let days_since_last = now.signed_duration_since(group.last_seen).num_days();
            if days_since_last <= 30 {
                report.temporal_analysis.active_groups_last_30_days += 1;
            }
            if days_since_last <= 90 {
                report.temporal_analysis.active_groups_last_90_days += 1;
            }
            let days_since_first = now.signed_duration_since(group.first_seen).num_days();
            if days_since_first >= 730 { // 2 years
                report.temporal_analysis.established_groups_over_2_years += 1;
            }
            if days_since_last > 180 { // 6 months
                report.temporal_analysis.dormant_groups += 1;
            }
        }
        
        report
    }
    
    fn calculate_statistics(values: &[f64]) -> StatisticalMetrics {
        if values.is_empty() {
            return StatisticalMetrics::default();
        }
        
        let mut sorted_values: Vec<f64> = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let min = sorted_values[0];
        let max = sorted_values[sorted_values.len() - 1];
        let mean = sorted_values.iter().sum::<f64>() / sorted_values.len() as f64;
        let p95_index = ((sorted_values.len() as f64 * 0.95) as usize).min(sorted_values.len() - 1);
        let p95 = sorted_values[p95_index];
        
        StatisticalMetrics { min, max, mean, p95 }
    }
    
    pub async fn save_analysis_report(&self, report: &AnalysisReport, output_dir: &str) -> Result<()> {
        use std::fs;
        use std::path::Path;
        
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        
        // Save JSON report
        let json_path = Path::new(output_dir).join("analysis_report.json");
        let json_content = serde_json::to_string_pretty(report)?;
        fs::write(&json_path, json_content)?;
        info!("Saved analysis report to: {:?}", json_path);
        
        // Save scene groups data
        let groups_path = Path::new(output_dir).join("scene_groups.json");
        let groups_content = self.export_analysis()?;
        fs::write(&groups_path, groups_content)?;
        info!("Saved scene groups data to: {:?}", groups_path);
        
        Ok(())
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub total_torrents_analyzed: u32,
    pub unique_scene_groups: u32,
    pub internal_releases: u32,
    pub external_releases: u32,
    pub collection_duration_seconds: u64,
    pub quality_distribution: AnalysisQualityDistribution,
    pub top_groups_by_reputation: Vec<SceneGroupSummary>,
    pub statistical_summary: StatisticalSummary,
    pub temporal_analysis: TemporalAnalysis,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AnalysisQualityDistribution {
    pub premium_groups: u32,
    pub high_quality_groups: u32,
    pub standard_groups: u32,
    pub low_quality_groups: u32,
    pub poor_groups: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneGroupSummary {
    pub group_name: String,
    pub reputation_score: f64,
    pub quality_tier: String,
    pub total_releases: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatisticalSummary {
    pub reputation_scores: StatisticalMetrics,
    pub seeder_counts: StatisticalMetrics,
    pub file_sizes_gb: StatisticalMetrics,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatisticalMetrics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub p95: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TemporalAnalysis {
    pub active_groups_last_30_days: u32,
    pub active_groups_last_90_days: u32,
    pub established_groups_over_2_years: u32,
    pub dormant_groups: u32,
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
        let mut analyzer = SceneGroupAnalyzer::new();
        
        // Create some sample release history
        let mut release_history = Vec::new();
        for i in 0..10 {
            release_history.push(ReleaseMetric {
                torrent_id: format!("torrent_{}", i),
                name: format!("Test.Movie.{}.1080p.BluRay-TEST", 2020 + i),
                seeders: (20 + i as u32) as i32,
                leechers: (2 + (i % 3) as u32) as i32,
                size_gb: 18.0 + (i as f64 * 0.5),
                completion_rate: 0.75 + (i as f64 * 0.02),
                is_internal: i % 2 == 0,
                added_date: Utc::now() - chrono::Duration::days(30 - i as i64),
                category: "Movies".to_string(),
                codec: "x264".to_string(),
                medium: "BluRay".to_string(),
            });
        }

        let mut metrics = SceneGroupMetrics {
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
            comprehensive_reputation_score: 0.0,
            evidence_based_tier: "Premium".to_string(),
            quality_tier: "Premium".to_string(),
            categories_covered: vec!["Movies".to_string()],
            seeder_health_score: 0.9,
            first_seen: Utc::now() - chrono::Duration::days(365),
            last_seen: Utc::now() - chrono::Duration::days(7),
            release_history,
        };

        SceneGroupAnalyzer::calculate_group_metrics_static(&mut metrics);
        
        assert!(metrics.reputation_score > 50.0); // Should be a good score
        assert!(metrics.reputation_score <= 100.0); // Should not exceed max
    }
}

// Helper functions to convert HDBits API IDs to human-readable names
pub fn get_category_name(id: u32) -> String {
    match id {
        1 => "Movie".to_string(),
        2 => "TV".to_string(),
        3 => "Documentary".to_string(),
        4 => "Music".to_string(),
        5 => "Sport".to_string(),
        6 => "Audio".to_string(),
        7 => "XXX".to_string(),
        8 => "Misc/Demo".to_string(),
        _ => "Unknown".to_string(),
    }
}

pub fn get_codec_name(id: u32) -> String {
    match id {
        1 => "H.264".to_string(),
        2 => "MPEG-2".to_string(),
        3 => "VC-1".to_string(),
        4 => "XviD".to_string(),
        5 => "HEVC".to_string(),
        _ => "Unknown".to_string(),
    }
}

pub fn get_medium_name(id: u32) -> String {
    match id {
        1 => "Blu-ray".to_string(),
        3 => "Encode".to_string(),
        4 => "Capture".to_string(),
        5 => "Remux".to_string(),
        6 => "WEB-DL".to_string(),
        _ => "Unknown".to_string(),
    }
}