//! Example usage of the Prowlarr client for searching releases
//!
//! This example demonstrates how to use the ProwlarrClient to search for movie releases.
//! To run this example, set the following environment variables:
//! - PROWLARR_BASE_URL (e.g., "http://localhost:9696")
//! - PROWLARR_API_KEY (your Prowlarr API key)
//!
//! Then run: cargo run --example search_example

use radarr_indexers::{ProwlarrClient, ProwlarrConfigBuilder, SearchRequest};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    // Get configuration from environment or use defaults
    let base_url = std::env::var("PROWLARR_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:9696".to_string());
    let api_key = std::env::var("PROWLARR_API_KEY")
        .expect("PROWLARR_API_KEY environment variable must be set");
    
    // Create the Prowlarr client
    let config = ProwlarrConfigBuilder::new()
        .base_url(base_url)
        .api_key(api_key)
        .timeout(30)
        .rate_limit(60) // 60 requests per minute
        .build();
        
    let client = ProwlarrClient::new(config)?;
    
    // Test connectivity
    info!("Testing Prowlarr connectivity...");
    match client.health_check().await {
        Ok(true) => info!("✓ Prowlarr is healthy and accessible"),
        Ok(false) => {
            eprintln!("✗ Prowlarr health check failed");
            return Ok(());
        }
        Err(e) => {
            eprintln!("✗ Failed to connect to Prowlarr: {}", e);
            return Ok(());
        }
    }
    
    // Get list of configured indexers
    info!("Fetching configured indexers...");
    match client.get_indexers().await {
        Ok(indexers) => {
            info!("Found {} configured indexers:", indexers.len());
            for indexer in &indexers {
                info!("  - {} (ID: {}, Enabled: {})", 
                    indexer.name, indexer.id, indexer.enable);
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch indexers: {}", e);
            return Ok(());
        }
    }
    
    // Search for a popular movie (The Shawshank Redemption)
    info!("Searching for 'The Shawshank Redemption' by IMDB ID...");
    let search_request = SearchRequest::for_movie_imdb("tt0111161")
        .with_min_seeders(5)  // Require at least 5 seeders
        .with_limit(10);      // Limit to 10 results
    
    match client.search(&search_request).await {
        Ok(response) => {
            info!("Search completed successfully!");
            info!("  Total results: {}", response.total);
            info!("  Indexers searched: {}", response.indexers_searched);
            info!("  Indexers with errors: {}", response.indexers_with_errors);
            
            if !response.errors.is_empty() {
                info!("  Errors encountered:");
                for error in &response.errors {
                    info!("    - {}: {}", error.indexer, error.message);
                }
            }
            
            info!("  Results:");
            for (i, result) in response.results.iter().enumerate() {
                info!("    {}. {}", i + 1, result.title);
                info!("       Indexer: {}", result.indexer);
                if let Some(size) = result.size {
                    info!("       Size: {:.2} GB", size as f64 / 1_073_741_824.0);
                }
                if let Some(seeders) = result.seeders {
                    info!("       Seeders: {}", seeders);
                }
                if let Some(leechers) = result.leechers {
                    info!("       Leechers: {}", leechers);
                }
                info!("       Download: {}", result.download_url);
                println!();
            }
        }
        Err(e) => {
            eprintln!("Search failed: {}", e);
            return Ok(());
        }
    }
    
    // Search by title as an alternative method
    info!("Searching for 'Inception' by title...");
    let title_search = SearchRequest::for_title("Inception 2010")
        .with_limit(5);
    
    match client.search(&title_search).await {
        Ok(response) => {
            info!("Title search found {} results", response.results.len());
            for result in response.results.iter().take(3) {
                info!("  - {}", result.title);
            }
        }
        Err(e) => {
            eprintln!("Title search failed: {}", e);
        }
    }
    
    // Test a specific indexer if any are available
    info!("Testing indexer connectivity...");
    match client.get_indexers().await {
        Ok(indexers) => {
            if let Some(indexer) = indexers.first() {
                match client.test_indexer(indexer.id).await {
                    Ok(true) => info!("✓ Indexer '{}' test passed", indexer.name),
                    Ok(false) => info!("✗ Indexer '{}' test failed", indexer.name),
                    Err(e) => info!("✗ Failed to test indexer '{}': {}", indexer.name, e),
                }
            } else {
                info!("No indexers configured to test");
            }
        }
        Err(_) => {
            info!("Could not fetch indexers for testing");
        }
    }
    
    info!("Example completed successfully!");
    Ok(())
}