// HDBits Session Cookie Scene Group Analysis
// Safe data collection using authenticated session cookies with strict rate limiting

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
pub struct HDBitsSessionConfig {
    pub session_cookie: String,
    pub base_url: String,
    pub rate_limit_seconds: u64, // Conservative rate limiting
}

impl Default for HDBitsSessionConfig {
    fn default() -> Self {
        Self {
            session_cookie: "".to_string(),
            base_url: "https://hdbits.org".to_string(),
            rate_limit_seconds: 35, // 35 second delays between requests
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRelease {
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
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGroupSessionAnalysis {
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
    pub releases: Vec<SessionRelease>,
    pub categories_covered: Vec<String>,
    pub size_consistency_score: f64,
    pub seeder_health_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionAnalysisReport {
    pub generated_at: DateTime<Utc>,
    pub total_releases_analyzed: u32,
    pub unique_scene_groups: u32,
    pub internal_releases: u32,
    pub external_releases: u32,
    pub collection_duration_minutes: u64,
    pub categories_analyzed: Vec<String>,
    pub top_groups: Vec<SceneGroupSessionSummary>,
    pub statistical_summary: SessionStatSummary,
    pub quality_distribution: SessionQualityDistribution,
    pub data_collection_notes: Vec<String>,
    pub methodology: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneGroupSessionSummary {
    pub group_name: String,
    pub reputation_score: f64,
    pub quality_tier: String,
    pub total_releases: u32,
    pub internal_ratio: f64,
    pub avg_seeders: f64,
    pub confidence_level: String,
    pub categories_covered: usize,
    pub seeder_health_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionStatSummary {
    pub reputation_scores: StatRange,
    pub seeder_counts: StatRange,
    pub file_sizes_gb: StatRange,
    pub internal_ratios: StatRange,
    pub completion_rates: StatRange,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatRange {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionQualityDistribution {
    pub premium: u32,    // 90-100
    pub excellent: u32,  // 80-89
    pub good: u32,       // 70-79
    pub average: u32,    // 60-69
    pub below_average: u32, // 40-59
    pub poor: u32,       // 0-39
}

pub struct HDBitsSessionAnalyzer {
    client: Client,
    config: HDBitsSessionConfig,
    requests_made: u32,
    last_request_time: DateTime<Utc>,
    scene_groups: HashMap<String, SceneGroupSessionAnalysis>,
}

impl HDBitsSessionAnalyzer {
    pub fn new(config: HDBitsSessionConfig) -> Self {
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

    pub async fn collect_comprehensive_data(&mut self) -> Result<Vec<SessionRelease>> {
        info!("Starting comprehensive data collection from HDBits browse interface with session cookies");
        
        let mut all_releases = Vec::new();
        let mut notes = Vec::new();
        
        // Collect from multiple categories with internal filter
        let categories = vec![
            ("Movies", vec![("c1", "1")]),
            ("TV", vec![("c2", "1")]),
            ("Documentaries", vec![("c3", "1")]),
        ];
        
        for (category_name, category_params) in categories {
            info!("Collecting {} releases with internal filter", category_name);
            
            let mut browse_params = vec![
                ("origin", "1"), // Internal only
                ("order", "added"),
                ("sort", "desc"),
            ];
            browse_params.extend(category_params);
            
            // Collect multiple pages per category
            for page in 0..3 { // Limited pages per category
                info!("Collecting {} page {} with {} second delay", category_name, page + 1, self.config.rate_limit_seconds);
                
                let mut url = Url::parse(&format!("{}/browse.php", self.config.base_url))?;
                for (key, value) in &browse_params {
                    url.query_pairs_mut().append_pair(key, value);
                }
                url.query_pairs_mut().append_pair("page", &page.to_string());

                self.rate_limit_delay().await?;
                
                let response = self.client
                    .get(url.as_str())
                    .header("Cookie", &self.config.session_cookie)
                    .send()
                    .await
                    .context("Failed to fetch browse page")?;

                if !response.status().is_success() {
                    warn!("{} page {} returned status: {}", category_name, page, response.status());
                    notes.push(format!("{} page {} failed with status {}", category_name, page, response.status()));
                    continue;
                }

                let html = response.text().await.context("Failed to read response body")?;
                
                // Check if we're still logged in
                if html.contains("login") || html.contains("Please log in") {
                    return Err(anyhow::anyhow!("Session expired or authentication failed"));
                }
                
                let releases = self.parse_browse_page(&html, category_name)?;
                
                if releases.is_empty() {
                    info!("No more {} releases found on page {}, stopping collection", category_name, page + 1);
                    break;
                }

                info!("Extracted {} {} releases from page {}", releases.len(), category_name, page + 1);
                all_releases.extend(releases);
                
                // Additional safety delay between pages
                if page < 2 {
                    info!("Additional safety delay before next page");
                    sleep(Duration::from_secs(3)).await;
                }
            }
            
            // Delay between categories
            info!("Category collection complete, brief delay before next category");
            sleep(Duration::from_secs(5)).await;
        }

        info!("Collection complete: {} total releases from all categories", all_releases.len());
        
        Ok(all_releases)
    }

    fn parse_browse_page(&self, html: &str, category: &str) -> Result<Vec<SessionRelease>> {
        let document = Html::parse_document(html);
        let mut releases = Vec::new();
        
        // HDBits browse table selectors
        let row_selector = Selector::parse("table.mainblockcontenttt tr, table[class*='torrent'] tr, tr[class*='torrent']")
            .map_err(|e| anyhow::anyhow!("Invalid row selector: {:?}", e))?;
        
        for row in document.select(&row_selector) {
            // Try different selector patterns for torrent name
            let name_selectors = vec![
                "td.mainblockcontent a[href*='details.php']",
                "a[href*='details.php']",
                "td a[href*='details']",
                "a[title][href*='details']",
            ];
            
            let mut name_element = None;
            for selector_str in &name_selectors {
                if let Ok(selector) = Selector::parse(selector_str) {
                    if let Some(element) = row.select(&selector).next() {
                        name_element = Some(element);
                        break;
                    }
                }
            }
            
            if let Some(name_el) = name_element {
                let name = name_el.text().collect::<String>().trim().to_string();
                
                if name.is_empty() || name.len() < 10 {
                    continue;
                }
                
                // Extract ID from href
                let id = name_el.value().attr("href")
                    .and_then(|href| href.split("id=").nth(1))
                    .and_then(|id_part| id_part.split("&").next())
                    .unwrap_or(&self.generate_id_from_name(&name))
                    .to_string();

                // Parse stats from table cells
                let stats: Vec<String> = row.select(&Selector::parse("td").unwrap())
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .collect();

                // Extract metadata (indices may need adjustment based on actual HTML structure)
                let seeders = self.extract_number_from_stats(&stats, &["seed"]);
                let leechers = self.extract_number_from_stats(&stats, &["leech"]);
                let completed = self.extract_number_from_stats(&stats, &["compl", "done"]);
                let size_gb = self.extract_size_from_stats(&stats);
                let added_date = self.extract_date_from_stats(&stats).unwrap_or_else(|| Utc::now());
                
                let release = SessionRelease {
                    id,
                    name: name.clone(),
                    seeders,
                    leechers,
                    completed,
                    size_gb,
                    added_date,
                    is_internal: true, // We're only collecting internal releases
                    category: category.to_string(),
                    codec: self.extract_codec_from_name(&name),
                    quality: self.extract_quality_from_name(&name),
                    source: self.extract_source_from_name(&name),
                    imdb_id: self.extract_imdb_from_name(&name),
                    tmdb_id: self.extract_tmdb_from_name(&name),
                };

                releases.push(release);
            }
        }

        debug!("Parsed {} releases from {} HTML page", releases.len(), category);
        Ok(releases)
    }

    fn generate_id_from_name(&self, name: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn extract_number_from_stats(&self, stats: &[String], keywords: &[&str]) -> u32 {
        for stat in stats {
            let stat_lower = stat.to_lowercase();
            for keyword in keywords {
                if stat_lower.contains(keyword) {
                    let number: String = stat.chars().filter(|c| c.is_numeric()).collect();
                    if let Ok(n) = number.parse::<u32>() {
                        return n;
                    }
                }
            }
        }
        
        // Fallback: look for any numeric values in stats
        for stat in stats {
            if let Ok(n) = stat.parse::<u32>() {
                if n > 0 && n < 10000 { // Reasonable range
                    return n;
                }
            }
        }
        
        0
    }

    fn extract_size_from_stats(&self, stats: &[String]) -> f64 {
        for stat in stats {
            if stat.contains("GB") || stat.contains("MB") || stat.contains("TB") {
                return self.parse_size_to_gb(stat);
            }
        }
        
        // Look for size patterns
        for stat in stats {
            if let Ok(size) = stat.parse::<f64>() {
                if size > 0.1 && size < 100.0 { // Reasonable GB range
                    return size;
                }
            }
        }
        
        0.0
    }

    fn extract_date_from_stats(&self, stats: &[String]) -> Option<DateTime<Utc>> {
        for stat in stats {
            // Try various date formats
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(stat, "%Y-%m-%d %H:%M:%S") {
                return Some(dt.and_utc());
            }
            if let Ok(dt) = chrono::NaiveDate::parse_from_str(stat, "%Y-%m-%d") {
                return Some(dt.and_hms_opt(0, 0, 0).unwrap().and_utc());
            }
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(stat, "%d/%m/%Y %H:%M") {
                return Some(dt.and_utc());
            }
        }
        None
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

    fn extract_codec_from_name(&self, name: &str) -> String {
        let name_upper = name.to_uppercase();
        if name_upper.contains("X265") || name_upper.contains("HEVC") {
            "x265".to_string()
        } else if name_upper.contains("X264") || name_upper.contains("AVC") {
            "x264".to_string()
        } else if name_upper.contains("AV1") {
            "AV1".to_string()
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
        } else if name_upper.contains("480P") {
            "480p".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn extract_source_from_name(&self, name: &str) -> String {
        let name_upper = name.to_uppercase();
        if name_upper.contains("BLURAY") || name_upper.contains("BDR") {
            "Blu-ray".to_string()
        } else if name_upper.contains("WEB-DL") {
            "WEB-DL".to_string()
        } else if name_upper.contains("WEB") {
            "WEB".to_string()
        } else if name_upper.contains("HDTV") {
            "HDTV".to_string()
        } else if name_upper.contains("REMUX") {
            "Remux".to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn extract_imdb_from_name(&self, name: &str) -> Option<String> {
        // Extract IMDB ID if present in name
        if let Some(start) = name.find("tt") {
            let imdb_part = &name[start..];
            if let Some(end) = imdb_part.find(|c: char| !c.is_alphanumeric()) {
                let imdb_id = &imdb_part[..end];
                if imdb_id.len() >= 7 { // Valid IMDB IDs are at least tt + 5 digits
                    return Some(imdb_id.to_string());
                }
            }
        }
        None
    }

    fn extract_tmdb_from_name(&self, _name: &str) -> Option<String> {
        // TMDB IDs are rarely in release names, would need to be extracted from page metadata
        None
    }

    pub fn extract_scene_group(&self, release_name: &str) -> Option<String> {
        // Enhanced scene group extraction patterns
        let patterns = [
            r"-([A-Za-z0-9]+)$",              // Standard: -GROUP
            r"\.([A-Za-z0-9]+)$",             // Dot: .GROUP
            r"\[([A-Za-z0-9]+)\]$",           // Brackets: [GROUP]
            r"\(([A-Za-z0-9]+)\)$",           // Parentheses: (GROUP)
            r"\s([A-Za-z0-9]+)$",             // Space: GROUP
            r"-([A-Za-z0-9]+)\.",             // Middle: -GROUP.
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
            "X264", "X265", "H264", "H265", "HEVC", "AVC", "AAC", "AC3", "DTS", "AV1",
            "BLURAY", "WEB", "HDTV", "1080P", "720P", "2160P", "4K", "INTERNAL",
            "PROPER", "REPACK", "LIMITED", "EXTENDED", "UNRATED", "DIRECTORS",
            "COMPLETE", "MULTI", "DUBBED", "SUBBED", "ENG", "GER", "FRA", "REMUX",
            "HDR", "DOLBY", "VISION", "ATMOS", "DTS-HD", "TRUEHD", "DD", "MA"
        ];
        false_positives.contains(&candidate)
    }

    pub fn analyze_scene_groups(&mut self, releases: Vec<SessionRelease>) -> Result<()> {
        info!("Analyzing {} releases for comprehensive scene group reputation data", releases.len());
        
        for release in releases {
            if let Some(group_name) = self.extract_scene_group(&release.name) {
                let group_analysis = self.scene_groups.entry(group_name.clone())
                    .or_insert_with(|| SceneGroupSessionAnalysis {
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
                        categories_covered: Vec::new(),
                        size_consistency_score: 0.0,
                        seeder_health_score: 0.0,
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
                
                // Track categories
                if !group_analysis.categories_covered.contains(&release.category) {
                    group_analysis.categories_covered.push(release.category.clone());
                }
                
                group_analysis.releases.push(release);
            }
        }

        // Calculate comprehensive metrics
        self.calculate_comprehensive_metrics();
        
        info!("Scene group analysis complete: {} unique groups identified", self.scene_groups.len());
        Ok(())
    }

    fn calculate_comprehensive_metrics(&mut self) {
        let mut calculated_values: HashMap<String, (f64, String, String, f64, f64)> = HashMap::new();
        
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
            
            // Quality consistency (inverse of size variance + codec consistency)
            let size_variance = releases.iter()
                .map(|r| (r.size_gb - group_analysis.avg_size_gb).powi(2))
                .sum::<f64>() / count;
            let size_consistency = 1.0 / (1.0 + size_variance);
            
            // Codec consistency
            let unique_codecs = releases.iter()
                .map(|r| &r.codec)
                .collect::<std::collections::HashSet<_>>()
                .len() as f64;
            let codec_consistency = 1.0 / unique_codecs;
            
            group_analysis.quality_consistency = (size_consistency + codec_consistency) / 2.0;
            group_analysis.size_consistency_score = size_consistency;
            
            // Recency score (weighted by release frequency)
            let days_since_last = Utc::now().signed_duration_since(group_analysis.last_seen).num_days() as f64;
            let activity_span = group_analysis.last_seen.signed_duration_since(group_analysis.first_seen).num_days() as f64;
            let activity_rate = if activity_span > 0.0 { count / activity_span } else { count };
            group_analysis.recency_score = (1.0 / (1.0 + days_since_last / 30.0)) * (1.0 + activity_rate.ln());
            
            // Seeder health score
            let seeder_scores: Vec<f64> = releases.iter()
                .map(|r| (r.seeders as f64).ln() + 1.0)
                .collect();
            group_analysis.seeder_health_score = seeder_scores.iter().sum::<f64>() / count;
            
            // Calculate derived values and store them
            let reputation_score = Self::calculate_comprehensive_reputation_score(&group_analysis);
            let quality_tier = Self::determine_quality_tier_comprehensive(reputation_score);
            let confidence_level = Self::calculate_confidence_level_comprehensive(&group_analysis);
            
            calculated_values.insert(group_name.clone(), (
                reputation_score, 
                quality_tier, 
                confidence_level,
                group_analysis.size_consistency_score,
                group_analysis.seeder_health_score,
            ));
        }
        
        // Apply calculated values
        for (group_name, (reputation_score, quality_tier, confidence_level, size_score, seeder_score)) in calculated_values {
            if let Some(group_analysis) = self.scene_groups.get_mut(&group_name) {
                group_analysis.reputation_score = reputation_score;
                group_analysis.quality_tier = quality_tier;
                group_analysis.confidence_level = confidence_level;
                group_analysis.size_consistency_score = size_score;
                group_analysis.seeder_health_score = seeder_score;
            }
        }
    }

    fn calculate_comprehensive_reputation_score(analysis: &SceneGroupSessionAnalysis) -> f64 {
        // Enhanced evidence-based weighted scoring
        let weights = [
            ("seeder_health", 0.25),        // High seeders indicate quality and popularity
            ("internal_ratio", 0.20),       // Internal releases are vetted for quality
            ("completion_rate", 0.15),      // High completion indicates good releases
            ("consistency", 0.12),          // Consistent quality over time
            ("recency", 0.10),              // Recent activity indicates active group
            ("category_diversity", 0.08),   // Multi-category groups show expertise
            ("volume", 0.05),               // Release volume indicates established group
            ("size_appropriateness", 0.05), // Reasonable file sizes
        ];

        let seeder_score = (analysis.seeder_health_score / 5.0).min(1.0);
        let internal_score = analysis.internal_ratio;
        let completion_score = (analysis.avg_completion_rate / 100.0).min(1.0);
        let consistency_score = analysis.quality_consistency;
        let recency_score = analysis.recency_score.min(1.0);
        let category_score = (analysis.categories_covered.len() as f64 / 3.0).min(1.0);
        let volume_score = (analysis.total_releases as f64 / 50.0).min(1.0);
        let size_score = Self::calculate_size_score_comprehensive(analysis.avg_size_gb);

        let weighted_score = 
            weights[0].1 * seeder_score +
            weights[1].1 * internal_score +
            weights[2].1 * completion_score +
            weights[3].1 * consistency_score +
            weights[4].1 * recency_score +
            weights[5].1 * category_score +
            weights[6].1 * volume_score +
            weights[7].1 * size_score;

        // Scale to 0-100 range with bonus for exceptional performance
        let base_score = (weighted_score * 100.0).min(100.0).max(0.0);
        
        // Bonus for exceptional groups
        let exceptional_bonus = if analysis.total_releases > 100 && analysis.internal_ratio > 0.8 && analysis.seeder_health_score > 4.0 {
            5.0
        } else {
            0.0
        };
        
        (base_score + exceptional_bonus).min(100.0)
    }

    fn calculate_size_score_comprehensive(avg_size_gb: f64) -> f64 {
        // Enhanced score based on appropriate size ranges for quality
        match avg_size_gb {
            size if size >= 15.0 && size <= 50.0 => 1.0,  // Optimal range for high quality
            size if size >= 8.0 && size < 15.0 => 0.95,   // Good compression
            size if size >= 5.0 && size < 8.0 => 0.85,    // Acceptable
            size if size >= 3.0 && size < 5.0 => 0.7,     // Lower quality
            size if size >= 1.0 && size < 3.0 => 0.5,     // Very compressed
            size if size > 50.0 && size <= 100.0 => 0.9,  // Remux/uncompressed
            size if size > 100.0 => 0.6,                   // Very large
            size if size < 1.0 => 0.3,                     // Extremely small
            _ => 0.4,                                       // Unusual
        }
    }

    fn determine_quality_tier_comprehensive(score: f64) -> String {
        match score {
            s if s >= 90.0 => "Premium".to_string(),
            s if s >= 80.0 => "Excellent".to_string(),
            s if s >= 70.0 => "Good".to_string(),
            s if s >= 60.0 => "Average".to_string(),
            s if s >= 40.0 => "Below Average".to_string(),
            _ => "Poor".to_string(),
        }
    }

    fn calculate_confidence_level_comprehensive(analysis: &SceneGroupSessionAnalysis) -> String {
        let release_count = analysis.total_releases;
        let days_since_last = Utc::now().signed_duration_since(analysis.last_seen).num_days();
        let category_diversity = analysis.categories_covered.len();
        
        // Multi-factor confidence calculation
        let volume_confidence = match release_count {
            count if count >= 50 => 3,
            count if count >= 20 => 2,
            count if count >= 10 => 1,
            _ => 0,
        };
        
        let recency_confidence = match days_since_last {
            days if days <= 30 => 3,
            days if days <= 90 => 2,
            days if days <= 180 => 1,
            _ => 0,
        };
        
        let diversity_confidence = match category_diversity {
            cats if cats >= 3 => 2,
            cats if cats >= 2 => 1,
            _ => 0,
        };
        
        let total_confidence = volume_confidence + recency_confidence + diversity_confidence;
        
        match total_confidence {
            score if score >= 7 => "Very High".to_string(),
            score if score >= 5 => "High".to_string(),
            score if score >= 3 => "Medium".to_string(),
            score if score >= 1 => "Low".to_string(),
            _ => "Very Low".to_string(),
        }
    }

    pub fn generate_comprehensive_report(&self, start_time: DateTime<Utc>) -> SessionAnalysisReport {
        let duration_minutes = Utc::now().signed_duration_since(start_time).num_minutes() as u64;
        
        let total_releases: u32 = self.scene_groups.values().map(|g| g.total_releases).sum();
        let internal_releases: u32 = self.scene_groups.values().map(|g| g.internal_releases).sum();
        let external_releases = total_releases - internal_releases;
        
        // Collect all categories
        let mut all_categories = std::collections::HashSet::new();
        for group in self.scene_groups.values() {
            for category in &group.categories_covered {
                all_categories.insert(category.clone());
            }
        }
        let categories_analyzed: Vec<String> = all_categories.into_iter().collect();
        
        let mut top_groups: Vec<SceneGroupSessionSummary> = self.scene_groups.values()
            .map(|g| SceneGroupSessionSummary {
                group_name: g.group_name.clone(),
                reputation_score: g.reputation_score,
                quality_tier: g.quality_tier.clone(),
                total_releases: g.total_releases,
                internal_ratio: g.internal_ratio,
                avg_seeders: g.avg_seeders,
                confidence_level: g.confidence_level.clone(),
                categories_covered: g.categories_covered.len(),
                seeder_health_score: g.seeder_health_score,
            })
            .collect();
        
        top_groups.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        
        SessionAnalysisReport {
            generated_at: Utc::now(),
            total_releases_analyzed: total_releases,
            unique_scene_groups: self.scene_groups.len() as u32,
            internal_releases,
            external_releases,
            collection_duration_minutes: duration_minutes,
            categories_analyzed,
            top_groups: top_groups.into_iter().take(30).collect(),
            statistical_summary: self.calculate_comprehensive_stats_summary(),
            quality_distribution: self.calculate_comprehensive_quality_distribution(),
            data_collection_notes: vec![
                "Data collected using authenticated session cookies with 35+ second rate limiting".to_string(),
                "Focus on internal releases across Movies, TV, and Documentaries".to_string(),
                "Conservative collection approach respects HDBits community guidelines".to_string(),
                "Multi-category analysis provides comprehensive scene group evaluation".to_string(),
                "Enhanced reputation scoring includes consistency, diversity, and health metrics".to_string(),
            ],
            methodology: "Evidence-based multi-factor weighted analysis with session authentication".to_string(),
        }
    }

    fn calculate_comprehensive_stats_summary(&self) -> SessionStatSummary {
        let groups: Vec<&SceneGroupSessionAnalysis> = self.scene_groups.values().collect();
        
        SessionStatSummary {
            reputation_scores: self.calculate_enhanced_stat_range(groups.iter().map(|g| g.reputation_score).collect()),
            seeder_counts: self.calculate_enhanced_stat_range(groups.iter().map(|g| g.avg_seeders).collect()),
            file_sizes_gb: self.calculate_enhanced_stat_range(groups.iter().map(|g| g.avg_size_gb).collect()),
            internal_ratios: self.calculate_enhanced_stat_range(groups.iter().map(|g| g.internal_ratio).collect()),
            completion_rates: self.calculate_enhanced_stat_range(groups.iter().map(|g| g.avg_completion_rate).collect()),
        }
    }

    fn calculate_enhanced_stat_range(&self, mut values: Vec<f64>) -> StatRange {
        if values.is_empty() {
            return StatRange { min: 0.0, max: 0.0, mean: 0.0, median: 0.0, p95: 0.0, p99: 0.0 };
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
        let p99_idx = ((len as f64 * 0.99) as usize).min(len - 1);
        let p99 = values[p99_idx];
        
        StatRange { min, max, mean, median, p95, p99 }
    }

    fn calculate_comprehensive_quality_distribution(&self) -> SessionQualityDistribution {
        let mut dist = SessionQualityDistribution {
            premium: 0, excellent: 0, good: 0, average: 0, below_average: 0, poor: 0,
        };
        
        for group in self.scene_groups.values() {
            match group.reputation_score {
                s if s >= 90.0 => dist.premium += 1,
                s if s >= 80.0 => dist.excellent += 1,
                s if s >= 70.0 => dist.good += 1,
                s if s >= 60.0 => dist.average += 1,
                s if s >= 40.0 => dist.below_average += 1,
                _ => dist.poor += 1,
            }
        }
        
        dist
    }

    pub fn export_reputation_system(&self) -> Result<String> {
        let mut reputation_system = std::collections::HashMap::new();
        
        for (group_name, analysis) in &self.scene_groups {
            reputation_system.insert(group_name.clone(), serde_json::json!({
                "reputation_score": analysis.reputation_score,
                "quality_tier": analysis.quality_tier,
                "total_releases": analysis.total_releases,
                "internal_ratio": analysis.internal_ratio,
                "avg_seeders": analysis.avg_seeders,
                "confidence_level": analysis.confidence_level,
                "categories_covered": analysis.categories_covered,
                "seeder_health_score": analysis.seeder_health_score,
                "size_consistency_score": analysis.size_consistency_score,
                "last_active": analysis.last_seen.format("%Y-%m-%d").to_string(),
                "data_source": "HDBits Session Analysis",
                "methodology": "Enhanced multi-factor weighted analysis",
            }));
        }
        
        let export_data = serde_json::json!({
            "version": "3.0",
            "generated_at": Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            "collection_method": "Authenticated Session with Conservative Rate Limiting",
            "total_groups": self.scene_groups.len(),
            "scoring_methodology": "Evidence-based multi-factor weighted analysis with session authentication",
            "quality_factors": [
                "seeder_health", "internal_ratio", "completion_rate", "consistency",
                "recency", "category_diversity", "volume", "size_appropriateness"
            ],
            "groups": reputation_system
        });
        
        serde_json::to_string_pretty(&export_data)
            .context("Failed to serialize reputation data")
    }

    pub fn export_comprehensive_csv(&self) -> String {
        let mut csv = String::from("group_name,reputation_score,quality_tier,total_releases,internal_releases,internal_ratio,avg_seeders,avg_leechers,avg_size_gb,seeder_leecher_ratio,recency_score,confidence_level,categories_covered,seeder_health_score,size_consistency_score,first_seen,last_seen\n");
        
        for analysis in self.scene_groups.values() {
            csv.push_str(&format!(
                "{},{:.2},{},{},{},{:.3},{:.1},{:.1},{:.2},{:.2},{:.3},{},{},{:.2},{:.3},{},{}\n",
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
                analysis.categories_covered.len(),
                analysis.seeder_health_score,
                analysis.size_consistency_score,
                analysis.first_seen.format("%Y-%m-%d"),
                analysis.last_seen.format("%Y-%m-%d")
            ));
        }
        
        csv
    }
    
    pub fn get_scene_groups(&self) -> &HashMap<String, SceneGroupSessionAnalysis> {
        &self.scene_groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_group_extraction() {
        let config = HDBitsSessionConfig::default();
        let analyzer = HDBitsSessionAnalyzer::new(config);
        
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay.x264-SPARKS"), Some("SPARKS".to_string()));
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay.x264.ROVERS"), Some("ROVERS".to_string()));
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay.x264[CMRG]"), Some("CMRG".to_string()));
        
        // Should filter out false positives
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.BluRay-x264"), None);
        assert_eq!(analyzer.extract_scene_group("Movie.Name.2023.1080p.HEVC"), None);
    }

    #[test]
    fn test_quality_tier_assignment() {
        assert_eq!(HDBitsSessionAnalyzer::determine_quality_tier_comprehensive(95.0), "Premium");
        assert_eq!(HDBitsSessionAnalyzer::determine_quality_tier_comprehensive(85.0), "Excellent");
        assert_eq!(HDBitsSessionAnalyzer::determine_quality_tier_comprehensive(75.0), "Good");
        assert_eq!(HDBitsSessionAnalyzer::determine_quality_tier_comprehensive(65.0), "Average");
        assert_eq!(HDBitsSessionAnalyzer::determine_quality_tier_comprehensive(45.0), "Below Average");
        assert_eq!(HDBitsSessionAnalyzer::determine_quality_tier_comprehensive(25.0), "Poor");
    }

    #[test]
    fn test_size_scoring() {
        assert_eq!(HDBitsSessionAnalyzer::calculate_size_score_comprehensive(25.0), 1.0);  // Optimal
        assert_eq!(HDBitsSessionAnalyzer::calculate_size_score_comprehensive(10.0), 0.95); // Good
        assert_eq!(HDBitsSessionAnalyzer::calculate_size_score_comprehensive(6.0), 0.85);  // Acceptable
        assert_eq!(HDBitsSessionAnalyzer::calculate_size_score_comprehensive(2.0), 0.5);   // Very compressed
    }

    #[test]
    fn test_false_positive_filtering() {
        let config = HDBitsSessionConfig::default();
        let analyzer = HDBitsSessionAnalyzer::new(config);
        
        assert!(analyzer.is_false_positive("X264"));
        assert!(analyzer.is_false_positive("HEVC"));
        assert!(analyzer.is_false_positive("BLURAY"));
        assert!(analyzer.is_false_positive("1080P"));
        assert!(!analyzer.is_false_positive("SPARKS"));
        assert!(!analyzer.is_false_positive("DON"));
    }
}