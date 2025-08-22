use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let config = HDBitsConfig {
        username: "blargdiesel".to_string(),
        session_cookie: "ed487790cd0dee98941ab5c132179bd2c8c5e23622c0c04a800ad543cde2990cd44ed960892d990214ea1618bf29780386a77246a21dc636d83420e077e69863".to_string(),
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