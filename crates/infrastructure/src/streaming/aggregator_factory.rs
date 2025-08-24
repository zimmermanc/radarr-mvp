use radarr_core::streaming::{
    aggregator::TrendingAggregator,
    traits::{
        StreamingAggregator, StreamingCacheRepository, TrendingRepository,
        AvailabilityRepository, OAuthTokenRepository, StreamingConfig,
    },
};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use crate::{
    // repositories::PostgresStreamingCache, // Temporarily disabled
    tmdb::TmdbStreamingClient,
    trakt::TraktClient,
    watchmode::WatchmodeClient,
};

/// Factory for creating streaming service components
pub struct StreamingServiceFactory {
    pool: PgPool,
    tmdb_api_key: String,
    trakt_client_id: Option<String>,
    trakt_client_secret: Option<String>,
    watchmode_api_key: Option<String>,
}

impl StreamingServiceFactory {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            tmdb_api_key: String::new(),
            trakt_client_id: None,
            trakt_client_secret: None,
            watchmode_api_key: None,
        }
    }

    pub fn with_tmdb(mut self, api_key: String) -> Self {
        self.tmdb_api_key = api_key;
        self
    }

    pub fn with_trakt(mut self, client_id: String, client_secret: String) -> Self {
        self.trakt_client_id = Some(client_id);
        self.trakt_client_secret = Some(client_secret);
        self
    }

    pub fn with_watchmode(mut self, api_key: String) -> Self {
        self.watchmode_api_key = Some(api_key);
        self
    }

    /// Build the complete streaming aggregator with all configured services
    pub fn build_aggregator(self) -> Arc<dyn StreamingAggregator> {
        info!("Building streaming service aggregator");

        // Create repositories
        // let cache_repo = Arc::new(PostgresStreamingCache::new(self.pool.clone())); // Temporarily disabled
        let trending_repo = cache_repo.clone() as Arc<dyn TrendingRepository>;
        let availability_repo = cache_repo.clone() as Arc<dyn AvailabilityRepository>;
        let token_repo = cache_repo.clone() as Arc<dyn OAuthTokenRepository>;

        // Create TMDB client (required)
        let tmdb_client: Arc<dyn radarr_core::streaming::traits::TmdbAdapter> = 
            Arc::new(TmdbStreamingClient::new(self.tmdb_api_key.clone()));

        // Create Trakt client (optional)
        let trakt_client: Arc<dyn radarr_core::streaming::traits::TraktAdapter> = 
            if let (Some(client_id), Some(client_secret)) = 
                (self.trakt_client_id.clone(), self.trakt_client_secret.clone()) {
                info!("Trakt integration enabled");
                Arc::new(TraktClient::new(client_id, client_secret, token_repo))
            } else {
                info!("Trakt integration disabled (no credentials)");
                // Create a dummy client that returns empty results
                Arc::new(TraktClient::new(
                    "dummy".to_string(),
                    "dummy".to_string(),
                    cache_repo.clone(),
                ))
            };

        // Create Watchmode client (optional)
        let watchmode_client: Option<Arc<dyn radarr_core::streaming::traits::WatchmodeAdapter>> = 
            if let Some(api_key) = self.watchmode_api_key.clone() {
                info!("Watchmode integration enabled");
                Some(Arc::new(WatchmodeClient::new(
                    Some(api_key),
                    cache_repo.clone(),
                )))
            } else {
                info!("Watchmode integration disabled (no API key)");
                None
            };

        // Build configuration
        let config = self.build_config();

        // Create aggregator
        let aggregator = TrendingAggregator::new(
            tmdb_client,
            trakt_client,
            watchmode_client,
            cache_repo,
            trending_repo,
            availability_repo,
            config,
        );

        Arc::new(aggregator)
    }

    /// Build just the cache repository
    pub fn build_cache_repository(self) -> Arc<dyn StreamingCacheRepository> {
        // Arc::new(PostgresStreamingCache::new(self.pool)) // Temporarily disabled
        unimplemented!("StreamingCache temporarily disabled for compilation")
    }

    /// Build streaming configuration with cache TTLs
    fn build_config(&self) -> StreamingConfig {
        let mut cache_ttl = HashMap::new();
        
        // Configure cache TTLs (in hours)
        cache_ttl.insert("tmdb_trending".to_string(), 3);      // 3 hours for TMDB trending
        cache_ttl.insert("tmdb_providers".to_string(), 24);    // 24 hours for watch providers
        cache_ttl.insert("trakt_trending".to_string(), 1);     // 1 hour for Trakt trending
        cache_ttl.insert("watchmode_availability".to_string(), 12); // 12 hours for Watchmode
        cache_ttl.insert("aggregated_trending".to_string(), 1); // 1 hour for aggregated data
        cache_ttl.insert("coming_soon".to_string(), 24);       // 24 hours for upcoming releases

        StreamingConfig {
            tmdb_api_key: self.tmdb_api_key.clone(),
            trakt_client_id: self.trakt_client_id.clone().unwrap_or_default(),
            trakt_client_secret: self.trakt_client_secret.clone().unwrap_or_default(),
            watchmode_api_key: self.watchmode_api_key.clone(),
            default_region: "US".to_string(),
            cache_ttl_hours: cache_ttl,
        }
    }
}

/// Create a default aggregator from environment variables
pub fn create_default_aggregator(pool: PgPool) -> Arc<dyn StreamingAggregator> {
    use std::env;

    let tmdb_api_key = env::var("TMDB_API_KEY")
        .expect("TMDB_API_KEY environment variable must be set");

    let trakt_client_id = env::var("TRAKT_CLIENT_ID").ok();
    let trakt_client_secret = env::var("TRAKT_CLIENT_SECRET").ok();
    let watchmode_api_key = env::var("WATCHMODE_API_KEY").ok();

    let mut factory = StreamingServiceFactory::new(pool)
        .with_tmdb(tmdb_api_key);

    if let (Some(id), Some(secret)) = (trakt_client_id, trakt_client_secret) {
        factory = factory.with_trakt(id, secret);
    }

    if let Some(api_key) = watchmode_api_key {
        factory = factory.with_watchmode(api_key);
    }

    factory.build_aggregator()
}