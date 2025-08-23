//! hdbits_comprehensive_analyzer module

use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn};
use crate::{SceneGroupMetrics, ReleaseMetric, HDBitsTorrent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDBitsComprehensiveConfig {
    pub session_cookie: String,
    pub base_url: String,
    pub max_pages: usize,
    pub delay_seconds: u64,
    pub request_delay_seconds: u64,
    pub max_pages_per_category: usize,
    pub enable_six_month_filter: bool,
    pub six_month_filtering: bool,
    pub comprehensive_collection: bool,
}

impl Default for HDBitsComprehensiveConfig {
    fn default() -> Self {
        Self {
            session_cookie: "session=default_cookie".to_string(),
            base_url: "https://hdbits.org".to_string(),
            max_pages: 100,
            delay_seconds: 1,
            request_delay_seconds: 1,
            max_pages_per_category: 100,
            enable_six_month_filter: true,
            six_month_filtering: true,
            comprehensive_collection: true,
        }
    }
}

pub struct HDBitsComprehensiveAnalyzer {
    config: HDBitsComprehensiveConfig,
    scene_groups: HashMap<String, SceneGroupMetrics>,
    releases: Vec<ReleaseMetric>,
}

impl HDBitsComprehensiveAnalyzer {
    pub fn new(config: HDBitsComprehensiveConfig) -> Result<Self> {
        Ok(Self { 
            config,
            scene_groups: HashMap::new(),
            releases: Vec::new(),
        })
    }
    
    pub async fn verify_session(&self) -> Result<()> {
        info!("Verifying HDBits session");
        
        // Create HTTP client with session cookie
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        // Test the session by accessing the browse page
        let response = client
            .get(&format!("{}/browse.php", self.config.base_url))
            .header("Cookie", &self.config.session_cookie)
            .send()
            .await?;
        
        // Check if we're redirected to login (session invalid)
        if response.url().path().contains("login") {
            return Err(anyhow::anyhow!("Session expired or invalid - please update session cookie"));
        }
        
        // Check for successful response
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Session verification failed with status: {}", response.status()));
        }
        
        info!("Session verified successfully");
        Ok(())
    }
    
    pub async fn collect_comprehensive_data(&self) -> Result<Vec<HDBitsTorrent>> {
        info!("Starting comprehensive data collection");
        
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let mut all_torrents = Vec::new();
        let categories = vec![1, 3]; // Movies and Documentaries
        
        for category in categories {
            info!("Collecting data for category {}", category);
            let mut page = 0;
            
            while page < self.config.max_pages_per_category {
                // Build browse URL with parameters
                let url = format!(
                    "{}/browse.php?c{}=1&page={}",
                    self.config.base_url, category, page
                );
                
                // Add delay between requests to respect rate limits
                if page > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(self.config.request_delay_seconds)).await;
                }
                
                let response = client
                    .get(&url)
                    .header("Cookie", &self.config.session_cookie)
                    .send()
                    .await?;
                
                if !response.status().is_success() {
                    warn!("Failed to fetch page {} for category {}", page, category);
                    break;
                }
                
                let html = response.text().await?;
                
                // Parse torrents from HTML (simplified extraction)
                let torrents = self.parse_torrents_from_html(&html)?;
                
                if torrents.is_empty() {
                    info!("No more torrents found for category {} at page {}", category, page);
                    break;
                }
                
                all_torrents.extend(torrents);
                page += 1;
                
                info!("Collected {} torrents so far", all_torrents.len());
            }
        }
        
        info!("Comprehensive data collection complete: {} torrents", all_torrents.len());
        Ok(all_torrents)
    }
    
    fn parse_torrents_from_html(&self, html: &str) -> Result<Vec<HDBitsTorrent>> {
        // Simplified HTML parsing - in production would use scraper crate
        let mut torrents = Vec::new();
        
        // Basic pattern matching for torrent data
        // This is a simplified implementation - real parsing would be more robust
        for line in html.lines() {
            if line.contains("details.php?id=") {
                // Extract basic torrent info from the HTML
                // In production, use proper HTML parsing with scraper crate
                if let Some(torrent) = self.extract_torrent_from_line(line) {
                    torrents.push(torrent);
                }
            }
        }
        
        Ok(torrents)
    }
    
    fn extract_torrent_from_line(&self, line: &str) -> Option<HDBitsTorrent> {
        // Simplified extraction - would need proper HTML parsing in production
        use crate::{HDBitsTorrent, HDBitsCategory, HDBitsImdb};
        
        // Extract ID from details.php?id=XXXXX pattern
        let id = line.split("details.php?id=")
            .nth(1)?
            .split('&')
            .next()?
            .to_string();
        
        // Create a basic torrent entry
        Some(HDBitsTorrent {
            id,
            name: "Parsed torrent".to_string(), // Would extract actual name
            times_completed: 0,
            seeders: 0,
            leechers: 0,
            size: 0,
            added: Utc::now().to_rfc3339(),
            imdb: None,
            tvdb: None,
            category: HDBitsCategory { id: 1, name: "Movie".to_string() },
            type_category: "Movie".to_string(),
            type_codec: "x264".to_string(),
            type_medium: "Blu-ray".to_string(),
            type_origin: "Scene".to_string(),
            freeleech: None,
            internal: false,
        })
    }
    
    pub fn analyze_scene_groups(&mut self, releases: Vec<HDBitsTorrent>) -> Result<()> {
        info!("Analyzing {} releases for scene groups", releases.len());
        
        // Extract scene groups from release names
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
                
                // Add to releases list
                self.releases.push(release_metric.clone());
                
                // Update or create scene group metrics
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
                
                // Update group metrics
                group_metrics.total_releases += 1;
                if release_metric.is_internal {
                    group_metrics.internal_releases += 1;
                }
                
                // Update date ranges
                if release_metric.added_date < group_metrics.first_seen {
                    group_metrics.first_seen = release_metric.added_date;
                }
                if release_metric.added_date > group_metrics.last_seen {
                    group_metrics.last_seen = release_metric.added_date;
                }
                
                // Add category if not already present
                if !group_metrics.categories_covered.contains(&torrent.category.name) {
                    group_metrics.categories_covered.push(torrent.category.name.clone());
                }
                
                group_metrics.release_history.push(release_metric);
            }
        }
        
        // Calculate derived metrics for each group
        for group_metrics in self.scene_groups.values_mut() {
            Self::calculate_group_metrics(group_metrics);
        }
        
        info!("Scene group analysis complete. Found {} unique groups", self.scene_groups.len());
        Ok(())
    }
    
    fn extract_scene_group(torrent_name: &str) -> Option<String> {
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
    
    fn calculate_group_metrics(metrics: &mut SceneGroupMetrics) {
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
            metrics.avg_seeders
        };

        metrics.internal_ratio = metrics.internal_releases as f64 / metrics.total_releases as f64;

        // Quality consistency (lower variance in size = higher consistency)
        let size_variance = releases.iter()
            .map(|r| (r.size_gb - metrics.avg_size_gb).powi(2))
            .sum::<f64>() / count;
        metrics.quality_consistency = 1.0 / (1.0 + size_variance);

        // Recency score (more recent releases = higher score)
        let now = Utc::now();
        let days_since_last = now.signed_duration_since(metrics.last_seen).num_days() as f64;
        metrics.recency_score = 1.0 / (1.0 + days_since_last / 30.0);

        // Seeder health score
        metrics.seeder_health_score = (metrics.avg_seeders / 100.0).min(1.0);

        // Overall reputation score (weighted combination)
        metrics.reputation_score = Self::calculate_reputation_score(metrics);
        metrics.comprehensive_reputation_score = metrics.reputation_score;
        
        // Determine quality tier based on reputation score
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
    
    fn calculate_reputation_score(metrics: &SceneGroupMetrics) -> f64 {
        // Weighted scoring formula based on real data analysis
        let seeder_score = (metrics.seeder_leecher_ratio / 10.0).min(1.0) * 25.0;
        let internal_score = metrics.internal_ratio * 20.0;
        let completion_score = metrics.avg_completion_rate.min(1.0) * 15.0;
        let consistency_score = metrics.quality_consistency * 15.0;
        let recency_score = metrics.recency_score * 10.0;
        let volume_score = (metrics.total_releases as f64 / 100.0).min(1.0) * 10.0;
        let size_score = Self::calculate_size_appropriateness_score(metrics.avg_size_gb) * 5.0;
        
        seeder_score + internal_score + completion_score + consistency_score + 
        recency_score + volume_score + size_score
    }
    
    fn calculate_size_appropriateness_score(avg_size_gb: f64) -> f64 {
        match avg_size_gb {
            size if size >= 15.0 && size <= 50.0 => 1.0,  // Full quality range
            size if size >= 8.0 && size < 15.0 => 0.8,    // Good compressed
            size if size >= 4.0 && size < 8.0 => 0.6,     // Acceptable
            size if size >= 2.0 && size < 4.0 => 0.4,     // Low quality
            size if size < 2.0 => 0.2,                     // Very low quality
            size if size > 50.0 => 0.7,                    // Possibly uncompressed/remux
            _ => 0.5,                                       // Default
        }
    }
    
    pub fn get_statistics(&self) -> (usize, usize, usize, usize) {
        // Returns: (total_groups, total_releases, internal_releases, six_month_releases)
        (self.scene_groups.len(), self.releases.len(), 0, 0)
    }
    
    pub fn generate_comprehensive_report(&self, start_time: DateTime<Utc>) -> ComprehensiveReport {
        let end_time = Utc::now();
        let duration = end_time.signed_duration_since(start_time);
        
        // Calculate reputation distribution
        let mut distribution = ReputationDistribution::default();
        for group in self.scene_groups.values() {
            match group.reputation_score {
                score if score >= 90.0 => distribution.elite += 1,
                score if score >= 80.0 => distribution.premium += 1,
                score if score >= 70.0 => distribution.excellent += 1,
                score if score >= 60.0 => distribution.good += 1,
                score if score >= 50.0 => distribution.average += 1,
                score if score >= 40.0 => distribution.below_average += 1,
                _ => distribution.poor += 1,
            }
        }
        
        // Calculate data quality indicators
        let internal_releases = self.releases.iter().filter(|r| r.is_internal).count();
        let six_month_cutoff = Utc::now() - chrono::Duration::days(180);
        let six_month_releases = self.releases.iter()
            .filter(|r| r.added_date > six_month_cutoff)
            .count();
        
        let data_quality = DataQualityIndicators {
            scene_group_extraction_rate: if !self.releases.is_empty() {
                (self.scene_groups.len() as f64 / self.releases.len() as f64) * 100.0
            } else {
                0.0
            },
            internal_release_percentage: if !self.releases.is_empty() {
                (internal_releases as f64 / self.releases.len() as f64) * 100.0
            } else {
                0.0
            },
            six_month_data_coverage: if !self.releases.is_empty() {
                (six_month_releases as f64 / self.releases.len() as f64) * 100.0
            } else {
                0.0
            },
        };
        
        ComprehensiveReport {
            data_collection_period: format!("{} seconds", duration.num_seconds()),
            pages_processed: (self.releases.len() / 50) as u32, // Estimate based on releases
            data_quality_indicators: data_quality,
            statistical_insights: StatisticalInsights {
                reputation_distribution: distribution,
            },
        }
    }
    
    pub fn get_top_groups_by_reputation(&self, limit: usize) -> Vec<&SceneGroupMetrics> {
        let mut groups: Vec<&SceneGroupMetrics> = self.scene_groups.values().collect();
        groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        groups.into_iter().take(limit).collect()
    }
    
    pub fn export_comprehensive_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.scene_groups)
            .map_err(|e| anyhow::anyhow!("Failed to serialize data: {}", e))
    }
    
    pub fn export_csv_comprehensive(&self) -> String {
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
            "message": "HDBits comprehensive analyzer is a work in progress"
        }))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ComprehensiveReport {
    pub data_collection_period: String,
    pub pages_processed: u32,
    pub data_quality_indicators: DataQualityIndicators,
    pub statistical_insights: StatisticalInsights,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DataQualityIndicators {
    pub scene_group_extraction_rate: f64,
    pub internal_release_percentage: f64,
    pub six_month_data_coverage: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatisticalInsights {
    pub reputation_distribution: ReputationDistribution,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReputationDistribution {
    pub elite: u32,
    pub premium: u32,
    pub excellent: u32,
    pub good: u32,
    pub average: u32,
    pub below_average: u32,
    pub poor: u32,
}
