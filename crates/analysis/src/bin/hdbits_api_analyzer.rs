use anyhow::Result;
use clap::Parser;
use radarr_analysis::hdbits_api_analyzer::{HDBitsApiAnalyzer, ApiAnalyzerConfig};
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// HDBits session cookie for authentication
    #[arg(long, env = "HDBITS_SESSION_COOKIE")]
    session_cookie: String,
    
    /// Output directory for analysis results
    #[arg(long, default_value = "/tmp/radarr/analysis")]
    output: PathBuf,
    
    /// Maximum pages to fetch (each page has up to 100 results)
    #[arg(long, default_value = "10")]
    max_pages: u32,
    
    /// Delay between requests in seconds
    #[arg(long, default_value = "5")]
    rate_limit_seconds: u64,
    
    /// Results per page (max 100 per API)
    #[arg(long, default_value = "100")]
    results_per_page: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    
    let args = Args::parse();
    
    info!("🚀 HDBits API Analyzer Starting");
    info!("Output directory: {:?}", args.output);
    info!("Max pages: {} (up to {} releases)", args.max_pages, args.max_pages * args.results_per_page);
    info!("Rate limit: {} seconds between requests", args.rate_limit_seconds);
    
    // Create configuration
    let config = ApiAnalyzerConfig {
        base_url: "https://hdbits.org".to_string(),
        session_cookie: args.session_cookie,
        output_dir: args.output,
        max_pages: args.max_pages,
        request_delay_seconds: args.rate_limit_seconds,
        results_per_page: args.results_per_page,
    };
    
    // Run analysis
    let mut analyzer = HDBitsApiAnalyzer::new(config);
    
    match analyzer.analyze().await {
        Ok(_) => {
            info!("✅ Analysis complete!");
            Ok(())
        }
        Err(e) => {
            error!("❌ Analysis failed: {}", e);
            Err(e)
        }
    }
}