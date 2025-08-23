use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn, debug};

/// Configuration for API-based analysis
#[derive(Debug, Clone)]
pub struct ApiAnalyzerConfig {
    pub base_url: String,
    pub session_cookie: String,
    pub output_dir: PathBuf,
    pub max_pages: u32,
    pub request_delay_seconds: u64,
    pub results_per_page: u32,  // Max 100 per API docs
}

impl Default for ApiAnalyzerConfig {
    fn default() -> Self {
        Self {
            base_url: "https://hdbits.org".to_string(),
            session_cookie: String::new(),
            output_dir: PathBuf::from("/tmp/radarr/analysis"),
            max_pages: 10,
            request_delay_seconds: 5,
            results_per_page: 100,  // Maximum allowed by API
        }
    }
}

/// API Response for torrents endpoint
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiTorrentResponse {
    #[serde(default)]
    pub data: Vec<ApiTorrent>,
}

/// Individual torrent from API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiTorrent {
    pub id: u32,
    pub hash: String,
    pub name: String,
    #[serde(default)]
    pub descr: String,
    pub times_completed: u32,
    pub size: u64,
    pub seeders: u32,
    pub leechers: u32,
    pub added: String,
    pub utadded: i64,
    pub comments: u32,
    pub numfiles: u32,
    pub filename: String,
    pub freeleech: String,
    pub type_category: u32,
    pub type_codec: u32,
    pub type_medium: u32,
    pub type_origin: u32,
    pub type_exclusive: u32,
    #[serde(default)]
    pub torrent_status: String,
    #[serde(default)]
    pub bookmarked: u32,
    #[serde(default)]
    pub wishlisted: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub imdb: Option<ImdbData>,
    #[serde(default)]
    pub tvdb: Option<TvdbData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImdbData {
    pub id: String,
    pub title: String,
    pub year: Option<u32>,
    pub rating: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TvdbData {
    pub id: u32,
    pub season: Option<u32>,
    pub episode: Option<u32>,
}

/// Scene group statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGroupStats {
    pub name: String,
    pub release_count: u32,
    pub total_size_gb: f64,
    pub avg_seeders: f32,
    pub avg_snatched: f32,
    pub exclusive_count: u32,
    pub internal_count: u32,
    pub quality_score: f64,
    pub codecs: HashMap<String, u32>,
    pub mediums: HashMap<String, u32>,
}

pub struct HDBitsApiAnalyzer {
    config: ApiAnalyzerConfig,
    scene_groups: HashMap<String, SceneGroupStats>,
}

impl HDBitsApiAnalyzer {
    pub fn new(config: ApiAnalyzerConfig) -> Self {
        Self {
            config,
            scene_groups: HashMap::new(),
        }
    }
    
    /// Main analysis entry point
    pub async fn analyze(&mut self) -> Result<()> {
        info!("Starting HDBits API-based analysis");
        
        // Ensure output directory exists
        fs::create_dir_all(&self.config.output_dir)?;
        
        // Collect data from multiple filter combinations
        let mut all_torrents = Vec::new();
        
        // 1. Collect exclusive/internal releases (highest quality)
        info!("Collecting exclusive/internal releases...");
        let exclusive_torrents = self.fetch_torrents_filtered(
            Some(vec![1]),  // Movies
            None,           // All codecs
            None,           // All mediums
            Some(vec![1]),  // Internal origin
            Some(vec![1]),  // Exclusive
        ).await?;
        info!("Found {} exclusive/internal releases", exclusive_torrents.len());
        all_torrents.extend(exclusive_torrents);
        
        // 2. Collect high-quality encodes
        info!("Collecting high-quality encodes...");
        let hq_torrents = self.fetch_torrents_filtered(
            Some(vec![1]),     // Movies
            Some(vec![1, 5]),  // H.264, HEVC
            Some(vec![1, 3, 5, 6]), // Blu-ray, Encode, Remux, WEB-DL
            None,              // Any origin
            None,              // Any exclusivity
        ).await?;
        info!("Found {} high-quality releases", hq_torrents.len());
        all_torrents.extend(hq_torrents);
        
        // 3. Analyze and generate reports
        self.analyze_torrents(&all_torrents)?;
        self.generate_reports().await?;
        
        Ok(())
    }
    
    /// Fetch torrents with specific filters
    async fn fetch_torrents_filtered(
        &self,
        category: Option<Vec<u32>>,
        codec: Option<Vec<u32>>,
        medium: Option<Vec<u32>>,
        origin: Option<Vec<u32>>,
        exclusive: Option<Vec<u32>>,
    ) -> Result<Vec<ApiTorrent>> {
        let client = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        let mut all_torrents = Vec::new();
        let mut page = 0;
        
        while page < self.config.max_pages {
            // Build query parameters
            let mut params = vec![
                ("limit".to_string(), self.config.results_per_page.to_string()),
                ("page".to_string(), page.to_string()),
            ];
            
            if let Some(ref cats) = category {
                for cat in cats {
                    params.push(("category[]".to_string(), cat.to_string()));
                }
            }
            
            if let Some(ref codecs) = codec {
                for c in codecs {
                    params.push(("codec[]".to_string(), c.to_string()));
                }
            }
            
            if let Some(ref mediums) = medium {
                for m in mediums {
                    params.push(("medium[]".to_string(), m.to_string()));
                }
            }
            
            if let Some(ref origins) = origin {
                for o in origins {
                    params.push(("origin[]".to_string(), o.to_string()));
                }
            }
            
            if let Some(ref exclusives) = exclusive {
                for e in exclusives {
                    params.push(("exclusive[]".to_string(), e.to_string()));
                }
            }
            
            // Make API request
            let url = format!("{}/api/torrents", self.config.base_url);
            debug!("Fetching page {} with params: {:?}", page, params);
            
            let response = client
                .get(&url)
                .header("Cookie", &self.config.session_cookie)
                .query(&params)
                .send()
                .await?;
            
            if !response.status().is_success() {
                warn!("Failed to fetch page {}: {}", page, response.status());
                break;
            }
            
            let text = response.text().await?;
            
            // Try to parse as JSON array directly (API might return array not object)
            let torrents: Vec<ApiTorrent> = if text.trim().starts_with('[') {
                serde_json::from_str(&text)?
            } else {
                // Try as response object
                let resp: ApiTorrentResponse = serde_json::from_str(&text)?;
                resp.data
            };
            
            if torrents.is_empty() {
                info!("No more torrents at page {}", page);
                break;
            }
            
            info!("Fetched {} torrents from page {}", torrents.len(), page);
            all_torrents.extend(torrents);
            
            page += 1;
            
            // Rate limiting
            if page < self.config.max_pages {
                tokio::time::sleep(tokio::time::Duration::from_secs(self.config.request_delay_seconds)).await;
            }
        }
        
        Ok(all_torrents)
    }
    
    /// Analyze collected torrents and extract scene group data
    fn analyze_torrents(&mut self, torrents: &[ApiTorrent]) -> Result<()> {
        info!("Analyzing {} torrents for scene group patterns", torrents.len());
        
        for torrent in torrents {
            // Extract scene group from release name
            let group_name = self.extract_scene_group(&torrent.name, torrent.type_exclusive == 1);
            
            let stats = self.scene_groups.entry(group_name.clone()).or_insert_with(|| {
                SceneGroupStats {
                    name: group_name,
                    release_count: 0,
                    total_size_gb: 0.0,
                    avg_seeders: 0.0,
                    avg_snatched: 0.0,
                    exclusive_count: 0,
                    internal_count: 0,
                    quality_score: 0.0,
                    codecs: HashMap::new(),
                    mediums: HashMap::new(),
                }
            });
            
            // Update statistics
            stats.release_count += 1;
            stats.total_size_gb += torrent.size as f64 / 1_073_741_824.0;
            stats.avg_seeders = (stats.avg_seeders * (stats.release_count - 1) as f32 + torrent.seeders as f32) / stats.release_count as f32;
            stats.avg_snatched = (stats.avg_snatched * (stats.release_count - 1) as f32 + torrent.times_completed as f32) / stats.release_count as f32;
            
            if torrent.type_exclusive == 1 {
                stats.exclusive_count += 1;
            }
            
            if torrent.type_origin == 1 {
                stats.internal_count += 1;
            }
            
            // Track codec distribution
            let codec_name = self.codec_id_to_name(torrent.type_codec);
            *stats.codecs.entry(codec_name).or_insert(0) += 1;
            
            // Track medium distribution
            let medium_name = self.medium_id_to_name(torrent.type_medium);
            *stats.mediums.entry(medium_name).or_insert(0) += 1;
        }
        
        // Calculate quality scores
        for stats in self.scene_groups.values_mut() {
            stats.quality_score = self.calculate_quality_score(stats);
        }
        
        info!("Identified {} unique scene groups", self.scene_groups.len());
        Ok(())
    }
    
    /// Extract scene group from release name
    fn extract_scene_group(&self, name: &str, is_exclusive: bool) -> String {
        // If exclusive/internal, use special group name
        if is_exclusive {
            return "EXCLUSIVE".to_string();
        }
        
        // Common scene group patterns
        let patterns = [
            r"-([A-Za-z0-9]+)$",           // Standard: Movie.2024.1080p.BluRay.x264-GROUP
            r"-([A-Za-z0-9]+)\[",          // With tags: -GROUP[tag]
            r"-([A-Za-z0-9]+)\s",          // With space: -GROUP something
            r"\.([A-Za-z0-9]+)$",          // Dot separator: Movie.2024.GROUP
        ];
        
        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(name) {
                    if let Some(group) = captures.get(1) {
                        let group_name = group.as_str();
                        // Filter out common false positives
                        if !["1080p", "720p", "2160p", "x264", "x265", "h264", "h265", "BluRay", "WEB", "HDTV"]
                            .contains(&group_name) {
                            return group_name.to_uppercase();
                        }
                    }
                }
            }
        }
        
        "UNKNOWN".to_string()
    }
    
    /// Calculate quality score for a scene group
    fn calculate_quality_score(&self, stats: &SceneGroupStats) -> f64 {
        let mut score = 0.0;
        
        // Base score from release count (logarithmic)
        score += (stats.release_count as f64).ln() * 10.0;
        
        // Bonus for exclusive/internal releases (very high quality)
        score += stats.exclusive_count as f64 * 50.0;
        score += stats.internal_count as f64 * 30.0;
        
        // Average seeders (health indicator)
        score += stats.avg_seeders as f64 * 2.0;
        
        // Average snatched (popularity)
        score += stats.avg_snatched as f64 * 0.5;
        
        // Codec quality bonus
        if stats.codecs.contains_key("HEVC") || stats.codecs.contains_key("x265") {
            score += 15.0;
        }
        
        // Medium quality bonus
        if stats.mediums.contains_key("Blu-ray") || stats.mediums.contains_key("Remux") {
            score += 20.0;
        }
        
        score
    }
    
    /// Convert codec ID to name
    fn codec_id_to_name(&self, id: u32) -> String {
        match id {
            1 => "H.264",
            2 => "MPEG-2",
            3 => "VC-1",
            4 => "XviD",
            5 => "HEVC",
            _ => "Unknown",
        }.to_string()
    }
    
    /// Convert medium ID to name
    fn medium_id_to_name(&self, id: u32) -> String {
        match id {
            1 => "Blu-ray",
            3 => "Encode",
            4 => "Capture",
            5 => "Remux",
            6 => "WEB-DL",
            _ => "Unknown",
        }.to_string()
    }
    
    /// Generate analysis reports
    async fn generate_reports(&self) -> Result<()> {
        info!("Generating analysis reports...");
        
        // Sort groups by quality score
        let mut sorted_groups: Vec<_> = self.scene_groups.values().collect();
        sorted_groups.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());
        
        // Generate JSON report
        let json_path = self.config.output_dir.join("api_analysis.json");
        let json_data = serde_json::to_string_pretty(&sorted_groups)?;
        fs::write(&json_path, json_data)?;
        info!("JSON report saved to: {:?}", json_path);
        
        // Generate CSV report
        let csv_path = self.config.output_dir.join("api_analysis.csv");
        let mut csv_content = String::from("group_name,quality_score,releases,exclusive,internal,avg_seeders,avg_snatched,total_gb\n");
        
        for group in sorted_groups.iter().take(50) {  // Top 50 groups
            csv_content.push_str(&format!(
                "{},{:.1},{},{},{},{:.1},{:.1},{:.1}\n",
                group.name,
                group.quality_score,
                group.release_count,
                group.exclusive_count,
                group.internal_count,
                group.avg_seeders,
                group.avg_snatched,
                group.total_size_gb
            ));
        }
        
        fs::write(&csv_path, csv_content)?;
        info!("CSV report saved to: {:?}", csv_path);
        
        // Print top 10 groups
        println!("\n📊 Top 10 Scene Groups by Quality Score:");
        println!("{:-<80}", "");
        println!("{:<20} {:>10} {:>10} {:>10} {:>10} {:>10}", 
                 "Group", "Score", "Releases", "Exclusive", "Seeders", "Snatched");
        println!("{:-<80}", "");
        
        for group in sorted_groups.iter().take(10) {
            println!("{:<20} {:>10.1} {:>10} {:>10} {:>10.1} {:>10.1}",
                     group.name,
                     group.quality_score,
                     group.release_count,
                     group.exclusive_count,
                     group.avg_seeders,
                     group.avg_snatched
            );
        }
        
        Ok(())
    }
}