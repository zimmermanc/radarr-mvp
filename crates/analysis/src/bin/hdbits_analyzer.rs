// HDBits Scene Group Data Collection CLI
// Command-line tool for collecting and analyzing real HDBits data

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use radarr_analysis::{HDBitsConfig, HDBitsClient, SceneGroupAnalyzer};

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

    // Initialize HDBits client and analyzer
    info!("Initializing HDBits client and analyzer...");
    let client = HDBitsClient::new(config);
    let mut analyzer = SceneGroupAnalyzer::new();

    // Run comprehensive data collection and analysis
    info!("Starting data collection and analysis...");
    info!("This process will:");
    info!("  1. Collect internal releases from HDBits (respecting rate limits)");
    info!("  2. Extract scene group information from release names");
    info!("  3. Calculate reputation scores based on multiple metrics");
    info!("  4. Generate comprehensive analysis reports");
    info!("");

    // Collect data from HDBits
    let torrents = match client.collect_comprehensive_data().await {
        Ok(torrents) => torrents,
        Err(e) => {
            eprintln!("❌ Error during data collection: {}", e);
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
    };

    // Analyze scene groups
    match analyzer.analyze_torrents(torrents) {
        Ok(()) => {
            info!("✅ Data collection and analysis completed successfully!");
            info!("");
            info!("RESULTS SUMMARY:");
            info!("================");
            info!("Unique Scene Groups: {}", analyzer.group_metrics.len());
            info!("");

            info!("TOP 10 SCENE GROUPS BY REPUTATION:");
            for (i, group) in analyzer.get_top_groups(10).iter().enumerate() {
                info!("  {}. {} - {:.1} ({} releases, {:.1}% internal)", 
                    i + 1, 
                    group.group_name, 
                    group.reputation_score, 
                    group.total_releases,
                    group.internal_ratio * 100.0
                );
            }
            info!("");

            info!("STATISTICAL SUMMARY:");
            let top_groups = analyzer.get_top_groups(analyzer.group_metrics.len());
            if !top_groups.is_empty() {
                let scores: Vec<f64> = top_groups.iter().map(|g| g.reputation_score).collect();
                let min_score = scores.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_score = scores.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let avg_score = scores.iter().sum::<f64>() / scores.len() as f64;
                
                info!("  Reputation Score Range: {:.1} - {:.1} (avg: {:.1})", min_score, max_score, avg_score);
            }
            info!("");

            info!("🎯 NEXT STEPS:");
            info!("1. Review the generated reports in: {}", cli.output.display());
            info!("2. Use the analysis data for integration with automation");
            info!("3. Update your automation system with evidence-based reputation scores");
            info!("");

            // Export analysis data
            let analysis_json = analyzer.export_analysis().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to export analysis: {}", e);
                "{}".to_string()
            });
            let analysis_file = cli.output.join("scene_groups_analysis.json");
            if let Err(e) = std::fs::write(&analysis_file, analysis_json) {
                eprintln!("Warning: Failed to write analysis file: {}", e);
            } else {
                info!("Analysis data saved to: {}", analysis_file.display());
            }
        }
        Err(e) => {
            eprintln!("❌ Error during scene group analysis: {}", e);
            eprintln!("");
            eprintln!("Possible causes:");
            eprintln!("  • Corrupted torrent data");
            eprintln!("  • Parsing errors in release names");
            eprintln!("  • Invalid date formats");
            eprintln!("");
            eprintln!("Troubleshooting:");
            eprintln!("  1. Check the collected torrent data format");
            eprintln!("  2. Run with --verbose for detailed logging");
            eprintln!("  3. Try --dry-run to test configuration");
            std::process::exit(1);
        }
    }

    Ok(())
}