// HDBits Scene Group Data Collection CLI
// Command-line tool for collecting and analyzing real HDBits data

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use radarr_analysis::{HDBitsConfig, SceneGroupAnalyzer};

#[derive(Parser)]
#[command(name = "hdbits-analyzer")]
#[command(about = "Collect and analyze HDBits scene group data for reputation scoring")]
#[command(version = "1.0")]
struct Cli {
    /// Output directory for analysis results
    #[arg(short, long, default_value = "./hdbits_analysis")]
    output: PathBuf,

    /// HDBits username
    #[arg(short, long, default_value = "blargdiesel")]
    username: String,

    /// HDBits passkey
    #[arg(short, long, default_value = "ed487790cd0dee98941ab5c132179bd2c8c5e23622c0c04a800ad543cde2990cd44ed960892d990214ea1618bf29780386a77246a21dc636d83420e077e69863")]
    passkey: String,

    /// API endpoint URL
    #[arg(long, default_value = "https://hdbits.org/api/torrents")]
    api_url: String,

    /// Rate limit per hour
    #[arg(long, default_value = "150")]
    rate_limit: u32,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Dry run - analyze configuration without making API calls
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    info!("HDBits Scene Group Data Analyzer v1.0");
    info!("==================================");

    // Validate configuration
    if cli.username.is_empty() || cli.passkey.is_empty() {
        eprintln!("Error: Username and passkey are required");
        std::process::exit(1);
    }

    // Create configuration
    let config = HDBitsConfig {
        username: cli.username.clone(),
        passkey: cli.passkey.clone(),
        api_url: cli.api_url.clone(),
        rate_limit_per_hour: cli.rate_limit,
    };

    // Display configuration (masking sensitive data)
    info!("Configuration:");
    info!("  Username: {}", cli.username);
    info!("  Passkey: {}...{}", &cli.passkey[..8], &cli.passkey[cli.passkey.len()-8..]);
    info!("  API URL: {}", cli.api_url);
    info!("  Rate Limit: {} requests/hour", cli.rate_limit);
    info!("  Output Directory: {}", cli.output.display());
    info!("");

    if cli.dry_run {
        info!("DRY RUN MODE - No API calls will be made");
        info!("Configuration validated successfully");
        return Ok(());
    }

    // Confirm before starting data collection
    println!("This will collect data from HDBits and analyze scene group reputation.");
    println!("The process may take 1-2 hours depending on rate limits.");
    println!("Continue? (y/N): ");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if !input.trim().to_lowercase().starts_with('y') {
        info!("Operation cancelled by user");
        return Ok(());
    }

    // Create output directory
    std::fs::create_dir_all(&cli.output)?;
    let output_dir = cli.output.to_string_lossy().to_string();

    // Initialize data collector
    info!("Initializing HDBits data collector...");
    let mut collector = SceneGroupAnalyzer::new(config, output_dir);

    // Run comprehensive data collection and analysis
    info!("Starting data collection and analysis...");
    info!("This process will:");
    info!("  1. Collect internal releases from HDBits (respecting rate limits)");
    info!("  2. Extract scene group information from release names");
    info!("  3. Calculate reputation scores based on multiple metrics");
    info!("  4. Generate comprehensive analysis reports");
    info!("");

    match collector.collect_and_analyze().await {
        Ok(report) => {
            info!("✅ Data collection and analysis completed successfully!");
            info!("");
            info!("RESULTS SUMMARY:");
            info!("================");
            info!("Total Torrents Analyzed: {}", report.total_torrents_analyzed);
            info!("Unique Scene Groups: {}", report.unique_scene_groups);
            info!("Internal Releases: {}", report.internal_releases);
            info!("External Releases: {}", report.external_releases);
            info!("Collection Duration: {} seconds", report.collection_duration_seconds);
            info!("");
            
            info!("QUALITY DISTRIBUTION:");
            info!("  Premium Groups (80-100): {}", report.quality_distribution.premium_groups);
            info!("  High Quality (60-80): {}", report.quality_distribution.high_quality_groups);
            info!("  Standard (40-60): {}", report.quality_distribution.standard_groups);
            info!("  Low Quality (20-40): {}", report.quality_distribution.low_quality_groups);
            info!("  Poor (0-20): {}", report.quality_distribution.poor_groups);
            info!("");

            info!("TOP 10 SCENE GROUPS BY REPUTATION:");
            for (i, group) in report.top_groups_by_reputation.iter().take(10).enumerate() {
                info!("  {}. {} - {:.1} ({}, {} releases)", 
                    i + 1, 
                    group.group_name, 
                    group.reputation_score, 
                    group.quality_tier,
                    group.total_releases
                );
            }
            info!("");

            info!("STATISTICAL SUMMARY:");
            info!("  Reputation Score Range: {:.1} - {:.1} (avg: {:.1})", 
                report.statistical_summary.reputation_scores.min,
                report.statistical_summary.reputation_scores.max,
                report.statistical_summary.reputation_scores.mean
            );
            info!("  Average Seeders Range: {:.1} - {:.1} (avg: {:.1})", 
                report.statistical_summary.seeder_counts.min,
                report.statistical_summary.seeder_counts.max,
                report.statistical_summary.seeder_counts.mean
            );
            info!("  File Size Range: {:.1} - {:.1} GB (avg: {:.1} GB)", 
                report.statistical_summary.file_sizes_gb.min,
                report.statistical_summary.file_sizes_gb.max,
                report.statistical_summary.file_sizes_gb.mean
            );
            info!("");

            info!("TEMPORAL ANALYSIS:");
            info!("  Active Groups (last 30 days): {}", report.temporal_analysis.active_groups_last_30_days);
            info!("  Active Groups (last 90 days): {}", report.temporal_analysis.active_groups_last_90_days);
            info!("  Established Groups (2+ years): {}", report.temporal_analysis.established_groups_over_2_years);
            info!("  Dormant Groups (1+ year): {}", report.temporal_analysis.dormant_groups);
            info!("");

            info!("🎯 NEXT STEPS:");
            info!("1. Review the generated reports in: {}", cli.output.display());
            info!("2. Use reputation_system_*.json for integration with automation");
            info!("3. Analyze scene_groups_data_*.csv for detailed insights");
            info!("4. Update your automation system with evidence-based reputation scores");
            info!("");

            info!("Files Generated:");
            info!("  📊 collection_report_*.json - Summary report with statistics");
            info!("  🔍 scene_groups_analysis_*.json - Detailed analysis data");
            info!("  ⚙️ reputation_system_*.json - Ready-to-integrate reputation scores");
            info!("  📈 scene_groups_data_*.csv - Raw data for further analysis");
        }
        Err(e) => {
            eprintln!("❌ Error during data collection and analysis: {}", e);
            eprintln!("");
            eprintln!("Possible causes:");
            eprintln!("  • Invalid credentials (username/passkey)");
            eprintln!("  • Network connectivity issues");
            eprintln!("  • HDBits API rate limiting");
            eprintln!("  • API endpoint changes");
            eprintln!("");
            eprintln!("Troubleshooting:");
            eprintln!("  1. Verify your HDBits credentials");
            eprintln!("  2. Check your internet connection");
            eprintln!("  3. Run with --verbose for detailed logging");
            eprintln!("  4. Try --dry-run to test configuration");
            
            std::process::exit(1);
        }
    }

    Ok(())
}