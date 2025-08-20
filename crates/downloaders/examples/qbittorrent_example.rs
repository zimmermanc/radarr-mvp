//! Example usage of the qBittorrent client
//!
//! This example demonstrates how to use the qBittorrent client to interact
//! with a running qBittorrent instance.
//!
//! To run this example:
//! ```bash
//! cargo run --example qbittorrent_example
//! ```
//!
//! Note: This requires a running qBittorrent instance with Web UI enabled.

use radarr_downloaders::{QBittorrentClient, QBittorrentConfig};

#[allow(unused_imports)]
use radarr_downloaders::{AddTorrentParams, TorrentData};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("qBittorrent Client Example");
    println!("==========================");

    // Configure the client
    let config = QBittorrentConfig {
        base_url: "http://localhost:8080".to_string(),
        username: "admin".to_string(),
        password: "adminpass".to_string(), // Change this to your actual password
        timeout: 30,
    };

    println!("Creating qBittorrent client...");
    let client = QBittorrentClient::new(config)?;

    // Test connection
    println!("Testing connection...");
    match client.test_connection().await {
        Ok(()) => println!("✓ Successfully connected to qBittorrent!"),
        Err(e) => {
            println!("✗ Failed to connect to qBittorrent: {}", e);
            println!("Make sure qBittorrent is running with Web UI enabled on http://localhost:8080");
            return Ok(());
        }
    }

    // Get current torrents
    println!("\nFetching current torrents...");
    match client.get_torrents().await {
        Ok(torrents) => {
            println!("✓ Found {} torrents:", torrents.len());
            for torrent in torrents.iter().take(5) {
                println!("  - {} ({}%) - {}", 
                    torrent.name, 
                    (torrent.progress * 100.0) as u32,
                    torrent.state
                );
            }
            if torrents.len() > 5 {
                println!("  ... and {} more", torrents.len() - 5);
            }
        }
        Err(e) => println!("✗ Failed to get torrents: {}", e),
    }

    // Get application preferences
    println!("\nFetching application preferences...");
    match client.get_preferences().await {
        Ok(prefs) => {
            println!("✓ Application preferences:");
            if let Some(save_path) = prefs.save_path {
                println!("  - Save path: {}", save_path);
            }
            if let Some(temp_path) = prefs.temp_path {
                println!("  - Temp path: {}", temp_path);
            }
            if let Some(dl_limit) = prefs.dl_limit {
                println!("  - Download limit: {} bytes/sec", dl_limit);
            }
        }
        Err(e) => println!("✗ Failed to get preferences: {}", e),
    }

    // Example of adding a torrent (commented out to avoid actually adding anything)
    /*
    println!("\nAdding example torrent...");
    let add_params = AddTorrentParams {
        torrent_data: TorrentData::Url("magnet:?xt=urn:btih:example".to_string()),
        category: Some("radarr".to_string()),
        save_path: Some("/downloads/movies".to_string()),
        paused: true, // Start paused for safety
        skip_checking: false,
        priority: 0,
    };

    match client.add_torrent(add_params).await {
        Ok(hash) => println!("✓ Successfully added torrent: {}", hash),
        Err(e) => println!("✗ Failed to add torrent: {}", e),
    }
    */

    println!("\nExample completed successfully!");
    Ok(())
}