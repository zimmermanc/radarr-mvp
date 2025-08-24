use clap::Parser;
use radarr_core::streaming::traits::OAuthTokenRepository;
use radarr_infrastructure::{
    database::create_pool,
    repositories::streaming_cache::PostgresStreamingCache,
    trakt::oauth::{TraktOAuth, TraktOAuthConfig},
};
use std::env;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Trakt Client ID (or set TRAKT_CLIENT_ID env var)
    #[arg(long, env = "TRAKT_CLIENT_ID")]
    client_id: Option<String>,

    /// Trakt Client Secret (or set TRAKT_CLIENT_SECRET env var)
    #[arg(long, env = "TRAKT_CLIENT_SECRET")]
    client_secret: Option<String>,

    /// Database URL (or set DATABASE_URL env var)
    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "postgresql://radarr:radarr@localhost:5432/radarr"
    )]
    database_url: String,

    /// Skip saving to database (just test the flow)
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("radarr=debug".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    let args = Args::parse();

    // Get credentials
    let client_id = args
        .client_id
        .or_else(|| env::var("TRAKT_CLIENT_ID").ok())
        .ok_or("TRAKT_CLIENT_ID not provided")?;

    let client_secret = args
        .client_secret
        .or_else(|| env::var("TRAKT_CLIENT_SECRET").ok())
        .ok_or("TRAKT_CLIENT_SECRET not provided")?;

    info!("Initializing Trakt OAuth flow");

    // Create OAuth client
    let config = TraktOAuthConfig::new(client_id, client_secret);
    let oauth_client = TraktOAuth::new(config);

    // Initiate device flow
    let device_code = oauth_client.initiate_device_flow().await?;

    println!("\n===========================================");
    println!("TRAKT AUTHORIZATION REQUIRED");
    println!("===========================================");
    println!(
        "\n1. Visit this URL in your browser:\n   {}",
        device_code.verification_url
    );
    println!("\n2. Enter this code: {}", device_code.user_code);
    println!("\n3. Authorize the application");
    println!(
        "\nWaiting for authorization (expires in {} seconds)...\n",
        device_code.expires_in
    );

    // Poll for token
    match oauth_client
        .poll_for_token_with_retry(
            &device_code.device_code,
            device_code.expires_in,
            device_code.interval,
        )
        .await
    {
        Ok(token_response) => {
            println!("\n✅ Authorization successful!");
            println!(
                "Access Token: {}...",
                &token_response.access_token[..20.min(token_response.access_token.len())]
            );
            println!("Token Type: {}", token_response.token_type);
            println!("Expires In: {} seconds", token_response.expires_in);
            println!("Scope: {}", token_response.scope);

            if !args.dry_run {
                // Save to database
                info!("Saving token to database");
                let db_config = radarr_infrastructure::DatabaseConfig {
                    database_url: args.database_url.clone(),
                    max_connections: 1,
                    ..Default::default()
                };
                let pool = create_pool(db_config).await?;
                let cache_repo = PostgresStreamingCache::new(pool);

                let oauth_token = oauth_client.token_to_oauth(token_response);
                cache_repo.store_token(oauth_token).await?;

                println!("\n✅ Token saved to database!");
                println!("\nYou can now use Trakt API features in the application.");
            } else {
                println!("\n(Dry run - token not saved to database)");
            }
        }
        Err(e) => {
            error!("Authorization failed: {}", e);
            println!("\n❌ Authorization failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
