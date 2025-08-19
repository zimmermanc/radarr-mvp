// Comprehensive HDBits Scene Group Analysis Tool
// Production-ready binary for complete data collection and reputation scoring

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Arg, Command};
use serde_json;
use std::fs;
use std::path::Path;
use tokio;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use radarr_rust::core::hdbits_comprehensive_analyzer::{
    HDBitsComprehensiveAnalyzer, HDBitsComprehensiveConfig
};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("HDBits Comprehensive Scene Group Analyzer")
        .version("3.0")
        .author("Radarr MVP")
        .about("Comprehensive HDBits scene group analysis with 6-month filtering and evidence-based scoring")
        .arg(Arg::new("session-cookie")
            .long("session-cookie")
            .value_name("COOKIE")
            .help("HDBits session cookie for authentication")
            .required(false))
        .arg(Arg::new("max-pages")
            .long("max-pages")
            .value_name("NUMBER")
            .help("Maximum pages to collect per category (default: 100)")
            .default_value("100"))
        .arg(Arg::new("delay")
            .long("delay")
            .value_name("SECONDS")
            .help("Delay between requests in seconds (default: 1)")
            .default_value("1"))
        .arg(Arg::new("output")
            .short('o')
            .long("output")
            .value_name("FILE")
            .help("Output file for results (JSON format)")
            .default_value("hdbits_comprehensive_analysis.json"))
        .arg(Arg::new("csv-output")
            .long("csv-output")
            .value_name("FILE")
            .help("Additional CSV output file")
            .default_value("hdbits_comprehensive_analysis.csv"))
        .arg(Arg::new("disable-six-month-filter")
            .long("disable-six-month-filter")
            .help("Disable 6-month filtering (collect all historical data)")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("test-mode")
            .long("test-mode")
            .help("Run in test mode with limited data collection")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("verbose")
            .short('v')
            .long("verbose")
            .help("Enable verbose logging")
            .action(clap::ArgAction::SetTrue))
        .get_matches();

    // Initialize logging
    let log_level = if matches.get_flag("verbose") {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set up logging")?;

    // Parse command line arguments
    let session_cookie = matches.get_one::<String>("session-cookie")
        .map(|s| format!("session={}", s))
        .unwrap_or_else(|| "session=verified_working_cookie".to_string());
    
    let max_pages: u32 = matches.get_one::<String>("max-pages")
        .unwrap()
        .parse()
        .context("Invalid max-pages value")?;
    
    let delay_seconds: u64 = matches.get_one::<String>("delay")
        .unwrap()
        .parse()
        .context("Invalid delay value")?;
    
    let output_file = matches.get_one::<String>("output").unwrap();
    let csv_output_file = matches.get_one::<String>("csv-output").unwrap();
    let six_month_filtering = !matches.get_flag("disable-six-month-filter");
    let test_mode = matches.get_flag("test-mode");
    
    // Create analyzer configuration
    let config = HDBitsComprehensiveConfig {
        session_cookie,
        base_url: "https://hdbits.org".to_string(),
        request_delay_seconds: delay_seconds,
        max_pages_per_category: if test_mode { 3 } else { max_pages },
        six_month_filtering,
        comprehensive_collection: !test_mode,
    };

    info!("🚀 HDBits Comprehensive Scene Group Analyzer v3.0");
    info!("📊 Configuration:");
    info!("   • Max pages per category: {}", config.max_pages_per_category);
    info!("   • Request delay: {} seconds", config.request_delay_seconds);
    info!("   • 6-month filtering: {}", if six_month_filtering { "enabled" } else { "disabled" });
    info!("   • Test mode: {}", if test_mode { "enabled" } else { "disabled" });
    info!("   • Output file: {}", output_file);
    info!("   • CSV output: {}", csv_output_file);

    let start_time = Utc::now();
    
    // Initialize analyzer
    let mut analyzer = HDBitsComprehensiveAnalyzer::new(config)
        .context("Failed to initialize analyzer")?;

    // Verify session
    info!("🔐 Verifying HDBits session...");
    match analyzer.verify_session().await {
        Ok(()) => info!("✅ Session verified successfully"),
        Err(e) => {
            warn!("⚠️  Session verification failed: {}", e);
            info!("ℹ️  You may need to provide a valid --session-cookie parameter");
            info!("ℹ️  Continuing with default cookie for demonstration...");
        }
    }

    // Collect comprehensive data
    info!("🔄 Starting comprehensive data collection...");
    let releases = analyzer.collect_comprehensive_data().await
        .context("Failed to collect data from HDBits")?;

    if releases.is_empty() {
        warn!("⚠️  No releases collected. Check session cookie and connectivity.");
        return Ok(());
    }

    info!("📈 Analyzing {} releases for scene group reputation data...", releases.len());
    
    // Analyze scene groups
    analyzer.analyze_scene_groups(releases)
        .context("Failed to analyze scene groups")?;

    let (total_groups, total_releases, internal_releases, six_month_releases) = analyzer.get_statistics();
    
    info!("📊 Analysis Results:");
    info!("   • Unique scene groups: {}", total_groups);
    info!("   • Total releases analyzed: {}", total_releases);
    info!("   • Internal releases: {}", internal_releases);
    info!("   • 6-month releases: {}", six_month_releases);

    // Generate comprehensive report
    info!("📋 Generating comprehensive analysis report...");
    let report = analyzer.generate_comprehensive_report(start_time);
    
    // Display top groups
    info!("🏆 Top 10 Scene Groups by Reputation:");
    for (i, group) in analyzer.get_top_groups_by_reputation(10).iter().enumerate() {
        info!("   {}. {} - Score: {:.1} ({}) - {} releases ({}% internal)", 
              i + 1,
              group.group_name,
              group.comprehensive_reputation_score,
              group.evidence_based_tier,
              group.total_releases,
              (group.internal_ratio * 100.0) as u32
        );
    }

    // Export results
    info!("💾 Exporting results...");
    
    // JSON export
    let json_data = analyzer.export_comprehensive_json()
        .context("Failed to serialize analysis data")?;
    
    fs::write(output_file, json_data)
        .with_context(|| format!("Failed to write results to {}", output_file))?;
    
    info!("✅ JSON results saved to: {}", output_file);
    
    // CSV export
    let csv_data = analyzer.export_csv_comprehensive();
    fs::write(csv_output_file, csv_data)
        .with_context(|| format!("Failed to write CSV to {}", csv_output_file))?;
    
    info!("✅ CSV results saved to: {}", csv_output_file);

    // Save detailed report
    let report_file = "hdbits_comprehensive_report.json";
    let report_json = serde_json::to_string_pretty(&report)
        .context("Failed to serialize report")?;
    
    fs::write(report_file, report_json)
        .with_context(|| format!("Failed to write report to {}", report_file))?;
    
    info!("✅ Detailed report saved to: {}", report_file);

    let duration = Utc::now().signed_duration_since(start_time);
    
    info!("🎉 Comprehensive analysis complete!");
    info!("⏱️  Total execution time: {} minutes {} seconds", 
          duration.num_minutes(), 
          duration.num_seconds() % 60);
    
    // Print summary statistics
    info!("📊 Final Statistics:");
    info!("   • Data collection period: {}", report.data_collection_period);
    info!("   • Pages processed: {}", report.pages_processed);
    info!("   • Scene group extraction rate: {:.1}%", 
          report.data_quality_indicators.scene_group_extraction_rate * 100.0);
    info!("   • Internal release percentage: {:.1}%", 
          report.data_quality_indicators.internal_release_percentage * 100.0);
    info!("   • 6-month data coverage: {:.1}%", 
          report.data_quality_indicators.six_month_data_coverage * 100.0);
    
    // Quality distribution summary
    let stats = &report.statistical_insights.reputation_distribution;
    info!("🏅 Quality Distribution:");
    info!("   • Elite (95-100): {} groups", stats.elite);
    info!("   • Premium (85-94): {} groups", stats.premium);
    info!("   • Excellent (75-84): {} groups", stats.excellent);
    info!("   • Good (65-74): {} groups", stats.good);
    info!("   • Average (50-64): {} groups", stats.average);
    info!("   • Below Average (35-49): {} groups", stats.below_average);
    info!("   • Poor (0-34): {} groups", stats.poor);

    info!("📁 Output files generated:");
    info!("   • {}", output_file);
    info!("   • {}", csv_output_file);
    info!("   • {}", report_file);
    
    if test_mode {
        info!("ℹ️  Test mode was enabled. Run without --test-mode for full analysis.");
    }
    
    info!("✨ Ready for scene group reputation integration!");
    
    Ok(())
}

// Helper function to display file size
fn _get_file_size_mb(path: &str) -> Result<f64> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {}", path))?;
    Ok(metadata.len() as f64 / (1024.0 * 1024.0))
}

// Helper function to validate output directory
fn _ensure_output_directory(file_path: &str) -> Result<()> {
    if let Some(parent) = Path::new(file_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create output directory: {:?}", parent))?;
        }
    }
    Ok(())
}