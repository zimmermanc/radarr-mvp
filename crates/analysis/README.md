# Radarr Analysis

HDBits scene group analysis and reputation scoring system for the Radarr movie automation system. This crate provides comprehensive analysis tools for evaluating scene groups, tracking quality metrics, and making evidence-based release selection decisions.

## Features

- **Scene Group Analysis**: Comprehensive reputation scoring for HDBits scene groups
- **Quality Metrics**: Statistical analysis of group performance and reliability
- **Reputation Scoring**: Evidence-based scoring algorithms for automatic decision making
- **Browse Analysis**: Web interface scraping for detailed group information
- **Session Management**: Authenticated session handling for HDBits integration
- **API Integration**: RESTful API client for programmatic access
- **Comprehensive Reporting**: Detailed analysis reports with actionable insights
- **Data Persistence**: Long-term tracking of group performance over time

## Key Dependencies

- **radarr-indexers**: HDBits client integration for data collection
- **tokio**: Async runtime for concurrent analysis operations
- **futures**: Stream processing for large data analysis
- **reqwest**: HTTP client for web scraping and API access
- **scraper**: HTML parsing for web interface analysis
- **regex**: Pattern matching for release name analysis
- **chrono**: Date/time handling for temporal analysis
- **serde/serde_json**: Data serialization for reports and caching
- **uuid**: Unique identifiers for analysis sessions

## Scene Group Analysis

### Reputation Scoring System

```rust
use radarr_analysis::{HDBitsComprehensiveAnalyzer, HDBitsComprehensiveConfig};

let config = HDBitsComprehensiveConfig {
    hdbits_username: "your_username".to_string(),
    hdbits_passkey: "your_passkey".to_string(),
    analysis_depth: AnalysisDepth::Comprehensive,
    min_releases_threshold: 10,
    quality_weight: 0.4,
    popularity_weight: 0.3,
    consistency_weight: 0.3,
};

let analyzer = HDBitsComprehensiveAnalyzer::new(config);

// Analyze specific scene group
let group_analysis = analyzer.analyze_scene_group("SPARKS").await?;

println!("Group: {}", group_analysis.group_name);
println!("Reputation Score: {:.2}/10", group_analysis.reputation_score);
println!("Quality Rating: {:.2}/10", group_analysis.quality_rating);
println!("Release Count: {}", group_analysis.total_releases);
println!("Average Seeds: {:.1}", group_analysis.avg_seeders);
println!("Consistency Score: {:.2}", group_analysis.consistency_score);

// Detailed metrics
for metric in group_analysis.quality_metrics {
    println!("  {}: {:.2}", metric.category, metric.score);
}
```

### Batch Group Analysis

```rust
use futures::StreamExt;

let scene_groups = vec!["SPARKS", "FGT", "DON", "HONE", "DRONES"];
let mut analysis_stream = analyzer.analyze_groups_stream(scene_groups).await?;

while let Some(result) = analysis_stream.next().await {
    match result {
        Ok(analysis) => {
            println!("{}: Score {:.2}/10 ({} releases)", 
                analysis.group_name, 
                analysis.reputation_score,
                analysis.total_releases
            );
        }
        Err(e) => {
            println!("Analysis failed: {}", e);
        }
    }
}
```

## Browse Interface Analysis

### Web Scraping for Detailed Metrics

```rust
use radarr_analysis::{HDBitsBrowseAnalyzer, HDBitsBrowseConfig};

let config = HDBitsBrowseConfig {
    session_cookie: "your_session_cookie".to_string(),
    max_pages_per_group: 5,
    concurrent_requests: 3,
    delay_between_requests: Duration::from_millis(500),
};

let browse_analyzer = HDBitsBrowseAnalyzer::new(config);

// Analyze group through browse interface
let browse_analysis = browse_analyzer.analyze_group_releases("SPARKS").await?;

println!("Analyzed {} releases", browse_analysis.releases_analyzed);
println!("Quality Distribution:");
for (quality, count) in browse_analysis.quality_distribution {
    println!("  {}: {} releases", quality, count);
}

println!("Seeder Statistics:");
println!("  Average: {:.1}", browse_analysis.seeder_stats.average);
println!("  Median: {}", browse_analysis.seeder_stats.median);
println!("  95th percentile: {}", browse_analysis.seeder_stats.p95);
```

## Session Management

### Authenticated HDBits Session

```rust
use radarr_analysis::{HDBitsSessionAnalyzer, HDBitsSessionConfig};

let config = HDBitsSessionConfig {
    username: "your_username".to_string(),
    password: "your_password".to_string(),
    session_timeout: Duration::from_hours(12),
    auto_renewal: true,
};

let session_analyzer = HDBitsSessionAnalyzer::new(config);

// Ensure authenticated session
let session_valid = session_analyzer.ensure_valid_session().await?;
if session_valid {
    println!("Session authenticated successfully");
    
    // Perform authenticated analysis
    let user_stats = session_analyzer.get_user_statistics().await?;
    println!("User ratio: {:.2}", user_stats.ratio);
    println!("Download count: {}", user_stats.downloads);
}
```

## API Integration

### RESTful API Client

```rust
use radarr_analysis::{HDBitsApiAnalyzer, ApiAnalyzerConfig};

let config = ApiAnalyzerConfig {
    api_endpoint: "https://hdbits.org/api".to_string(),
    api_key: "your_api_key".to_string(),
    rate_limit_per_hour: 100,
    timeout_seconds: 30,
};

let api_analyzer = HDBitsApiAnalyzer::new(config);

// Get trending releases with analysis
let trending_analysis = api_analyzer.analyze_trending_releases().await?;

for release_analysis in trending_analysis.releases {
    println!("{} - Group: {} (Score: {:.2})",
        release_analysis.title,
        release_analysis.group_name,
        release_analysis.group_reputation_score
    );
}
```

## Analysis Reports

### Comprehensive Group Reports

```rust
use radarr_analysis::{AnalysisReport, ReportFormat};

// Generate comprehensive report
let report = analyzer.generate_comprehensive_report(
    &["SPARKS", "FGT", "DON"], 
    ReportFormat::Json
).await?;

// Report contains:
// - Overall scene group rankings
// - Quality trend analysis
// - Recommendation scores
// - Historical performance data
// - Statistical confidence intervals

match report.format {
    ReportFormat::Json => {
        let json_report = serde_json::to_string_pretty(&report.data)?;
        std::fs::write("analysis_report.json", json_report)?;
    }
    ReportFormat::Html => {
        std::fs::write("analysis_report.html", report.html_content)?;
    }
}
```

### Quality Trend Analysis

```rust
use radarr_analysis::QualityTrendAnalysis;

let trend_analysis = analyzer.analyze_quality_trends(
    "SPARKS", 
    Duration::from_days(90)
).await?;

println!("Quality Trends for SPARKS (90 days):");
println!("  Improvement trend: {:.2}%", trend_analysis.improvement_percentage);
println!("  Consistency trend: {:.2}", trend_analysis.consistency_change);

for period in trend_analysis.time_periods {
    println!("  {}: Score {:.2}/10", 
        period.date_range, 
        period.average_score
    );
}
```

## Binary Tools

### Command-Line Analysis Tools

The crate includes several binary tools for standalone analysis:

```bash
# Comprehensive scene group analysis
cargo run --bin hdbits-comprehensive-analyzer -- \
    --groups SPARKS,FGT,DON \
    --output analysis_report.json

# Browse interface analysis
cargo run --bin hdbits-browse-analyzer -- \
    --group SPARKS \
    --max-pages 10 \
    --output browse_analysis.json

# Session-based analysis
cargo run --bin hdbits-session-analyzer -- \
    --username your_user \
    --password your_pass \
    --analyze-user-stats

# API-based analysis
cargo run --bin hdbits-api-analyzer -- \
    --api-key your_key \
    --trending \
    --top-groups 20
```

## Data Models

### Scene Group Analysis Result

```rust
use radarr_analysis::{SceneGroupAnalysis, QualityMetric, ConsistencyMetric};

pub struct SceneGroupAnalysis {
    pub group_name: String,
    pub reputation_score: f64,        // 0.0 - 10.0
    pub quality_rating: f64,          // 0.0 - 10.0  
    pub consistency_score: f64,       // 0.0 - 1.0
    pub total_releases: u32,
    pub avg_seeders: f64,
    pub avg_file_size_gb: f64,
    pub quality_metrics: Vec<QualityMetric>,
    pub consistency_metrics: ConsistencyMetric,
    pub recommendation: GroupRecommendation,
    pub confidence_interval: (f64, f64),
    pub analysis_date: DateTime<Utc>,
}

pub enum GroupRecommendation {
    HighlyRecommended,      // Score >= 8.0
    Recommended,            // Score >= 6.0
    Acceptable,             // Score >= 4.0
    Questionable,           // Score >= 2.0
    NotRecommended,         // Score < 2.0
}
```

### Quality Metrics

```rust
pub struct QualityMetric {
    pub category: QualityCategory,
    pub score: f64,
    pub sample_size: u32,
    pub confidence: f64,
}

pub enum QualityCategory {
    VideoQuality,           // Encoding quality assessment
    AudioQuality,           // Audio track quality
    FileIntegrity,          // File corruption/completeness
    ReleaseSpeed,           // How quickly releases appear
    NamingConsistency,      // Naming convention adherence
    TechnicalAccuracy,      // Proper codec/quality reporting
}
```

## Integration Examples

### With Decision Engine

```rust
use radarr_decision::{CustomFormat, FormatSpecification};
use radarr_analysis::SceneGroupAnalysis;

async fn create_dynamic_format_from_analysis(
    analysis: &SceneGroupAnalysis
) -> CustomFormat {
    let score = match analysis.recommendation {
        GroupRecommendation::HighlyRecommended => 200,
        GroupRecommendation::Recommended => 100,
        GroupRecommendation::Acceptable => 50,
        GroupRecommendation::Questionable => -50,
        GroupRecommendation::NotRecommended => -200,
    };
    
    CustomFormat {
        name: format!("HDBits Group: {}", analysis.group_name),
        score,
        specifications: vec![
            FormatSpecification::ReleaseName {
                pattern: format!(r"-{}$", analysis.group_name),
                negate: false,
            }
        ],
        // ... other fields
    }
}
```

### With Core Domain

```rust
use radarr_core::{Release, QualityRepository};

async fn enrich_release_with_analysis(
    release: &mut Release,
    analyzer: &HDBitsComprehensiveAnalyzer,
) -> Result<()> {
    if let Some(group) = extract_group_from_release_name(&release.title) {
        let analysis = analyzer.get_cached_analysis(&group)
            .await
            .or_else(|_| analyzer.analyze_scene_group(&group).await)?;
        
        // Enrich release with analysis data
        release.group_reputation_score = Some(analysis.reputation_score);
        release.group_recommendation = Some(analysis.recommendation);
        release.analysis_confidence = Some(analysis.confidence_interval);
    }
    
    Ok(())
}
```

## Configuration

### Environment Variables

```bash
# HDBits credentials
HDBITS_USERNAME=your_username  
HDBITS_PASSKEY=your_passkey
HDBITS_SESSION_COOKIE=your_session_cookie

# Analysis settings
ANALYSIS_CACHE_TTL_HOURS=24
ANALYSIS_MIN_SAMPLE_SIZE=10
ANALYSIS_CONCURRENT_REQUESTS=3

# Reporting settings
ANALYSIS_REPORTS_DIR=/var/lib/radarr/analysis
ANALYSIS_ENABLE_HTML_REPORTS=true
```

### Configuration Files

```json
{
  "comprehensiveAnalysis": {
    "analysisDepth": "Comprehensive",
    "minReleasesThreshold": 10,
    "qualityWeight": 0.4,
    "popularityWeight": 0.3,
    "consistencyWeight": 0.3
  },
  "sessionConfig": {
    "sessionTimeout": "12h",
    "autoRenewal": true,
    "maxRetries": 3
  },
  "reportingConfig": {
    "outputFormat": ["json", "html"],
    "includeCharts": true,
    "confidenceThreshold": 0.8
  }
}
```

## Testing

### Unit Tests

```bash
# Run analysis tests
cargo test -p radarr-analysis

# Test specific analyzers
cargo test -p radarr-analysis comprehensive_analyzer::tests
cargo test -p radarr-analysis browse_analyzer::tests
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires HDBits credentials
async fn test_comprehensive_analysis() {
    let config = HDBitsComprehensiveConfig::from_env().unwrap();
    let analyzer = HDBitsComprehensiveAnalyzer::new(config);
    
    let analysis = analyzer.analyze_scene_group("SPARKS").await.unwrap();
    
    assert!(!analysis.group_name.is_empty());
    assert!(analysis.reputation_score >= 0.0 && analysis.reputation_score <= 10.0);
    assert!(analysis.total_releases > 0);
}
```

## Performance Considerations

- **Caching**: Intelligent caching of analysis results to reduce API calls
- **Rate Limiting**: Respect HDBits rate limits and terms of service
- **Concurrent Processing**: Parallel analysis of multiple groups
- **Incremental Updates**: Update only changed data rather than full re-analysis
- **Memory Efficiency**: Stream processing for large datasets
- **Database Storage**: Persistent storage of analysis results for trend tracking