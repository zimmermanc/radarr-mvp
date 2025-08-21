//! HDBits indexer usage example
//! 
//! This example demonstrates how to use the HDBits indexer to search for movies.
//! Run with: cargo run --example hdbits_example

use radarr_indexers::{HDBitsClient, HDBitsConfig, IndexerClient, MovieSearchRequest, SearchRequest};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create HDBits client with default configuration
    let config = HDBitsConfig::default();
    let client = HDBitsClient::new(config)?;

    println!("Testing HDBits indexer connection...");

    // Test connection
    match client.health_check().await {
        Ok(true) => println!("✅ HDBits connection test successful"),
        Ok(false) => println!("❌ HDBits connection test failed"),
        Err(e) => println!("❌ HDBits connection error: {}", e),
    }

    println!("\nSearching for movies with title 'Dune'...");

    // Search for movies using the direct HDBits API
    let movie_request = MovieSearchRequest::new()
        .with_title("Dune")
        .with_limit(5);

    match client.search_movies(&movie_request).await {
        Ok(releases) => {
            println!("Found {} releases:", releases.len());
            for release in releases.iter().take(3) {
                println!("  📦 {}", release.title);
                println!("     Size: {}", release.human_readable_size().unwrap_or("Unknown".to_string()));
                println!("     Seeders: {}", release.seeders.unwrap_or(0));
                println!("     Quality: {}", serde_json::to_string_pretty(&release.quality)?);
                println!();
            }
        }
        Err(e) => println!("❌ Search failed: {}", e),
    }

    println!("\nSearching using generic IndexerClient interface...");

    // Search using the generic IndexerClient trait
    let search_request = SearchRequest {
        query: Some("Blade Runner 2049".to_string()),
        imdb_id: None,
        tmdb_id: None,
        categories: vec![2000], // Movies
        indexer_ids: vec![],
        limit: Some(3),
        offset: None,
        min_seeders: Some(5),
        min_size: None,
        max_size: None,
    };

    match client.search(&search_request).await {
        Ok(response) => {
            println!("Found {} results from {} indexers:", response.total, response.indexers_searched);
            for result in &response.results {
                println!("  🎬 {}", result.title);
                println!("     Indexer: {}", result.indexer);
                println!("     Size: {}", result.size.map(|s| format_size(s)).unwrap_or("Unknown".to_string()));
                println!("     Seeders: {}", result.seeders.unwrap_or(0));
                if let Some(freeleech) = result.freeleech {
                    println!("     Freeleech: {}", freeleech);
                }
                println!();
            }
        }
        Err(e) => println!("❌ Generic search failed: {}", e),
    }

    println!("\nGetting indexer information...");

    // Get indexer information
    match client.get_indexers().await {
        Ok(indexers) => {
            for indexer in &indexers {
                println!("📡 Indexer: {}", indexer.name);
                println!("   Implementation: {}", indexer.implementation);
                println!("   Movie search: {}", indexer.capabilities.movie_search);
                println!("   Status: {}", indexer.status.status);
                println!();
            }
        }
        Err(e) => println!("❌ Failed to get indexers: {}", e),
    }

    Ok(())
}

/// Format file size in human readable format
fn format_size(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}