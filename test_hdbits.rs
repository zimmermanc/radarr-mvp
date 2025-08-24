use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};

#[tokio::main]
async fn main() {
    let config = HDBitsConfig {
        username: std::env::var("HDBITS_USERNAME").expect("HDBITS_USERNAME must be set"),
        passkey: std::env::var("HDBITS_PASSKEY").expect("HDBITS_PASSKEY must be set"),
        api_url: "https://hdbits.org/api/torrents".to_string(),
        timeout_seconds: 30,
        rate_limit_per_hour: 120,
    };
    
    let client = HDBitsClient::new(config).unwrap();
    let search = MovieSearchRequest {
        title: Some("Matrix".to_string()),
        year: None,
        imdb_id: None,
        limit: Some(2),
        min_seeders: None,
    };
    
    match client.search_movies(&search).await {
        Ok(results) => {
            println!("Found {} results", results.len());
            for r in results.iter().take(2) {
                println!("- {}", r.title);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
