// HDBits Browse Interface Scene Group Analysis
// Safe data collection using browse interface with strict rate limiting

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};
use url::Url;

#[derive(Debug, Clone)]
pub struct HDBitsBrowseConfig {
    pub username: String,
    pub passkey: String,
    pub base_url: String,
    pub rate_limit_seconds: u64, // Conservative rate limiting
}

impl Default for HDBitsBrowseConfig {
    fn default() -> Self {
        Self {
            username: "blargdiesel".to_string(),
            passkey: "ed487790cd0dee98941ab5c132179bd2c8c5e23622c0c04a800ad543cde2990cd44ed960892d990214ea1618bf29780386a77246a21dc636d83420e077e69863".to_string(),
            base_url: "https://hdbits.org".to_string(),
            rate_limit_seconds: 30, // 30 second delays between requests
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowseRelease {
    pub id: String,
    pub name: String,
    pub seeders: u32,
    pub leechers: u32,
    pub completed: u32,
    pub size_gb: f64,
    pub added_date: DateTime<Utc>,
    pub is_internal: bool,
    pub category: String,
    pub codec: String,
    pub quality: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGroupAnalysis {
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
    pub confidence_level: String,
    pub quality_tier: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub releases: Vec<BrowseRelease>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowseAnalysisReport {
    pub generated_at: DateTime<Utc>,
    pub total_releases_analyzed: u32,
    pub unique_scene_groups: u32,
    pub internal_releases: u32,
    pub external_releases: u32,
    pub collection_duration_minutes: u64,
    pub top_groups: Vec<SceneGroupSummary>,
    pub statistical_summary: BrowseStatSummary,
    pub quality_distribution: QualityDistribution,
    pub data_collection_notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneGroupSummary {
    pub group_name: String,
    pub reputation_score: f64,
    pub quality_tier: String,
    pub total_releases: u32,
    pub internal_ratio: f64,
    pub avg_seeders: f64,
    pub confidence_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowseStatSummary {
    pub reputation_scores: StatRange,
    pub seeder_counts: StatRange,
    pub file_sizes_gb: StatRange,
    pub internal_ratios: StatRange,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatRange {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QualityDistribution {
    pub premium: u32,    // 85-100
    pub excellent: u32,  // 75-84
    pub good: u32,       // 65-74
    pub average: u32,    // 50-64
    pub below_average: u32, // 35-49
    pub poor: u32,       // 0-34
}

pub struct HDBitsBrowseAnalyzer {
    client: Client,
    config: HDBitsBrowseConfig,
    requests_made: u32,
    last_request_time: DateTime<Utc>,
    scene_groups: HashMap<String, SceneGroupAnalysis>,
}

impl HDBitsBrowseAnalyzer {
    pub fn new(config: HDBitsBrowseConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .cookie_store(true)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            config,
            requests_made: 0,
            last_request_time: Utc::now(),
            scene_groups: HashMap::new(),
        }
    }

    async fn rate_limit_delay(&mut self) -> Result<()> {
        let now = Utc::now();
        let time_since_last = now.signed_duration_since(self.last_request_time);
        
        if time_since_last.num_seconds() < self.config.rate_limit_seconds as i64 {
            let wait_time = self.config.rate_limit_seconds - time_since_last.num_seconds() as u64;
            info!("Rate limiting: waiting {} seconds before next request", wait_time);
            sleep(Duration::from_secs(wait_time)).await;
        }

        self.requests_made += 1;
        self.last_request_time = Utc::now();
        
        info!("Request #{} - respecting {} second rate limit", self.requests_made, self.config.rate_limit_seconds);
        Ok(())
    }

    pub async fn login(&mut self) -> Result<()> {
        info!("Authenticating with HDBits using provided credentials");
        
        let login_url = format!("{}/login.php", self.config.base_url);
        let username = self.config.username.clone();
        let passkey = self.config.passkey.clone();
        let login_data = [
            ("username", username.as_str()),
            ("password", passkey.as_str()),
            ("login", "Login"),
        ];

        self.rate_limit_delay().await?;
        
        let response = self.client
            .post(&login_url)
            .form(&login_data)
            .send()
            .await
            .context("Failed to send login request")?;

        if response.status().is_success() {
            info!("Successfully authenticated with HDBits");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Login failed with status: {}", response.status()))
        }
    }

    pub async fn collect_internal_releases(&mut self) -> Result<Vec<BrowseRelease>> {
        info!("Starting safe data collection from HDBits browse interface");
        
        let mut all_releases = Vec::new();
        let mut notes = Vec::new();
        
        // Focus on movie categories with internal filter
        let browse_params = [
            ("c1", "1"),  // Movies
            ("co1", "1"), // H.264
            ("co2", "1"), // MPEG-2 
            ("co3", "1"), // x265/HEVC
            ("m1", "1"),  // Blu-ray
            ("m3", "1"),  // Encode
            ("m4", "1"),  // Capture
            ("m6", "1"),  // WEB-DL
            ("origin", "1"), // Internal only
            ("order", "added"),
            ("sort", "desc"),
        ];

        // Collect from multiple pages with strict rate limiting
        for page in 0..5 { // Limited pages to respect community
            info!("Collecting page {} with {} second delay", page + 1, self.config.rate_limit_seconds);
            
            let mut url = Url::parse(&format!("{}/browse.php", self.config.base_url))?;
            for (key, value) in &browse_params {
                url.query_pairs_mut().append_pair(key, value);
            }
            url.query_pairs_mut().append_pair("page", &page.to_string());

            self.rate_limit_delay().await?;
            
            let response = self.client
                .get(url.as_str())
                .send()
                .await
                .context("Failed to fetch browse page")?;

            if !response.status().is_success() {
                warn!("Browse page {} returned status: {}", page, response.status());
                notes.push(format!("Page {} failed with status {}", page, response.status()));
                continue;
            }

            let html = response.text().await.context("Failed to read response body")?;
            let releases = self.parse_browse_page(&html)?;
            
            if releases.is_empty() {
                info!("No more releases found on page {}, stopping collection", page + 1);
                break;
            }

            info!("Extracted {} releases from page {}", releases.len(), page + 1);
            all_releases.extend(releases);
            
            // Additional safety delay between pages
            if page < 4 {
                info!("Additional safety delay before next page");
                sleep(Duration::from_secs(5)).await;
            }
        }

        info!("Collection complete: {} total releases from {} pages", 
              all_releases.len(), 
              (all_releases.len() / 50).min(5)); // Estimate pages
        
        Ok(all_releases)
    }

    fn parse_browse_page(&self, html: &str) -> Result<Vec<BrowseRelease>> {
        let document = Html::parse_document(html);
        let mut releases = Vec::new();
        
        // HDBits browse table selectors (these may need adjustment based on actual HTML structure)
        let row_selector = Selector::parse("table.mainblockcontenttt tr")
            .map_err(|e| anyhow::anyhow!("Invalid row selector: {:?}", e))?;
        let name_selector = Selector::parse("td.mainblockcontent a[href*='details.php']")
            .map_err(|e| anyhow::anyhow!("Invalid name selector: {:?}", e))?;
        let stats_selector = Selector::parse("td.mainblockcontent")
            .map_err(|e| anyhow::anyhow!("Invalid stats selector: {:?}", e))?;

        for row in document.select(&row_selector) {
            if let Some(name_element) = row.select(&name_selector).next() {
                let name = name_element.text().collect::<String>().trim().to_string();
                
                if name.is_empty() {
                    continue;
                }

                let stats: Vec<String> = row.select(&stats_selector)
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .collect();

                // Parse the extracted data (indices may need adjustment)
                let release = BrowseRelease {
                    id: self.extract_id_from_name(&name),
                    name: name.clone(),
                    seeders: self.parse_number_or_default(&stats.get(5).unwrap_or(&"0".to_string()), 0),
                    leechers: self.parse_number_or_default(&stats.get(6).unwrap_or(&"0".to_string()), 0),
                    completed: self.parse_number_or_default(&stats.get(7).unwrap_or(&"0".to_string()), 0),
                    size_gb: self.parse_size_to_gb(&stats.get(4).unwrap_or(&"0 GB".to_string())),
                    added_date: self.parse_date(&stats.get(8).unwrap_or(&"".to_string())).unwrap_or_else(|| Utc::now()),
                    is_internal: true, // We're only collecting internal releases
                    category: "Movie".to_string(),
                    codec: self.extract_codec_from_name(&name),
                    quality: self.extract_quality_from_name(&name),
                    source: self.extract_source_from_name(&name),
                };

                releases.push(release);
            }
        }

        debug!("Parsed {} releases from HTML page", releases.len());
        Ok(releases)
    }

    fn extract_id_from_name(&self, name: &str) -> String {
        // Generate a simple hash-based ID from the name
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn parse_number_or_default(&self, text: &str, default: u32) -> u32 {
        text.chars().filter(|c| c.is_numeric()).collect::<String>()
            .parse().unwrap_or(default)
    }

    fn parse_size_to_gb(&self, size_text: &str) -> f64 {
        let size_str = size_text.to_uppercase();
        let number: f64 = size_str.chars().filter(|c| c.is_numeric() || *c == '.')
            .collect::<String>().parse().unwrap_or(0.0);
        
        if size_str.contains("TB") {
            number * 1024.0
        } else if size_str.contains("GB") {
            number
        } else if size_str.contains("MB") {
            number / 1024.0
        } else {
            number // Assume GB
        }
    }

    fn parse_date(&self, date_text: &str) -> Option<DateTime<Utc>> {
        // Parse HDBits date format (may need adjustment)
        chrono::NaiveDateTime::parse_from_str(date_text, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc())
    }

    fn extract_codec_from_name(&self, name: &str) -> String {
        let name_upper = name.to_uppercase();
        if name_upper.contains("X265") || name_upper.contains("HEVC") {
            "x265".to_string()
        } else if name_upper.contains("X264") || name_upper.contains("AVC") {
            "x264".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn extract_quality_from_name(&self, name: &str) -> String {
        let name_upper = name.to_uppercase();
        if name_upper.contains("2160P") || name_upper.contains("4K") {
            "2160p".to_string()
        } else if name_upper.contains("1080P") {
            "1080p".to_string()
        } else if name_upper.contains("720P") {
            "720p".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn extract_source_from_name(&self, name: &str) -> String {
        let name_upper = name.to_uppercase();
        if name_upper.contains("BLURAY") || name_upper.contains("BDR") {
            "Blu-ray".to_string()
        } else if name_upper.contains("WEB") {
            "WEB".to_string()
        } else if name_upper.contains("HDTV") {
            "HDTV".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    pub fn extract_scene_group(&self, release_name: &str) -> Option<String> {
        // Enhanced scene group extraction patterns
        let patterns = [
            r"-([A-Za-z0-9]+)$",              // Standard: -GROUP
            r"\.([A-Za-z0-9]+)$",             // Dot: .GROUP
            r"\[([A-Za-z0-9]+)\]$",           // Brackets: [GROUP]
            r"\(([A-Za-z0-9]+)\)$",           // Parentheses: (GROUP)
            r"\s([A-Za-z0-9]+)$",             // Space: GROUP
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(release_name) {
                    if let Some(group) = captures.get(1) {
                        let group_name = group.as_str().to_uppercase();
                        // Filter out codecs, formats, and common false positives
                        if !self.is_false_positive(&group_name) {
                            return Some(group_name);
                        }
                    }
                }
            }
        }
        None
    }

    fn is_false_positive(&self, candidate: &str) -> bool {
        let false_positives = [
            "X264", "X265", "H264", "H265", "HEVC", "AVC", "AAC", "AC3", "DTS",
            "BLURAY", "WEB", "HDTV", "1080P", "720P", "2160P", "4K", "INTERNAL",
            "PROPER", "REPACK", "LIMITED", "EXTENDED", "UNRATED", "DIRECTORS",
            "COMPLETE", "MULTI", "DUBBED", "SUBBED", "ENG", "GER", "FRA"
        ];
        false_positives.contains(&candidate)
    }

    pub fn analyze_scene_groups(&mut self, releases: Vec<BrowseRelease>) -> Result<()> {
        info!("Analyzing {} releases for scene group reputation data", releases.len());
        
        for release in releases {
            if let Some(group_name) = self.extract_scene_group(&release.name) {
                let group_analysis = self.scene_groups.entry(group_name.clone())
                    .or_insert_with(|| SceneGroupAnalysis {
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
                        confidence_level: "Unknown".to_string(),
                        quality_tier: "Unknown".to_string(),
                        first_seen: release.added_date,
                        last_seen: release.added_date,
                        releases: Vec::new(),
                    });

                // Update group statistics
                group_analysis.total_releases += 1;
                if release.is_internal {
                    group_analysis.internal_releases += 1;
                }
                
                // Track first and last seen dates
                if release.added_date < group_analysis.first_seen {
                    group_analysis.first_seen = release.added_date;
                }
                if release.added_date > group_analysis.last_seen {
                    group_analysis.last_seen = release.added_date;
                }
                
                group_analysis.releases.push(release);
            }
        }

        // Calculate derived metrics
        self.calculate_group_metrics();
        
        info!("Scene group analysis complete: {} unique groups identified", self.scene_groups.len());
        Ok(())
    }

    fn calculate_group_metrics(&mut self) {
        // Store calculated values to avoid borrowing issues
        let mut calculated_values: HashMap<String, (f64, String, String)> = HashMap::new();
        
        for (group_name, group_analysis) in &mut self.scene_groups {
            if group_analysis.releases.is_empty() {
                continue;
            }
            
            let releases = &group_analysis.releases;
            let count = releases.len() as f64;
            
            // Calculate basic averages
            group_analysis.avg_seeders = releases.iter().map(|r| r.seeders as f64).sum::<f64>() / count;
            group_analysis.avg_leechers = releases.iter().map(|r| r.leechers as f64).sum::<f64>() / count;
            group_analysis.avg_size_gb = releases.iter().map(|r| r.size_gb).sum::<f64>() / count;
            group_analysis.avg_completion_rate = releases.iter().map(|r| r.completed as f64).sum::<f64>() / count;
            
            // Calculate ratios
            group_analysis.internal_ratio = group_analysis.internal_releases as f64 / group_analysis.total_releases as f64;
            group_analysis.seeder_leecher_ratio = if group_analysis.avg_leechers > 0.0 {
                group_analysis.avg_seeders / group_analysis.avg_leechers
            } else {
                group_analysis.avg_seeders
            };
            
            // Quality consistency (inverse of size variance)
            let size_variance = releases.iter()
                .map(|r| (r.size_gb - group_analysis.avg_size_gb).powi(2))
                .sum::<f64>() / count;
            group_analysis.quality_consistency = 1.0 / (1.0 + size_variance);
            
            // Recency score
            let days_since_last = Utc::now().signed_duration_since(group_analysis.last_seen).num_days() as f64;
            group_analysis.recency_score = 1.0 / (1.0 + days_since_last / 30.0);
            
            // Calculate derived values and store them
            let reputation_score = Self::calculate_reputation_score_static(&group_analysis);
            let quality_tier = Self::determine_quality_tier_static(reputation_score);
            let confidence_level = Self::calculate_confidence_level_static(&group_analysis);
            
            calculated_values.insert(group_name.clone(), (reputation_score, quality_tier, confidence_level));
        }
        
        // Apply calculated values
        for (group_name, (reputation_score, quality_tier, confidence_level)) in calculated_values {
            if let Some(group_analysis) = self.scene_groups.get_mut(&group_name) {
                group_analysis.reputation_score = reputation_score;
                group_analysis.quality_tier = quality_tier;
                group_analysis.confidence_level = confidence_level;
            }
        }
    }

    fn calculate_reputation_score(&self, analysis: &SceneGroupAnalysis) -> f64 {
        Self::calculate_reputation_score_static(analysis)
    }
    
    fn calculate_reputation_score_static(analysis: &SceneGroupAnalysis) -> f64 {
        // Evidence-based weighted scoring
        let weights = [
            ("seeder_health", 0.30),      // High seeders indicate quality and popularity
            ("internal_ratio", 0.25),     // Internal releases are vetted for quality
            ("completion_rate", 0.15),    // High completion indicates good releases
            ("consistency", 0.10),        // Consistent quality over time
            ("recency", 0.10),            // Recent activity indicates active group
            ("volume", 0.05),             // Release volume indicates established group
            ("size_appropriateness", 0.05), // Reasonable file sizes
        ];

        let seeder_score = (analysis.seeder_leecher_ratio / 5.0).min(1.0);
        let internal_score = analysis.internal_ratio;
        let completion_score = (analysis.avg_completion_rate / 50.0).min(1.0); // Normalize to reasonable range
        let consistency_score = analysis.quality_consistency;
        let recency_score = analysis.recency_score;
        let volume_score = (analysis.total_releases as f64 / 20.0).min(1.0);
        let size_score = Self::calculate_size_score_static(analysis.avg_size_gb);

        let weighted_score = 
            weights[0].1 * seeder_score +
            weights[1].1 * internal_score +
            weights[2].1 * completion_score +
            weights[3].1 * consistency_score +
            weights[4].1 * recency_score +
            weights[5].1 * volume_score +
            weights[6].1 * size_score;

        // Scale to 0-100 range
        (weighted_score * 100.0).min(100.0).max(0.0)
    }

    fn calculate_size_score(&self, avg_size_gb: f64) -> f64 {
        Self::calculate_size_score_static(avg_size_gb)
    }
    
    fn calculate_size_score_static(avg_size_gb: f64) -> f64 {
        // Score based on appropriate size ranges for quality
        match avg_size_gb {
            size if size >= 15.0 && size <= 45.0 => 1.0,  // Optimal range
            size if size >= 8.0 && size < 15.0 => 0.9,    // Good compression
            size if size >= 5.0 && size < 8.0 => 0.7,     // Acceptable
            size if size >= 3.0 && size < 5.0 => 0.5,     // Lower quality
            size if size < 3.0 => 0.3,                     // Very compressed
            size if size > 45.0 && size <= 80.0 => 0.8,   // Remux/uncompressed
            _ => 0.4,                                       // Very large or unusual
        }
    }

    fn determine_quality_tier(&self, score: f64) -> String {
        Self::determine_quality_tier_static(score)
    }
    
    fn determine_quality_tier_static(score: f64) -> String {
        match score {
            s if s >= 85.0 => "Premium".to_string(),
            s if s >= 75.0 => "Excellent".to_string(),
            s if s >= 65.0 => "Good".to_string(),
            s if s >= 50.0 => "Average".to_string(),
            s if s >= 35.0 => "Below Average".to_string(),
            _ => "Poor".to_string(),
        }
    }

    fn calculate_confidence_level(&self, analysis: &SceneGroupAnalysis) -> String {
        Self::calculate_confidence_level_static(analysis)
    }
    
    fn calculate_confidence_level_static(analysis: &SceneGroupAnalysis) -> String {
        let release_count = analysis.total_releases;
        let days_since_last = Utc::now().signed_duration_since(analysis.last_seen).num_days();
        
        match (release_count, days_since_last) {
            (count, days) if count >= 20 && days <= 30 => "Very High".to_string(),
            (count, days) if count >= 10 && days <= 90 => "High".to_string(),
            (count, days) if count >= 5 && days <= 180 => "Medium".to_string(),
            (count, days) if count >= 3 && days <= 365 => "Low".to_string(),
            _ => "Very Low".to_string(),
        }
    }

    pub fn generate_analysis_report(&self, start_time: DateTime<Utc>) -> BrowseAnalysisReport {
        let duration_minutes = Utc::now().signed_duration_since(start_time).num_minutes() as u64;
        
        let total_releases: u32 = self.scene_groups.values().map(|g| g.total_releases).sum();
        let internal_releases: u32 = self.scene_groups.values().map(|g| g.internal_releases).sum();
        let external_releases = total_releases - internal_releases;
        
        let mut top_groups: Vec<SceneGroupSummary> = self.scene_groups.values()
            .map(|g| SceneGroupSummary {
                group_name: g.group_name.clone(),
                reputation_score: g.reputation_score,
                quality_tier: g.quality_tier.clone(),
                total_releases: g.total_releases,
                internal_ratio: g.internal_ratio,
                avg_seeders: g.avg_seeders,
                confidence_level: g.confidence_level.clone(),
            })
            .collect();
        
        top_groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        
        BrowseAnalysisReport {
            generated_at: Utc::now(),
            total_releases_analyzed: total_releases,
            unique_scene_groups: self.scene_groups.len() as u32,
            internal_releases,
            external_releases,
            collection_duration_minutes: duration_minutes,
            top_groups: top_groups.into_iter().take(20).collect(),
            statistical_summary: self.calculate_stats_summary(),
            quality_distribution: self.calculate_quality_distribution(),
            data_collection_notes: vec![
                "Data collected using safe browse interface with 30+ second rate limiting".to_string(),
                "Focus on internal releases for highest quality scene groups".to_string(),
                "Conservative collection approach respects HDBits community guidelines".to_string(),
            ],
        }
    }

    fn calculate_stats_summary(&self) -> BrowseStatSummary {
        let groups: Vec<&SceneGroupAnalysis> = self.scene_groups.values().collect();
        
        BrowseStatSummary {
            reputation_scores: self.calculate_stat_range(groups.iter().map(|g| g.reputation_score).collect()),
            seeder_counts: self.calculate_stat_range(groups.iter().map(|g| g.avg_seeders).collect()),
            file_sizes_gb: self.calculate_stat_range(groups.iter().map(|g| g.avg_size_gb).collect()),
            internal_ratios: self.calculate_stat_range(groups.iter().map(|g| g.internal_ratio).collect()),
        }
    }

    fn calculate_stat_range(&self, mut values: Vec<f64>) -> StatRange {
        if values.is_empty() {
            return StatRange { min: 0.0, max: 0.0, mean: 0.0, median: 0.0, p95: 0.0 };
        }
        
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let len = values.len();
        let min = values[0];
        let max = values[len - 1];
        let mean = values.iter().sum::<f64>() / len as f64;
        let median = if len % 2 == 0 {
            (values[len / 2 - 1] + values[len / 2]) / 2.0
        } else {
            values[len / 2]
        };
        let p95_idx = ((len as f64 * 0.95) as usize).min(len - 1);
        let p95 = values[p95_idx];
        
        StatRange { min, max, mean, median, p95 }
    }

    fn calculate_quality_distribution(&self) -> QualityDistribution {
        let mut dist = QualityDistribution {
            premium: 0, excellent: 0, good: 0, average: 0, below_average: 0, poor: 0,
        };
        
        for group in self.scene_groups.values() {
            match group.reputation_score {
                s if s >= 85.0 => dist.premium += 1,
                s if s >= 75.0 => dist.excellent += 1,
                s if s >= 65.0 => dist.good += 1,
                s if s >= 50.0 => dist.average += 1,
                s if s >= 35.0 => dist.below_average += 1,
                _ => dist.poor += 1,
            }
        }
        
        dist
    }

    pub fn export_reputation_data(&self) -> Result<String> {
        let mut reputation_system = std::collections::HashMap::new();
        
        for (group_name, analysis) in &self.scene_groups {
            reputation_system.insert(group_name.clone(), serde_json::json!({
                "reputation_score": analysis.reputation_score,
                "quality_tier": analysis.quality_tier,
                "total_releases": analysis.total_releases,
                "internal_ratio": analysis.internal_ratio,
                "avg_seeders": analysis.avg_seeders,
                "confidence_level": analysis.confidence_level,
                "last_active": analysis.last_seen.format("%Y-%m-%d").to_string(),
                "data_source": "HDBits Browse Analysis",
            }));
        }
        
        let export_data = serde_json::json!({
            "version": "2.0",
            "generated_at": Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            "collection_method": "Safe Browse Interface with Rate Limiting",
            "total_groups": self.scene_groups.len(),
            "scoring_methodology": "Evidence-based multi-factor weighted analysis",
            "groups": reputation_system
        });
        
        serde_json::to_string_pretty(&export_data)
            .context("Failed to serialize reputation data")
    }

    pub fn export_csv_data(&self) -> String {
        let mut csv = String::from("group_name,reputation_score,quality_tier,total_releases,internal_releases,internal_ratio,avg_seeders,avg_leechers,avg_size_gb,seeder_leecher_ratio,recency_score,confidence_level,first_seen,last_seen\n");
        
        for analysis in self.scene_groups.values() {
            csv.push_str(&format!(
                "{},{:.2},{},{},{},{:.3},{:.1},{:.1},{:.2},{:.2},{:.3},{},{},{}\n",
                analysis.group_name,
                analysis.reputation_score,
                analysis.quality_tier,
                analysis.total_releases,
                analysis.internal_releases,
                analysis.internal_ratio,
                analysis.avg_seeders,
                analysis.avg_leechers,
                analysis.avg_size_gb,
                analysis.seeder_leecher_ratio,
                analysis.recency_score,
                analysis.confidence_level,
                analysis.first_seen.format("%Y-%m-%d"),
                analysis.last_seen.format("%Y-%m-%d")
            ));
        }
        
        csv
    }
    
    pub fn get_scene_groups(&self) -> &HashMap<String, SceneGroupAnalysis> {
        &self.scene_groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_group_extraction() {
        let analyzer = HDBitsBrowseAnalyzer::new(HDBitsBrowseConfig::default());
        
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay.x264-SPARKS"), Some("SPARKS".to_string()));
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay.x264.ROVERS"), Some("ROVERS".to_string()));
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay.x264[CMRG]"), Some("CMRG".to_string()));
        
        // Should filter out false positives
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay-x264"), None);
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.HEVC"), None);
    }

    #[test]
    fn test_quality_tier_assignment() {
        let analyzer = HDBitsBrowseAnalyzer::new(HDBitsBrowseConfig::default());
        
        assert_eq!(analyzer.determine_quality_tier(90.0), "Premium");
        assert_eq!(analyzer.determine_quality_tier(80.0), "Excellent");
        assert_eq!(analyzer.determine_quality_tier(70.0), "Good");
        assert_eq!(analyzer.determine_quality_tier(55.0), "Average");
        assert_eq!(analyzer.determine_quality_tier(40.0), "Below Average");
        assert_eq!(analyzer.determine_quality_tier(20.0), "Poor");
    }

    #[test]
    fn test_size_parsing() {
        let analyzer = HDBitsBrowseAnalyzer::new(HDBitsBrowseConfig::default());
        
        assert_eq!(analyzer.parse_size_to_gb("15.5 GB"), 15.5);
        assert_eq!(analyzer.parse_size_to_gb("1.2 TB"), 1228.8);
        assert_eq!(analyzer.parse_size_to_gb("850 MB"), 0.830078125);
    }
}