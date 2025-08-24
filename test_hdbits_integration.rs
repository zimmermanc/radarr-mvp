use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = HDBitsConfig {
        username: std::env::var("HDBITS_USERNAME").expect("HDBITS_USERNAME must be set"),
        passkey: std::env::var("HDBITS_PASSKEY").expect("HDBITS_PASSKEY must be set"),
        timeout_seconds: 30,
        rate_limit_per_hour: 120,
    };

    println!("Creating HDBits client...");
    let client = HDBitsClient::new(config)?;

    let search = MovieSearchRequest {
        title: Some("Matrix".to_string()),
        year: None,
        imdb_id: None,
        limit: Some(2),
        min_seeders: None,
    };

    println!("Searching for Matrix movies...");
    match client.search_movies(&search).await {
        Ok(results) => {
            println!("✅ Found {} results", results.len());
            for r in results.iter().take(3) {
                println!("  - {} ({})", r.title, r.download_url);
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
