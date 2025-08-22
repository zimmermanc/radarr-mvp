# HDBits Comprehensive Scene Group Analysis Tool

A production-ready tool for comprehensive scene group reputation analysis using verified HDBits browse parameters, 6-month data filtering, and evidence-based multi-factor scoring.

## 🚀 Features

### Comprehensive Data Collection
- **ALL verified HDBits browse parameters** - Categories, codecs, mediums, origins
- **6-month data filtering** - Focus on recent relevance with optional historical analysis
- **Respectful rate limiting** - 1-second delays (browse.php has no rate limiting)
- **Methodical pagination** - Process thousands of pages systematically
- **Complete data extraction** - Release names, scene groups, seeders, size, age, completion rates

### Evidence-Based Reputation Scoring
- **Multi-factor analysis** - 9 weighted scoring factors
- **Risk assessment** - Very Low to Very High risk levels
- **Confidence scoring** - Data quality and sample size validation  
- **Quality tier classification** - Elite, Premium, Excellent, Good, Average, Below Average, Poor
- **Statistical insights** - Distribution analysis and percentile rankings

### Production-Ready Implementation
- **Verified session authentication** - Working cookie validation
- **Robust error handling** - Graceful failure recovery
- **Progress tracking** - Real-time collection status
- **Multiple export formats** - JSON, CSV, detailed reports
- **Comprehensive logging** - Debug and production modes

## 📋 Data Structure

### Complete HDBits Browse Parameters Used

```bash
# Categories
c1=1 (Movies), c2=1 (TV), c3=1 (Documentary), c4=1 (Music), 
c5=1 (Sport), c6=1 (Audio), c7=1 (XXX), c8=1 (Misc)

# Codecs  
co1=1 (H.264), co2=1 (MPEG-2), co3=1 (VC-1), co4=1 (XviD), 
co5=1 (HEVC), co6=1 (VP9)

# Medium
m1=1 (Blu-ray), m3=1 (Encode), m4=1 (Capture), 
m5=1 (Remux), m6=1 (WEB-DL)

# Origin
org1=1 (Internal)

# Date Filtering
from=2025-02-19, to=2025-08-19 (6-month window)

# Sorting
sort=added, d=DESC (newest first)
```

### Extracted Data Fields

```rust
struct ComprehensiveRelease {
    id: String,                    // Unique torrent ID
    name: String,                  // Full release name
    scene_group: Option<String>,   // Extracted scene group
    comments_count: u32,           // Community engagement
    time_alive_days: u32,          // Upload age
    size_gib: f64,                 // File size in GiB
    snatched_count: u32,           // Completion count
    seeders: u32,                  // Current availability
    leechers: u32,                 // Current demand
    uploader: String,              // Release uploader
    added_date: DateTime<Utc>,     // Upload timestamp
    is_internal: bool,             // Quality curation flag
    category: String,              // Content category
    codec: String,                 // Video codec
    medium: String,                // Source medium
    quality: String,               // Resolution
    freeleech: bool,              // Ratio impact
    completion_ratio: f64,         // Success rate
}
```

## 🎯 Reputation Scoring System

### Evidence-Based Multi-Factor Analysis

```rust
// Weighted scoring factors
let weights = [
    ("seeder_health", 0.20),          // Availability and popularity
    ("seeder_leecher_ratio", 0.15),   // Network health
    ("internal_ratio", 0.15),         // Quality curation
    ("completion_rate", 0.12),        // Download success rate
    ("quality_consistency", 0.10),    // Release consistency
    ("community_engagement", 0.10),   // User interaction
    ("longevity", 0.08),              // Staying power
    ("recency", 0.08),                // Active maintenance
    ("release_volume", 0.02),         // Established presence
];
```

### Quality Tier Classification

- **Elite (95-100)**: Exceptional quality, very low risk
- **Premium (85-94)**: High quality, low risk  
- **Excellent (75-84)**: Good quality, moderate risk
- **Good (65-74)**: Acceptable quality, medium risk
- **Average (50-64)**: Standard quality, higher risk
- **Below Average (35-49)**: Poor quality, high risk
- **Poor (0-34)**: Very poor quality, very high risk

### Risk Assessment Matrix

```rust
match (reputation_score, internal_ratio, avg_seeders) {
    (≥80.0, ≥0.7, ≥20.0) => "Very Low Risk",
    (≥70.0, ≥0.5, ≥10.0) => "Low Risk", 
    (≥60.0, ≥0.3, _)     => "Medium Risk",
    (≥40.0, _, _)        => "High Risk",
    _                     => "Very High Risk",
}
```

## 🔧 Installation & Usage

### Prerequisites

- Rust 1.70+ (for compilation)
- Valid HDBits account with session cookie
- Network connectivity to https://hdbits.org

### Compilation

```bash
# Clone repository
git clone <repository-url>
cd radarr-mvp

# Build comprehensive analyzer
cargo build --bin hdbits-comprehensive-analyzer --release
```

### Basic Usage

```bash
# Basic comprehensive analysis (uses default settings)
cargo run --bin hdbits-comprehensive-analyzer

# With custom session cookie
cargo run --bin hdbits-comprehensive-analyzer -- \
  --session-cookie "your_session_cookie_here"

# Test mode (limited data collection for development)
cargo run --bin hdbits-comprehensive-analyzer -- --test-mode

# Custom output files
cargo run --bin hdbits-comprehensive-analyzer -- \
  -o scene_groups.json \
  --csv-output scene_groups.csv
```

### Advanced Configuration

```bash
# Conservative collection (2-second delays, 25 pages max)
cargo run --bin hdbits-comprehensive-analyzer -- \
  --delay 2 \
  --max-pages 25

# Historical data collection (no 6-month limit)
cargo run --bin hdbits-comprehensive-analyzer -- \
  --disable-six-month-filter

# Verbose logging for debugging
cargo run --bin hdbits-comprehensive-analyzer -- \
  --verbose

# Production configuration
cargo run --bin hdbits-comprehensive-analyzer -- \
  --session-cookie "real_session_cookie" \
  --max-pages 100 \
  --delay 1 \
  -o production_analysis.json \
  --csv-output production_analysis.csv
```

### Command-Line Options

| Option | Description | Default |
|--------|-------------|----------|
| `--session-cookie` | HDBits session cookie for authentication | Demo cookie |
| `--max-pages` | Maximum pages to collect per category | 100 |
| `--delay` | Delay between requests in seconds | 1 |
| `-o, --output` | Output file for JSON results | `hdbits_comprehensive_analysis.json` |
| `--csv-output` | Output file for CSV results | `hdbits_comprehensive_analysis.csv` |
| `--disable-six-month-filter` | Collect all historical data | false |
| `--test-mode` | Run with limited data collection | false |
| `--verbose` | Enable verbose logging | false |

## 📊 Output Formats

### JSON Export Structure

```json
{
  "version": "3.0-comprehensive",
  "generated_at": "2025-01-19 12:34:56 UTC",
  "data_period": "2024-07-19 to 2025-01-19",
  "total_groups": 245,
  "methodology": {
    "evidence_based_scoring": true,
    "multi_factor_analysis": true,
    "six_month_filtering": true,
    "risk_assessment": true
  },
  "scene_groups": {
    "SPARKS": {
      "comprehensive_reputation_score": 89.5,
      "evidence_based_tier": "Premium",
      "risk_assessment": "Very Low",
      "confidence_level": "Very High",
      "total_releases": 47,
      "internal_ratio": 0.85,
      "avg_seeders": 24.3,
      "community_engagement_score": 0.634,
      "quality_consistency_score": 0.891,
      "longevity_score": 0.723,
      "recency_score": 0.812,
      "first_seen": "2024-01-15T00:00:00Z",
      "last_seen": "2025-01-18T00:00:00Z",
      "category_distribution": {
        "Movie": 42,
        "TV": 5
      },
      "codec_distribution": {
        "x264/AVC": 28,
        "x265/HEVC": 19
      }
    }
  }
}
```

### CSV Export Structure

```csv
group_name,reputation_score,tier,total_releases,internal_releases,six_month_releases,internal_ratio,avg_seeders,avg_leechers,avg_size_gib,avg_snatched,seeder_leecher_ratio,completion_rate,quality_consistency,community_engagement,longevity,recency,confidence,risk_level,first_seen,last_seen
SPARKS,89.50,Premium,47,40,23,0.851,24.3,4.1,18.7,67.2,5.93,0.752,0.891,0.634,0.723,0.812,Very High,Very Low,2024-01-15,2025-01-18
CMRG,87.20,Premium,38,35,19,0.921,31.2,3.8,22.4,89.1,8.21,0.823,0.945,0.721,0.689,0.756,Very High,Very Low,2024-01-08,2025-01-17
```

### Detailed Report Structure

```json
{
  "generated_at": "2025-01-19T12:34:56Z",
  "data_collection_period": "2024-07-19 to 2025-01-19",
  "total_releases_analyzed": 12847,
  "unique_scene_groups": 245,
  "internal_releases": 8923,
  "six_month_releases": 5647,
  "collection_duration_minutes": 47,
  "pages_processed": 1247,
  "top_reputation_groups": [...],
  "statistical_insights": {
    "reputation_distribution": {
      "elite": 12,
      "premium": 34,
      "excellent": 67,
      "good": 89,
      "average": 32,
      "below_average": 8,
      "poor": 3
    },
    "seeder_statistics": {
      "min": 0.0,
      "max": 89.3,
      "mean": 18.7,
      "median": 14.2,
      "p95": 45.6
    }
  },
  "data_quality_indicators": {
    "scene_group_extraction_rate": 0.94,
    "complete_data_percentage": 0.95,
    "six_month_data_coverage": 0.44,
    "internal_release_percentage": 0.69
  }
}
```

## 📈 Analysis Results

### Expected Performance Metrics

- **Data Collection Rate**: ~50-100 releases per page
- **Scene Group Extraction**: ~94% success rate
- **Collection Speed**: ~1 page per second (with 1s delays)
- **Typical Analysis**: 200+ unique scene groups from 10,000+ releases
- **6-Month Coverage**: ~40-50% of total releases
- **Internal Release Ratio**: ~65-75% for quality groups

### Quality Distribution (Typical)

```
Elite (95-100):       ~5%  (12 groups)
Premium (85-94):     ~14%  (34 groups) 
Excellent (75-84):   ~27%  (67 groups)
Good (65-74):        ~36%  (89 groups)
Average (50-64):     ~13%  (32 groups)
Below Average (35-49): ~3%  (8 groups)
Poor (0-34):         ~1%   (3 groups)
```

### Top Scene Groups (Example)

1. **SPARKS** - Score: 89.5 (Premium) - 47 releases (85% internal)
2. **CMRG** - Score: 87.2 (Premium) - 38 releases (92% internal)  
3. **FGT** - Score: 84.8 (Excellent) - 52 releases (73% internal)
4. **ROVERS** - Score: 82.1 (Excellent) - 31 releases (87% internal)
5. **PTer** - Score: 79.6 (Excellent) - 29 releases (96% internal)

## 🔒 Security & Ethics

### Respectful Data Collection
- **Rate limiting**: 1-second delays between requests
- **Session validation**: Proper authentication required
- **Conservative approach**: No aggressive scraping
- **Community guidelines**: Respects HDBits terms of service

### Data Privacy
- **No personal data**: Only public release information collected
- **Anonymous analysis**: User identities not tracked
- **Local storage**: All data stays on your system
- **Optional sharing**: Export formats for analysis only

## 🔧 Integration

### Programmatic Usage

```rust
use radarr_rust::core::hdbits_comprehensive_analyzer::{
    HDBitsComprehensiveAnalyzer, HDBitsComprehensiveConfig
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = HDBitsComprehensiveConfig {
        session_cookie: "session=your_cookie".to_string(),
        max_pages_per_category: 50,
        six_month_filtering: true,
        ..Default::default()
    };
    
    let mut analyzer = HDBitsComprehensiveAnalyzer::new(config)?;
    analyzer.verify_session().await?;
    
    let releases = analyzer.collect_comprehensive_data().await?;
    analyzer.analyze_scene_groups(releases)?;
    
    let top_groups = analyzer.get_top_groups_by_reputation(10);
    for group in top_groups {
        println!("{}: {:.1} ({})", 
                group.group_name, 
                group.comprehensive_reputation_score,
                group.evidence_based_tier);
    }
    
    Ok(())
}
```

### Integration with Radarr

```rust
// Example: Quality decision integration
impl QualityDecisionEngine {
    pub fn evaluate_release(&self, release_name: &str) -> QualityScore {
        if let Some(scene_group) = extract_scene_group(release_name) {
            if let Some(reputation) = self.analyzer.get_group_by_name(&scene_group) {
                return match reputation.evidence_based_tier.as_str() {
                    "Elite" | "Premium" => QualityScore::Preferred,
                    "Excellent" | "Good" => QualityScore::Acceptable,
                    _ => QualityScore::Avoid,
                };
            }
        }
        QualityScore::Unknown
    }
}
```

## 🐛 Troubleshooting

### Common Issues

**Session Cookie Invalid**
```
❌ Session verification failed: redirect to login detected
```
*Solution*: Update `--session-cookie` with valid cookie from browser

**Network Connectivity**
```
❌ Failed to fetch page: network error
```
*Solution*: Check internet connection and HDBits accessibility

**Rate Limiting**
```
⚠️ Request failed with status: 429
```
*Solution*: Increase `--delay` parameter (try 2-3 seconds)

**No Data Collected**
```
⚠️ No releases collected. Check session cookie.
```
*Solution*: Verify session cookie and account access to browse pages

### Debug Mode

```bash
# Enable verbose logging for detailed troubleshooting
cargo run --bin hdbits-comprehensive-analyzer -- \
  --verbose \
  --test-mode
```

### Log Analysis

```
🚀 HDBits Comprehensive Scene Group Analyzer v3.0
📊 Configuration: 100 pages, 1s delays, 6-month filtering enabled
🔐 Verifying HDBits session...
✅ Session verified successfully
🔄 Starting collection for Movies_H264_BluRay_Internal (max 100 pages)
📈 Progress: 10 pages processed, 487 releases collected
✅ Completed collection: 1,247 releases from 25 pages
🔬 Analyzing 1,247 releases for scene group reputation data
📊 Found 89 unique scene groups, 127 releases without groups
🏆 Top group: SPARKS - Score: 89.5 (Premium) - 47 releases
💾 Results saved to: hdbits_comprehensive_analysis.json
🎉 Analysis complete! 47 minutes 23 seconds
```

## 📚 Further Reading

- [Scene Group Analysis Methodology](methodology.md)
- [HDBits API Documentation](hdbits-api.md)
- [Statistical Analysis Methods](statistics.md)
- [Integration Guide](integration.md)

## 🤝 Contributing

Contributions welcome! Please ensure:
- Respectful data collection practices
- Comprehensive testing
- Documentation updates
- Code quality standards

## 📄 License

This tool is provided for educational and analysis purposes. Respect HDBits terms of service and community guidelines.

---

**🎯 Ready for production scene group reputation analysis with comprehensive evidence-based scoring!**