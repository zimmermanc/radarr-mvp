//! HDBits Integration Example
//!
//! Demonstrates how to integrate HDBits indexer into the Radarr API layer.
//! This shows the complete integration pattern that can be used in production.

use radarr_core::models::indexer::{Indexer, IndexerImplementation};
use radarr_indexers::{HDBitsClient, HDBitsConfig, IndexerClient};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio;
use tracing_subscriber;

/// Service that wraps multiple indexer clients
pub struct IndexerService {
    hdbits_client: Option<Arc<HDBitsClient>>,
    // Other indexer clients can be added here
    // prowlarr_client: Option<Arc<ProwlarrClient>>,
}

impl IndexerService {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to create HDBits client from environment or use default config
        let hdbits_client = match HDBitsConfig::from_env() {
            Ok(config) => {
                match HDBitsClient::new(config) {
                    Ok(client) => {
                        // Test connection
                        if client.health_check().await.unwrap_or(false) {
                            println!("✅ HDBits client initialized successfully");
                            Some(Arc::new(client))
                        } else {
                            println!("⚠️ HDBits client created but health check failed");
                            Some(Arc::new(client))
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to create HDBits client: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                println!("❌ HDBits configuration error: {}", e);
                None
            }
        };

        Ok(Self { hdbits_client })
    }

    /// Get all available indexers
    pub async fn get_indexers(&self) -> Vec<Indexer> {
        let mut indexers = Vec::new();

        if let Some(client) = &self.hdbits_client {
            // Create an Indexer domain model for HDBits
            let mut hdbits_indexer =
                Indexer::new("HDBits".to_string(), IndexerImplementation::HDBits);

            // Configure HDBits-specific settings
            hdbits_indexer.update_settings(json!({
                "base_url": "https://hdbits.org",
                "supports_search": true,
                "supports_rss": false,
                "categories": ["Movies"],
                "rate_limit_per_hour": 150
            }));

            // Check health and update status
            match client.health_check().await {
                Ok(true) => {
                    hdbits_indexer.set_enabled(true);
                    println!("🟢 HDBits is healthy and enabled");
                }
                Ok(false) => {
                    hdbits_indexer.set_enabled(false);
                    println!("🔴 HDBits health check failed - disabled");
                }
                Err(e) => {
                    hdbits_indexer.set_enabled(false);
                    println!("🔴 HDBits error: {} - disabled", e);
                }
            }

            indexers.push(hdbits_indexer);
        }

        indexers
    }

    /// Search across all available indexers
    pub async fn search(
        &self,
        query: &str,
        limit: Option<i32>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let mut total_searched = 0;
        let mut errors = Vec::new();

        // Search HDBits if available
        if let Some(client) = &self.hdbits_client {
            total_searched += 1;

            let search_request = radarr_indexers::SearchRequest {
                query: Some(query.to_string()),
                imdb_id: None,
                tmdb_id: None,
                categories: vec![2000], // Movies
                indexer_ids: vec![],
                limit,
                offset: None,
                min_seeders: Some(1),
                min_size: None,
                max_size: None,
            };

            match client.search(&search_request).await {
                Ok(response) => {
                    results.extend(response.results);
                    println!("✅ HDBits returned {} results", response.total);
                }
                Err(e) => {
                    errors.push(format!("HDBits: {}", e));
                    println!("❌ HDBits search failed: {}", e);
                }
            }
        }

        Ok(json!({
            "query": query,
            "total_results": results.len(),
            "indexers_searched": total_searched,
            "indexers_with_errors": errors.len(),
            "errors": errors,
            "results": results.into_iter().take(limit.unwrap_or(50) as usize).collect::<Vec<_>>()
        }))
    }

    /// Test all indexer connections
    pub async fn test_all_connections(&self) -> Value {
        let mut results = Vec::new();

        if let Some(client) = &self.hdbits_client {
            let start = std::time::Instant::now();
            match client.health_check().await {
                Ok(healthy) => {
                    results.push(json!({
                        "indexer": "HDBits",
                        "healthy": healthy,
                        "response_time_ms": start.elapsed().as_millis(),
                        "error": null
                    }));
                }
                Err(e) => {
                    results.push(json!({
                        "indexer": "HDBits",
                        "healthy": false,
                        "response_time_ms": start.elapsed().as_millis(),
                        "error": e.to_string()
                    }));
                }
            }
        }

        json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "tests": results
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🚀 Starting HDBits Integration Example");
    println!("=====================================");

    // Initialize indexer service
    let service = IndexerService::new().await?;

    // Test 1: Get all available indexers
    println!("\n📋 Available Indexers:");
    println!("---------------------");
    let indexers = service.get_indexers().await;
    for indexer in &indexers {
        println!(
            "  🔗 {} ({}) - Enabled: {}",
            indexer.name, indexer.implementation, indexer.enabled
        );

        if let Some(base_url) = indexer.base_url() {
            println!("      URL: {}", base_url);
        }
    }

    // Test 2: Test all connections
    println!("\n🔍 Testing Connections:");
    println!("----------------------");
    let health_results = service.test_all_connections().await;
    println!("{}", serde_json::to_string_pretty(&health_results)?);

    // Test 3: Search for popular movies
    println!("\n🎬 Searching for Movies:");
    println!("-----------------------");

    let search_queries = vec!["Dune", "The Matrix", "Blade Runner 2049"];

    for query in search_queries {
        println!("\n🔍 Searching for: '{}'", query);

        match service.search(query, Some(3)).await {
            Ok(results) => {
                let total = results["total_results"].as_u64().unwrap_or(0);
                let searched = results["indexers_searched"].as_u64().unwrap_or(0);
                let errors = results["indexers_with_errors"].as_u64().unwrap_or(0);

                println!(
                    "   📊 Summary: {} results from {} indexers ({} errors)",
                    total, searched, errors
                );

                if let Some(releases) = results["results"].as_array() {
                    for (i, release) in releases.iter().enumerate() {
                        if let (Some(title), Some(indexer)) =
                            (release["title"].as_str(), release["indexer"].as_str())
                        {
                            let size = release["size"]
                                .as_i64()
                                .map(|s| format_size(s))
                                .unwrap_or_else(|| "Unknown".to_string());
                            let seeders = release["seeders"].as_i64().unwrap_or(0);

                            println!("   {}. 📦 {} [{}]", i + 1, title, indexer);
                            println!("      💾 Size: {} | 🌱 Seeders: {}", size, seeders);
                        }
                    }
                }

                if let Some(error_list) = results["errors"].as_array() {
                    if !error_list.is_empty() {
                        println!("   ⚠️ Errors:");
                        for error in error_list {
                            if let Some(error_str) = error.as_str() {
                                println!("      - {}", error_str);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Search failed: {}", e);
            }
        }
    }

    println!("\n✅ HDBits Integration Example Complete!");
    println!("========================================");

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
