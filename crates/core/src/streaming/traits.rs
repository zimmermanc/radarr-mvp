use async_trait::async_trait;
use std::collections::HashMap;

use super::models::*;
use crate::error::RadarrError;

#[async_trait]
pub trait TmdbAdapter: Send + Sync {
    async fn trending_movies(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError>;
    async fn trending_tv(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError>;
    async fn upcoming_movies(&self) -> Result<Vec<ComingSoon>, RadarrError>;
    async fn on_the_air(&self) -> Result<Vec<ComingSoon>, RadarrError>;
    async fn watch_providers(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        region: &str,
    ) -> Result<Availability, RadarrError>;
}

#[async_trait]
pub trait TraktAdapter: Send + Sync {
    async fn authenticate_device(&self) -> Result<TraktDeviceCode, RadarrError>;
    async fn poll_for_token(&self, device_code: &str) -> Result<TraktTokenResponse, RadarrError>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TraktTokenResponse, RadarrError>;
    async fn trending_movies(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError>;
    async fn trending_shows(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError>;
}

#[async_trait]
pub trait WatchmodeAdapter: Send + Sync {
    async fn refresh_id_mappings(&self) -> Result<Vec<IdMapping>, RadarrError>;
    async fn get_watchmode_id(&self, tmdb_id: i32, media_type: MediaType) -> Result<Option<i32>, RadarrError>;
    async fn sources_by_tmdb(&self, tmdb_id: i32, media_type: MediaType) -> Result<Availability, RadarrError>;
    async fn streaming_releases(&self, region: &str) -> Result<Vec<ComingSoon>, RadarrError>;
}

#[async_trait]
pub trait StreamingCacheRepository: Send + Sync {
    async fn get_raw(&self, key: &str) -> Result<Option<serde_json::Value>, RadarrError>;
    async fn set_raw(&self, key: &str, data: serde_json::Value, ttl_hours: i64) -> Result<(), RadarrError>;
    async fn delete(&self, key: &str) -> Result<(), RadarrError>;
    async fn clear_expired(&self) -> Result<usize, RadarrError>;
    
    // ID mapping specific methods
    async fn store_id_mappings(&self, mappings: Vec<IdMapping>) -> Result<usize, RadarrError>;
    async fn get_watchmode_id(&self, tmdb_id: i32, media_type: MediaType) -> Result<Option<i32>, RadarrError>;
    async fn get_id_mapping(&self, tmdb_id: i32, media_type: MediaType) -> Result<Option<IdMapping>, RadarrError>;
}

#[async_trait]
pub trait OAuthTokenRepository: Send + Sync {
    async fn get_token(&self, service: &str) -> Result<Option<OAuthToken>, RadarrError>;
    async fn store_token(&self, token: OAuthToken) -> Result<(), RadarrError>;
    async fn update_token(&self, service: &str, token: OAuthToken) -> Result<(), RadarrError>;
    async fn delete_token(&self, service: &str) -> Result<(), RadarrError>;
}

#[async_trait]
pub trait TrendingRepository: Send + Sync {
    async fn store_trending(&self, entries: Vec<TrendingEntry>) -> Result<usize, RadarrError>;
    async fn get_trending(
        &self,
        media_type: MediaType,
        source: TrendingSource,
        window: TimeWindow,
    ) -> Result<Vec<TrendingEntry>, RadarrError>;
    async fn clear_expired_trending(&self) -> Result<usize, RadarrError>;
}

#[async_trait]
pub trait AvailabilityRepository: Send + Sync {
    async fn store_availability(&self, items: Vec<AvailabilityItem>) -> Result<usize, RadarrError>;
    async fn get_availability(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        region: &str,
    ) -> Result<Vec<AvailabilityItem>, RadarrError>;
    async fn clear_expired_availability(&self) -> Result<usize, RadarrError>;
}

#[async_trait]
pub trait ComingSoonRepository: Send + Sync {
    async fn store_coming_soon(&self, entries: Vec<ComingSoon>) -> Result<usize, RadarrError>;
    async fn get_coming_soon(
        &self,
        media_type: MediaType,
        region: &str,
    ) -> Result<Vec<ComingSoon>, RadarrError>;
    async fn clear_expired_coming_soon(&self) -> Result<usize, RadarrError>;
}

#[async_trait]
pub trait StreamingAggregator: Send + Sync {
    async fn get_trending(
        &self,
        media_type: MediaType,
        window: TimeWindow,
    ) -> Result<TrendingResponse, RadarrError>;
    
    async fn get_availability(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        region: &str,
    ) -> Result<AvailabilityResponse, RadarrError>;
    
    async fn get_coming_soon(
        &self,
        media_type: MediaType,
        region: &str,
    ) -> Result<ComingSoonResponse, RadarrError>;
    
    async fn refresh_cache(&self) -> Result<(), RadarrError>;
}

pub struct StreamingConfig {
    pub tmdb_api_key: String,
    pub trakt_client_id: String,
    pub trakt_client_secret: String,
    pub watchmode_api_key: Option<String>,
    pub default_region: String,
    pub cache_ttl_hours: HashMap<String, i64>,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        let mut cache_ttl = HashMap::new();
        cache_ttl.insert("tmdb_trending".to_string(), 3);
        cache_ttl.insert("tmdb_providers".to_string(), 24);
        cache_ttl.insert("trakt_trending".to_string(), 1);
        cache_ttl.insert("watchmode_availability".to_string(), 12);
        cache_ttl.insert("aggregated_trending".to_string(), 1);
        cache_ttl.insert("coming_soon".to_string(), 24);
        
        Self {
            tmdb_api_key: String::new(),
            trakt_client_id: String::new(),
            trakt_client_secret: String::new(),
            watchmode_api_key: None,
            default_region: "US".to_string(),
            cache_ttl_hours: cache_ttl,
        }
    }
}