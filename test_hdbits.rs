use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};

#[tokio::main]
async fn main() {
    let config = HDBitsConfig {
        username: "blargdiesel".to_string(),
        passkey: "ed487790cd0dee98941ab5c132179bd2c8c5e23622c0c04a800ad543cde2990cd44ed960892d990214ea1618bf29780386a77246a21dc636d83420e077e69863".to_string(),
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
