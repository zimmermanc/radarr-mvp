//! Full workflow test: Search → Queue → Download → Monitor
//!
//! This example demonstrates the complete download workflow:
//! 1. Search for a movie on HDBits via the API
//! 2. Add selected release to the queue
//! 3. Send to qBittorrent for downloading  
//! 4. Monitor download progress

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use tokio::time::{sleep, Duration};
use anyhow::Result;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    id: String,
    title: String,
    download_url: String,
    size_bytes: Option<u64>,
    seeders: Option<u32>,
    leechers: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct QueueItem {
    id: String,
    title: String,
    status: String,
    progress: f64,
    download_speed: Option<u64>,
    upload_speed: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    println!("🎬 Radarr Full Workflow Test");
    println!("================================\n");
    
    // API configuration
    let api_base = "http://localhost:7878";
    let api_key = env::var("RADARR_API_KEY")
        .unwrap_or_else(|_| "mysecurekey123".to_string());
    
    let client = Client::new();
    
    // Test API connection
    println!("🎬 Testing API connection...");
    let response = client
        .get(format!("{}/health", api_base))
        .header("X-API-Key", &api_key)
        .send()
        .await?;
    
    if response.status().is_success() {
        println!("✅ API is healthy\n");
    } else {
        println!("❌ API health check failed: {}", response.status());
        return Ok(());
    }
    
    // Search for a movie via HDBits indexer
    let search_query = "The Matrix";
    println!("🔍 Searching for: {}", search_query);
    
    let search_response = client
        .post(format!("{}/api/v3/indexer/search", api_base))
        .header("X-API-Key", &api_key)
        .json(&json!({
            "query": search_query,
            "limit": 10
        }))
        .send()
        .await?;
    
    if !search_response.status().is_success() {
        println!("❌ Search failed: {}", search_response.status());
        println!("   Make sure HDBits credentials are configured");
        return Ok(());
    }
    
    let search_results: SearchResponse = search_response.json().await?;
    
    if search_results.results.is_empty() {
        println!("❌ No results found for: {}", search_query);
        return Ok(());
    }
    
    println!("✅ Found {} releases:\n", search_results.results.len());
    
    // Display search results
    for (i, release) in search_results.results.iter().enumerate().take(5) {
        println!("  {}. {}", i + 1, release.title);
        println!("     Size: {} | Seeders: {} | Leechers: {}",
            format_size(release.size_bytes.unwrap_or(0)),
            release.seeders.unwrap_or(0),
            release.leechers.unwrap_or(0)
        );
        println!();
    }
    
    // Select the first release with good seeders
    let selected_release = search_results.results.iter()
        .find(|r| r.seeders.unwrap_or(0) > 5)
        .or_else(|| search_results.results.first())
        .expect("No suitable release found");
    
    println!("📦 Selected release: {}", selected_release.title);
    
    // Add to download queue
    println!("\n⬇️  Adding to download queue...");
    
    let queue_response = client
        .post(format!("{}/api/v3/queue", api_base))
        .header("X-API-Key", &api_key)
        .json(&json!({
            "release_id": selected_release.id,
            "title": selected_release.title,
            "download_url": selected_release.download_url,
            "priority": "normal"
        }))
        .send()
        .await?;
    
    if !queue_response.status().is_success() {
        println!("❌ Failed to add to queue: {}", queue_response.status());
        let error_text = queue_response.text().await?;
        println!("   Error: {}", error_text);
        return Ok(());
    }
    
    let queue_item: QueueItem = queue_response.json().await?;
    println!("✅ Added to queue with ID: {}", queue_item.id);
    
    // Monitor download progress
    println!("\n📊 Monitoring download progress...");
    println!("   (Press Ctrl+C to stop monitoring)\n");
    
    let mut last_progress = 0.0;
    let mut stalled_count = 0;
    
    loop {
        // Get queue item status
        let status_response = client
            .get(format!("{}/api/v3/queue/{}", api_base, queue_item.id))
            .header("X-API-Key", &api_key)
            .send()
            .await?;
        
        if !status_response.status().is_success() {
            println!("\n❌ Error getting queue status: {}", status_response.status());
            break;
        }
        
        let current_item: QueueItem = status_response.json().await?;
        
        let progress = current_item.progress * 100.0;
        let download_speed = format_speed(current_item.download_speed.unwrap_or(0));
        let upload_speed = format_speed(current_item.upload_speed.unwrap_or(0));
        
        // Clear line and print status
        print!("\r");
        print!("📥 {} | Progress: {:.1}% | ↓ {} | ↑ {}    ",
            current_item.status,
            progress,
            download_speed,
            upload_speed
        );
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        // Check if download is complete
        if current_item.status == "completed" || current_item.progress >= 1.0 {
            println!("\n\n✅ Download complete!");
            break;
        }
        
        // Check if failed
        if current_item.status == "failed" {
            println!("\n\n❌ Download failed!");
            break;
        }
        
        // Check if stalled
        if progress == last_progress {
            stalled_count += 1;
            if stalled_count > 30 {
                println!("\n\n⚠️  Download appears to be stalled");
                break;
            }
        } else {
            stalled_count = 0;
            last_progress = progress;
        }
        
        sleep(Duration::from_secs(2)).await;
    }
    
    println!("\n🎉 Workflow test complete!");
    println!("\nNext steps:");
    println!("  1. Implement import pipeline to move completed downloads");
    println!("  2. Add metadata enrichment from TMDB");
    println!("  3. Set up automated quality upgrades");
    println!("  4. Configure post-processing scripts");
    
    Ok(())
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

fn format_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec == 0 {
        "0 B/s".to_string()
    } else {
        format!("{}/s", format_size(bytes_per_sec))
    }
}