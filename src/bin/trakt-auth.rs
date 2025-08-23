use chrono::Utc;
use radarr_core::streaming::traits::OAuthTokenRepository;
use radarr_infrastructure::{
    database::create_pool,
    repositories::PostgresStreamingCache,
    trakt::{TraktOAuth, TraktClient},
};
use std::env;
use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get configuration from environment
    let client_id = env::var("TRAKT_CLIENT_ID")
        .expect("TRAKT_CLIENT_ID environment variable must be set");
    let client_secret = env::var("TRAKT_CLIENT_SECRET")
        .expect("TRAKT_CLIENT_SECRET environment variable must be set");
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://radarr:password@localhost:5432/radarr".to_string());

    info!("Initializing Trakt OAuth device flow authentication");

    // Create database pool
    let pool = create_pool(&database_url).await?;
    let token_repo = Arc::new(PostgresStreamingCache::new(pool.clone()));

    // Create OAuth client
    let oauth_config = radarr_infrastructure::trakt::oauth::TraktOAuthConfig::new(
        client_id.clone(),
        client_secret.clone(),
    );
    let oauth = TraktOAuth::new(oauth_config);

    // Initiate device flow
    let device_code = oauth.initiate_device_flow().await?;

    println!("\n===========================================");
    println!("Trakt Authentication Required");
    println!("===========================================");
    println!("\n1. Visit this URL in your browser:");
    println!("   {}", device_code.verification_url);
    println!("\n2. Enter this code:");
    println!("   {}", device_code.user_code);
    println!("\n3. Authorize the application");
    println!("\nWaiting for authorization (expires in {} seconds)...", device_code.expires_in);
    println!("===========================================\n");

    // Poll for token
    match oauth.poll_for_token_with_retry(
        &device_code.device_code,
        device_code.expires_in,
        device_code.interval,
    ).await {
        Ok(token_response) => {
            info!("Successfully obtained access token!");
            
            // Convert to OAuthToken and store
            let oauth_token = oauth.token_to_oauth(token_response);
            
            // Store in database
            token_repo.store_token(oauth_token.clone()).await?;
            
            println!("\n✅ Authentication successful!");
            println!("Token expires at: {}", oauth_token.expires_at);
            println!("Token stored in database for automatic refresh");
            
            // Test the token by fetching trending movies
            println!("\nTesting token by fetching trending movies...");
            let client = TraktClient::new(client_id, client_secret, token_repo);
            
            use radarr_core::streaming::{traits::TraktAdapter, TimeWindow};
            match client.trending_movies(TimeWindow::Day).await {
                Ok(movies) => {
                    println!("✅ Successfully fetched {} trending movies", movies.len());
                    if !movies.is_empty() {
                        println!("\nTop 5 trending movies:");
                        for (i, movie) in movies.iter().take(5).enumerate() {
                            println!("  {}. {}", i + 1, movie.title);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch trending movies: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Authentication failed: {}", e);
            println!("\n❌ Authentication failed!");
            println!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}