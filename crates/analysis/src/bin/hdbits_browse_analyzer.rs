// HDBits Browse Interface Scene Group Analyzer CLI
// Safe data collection using browse interface with strict rate limiting

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use std::path::PathBuf;
use tokio;
use tracing::{info, Level};
use tracing_subscriber;

use radarr_analysis::{HDBitsBrowseAnalyzer, HDBitsBrowseConfig};

#[derive(Parser)]
#[command(name = "hdbits-browse-analyzer")]
#[command(about = "Safe HDBits scene group analysis using browse interface")]
#[command(version = "2.0")]
struct Cli {
    /// Output directory for analysis results
    #[arg(short, long, default_value = "./hdbits_analysis_results")]
    output: PathBuf,

    /// HDBits username
    #[arg(short, long, default_value = "your_username")]
    username: String,

    /// HDBits passkey
    #[arg(
        short,
        long,
        default_value = "your_hdbits_passkey_here"
    )]
    passkey: String,

    /// Rate limiting delay in seconds (minimum 30 seconds recommended)
    #[arg(long, default_value = "35")]
    rate_limit_seconds: u64,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Dry run - test configuration without data collection
    #[arg(long)]
    dry_run: bool,

    /// Generate only reputation system output (skip detailed analysis)
    #[arg(long)]
    reputation_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    info!("🎯 HDBits Browse Interface Scene Group Analyzer v2.0");
    info!("==================================================");
    info!("");

    // Validate rate limiting
    if cli.rate_limit_seconds < 30 {
        eprintln!(
            "⚠️  WARNING: Rate limit below 30 seconds is not recommended for community safety"
        );
        eprintln!(
            "   Using minimum safe value of 30 seconds instead of {}",
            cli.rate_limit_seconds
        );
    }
    let safe_rate_limit = cli.rate_limit_seconds.max(30);

    // Validate configuration
    if cli.username.is_empty() || cli.passkey.is_empty() {
        eprintln!("❌ Error: Username and passkey are required");
        std::process::exit(1);
    }

    // Create configuration
    let config = HDBitsBrowseConfig {
        username: cli.username.clone(),
        passkey: cli.passkey.clone(),
        base_url: "https://hdbits.org".to_string(),
        max_pages: 5,
        delay_seconds: safe_rate_limit,
        rate_limit_seconds: safe_rate_limit,
        request_delay_seconds: safe_rate_limit,
    };

    // Display configuration (masking sensitive data)
    info!("📋 CONFIGURATION:");
    info!("   Username: {}", cli.username);
    info!(
        "   Passkey: {}...{}",
        &cli.passkey[..8],
        &cli.passkey[cli.passkey.len() - 8..]
    );
    info!("   Base URL: https://hdbits.org/browse.php");
    info!(
        "   Rate Limit: {} seconds between requests",
        safe_rate_limit
    );
    info!("   Output Directory: {}", cli.output.display());
    info!("");

    info!("🛡️  SAFETY FEATURES:");
    info!("   ✓ Browse interface only (no torrent downloads)");
    info!("   ✓ Conservative {} second rate limiting", safe_rate_limit);
    info!("   ✓ Limited to 5 pages maximum");
    info!("   ✓ Internal releases focus for quality data");
    info!("   ✓ Community-respectful collection approach");
    info!("");

    if cli.dry_run {
        info!("🧪 DRY RUN MODE - Configuration validated successfully");
        info!("   No actual data collection will be performed");
        info!("   All safety checks passed");
        return Ok(());
    }

    // Confirm before starting
    println!("🚀 READY TO START DATA COLLECTION");
    println!("");
    println!("This will:");
    println!("  • Collect scene group data from HDBits browse interface");
    println!("  • Focus on internal releases for highest quality data");
    println!(
        "  • Use {}-second delays between requests for safety",
        safe_rate_limit
    );
    println!("  • Generate evidence-based reputation scores");
    println!("  • Estimated time: 10-15 minutes with safe rate limiting");
    println!("");
    println!("Continue? (y/N): ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().to_lowercase().starts_with('y') {
        info!("Operation cancelled by user");
        return Ok(());
    }

    // Create output directory
    std::fs::create_dir_all(&cli.output).context("Failed to create output directory")?;

    let start_time = Utc::now();
    info!(
        "📊 Starting HDBits scene group analysis at {}",
        start_time.format("%Y-%m-%d %H:%M:%S UTC")
    );
    info!("");

    // Initialize analyzer
    let mut analyzer = HDBitsBrowseAnalyzer::new(config);

    // Phase 1: Authentication
    info!("🔐 Phase 1: Authenticating with HDBits...");
    match analyzer.login().await {
        Ok(_) => info!("✅ Authentication successful"),
        Err(e) => {
            eprintln!("❌ Authentication failed: {}", e);
            eprintln!("");
            eprintln!("Possible issues:");
            eprintln!("  • Invalid credentials");
            eprintln!("  • Network connectivity problems");
            eprintln!("  • HDBits site issues");
            std::process::exit(1);
        }
    }

    // Phase 2: Data Collection
    info!("");
    info!("📥 Phase 2: Collecting release data from browse interface...");
    info!(
        "   Using {}-second delays between requests for community safety",
        safe_rate_limit
    );

    let releases = match analyzer.collect_internal_releases().await {
        Ok(releases) => {
            info!(
                "✅ Data collection complete: {} releases collected",
                releases.len()
            );
            releases
        }
        Err(e) => {
            eprintln!("❌ Data collection failed: {}", e);
            eprintln!("");
            eprintln!("Possible issues:");
            eprintln!("  • Network connectivity problems");
            eprintln!("  • HDBits site structure changes");
            eprintln!("  • Rate limiting (try increasing --rate-limit-seconds)");
            std::process::exit(1);
        }
    };

    if releases.is_empty() {
        eprintln!("⚠️  No releases found - this might indicate:");
        eprintln!("   • Site structure changes requiring parser updates");
        eprintln!("   • Network or authentication issues");
        eprintln!("   • Temporary site problems");
        std::process::exit(1);
    }

    // Phase 3: Scene Group Analysis
    info!("");
    info!("🔍 Phase 3: Analyzing scene groups and calculating reputation scores...");

    analyzer
        .analyze_scene_groups(releases)
        .context("Failed to analyze scene groups")?;
    let scene_groups = analyzer.get_scene_groups();

    if scene_groups.is_empty() {
        eprintln!("⚠️  No scene groups identified from releases");
        eprintln!("   This might indicate issues with release name parsing");
        std::process::exit(1);
    }

    info!(
        "✅ Scene group analysis complete: {} unique groups identified",
        scene_groups.len()
    );

    // Phase 4: Report Generation
    info!("");
    info!("📋 Phase 4: Generating comprehensive analysis reports...");

    let report = analyzer.generate_analysis_report(start_time);
    let timestamp = start_time.format("%Y%m%d_%H%M%S");

    // Save detailed analysis report
    let report_json =
        serde_json::to_string_pretty(&report).context("Failed to serialize analysis report")?;
    let report_path = cli
        .output
        .join(format!("hdbits_browse_analysis_{}.json", timestamp));
    std::fs::write(&report_path, report_json).context("Failed to write analysis report")?;
    info!("✅ Detailed analysis saved to: {}", report_path.display());

    // Save reputation system data (for integration)
    let reputation_json = analyzer
        .export_reputation_data()
        .context("Failed to export reputation data")?;
    let reputation_path = cli
        .output
        .join(format!("reputation_system_{}.json", timestamp));
    std::fs::write(&reputation_path, reputation_json)
        .context("Failed to write reputation system data")?;
    info!(
        "✅ Reputation system saved to: {}",
        reputation_path.display()
    );

    if !cli.reputation_only {
        // Save CSV data for analysis
        let csv_data = analyzer.export_csv_data();
        let csv_path = cli
            .output
            .join(format!("scene_groups_data_{}.csv", timestamp));
        std::fs::write(&csv_path, csv_data).context("Failed to write CSV data")?;
        info!("✅ CSV data saved to: {}", csv_path.display());
    }

    let end_time = Utc::now();
    let duration = end_time.signed_duration_since(start_time);

    info!("");
    info!("🎉 ANALYSIS COMPLETE!");
    info!("====================");
    info!("");

    // Display summary statistics
    info!("📊 COLLECTION SUMMARY:");
    info!(
        "   Total Releases Analyzed: {}",
        report.total_torrents_analyzed
    );
    info!("   Unique Scene Groups: {}", report.unique_scene_groups);
    info!(
        "   Internal Releases: {}",
        report.internal_releases_analyzed
    );
    info!("   Collection Duration: {} minutes", duration.num_minutes());
    info!("");

    info!("🌟 TOP INTERNAL GROUPS:");
    for (i, group) in report.top_internal_groups.iter().take(10).enumerate() {
        info!(
            "   {}. {} - Score: {:.1} ({} releases)",
            i + 1,
            group.name,
            group.score,
            group.releases
        );
    }
    info!("");

    info!("🎯 NEXT STEPS:");
    info!("   1. Review detailed analysis: {}", report_path.display());
    info!(
        "   2. Integrate reputation scores: {}",
        reputation_path.display()
    );
    if !cli.reputation_only {
        info!(
            "   3. Analyze CSV data for insights: scene_groups_data_{}.csv",
            timestamp
        );
    }
    info!("   4. Update your automation system with evidence-based scores");
    info!("   5. Schedule regular analysis runs for updated data");
    info!("");

    info!("💡 INTEGRATION EXAMPLE:");
    info!(
        "   Load reputation_system_{}.json in your automation system",
        timestamp
    );
    info!("   Use reputation_score >= 75.0 for premium quality filtering");
    info!("   Consider confidence_level for decision making");
    info!("");

    info!("✅ Safe data collection completed successfully!");
    info!("   All operations used browse interface with conservative rate limiting");
    info!("   HDBits community guidelines respected throughout collection");

    Ok(())
}
