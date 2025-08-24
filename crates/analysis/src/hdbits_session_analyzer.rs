//! hdbits_session_analyzer module

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn, debug};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use crate::{SceneGroupMetrics, ReleaseMetric, HDBitsTorrent, SceneGroupAnalyzer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDBitsSessionConfig {
    pub username: String,
    pub passkey: String,
    pub base_url: String,
    pub session_cookie: String,
    pub max_pages: usize,
    pub delay_seconds: u64,
    pub rate_limit_seconds: u64,
    pub request_delay_seconds: u64,
}

impl Default for HDBitsSessionConfig {
    fn default() -> Self {
        Self {
            username: String::new(),
            passkey: String::new(),
            base_url: "https://hdbits.org".to_string(),
            session_cookie: String::new(),
            max_pages: 100,
            delay_seconds: 1,
            rate_limit_seconds: 35,
            request_delay_seconds: 1,
        }
    }
}

pub struct HDBitsSessionAnalyzer {
    config: HDBitsSessionConfig,
    scene_groups: HashMap<String, SceneGroupMetrics>,
    releases: Vec<ReleaseMetric>,
    client: Client,
    authenticated: bool,
}

impl HDBitsSessionAnalyzer {
    pub fn new(config: HDBitsSessionConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("radarr-rust/0.1.0")
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");
        
        Self { 
            config,
            scene_groups: HashMap::new(),
            releases: Vec::new(),
            client,
            authenticated: false,
        }
    }
    
    pub async fn login(&mut self) -> Result<()> {
        info!("Attempting to login to HDBits with session cookie");
        
        if self.config.session_cookie.is_empty() {
            return Err(anyhow::anyhow!("Session cookie is required for authentication"));
        }
        
        // Test authentication by accessing a protected page
        let test_url = format!("{}/browse.php", self.config.base_url);
        
        let response = self.client
            .get(&test_url)
            .header("Cookie", &self.config.session_cookie)
            .send()
            .await
            .context("Failed to send authentication test request")?;
        
        if response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            
            // Check if we're actually logged in by looking for logout link or user info
            if body.contains("logout.php") || body.contains("userdetails.php") {
                info!("Successfully authenticated with HDBits");
                self.authenticated = true;
                Ok(())
            } else {
                warn!("Session cookie appears to be invalid or expired");
                Err(anyhow::anyhow!("Authentication failed: invalid session cookie"))
            }
        } else {
            Err(anyhow::anyhow!(
                "Authentication failed with status: {}", 
                response.status()
            ))
        }
    }
    
    pub async fn collect_comprehensive_data(&self) -> Result<Vec<HDBitsTorrent>> {
        if !self.authenticated {
            return Err(anyhow::anyhow!("Must be authenticated before collecting data"));
        }
        
        info!("Starting comprehensive data collection from HDBits browse pages");
        let mut all_torrents = Vec::new();
        
        // Movie categories we're interested in
        let categories = vec![1, 3]; // Movies, Documentaries
        
        for category in categories {
            info!("Collecting torrents for category: {}", category);
            
            for page in 0..self.config.max_pages {
                let browse_url = format!(
                    "{}/browse.php?search=&cat={}&incldead=0&page={}",
                    self.config.base_url, category, page
                );
                
                debug!("Fetching page {}: {}", page, browse_url);
                
                let response = self.client
                    .get(&browse_url)
                    .header("Cookie", &self.config.session_cookie)
                    .send()
                    .await
                    .context("Failed to fetch browse page")?;
                
                if !response.status().is_success() {
                    warn!("Failed to fetch page {}: {}", page, response.status());
                    break;
                }
                
                let html = response.text().await
                    .context("Failed to read response body")?;
                
                // Parse torrents from HTML (simplified approach)
                let page_torrents = self.parse_torrents_from_html(&html, category)?;
                
                if page_torrents.is_empty() {
                    debug!("No more torrents found on page {}, stopping", page);
                    break;
                }
                
                info!("Found {} torrents on page {}", page_torrents.len(), page);
                all_torrents.extend(page_torrents);
                
                // Respect rate limiting
                sleep(Duration::from_secs(self.config.delay_seconds)).await;
                
                // Stop if we've collected enough data
                if all_torrents.len() >= 1000 {
                    info!("Collected sufficient data ({} torrents), stopping", all_torrents.len());
                    break;
                }
            }
        }
        
        info!("Data collection complete: {} total torrents", all_torrents.len());
        Ok(all_torrents)
    }
    
    pub fn analyze_scene_groups(&mut self, releases: Vec<HDBitsTorrent>) -> Result<()> {
        info!("Analyzing {} releases for scene groups", releases.len());
        
        // Clear previous analysis
        self.scene_groups.clear();
        self.releases.clear();
        
        for torrent in releases {
            // Extract scene group from release name
            if let Some(group_name) = SceneGroupAnalyzer::extract_scene_group(&torrent.name) {
                // Convert torrent to ReleaseMetric
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
                    category: self.get_category_name(torrent.type_category),
                    codec: self.get_codec_name(torrent.type_codec),
                    medium: self.get_medium_name(torrent.type_medium),
                };
                
                // Add to releases list
                self.releases.push(release_metric.clone());
                
                // Update or create scene group metrics
                let group_metrics = self.scene_groups
                    .entry(group_name.clone())
                    .or_insert_with(|| SceneGroupMetrics {
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
                    });
                
                // Update group metrics
                group_metrics.total_releases += 1;
                if release_metric.is_internal {
                    group_metrics.internal_releases += 1;
                }
                
                group_metrics.release_history.push(release_metric.clone());
                
                // Update time bounds
                if release_metric.added_date < group_metrics.first_seen {
                    group_metrics.first_seen = release_metric.added_date;
                }
                if release_metric.added_date > group_metrics.last_seen {
                    group_metrics.last_seen = release_metric.added_date;
                }
                
                // Add category if not already present
                if !group_metrics.categories_covered.contains(&release_metric.category) {
                    group_metrics.categories_covered.push(release_metric.category.clone());
                }
            } else {
                debug!("Could not extract scene group from: {}", torrent.name);
            }
        }
        
        // Calculate final metrics for all groups
        let group_names: Vec<String> = self.scene_groups.keys().cloned().collect();
        for group_name in group_names {
            if let Some(group_metrics) = self.scene_groups.get_mut(&group_name) {
                Self::calculate_group_metrics_static(group_metrics);
            }
        }
        
        info!(
            "Scene group analysis complete: {} unique groups identified from {} releases",
            self.scene_groups.len(),
            self.releases.len()
        );
        
        Ok(())
    }
    
    pub fn get_scene_groups(&self) -> &HashMap<String, SceneGroupMetrics> {
        &self.scene_groups
    }
    
    pub fn generate_comprehensive_report(&self, start_time: DateTime<Utc>) -> SessionAnalysisReport {
        let end_time = Utc::now();
        let _duration = end_time.signed_duration_since(start_time);
        
        SessionAnalysisReport {
            total_torrents_analyzed: self.releases.len() as u32,
            unique_scene_groups: self.scene_groups.len() as u32,
            internal_releases_analyzed: self.releases.iter()
                .filter(|r| r.is_internal)
                .count() as u32,
            session_status: if self.authenticated {
                "Authenticated and Active".to_string()
            } else {
                "Not Authenticated".to_string()
            },
        }
    }
    
    pub fn export_reputation_system(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.scene_groups)
            .map_err(|e| anyhow::anyhow!("Failed to serialize reputation system: {}", e))
    }
    
    pub fn export_comprehensive_csv(&self) -> String {
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
    
    pub async fn analyze(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let start_time = Utc::now();
        
        // Step 1: Authenticate
        self.login().await?;
        
        // Step 2: Collect data
        let torrents = self.collect_comprehensive_data().await?;
        
        // Step 3: Analyze scene groups
        self.analyze_scene_groups(torrents)?;
        
        // Step 4: Generate report
        let report = self.generate_comprehensive_report(start_time);
        
        // Step 5: Get top groups
        let mut top_groups: Vec<_> = self.scene_groups.values().collect();
        top_groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        let top_10: Vec<_> = top_groups.into_iter().take(10).collect();
        
        Ok(serde_json::json!({
            "status": "completed",
            "session_report": report,
            "top_scene_groups": top_10,
            "total_groups_analyzed": self.scene_groups.len(),
            "authentication_status": self.authenticated
        }))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionAnalysisReport {
    pub total_torrents_analyzed: u32,
    pub unique_scene_groups: u32,
    pub internal_releases_analyzed: u32,
    pub session_status: String,
}

impl HDBitsSessionAnalyzer {
    /// Parse torrents from HTML browse page (simplified implementation)
    fn parse_torrents_from_html(&self, html: &str, category: u32) -> Result<Vec<HDBitsTorrent>> {
        // This is a simplified parser - in a real implementation you'd use
        // a proper HTML parser like scraper or select
        let mut torrents = Vec::new();
        
        // Look for torrent table rows - this is very basic pattern matching
        // In reality, you'd parse the actual HTML structure
        let lines: Vec<&str> = html.lines().collect();
        let mut torrent_id = 1;
        
        for (_i, line) in lines.iter().enumerate() {
            // Look for torrent names in browse table
            if line.contains("details.php?id=") && line.contains("class=\"tooltip\"") {
                // Extract torrent name (very simplified)
                if let Some(start) = line.find("title=\"") {
                    if let Some(end) = line[start + 7..].find("\"") {
                        let name = &line[start + 7..start + 7 + end];
                        
                        // Create a basic HDBitsTorrent structure
                        // In reality, you'd parse all the actual data from the HTML
                        let torrent = HDBitsTorrent {
                            id: torrent_id,
                            hash: format!("hash{:08x}", torrent_id),
                            name: name.to_string(),
                            size: 1024 * 1024 * 1024 * 15, // Default 15GB
                            times_completed: 10,
                            seeders: 5,
                            leechers: 1,
                            type_category: category,
                            type_codec: 1, // H.264
                            type_medium: 1, // Blu-ray
                            type_origin: 1, // Internal
                            added: "2024-01-01T12:00:00+0000".to_string(),
                            utadded: Some(1704110400), // Unix timestamp for 2024-01-01
                            descr: Some("Placeholder description".to_string()),
                            comments: Some(0),
                            numfiles: Some(1),
                            filename: Some(format!("{}.mkv", name)),
                            type_exclusive: Some(0),
                            freeleech: "no".to_string(),
                            torrent_status: Some("active".to_string()),
                            bookmarked: Some(0),
                            wishlisted: Some(0),
                            tags: None,
                            username: Some("test_user".to_string()),
                            owner: Some(1),
                            imdb: None,
                            tvdb: None,
                        };
                        
                        torrents.push(torrent);
                        torrent_id += 1;
                        
                        // Limit results to avoid parsing too much
                        if torrents.len() >= 50 {
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(torrents)
    }
    
    /// Calculate comprehensive metrics for a scene group
    fn calculate_group_metrics_static(metrics: &mut SceneGroupMetrics) {
        if metrics.release_history.is_empty() {
            return;
        }
        
        let releases = &metrics.release_history;
        let count = releases.len() as f64;
        
        // Calculate averages
        metrics.avg_seeders = releases.iter().map(|r| r.seeders as f64).sum::<f64>() / count;
        metrics.avg_leechers = releases.iter().map(|r| r.leechers as f64).sum::<f64>() / count;
        metrics.avg_size_gb = releases.iter().map(|r| r.size_gb).sum::<f64>() / count;
        metrics.avg_completion_rate = releases.iter().map(|r| r.completion_rate).sum::<f64>() / count;
        
        // Calculate ratios
        metrics.seeder_leecher_ratio = if metrics.avg_leechers > 0.0 {
            metrics.avg_seeders / metrics.avg_leechers
        } else {
            metrics.avg_seeders
        };
        
        metrics.internal_ratio = metrics.internal_releases as f64 / metrics.total_releases as f64;
        
        // Quality consistency (based on size variance)
        let size_variance = releases.iter()
            .map(|r| (r.size_gb - metrics.avg_size_gb).powi(2))
            .sum::<f64>() / count;
        metrics.quality_consistency = 1.0 / (1.0 + size_variance);
        
        // Recency score
        let now = Utc::now();
        let days_since_last = now.signed_duration_since(metrics.last_seen).num_days() as f64;
        metrics.recency_score = 1.0 / (1.0 + days_since_last / 30.0);
        
        // Seeder health score
        metrics.seeder_health_score = (metrics.avg_seeders / 10.0).min(1.0);
        
        // Overall reputation score
        metrics.reputation_score = Self::calculate_reputation_score_static(metrics);
        metrics.comprehensive_reputation_score = metrics.reputation_score;
        
        // Assign quality tier based on reputation score
        metrics.quality_tier = match metrics.reputation_score {
            score if score >= 80.0 => "Premium".to_string(),
            score if score >= 70.0 => "High Quality".to_string(),
            score if score >= 60.0 => "Standard".to_string(),
            score if score >= 40.0 => "Low Quality".to_string(),
            _ => "Poor".to_string(),
        };
        
        metrics.evidence_based_tier = metrics.quality_tier.clone();
    }
    
    /// Calculate weighted reputation score
    fn calculate_reputation_score_static(metrics: &SceneGroupMetrics) -> f64 {
        let seeder_score = (metrics.seeder_leecher_ratio / 10.0).min(1.0);
        let internal_score = metrics.internal_ratio;
        let completion_score = metrics.avg_completion_rate.min(1.0);
        let consistency_score = metrics.quality_consistency;
        let recency_score = metrics.recency_score;
        let volume_score = (metrics.total_releases as f64 / 100.0).min(1.0);
        let health_score = metrics.seeder_health_score;
        
        // Weighted combination
        let weighted_score = 
            0.25 * seeder_score +
            0.20 * internal_score +
            0.15 * completion_score +
            0.15 * consistency_score +
            0.10 * recency_score +
            0.10 * volume_score +
            0.05 * health_score;
        
        weighted_score * 100.0
    }
    
    // Helper functions for category/codec/medium names
    fn get_category_name(&self, id: u32) -> String {
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
    
    fn get_codec_name(&self, id: u32) -> String {
        match id {
            1 => "H.264".to_string(),
            2 => "MPEG-2".to_string(),
            3 => "VC-1".to_string(),
            4 => "XviD".to_string(),
            5 => "HEVC".to_string(),
            _ => "Unknown".to_string(),
        }
    }
    
    fn get_medium_name(&self, id: u32) -> String {
        match id {
            1 => "Blu-ray".to_string(),
            3 => "Encode".to_string(),
            4 => "Capture".to_string(),
            5 => "Remux".to_string(),
            6 => "WEB-DL".to_string(),
            _ => "Unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_analyzer_creation() {
        let config = HDBitsSessionConfig::default();
        let analyzer = HDBitsSessionAnalyzer::new(config);
        
        assert_eq!(analyzer.scene_groups.len(), 0);
        assert_eq!(analyzer.releases.len(), 0);
        assert!(!analyzer.authenticated);
    }
    
    #[tokio::test]
    async fn test_scene_group_analysis() {
        let config = HDBitsSessionConfig::default();
        let mut analyzer = HDBitsSessionAnalyzer::new(config);
        
        // Create some test torrents
        let test_torrent = HDBitsTorrent {
            id: 1,
            hash: "test_hash".to_string(),
            name: "Test.Movie.2024.1080p.BluRay.x264-TESTGROUP".to_string(),
            size: 1024 * 1024 * 1024 * 20, // 20GB
            times_completed: 15,
            seeders: 10,
            leechers: 2,
            type_category: 1, // Movie
            type_codec: 1,    // H.264
            type_medium: 1,   // Blu-ray
            type_origin: 1,   // Internal
            added: "2024-01-01T12:00:00+0000".to_string(),
            utadded: Some(1704110400),
            descr: Some("Test description".to_string()),
            comments: Some(0),
            numfiles: Some(1),
            filename: Some("test.mkv".to_string()),
            type_exclusive: Some(0),
            freeleech: "no".to_string(),
            torrent_status: Some("active".to_string()),
            bookmarked: Some(0),
            wishlisted: Some(0),
            tags: None,
            username: Some("test_user".to_string()),
            owner: Some(1),
            imdb: None,
            tvdb: None,
        };
        
        let torrents = vec![test_torrent];
        
        // Analyze the torrents
        let result = analyzer.analyze_scene_groups(torrents);
        assert!(result.is_ok());
        
        // Verify scene group was extracted and analyzed
        assert_eq!(analyzer.scene_groups.len(), 1);
        assert_eq!(analyzer.releases.len(), 1);
        
        let scene_group = analyzer.scene_groups.get("TESTGROUP").unwrap();
        assert_eq!(scene_group.group_name, "TESTGROUP");
        assert_eq!(scene_group.total_releases, 1);
        assert_eq!(scene_group.internal_releases, 1);
        assert!(scene_group.reputation_score > 0.0);
    }
    
    #[test]
    fn test_comprehensive_report_generation() {
        let config = HDBitsSessionConfig::default();
        let analyzer = HDBitsSessionAnalyzer::new(config);
        
        let start_time = Utc::now() - chrono::Duration::hours(1);
        let report = analyzer.generate_comprehensive_report(start_time);
        
        assert_eq!(report.total_torrents_analyzed, 0);
        assert_eq!(report.unique_scene_groups, 0);
        assert_eq!(report.internal_releases_analyzed, 0);
        assert_eq!(report.session_status, "Not Authenticated");
    }
}