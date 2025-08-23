//! hdbits_browse_analyzer module

use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn};
use crate::{SceneGroupMetrics, ReleaseMetric, HDBitsTorrent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDBitsBrowseConfig {
    pub username: String,
    pub passkey: String,
    pub base_url: String,
    pub max_pages: usize,
    pub delay_seconds: u64,
    pub rate_limit_seconds: u64,
    pub request_delay_seconds: u64,
}

impl Default for HDBitsBrowseConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            passkey: String::new(),
            base_url: "https://hdbits.org".to_string(),
            max_pages: 100,
            delay_seconds: 1,
            rate_limit_seconds: 35,
            request_delay_seconds: 1,
        }
    }
}

pub struct HDBitsBrowseAnalyzer {
    config: HDBitsBrowseConfig,
    scene_groups: HashMap<String, SceneGroupMetrics>,
    releases: Vec<ReleaseMetric>,
}

impl HDBitsBrowseAnalyzer {
    pub fn new(config: HDBitsBrowseConfig) -> Self {
        Self { 
            config,
            scene_groups: HashMap::new(),
            releases: Vec::new(),
        }
    }
    
    pub async fn login(&self) -> Result<()> {
        info!("Authenticating with HDBits using API credentials");
        
        // HDBits uses API key authentication, not traditional login
        // The username and passkey are used directly in API requests
        // Verify credentials by making a test API call
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let test_request = serde_json::json!({
            "username": &self.config.username,
            "passkey": &self.config.passkey,
            "limit": 1
        });
        
        let response = client
            .post(&format!("{}/api/torrents", self.config.base_url))
            .json(&test_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Authentication failed: {}", response.status()));
        }
        
        info!("Authentication successful");
        Ok(())
    }
    
    pub async fn collect_internal_releases(&self) -> Result<Vec<HDBitsTorrent>> {
        info!("Collecting internal releases from HDBits");
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let mut all_torrents = Vec::new();
        let categories = vec![1, 3]; // Movies and Documentaries
        
        for category in categories {
            info!("Collecting internal releases for category {}", category);
            let mut page = 0;
            
            while page < self.config.max_pages {
                // Add delay between requests
                if page > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(self.config.request_delay_seconds)).await;
                }
                
                let request = serde_json::json!({
                    "username": &self.config.username,
                    "passkey": &self.config.passkey,
                    "category": [category],
                    "origin": [1], // Internal releases only
                    "limit": 100,
                    "page": page
                });
                
                let response = client
                    .post(&format!("{}/api/torrents", self.config.base_url))
                    .json(&request)
                    .send()
                    .await?;
                
                if !response.status().is_success() {
                    warn!("Failed to fetch page {} for category {}", page, category);
                    break;
                }
                
                let api_response: serde_json::Value = response.json().await?;
                
                if let Some(data) = api_response["data"].as_array() {
                    if data.is_empty() {
                        break; // No more results
                    }
                    
                    for torrent_json in data {
                        if let Ok(torrent) = serde_json::from_value::<HDBitsTorrent>(torrent_json.clone()) {
                            all_torrents.push(torrent);
                        }
                    }
                    
                    page += 1;
                } else {
                    break;
                }
            }
        }
        
        info!("Collected {} internal releases", all_torrents.len());
        Ok(all_torrents)
    }
    
    pub fn analyze_scene_groups(&mut self, releases: Vec<HDBitsTorrent>) -> Result<()> {
        info!("Analyzing {} releases for scene groups", releases.len());
        
        for torrent in releases {
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
                    category: torrent.category.name.clone(),
                    codec: torrent.type_codec.clone(),
                    medium: torrent.type_medium.clone(),
                };
                
                self.releases.push(release_metric.clone());
                
                let group_metrics = self.scene_groups.entry(group_name.clone()).or_insert_with(|| {
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
                
                group_metrics.total_releases += 1;
                if release_metric.is_internal {
                    group_metrics.internal_releases += 1;
                }
                
                if release_metric.added_date < group_metrics.first_seen {
                    group_metrics.first_seen = release_metric.added_date;
                }
                if release_metric.added_date > group_metrics.last_seen {
                    group_metrics.last_seen = release_metric.added_date;
                }
                
                if !group_metrics.categories_covered.contains(&torrent.category.name) {
                    group_metrics.categories_covered.push(torrent.category.name.clone());
                }
                
                group_metrics.release_history.push(release_metric);
            }
        }
        
        // Calculate metrics for each group
        for group_metrics in self.scene_groups.values_mut() {
            Self::calculate_group_metrics(group_metrics);
        }
        
        info!("Scene group analysis complete. Found {} unique groups", self.scene_groups.len());
        Ok(())
    }
    
    fn extract_scene_group(torrent_name: &str) -> Option<String> {
        let patterns = [
            r"-([A-Za-z0-9]+)$",
            r"\.([A-Za-z0-9]+)$",
            r"\[([A-Za-z0-9]+)\]$",
            r"\(([A-Za-z0-9]+)\)$",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(torrent_name) {
                    if let Some(group) = captures.get(1) {
                        let group_name = group.as_str().to_uppercase();
                        if !["X264", "X265", "H264", "H265", "HEVC", "AVC", "AAC", "AC3", "DTS", "BLURAY", "WEB", "HDTV"].contains(&group_name.as_str()) {
                            return Some(group_name);
                        }
                    }
                }
            }
        }
        None
    }
    
    fn calculate_group_metrics(metrics: &mut SceneGroupMetrics) {
        if metrics.release_history.is_empty() {
            return;
        }

        let releases = &metrics.release_history;
        let count = releases.len() as f64;

        metrics.avg_seeders = releases.iter().map(|r| r.seeders as f64).sum::<f64>() / count;
        metrics.avg_leechers = releases.iter().map(|r| r.leechers as f64).sum::<f64>() / count;
        metrics.avg_size_gb = releases.iter().map(|r| r.size_gb).sum::<f64>() / count;
        metrics.avg_completion_rate = releases.iter().map(|r| r.completion_rate).sum::<f64>() / count;

        metrics.seeder_leecher_ratio = if metrics.avg_leechers > 0.0 {
            metrics.avg_seeders / metrics.avg_leechers
        } else {
            metrics.avg_seeders
        };

        metrics.internal_ratio = metrics.internal_releases as f64 / metrics.total_releases as f64;

        let size_variance = releases.iter()
            .map(|r| (r.size_gb - metrics.avg_size_gb).powi(2))
            .sum::<f64>() / count;
        metrics.quality_consistency = 1.0 / (1.0 + size_variance);

        let now = Utc::now();
        let days_since_last = now.signed_duration_since(metrics.last_seen).num_days() as f64;
        metrics.recency_score = 1.0 / (1.0 + days_since_last / 30.0);
        
        metrics.seeder_health_score = (metrics.avg_seeders / 100.0).min(1.0);
        
        // Calculate reputation score
        let seeder_score = (metrics.seeder_leecher_ratio / 10.0).min(1.0) * 25.0;
        let internal_score = metrics.internal_ratio * 20.0;
        let completion_score = metrics.avg_completion_rate.min(1.0) * 15.0;
        let consistency_score = metrics.quality_consistency * 15.0;
        let recency_score = metrics.recency_score * 10.0;
        let volume_score = (metrics.total_releases as f64 / 100.0).min(1.0) * 10.0;
        let size_score = Self::calculate_size_score(metrics.avg_size_gb) * 5.0;
        
        metrics.reputation_score = seeder_score + internal_score + completion_score + 
                                   consistency_score + recency_score + volume_score + size_score;
        metrics.comprehensive_reputation_score = metrics.reputation_score;
        
        metrics.quality_tier = match metrics.reputation_score {
            score if score >= 90.0 => "Elite",
            score if score >= 80.0 => "Premium",
            score if score >= 70.0 => "Excellent",
            score if score >= 60.0 => "Good",
            score if score >= 50.0 => "Average",
            score if score >= 40.0 => "Below Average",
            _ => "Poor",
        }.to_string();
        
        metrics.evidence_based_tier = metrics.quality_tier.clone();
    }
    
    fn calculate_size_score(avg_size_gb: f64) -> f64 {
        match avg_size_gb {
            size if size >= 15.0 && size <= 50.0 => 1.0,
            size if size >= 8.0 && size < 15.0 => 0.8,
            size if size >= 4.0 && size < 8.0 => 0.6,
            size if size >= 2.0 && size < 4.0 => 0.4,
            size if size < 2.0 => 0.2,
            size if size > 50.0 => 0.7,
            _ => 0.5,
        }
    }
    
    pub fn get_scene_groups(&self) -> &HashMap<String, SceneGroupMetrics> {
        &self.scene_groups
    }
    
    pub fn generate_analysis_report(&self, _start_time: DateTime<Utc>) -> BrowseAnalysisReport {
        let mut report = BrowseAnalysisReport::default();
        
        report.total_torrents_analyzed = self.releases.len() as u32;
        report.unique_scene_groups = self.scene_groups.len() as u32;
        report.internal_releases_analyzed = self.releases.iter()
            .filter(|r| r.is_internal)
            .count() as u32;
        
        // Get top groups
        let mut top_groups: Vec<_> = self.scene_groups.values().collect();
        top_groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        report.top_internal_groups = top_groups.into_iter()
            .take(10)
            .map(|g| GroupSummary {
                name: g.group_name.clone(),
                score: g.reputation_score,
                releases: g.total_releases,
            })
            .collect();
        
        report
    }
    
    pub fn export_reputation_data(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.scene_groups)
            .map_err(|e| anyhow::anyhow!("Failed to serialize reputation data: {}", e))
    }
    
    pub fn export_csv_data(&self) -> String {
        let mut csv = String::from("group_name,reputation_score,total_releases,internal_releases\n");
        for group in self.scene_groups.values() {
            csv.push_str(&format!(
                "{},{},{},{}\n",
                group.group_name,
                group.reputation_score,
                group.total_releases,
                group.internal_releases
            ));
        }
        csv
    }
    
    pub async fn analyze(&self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // Placeholder implementation
        Ok(serde_json::json!({
            "status": "not_implemented",
            "message": "HDBits browse analyzer is a work in progress"
        }))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BrowseAnalysisReport {
    pub total_torrents_analyzed: u32,
    pub unique_scene_groups: u32,
    pub internal_releases_analyzed: u32,
    pub top_internal_groups: Vec<GroupSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupSummary {
    pub name: String,
    pub score: f64,
    pub releases: u32,
}