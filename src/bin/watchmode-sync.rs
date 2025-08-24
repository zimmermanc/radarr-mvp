use radarr_infrastructure::{
    database::create_pool,
    repositories::PostgresStreamingCache,
    watchmode::WatchmodeCsvSync,
};
use std::env;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://radarr:password@localhost:5432/radarr".to_string());

    info!("Starting Watchmode CSV ID mapping sync");

    // Create database pool
    let db_config = radarr_infrastructure::DatabaseConfig {
        database_url,
        max_connections: 1,
        ..Default::default()
    };
    let pool = create_pool(db_config).await?;
    let cache_repo = Arc::new(PostgresStreamingCache::new(pool));

    // Create CSV sync
    let csv_sync = WatchmodeCsvSync::new(cache_repo);

    // Perform sync
    match csv_sync.refresh_id_mappings().await {
        Ok(mappings) => {
            println!("\n===========================================");
            println!("Watchmode ID Mapping Sync Complete");
            println!("===========================================");
            println!("\n✅ Successfully synced {} ID mappings", mappings.len());
            
            // Show some statistics
            let movie_count = mappings.iter()
                .filter(|m| matches!(m.media_type, radarr_core::streaming::MediaType::Movie))
                .count();
            let tv_count = mappings.iter()
                .filter(|m| matches!(m.media_type, radarr_core::streaming::MediaType::Tv))
                .count();
            
            println!("\nBreakdown:");
            println!("  Movies: {}", movie_count);
            println!("  TV Shows: {}", tv_count);
            
            // Show a few sample mappings
            println!("\nSample mappings:");
            for mapping in mappings.iter().take(5) {
                if let Some(wm_id) = mapping.watchmode_id {
                    println!("  TMDB {} ({}) -> Watchmode {}", 
                        mapping.tmdb_id, 
                        mapping.media_type, 
                        wm_id
                    );
                }
            }
            
            println!("\n💡 Tip: This sync should be run weekly to keep mappings up to date");
            println!("   Consider setting up a cron job or scheduled task");
        }
        Err(e) => {
            error!("Failed to sync Watchmode ID mappings: {}", e);
            println!("\n❌ Sync failed!");
            println!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}