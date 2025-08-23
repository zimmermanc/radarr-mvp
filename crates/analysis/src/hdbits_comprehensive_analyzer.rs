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
        
        // Use advanced filters for high-quality content
        // Categories: c1=1 (Movies), c3=1 (Documentaries)
        // Codecs: co1=1 (H.264), co5=1 (x264), co2=1 (Xvid), co3=1 (MPEG2)
        // Media: m1=1 (Blu-ray), m4=1 (HDTV), m3=1 (WEB-DL), m5=1 (Encode), m6=1 (Capture)
        // Language: English only
        let base_filters = "c1=1&co1=1&co5=1&co2=1&co3=1&m1=1&m4=1&m3=1&m5=1&m6=1&descriptions=0&season_packs=0&selected_languages%5B%5D=English&languagesearchtype=showonly";
        
        info!("Collecting data with advanced quality filters");
        let mut page = 0;
        
        while page < self.config.max_pages_per_category {
            // Build browse URL with advanced filters
            let url = format!(
                "{}/browse.php?{}&page={}",
                self.config.base_url, base_filters, page
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
                    warn!("Failed to fetch page {}", page);
                    break;
                }
                
                let html = response.text().await?;
                
                // Parse torrents from HTML (simplified extraction)
                let torrents = self.parse_torrents_from_html(&html)?;
                
                if torrents.is_empty() {
                    info!("No more torrents found at page {}", page);
                    break;
                }
                
                all_torrents.extend(torrents);
                page += 1;
                
                info!("Collected {} torrents so far", all_torrents.len());
            }
        
        info!("Comprehensive data collection complete: {} torrents", all_torrents.len());
        Ok(all_torrents)
    }
    
    fn parse_torrents_from_html(&self, html: &str) -> Result<Vec<HDBitsTorrent>> {
        // Simplified HTML parsing - in production would use scraper crate
        let mut torrents = Vec::new();
        
        // Parse complete table rows for torrent data
        let mut in_row = false;
        let mut current_row = String::new();
        
        for line in html.lines() {
            // Check for start of torrent row
            if line.contains("<tr class=\"t_row\">") {
                in_row = true;
                current_row = line.to_string();
            } else if in_row {
                current_row.push_str(line);
                
                // Check for end of row
                if line.contains("</tr>") {
                    in_row = false;
                    
                    // Parse the complete row
                    if let Some(torrent) = self.parse_torrent_row(&current_row) {
                        torrents.push(torrent);
                    }
                    current_row.clear();
                }
            } else if line.contains("details.php?id=") && line.contains("href=\"/details.php") {
                // Fallback: single-line parsing for simple format
                if let Some(torrent) = self.extract_torrent_from_line(line) {
                    torrents.push(torrent);
                }
            }
        }
        
        Ok(torrents)
    }
    
    fn parse_torrent_row(&self, row: &str) -> Option<HDBitsTorrent> {
        // Parse complete table row data
        if !row.contains("class=\"t_row\"") {
            return self.extract_torrent_from_line(row); // Fallback for old format
        }
        
        // Extract all cells from the row
        let cells: Vec<&str> = row.split("<td").collect();
        if cells.len() < 9 {
            return None;
        }
        
        // Parse title cell (index 2)
        let title_cell = cells.get(2)?;
        let (id, name, is_exclusive) = self.parse_title_cell(title_cell)?;
        
        // Parse size cell (index 4)
        let size_bytes = if let Some(size_cell) = cells.get(4) {
            self.parse_size(size_cell)
        } else {
            0
        };
        
        // Parse snatched cell (index 5)
        let snatched = if let Some(snatched_cell) = cells.get(5) {
            self.parse_number(snatched_cell)
        } else {
            0
        };
        
        // Parse seeders cell (index 6)
        let seeders = if let Some(seeders_cell) = cells.get(6) {
            self.parse_number(seeders_cell)
        } else {
            0
        };
        
        // Parse leechers cell (index 7)
        let leechers = if let Some(leechers_cell) = cells.get(7) {
            self.parse_number(leechers_cell)
        } else {
            0
        };
        
        // Parse time alive cell (index 8)
        let added = if let Some(time_cell) = cells.get(8) {
            self.parse_time_alive(time_cell)
        } else {
            Utc::now().to_rfc3339()
        };
        
        Some(HDBitsTorrent {
            id: id.clone(),
            name,
            times_completed: snatched as i32,
            seeders: seeders as i32,
            leechers: leechers as i32,
            size: size_bytes as i64,
            added,
            imdb: None,
            tvdb: None,
            category: HDBitsCategory { id: 1, name: "Movie".to_string() },
            type_category: "Movie".to_string(),
            type_codec: self.detect_codec(&id),
            type_medium: self.detect_medium(&id),
            type_origin: if is_exclusive { "Internal".to_string() } else { "Scene".to_string() },
            freeleech: None,
            internal: is_exclusive,
        })
    }
    
    fn parse_title_cell(&self, cell: &str) -> Option<(String, String, bool)> {
        // Extract ID from details.php?id=XXXXX
        let id = cell.split("details.php?id=")
            .nth(1)?
            .split('&')
            .next()?
            .to_string();
        
        // Extract name from link text (between > and </a>)
        let name = if let Some(start) = cell.find(">") {
            let text_start = &cell[start + 1..];
            if let Some(end) = text_start.find("</a>") {
                text_start[..end].to_string()
            } else {
                format!("Torrent_{}", id)
            }
        } else {
            format!("Torrent_{}", id)
        };
        
        // Check for exclusive/internal tag
        let is_exclusive = cell.contains("class=\"tag exclusive\"") || 
                          cell.contains(">Exclusive<");
        
        Some((id, name, is_exclusive))
    }
    
    fn parse_size(&self, cell: &str) -> u64 {
        // Extract size like "4.42 GB" and convert to bytes
        if let Some(start) = cell.find(">") {
            let content = &cell[start + 1..];
            if let Some(end) = content.find("<") {
                let size_str = &content[..end];
                return self.parse_size_string(size_str);
            }
        }
        0
    }
    
    fn parse_size_string(&self, size_str: &str) -> u64 {
        let parts: Vec<&str> = size_str.trim().split_whitespace().collect();
        if parts.len() != 2 {
            return 0;
        }
        
        let value: f64 = parts[0].parse().unwrap_or(0.0);
        let multiplier = match parts[1].to_uppercase().as_str() {
            "TB" => 1_099_511_627_776.0,
            "GB" => 1_073_741_824.0,
            "MB" => 1_048_576.0,
            "KB" => 1_024.0,
            _ => 1.0,
        };
        
        (value * multiplier) as u64
    }
    
    fn parse_number(&self, cell: &str) -> u32 {
        // Extract number from cell content
        if let Some(start) = cell.find(">") {
            let content = &cell[start + 1..];
            if let Some(end) = content.find("<") {
                let num_str = &content[..end];
                return num_str.trim().parse().unwrap_or(0);
            }
        }
        0
    }
    
    fn parse_time_alive(&self, cell: &str) -> String {
        // Parse time like "3 years<br />1 month" and convert to timestamp
        // For now, just return current time minus approximation
        // In production, parse properly
        Utc::now().to_rfc3339()
    }
    
    fn detect_codec(&self, name: &str) -> String {
        if name.contains("x265") || name.contains("HEVC") {
            "x265".to_string()
        } else if name.contains("x264") || name.contains("H.264") {
            "x264".to_string()
        } else if name.contains("XviD") {
            "XviD".to_string()
        } else {
            "x264".to_string() // Default
        }
    }
    
    fn detect_medium(&self, name: &str) -> String {
        if name.contains("BluRay") || name.contains("Blu-ray") || name.contains("BD") {
            "Blu-ray".to_string()
        } else if name.contains("WEB-DL") || name.contains("WEBDL") {
            "WEB-DL".to_string()
        } else if name.contains("WEBRip") || name.contains("WEB-Rip") {
            "WEBRip".to_string()
        } else if name.contains("HDTV") {
            "HDTV".to_string()
        } else if name.contains("DVDRip") || name.contains("DVD") {
            "DVD".to_string()
        } else {
            "Unknown".to_string()
        }
    }
    
    fn extract_torrent_from_line(&self, line: &str) -> Option<HDBitsTorrent> {
        // Simplified extraction - would need proper HTML parsing in production
        use crate::{HDBitsTorrent, HDBitsCategory};
        
        // Extract ID from details.php?id=XXXXX pattern
        let id = line.split("details.php?id=")
            .nth(1)?
            .split('&')
            .next()?
            .to_string();
        
        // Extract torrent name from link text (NOT from title attribute which is the FL tooltip)
        // The actual torrent name is between > and </a>
        let name = if let Some(link_start) = line.find("details.php?id=") {
            // Find the end of the opening <a> tag
            let remaining = &line[link_start..];
            if let Some(text_start) = remaining.find('>') {
                let text_content = &remaining[text_start + 1..];
                if let Some(text_end) = text_content.find("</a>") {
                    text_content[..text_end].to_string()
                } else {
                    format!("Torrent_{}", id)
                }
            } else {
                format!("Torrent_{}", id)
            }
        } else {
            format!("Torrent_{}", id)
        };
        
        // Check if this is an exclusive/internal release
        let is_exclusive = line.contains("class=\"tag exclusive\"") || 
                          line.contains("class=\" exclusive") ||
                          line.contains(">Exclusive<");
        
        // Create a basic torrent entry
        Some(HDBitsTorrent {
            id: id.clone(),
            name,
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
            internal: is_exclusive,
        })
    }
    
    pub fn analyze_scene_groups(&mut self, releases: Vec<HDBitsTorrent>) -> Result<()> {
        info!("Analyzing {} releases for scene groups", releases.len());
        
        // Log a sample of release names for debugging
        if !releases.is_empty() {
            info!("Sample release names:");
            for (i, torrent) in releases.iter().take(5).enumerate() {
                info!("  {}: {}", i + 1, torrent.name);
            }
        }
        
        // Extract scene groups from release names
        for torrent in releases {
            // Try to extract scene group from name, or check if it's exclusive
            let group_name = if let Some(group) = Self::extract_scene_group(&torrent.name) {
                Some(group)
            } else if torrent.internal {
                // If no group found and it's marked as internal/exclusive, use "EXCLUSIVE"
                Some("EXCLUSIVE".to_string())
            } else {
                None
            };
            
            if let Some(group_name) = group_name {
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
                        if !["X264", "X265", "H264", "H265", "HEVC", "AVC", "AAC", "AC3", "DTS", "BLURAY", "WEB", "HDTV", "MA", "1", "0", "5"].contains(&group_name.as_str()) {
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
