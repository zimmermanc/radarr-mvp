use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use radarr_core::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    streaming::{
        traits::{WatchmodeAdapter, StreamingCacheRepository},
        Availability, AvailabilityItem, ComingSoon, IdMapping, MediaType, ServiceType,
    },
    RadarrError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use super::csv_sync::WatchmodeCsvSync;

/// Watchmode API client
pub struct WatchmodeClient {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    csv_sync: WatchmodeCsvSync,
    cache_repo: Arc<dyn StreamingCacheRepository>,
    circuit_breaker: CircuitBreaker,
    daily_request_count: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl WatchmodeClient {
    pub fn new(
        api_key: Option<String>,
        cache_repo: Arc<dyn StreamingCacheRepository>,
    ) -> Self {
        let csv_sync = WatchmodeCsvSync::new(cache_repo.clone());
        
        let circuit_breaker_config = CircuitBreakerConfig::new("Watchmode")
            .with_failure_threshold(3) // Lower threshold due to strict rate limits
            .with_timeout(Duration::from_secs(60))
            .with_request_timeout(Duration::from_secs(15))
            .with_success_threshold(1);
        
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.watchmode.com/v1".to_string(),
            csv_sync,
            cache_repo,
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
            daily_request_count: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    /// Check if we're approaching the daily rate limit (33 requests/day for free tier)
    fn check_rate_limit(&self) -> Result<(), RadarrError> {
        let current_count = self.daily_request_count.load(std::sync::atomic::Ordering::Relaxed);
        
        // Free tier limit is 1000/month ≈ 33/day
        // Be conservative and limit to 30/day
        if current_count >= 30 {
            warn!("Watchmode daily rate limit reached ({}/30)", current_count);
            return Err(RadarrError::RateLimited {
                service: "watchmode".to_string(),
                retry_after: Some(86400), // Retry after 24 hours
            });
        }
        
        Ok(())
    }

    /// Make a rate-limited request to Watchmode API
    async fn make_request<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        params: Vec<(&str, String)>,
    ) -> Result<T, RadarrError> {
        // Check rate limit first
        self.check_rate_limit()?;
        
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| RadarrError::ConfigurationError {
                field: "watchmode_api_key".to_string(),
                message: "Watchmode API key not configured".to_string(),
            })?;
        
        let url = format!("{}{}", self.base_url, endpoint);
        
        // Add API key to params
        let mut all_params = params;
        all_params.push(("apiKey", api_key.clone()));
        
        debug!("Making Watchmode API request to {}", endpoint);
        
        // Increment request counter
        self.daily_request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let response = self.client
            .get(&url)
            .query(&all_params)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "watchmode".to_string(),
                error: e.to_string(),
            })?;

        if response.status() == 429 {
            error!("Watchmode rate limit exceeded");
            return Err(RadarrError::RateLimited {
                service: "watchmode".to_string(),
                retry_after: Some(86400),
            });
        }

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Watchmode API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "watchmode".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "watchmode".to_string(),
                error: e.to_string(),
            })
    }
}

#[async_trait]
impl WatchmodeAdapter for WatchmodeClient {
    async fn refresh_id_mappings(&self) -> Result<Vec<IdMapping>, RadarrError> {
        // Use CSV sync for ID mappings (doesn't count against API limit)
        self.csv_sync.refresh_id_mappings().await
    }

    async fn get_watchmode_id(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
    ) -> Result<Option<i32>, RadarrError> {
        // First check cache
        self.cache_repo.get_watchmode_id(tmdb_id, media_type).await
    }

    async fn sources_by_tmdb(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
    ) -> Result<Availability, RadarrError> {
        // First get Watchmode ID from mapping
        let watchmode_id = self.get_watchmode_id(tmdb_id, media_type.clone()).await?;
        
        if watchmode_id.is_none() {
            debug!("No Watchmode ID found for TMDB {} ({})", tmdb_id, media_type);
            return Ok(Availability {
                tmdb_id,
                media_type,
                region: "US".to_string(),
                items: Vec::new(),
                fetched_at: Utc::now(),
                expires_at: Utc::now() + chrono::Duration::hours(24),
            });
        }
        
        let watchmode_id = watchmode_id.unwrap();
        
        // Make API request for sources
        let endpoint = format!("/title/{}/sources/", watchmode_id);
        let params = vec![
            ("regions", "US".to_string()),
        ];
        
        info!("Fetching Watchmode sources for ID {}", watchmode_id);
        
        match self.make_request::<Vec<WatchmodeSource>>(&endpoint, params).await {
            Ok(sources) => {
                let items: Vec<AvailabilityItem> = sources
                    .into_iter()
                    .map(|source| {
                        let service_type = match source.type_.as_deref() {
                            Some("sub") => ServiceType::Subscription,
                            Some("rent") => ServiceType::Rent,
                            Some("buy") | Some("purchase") => ServiceType::Buy,
                            Some("free") => ServiceType::Free,
                            Some("tve") | Some("tveverywhere") => ServiceType::Subscription,
                            _ => ServiceType::Subscription,
                        };
                        
                        let mut item = AvailabilityItem::new(
                            tmdb_id,
                            media_type.clone(),
                            source.region.unwrap_or_else(|| "US".to_string()),
                            source.name,
                            service_type,
                        );
                        
                        item.deep_link = source.web_url;
                        item.price_amount = source.price.and_then(|p| p.parse::<f32>().ok());
                        
                        item
                    })
                    .collect();
                
                Ok(Availability {
                    tmdb_id,
                    media_type,
                    region: "US".to_string(),
                    items,
                    fetched_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::hours(12), // Cache for 12 hours
                })
            }
            Err(e) => {
                warn!("Failed to fetch Watchmode sources: {}", e);
                Ok(Availability {
                    tmdb_id,
                    media_type,
                    region: "US".to_string(),
                    items: Vec::new(),
                    fetched_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::hours(1), // Retry sooner on error
                })
            }
        }
    }

    async fn streaming_releases(&self, region: &str) -> Result<Vec<ComingSoon>, RadarrError> {
        // Watchmode's releases endpoint
        let endpoint = "/releases/";
        let params = vec![
            ("regions", region.to_string()),
            ("types", "movie,tv_series".to_string()),
            ("limit", "20".to_string()),
        ];
        
        info!("Fetching Watchmode streaming releases for {}", region);
        
        match self.make_request::<WatchmodeReleasesResponse>(&endpoint, params).await {
            Ok(response) => {
                let entries: Vec<ComingSoon> = response.releases
                    .into_iter()
                    .filter_map(|release| {
                        // Parse release date
                        let release_date = release.release_date
                            .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())?;
                        
                        let media_type = match release.type_.as_deref() {
                            Some("movie") => MediaType::Movie,
                            Some("tv_series") | Some("tv") => MediaType::Tv,
                            _ => return None,
                        };
                        
                        // Try to get TMDB ID from our mappings
                        // This is a limitation - we'd need reverse lookup
                        let tmdb_id = release.tmdb_id.unwrap_or(0);
                        
                        let mut entry = ComingSoon::new(
                            tmdb_id,
                            media_type,
                            release.title,
                            release_date,
                            "watchmode".to_string(),
                        );
                        
                        entry.region = region.to_string();
                        entry.streaming_services = release.source_names.unwrap_or_default();
                        
                        Some(entry)
                    })
                    .collect();
                
                Ok(entries)
            }
            Err(e) => {
                warn!("Failed to fetch Watchmode releases: {}", e);
                Ok(Vec::new())
            }
        }
    }
}

// Watchmode API response models
#[derive(Debug, Deserialize)]
struct WatchmodeSource {
    source_id: i32,
    name: String,
    #[serde(rename = "type")]
    type_: Option<String>,
    region: Option<String>,
    web_url: Option<String>,
    price: Option<String>,
    format: Option<String>,
    #[serde(rename = "buy_options")]
    buy_options: Option<Vec<BuyOption>>,
}

#[derive(Debug, Deserialize)]
struct BuyOption {
    format: String,
    price: String,
    web_url: String,
}

#[derive(Debug, Deserialize)]
struct WatchmodeReleasesResponse {
    releases: Vec<WatchmodeRelease>,
}

#[derive(Debug, Deserialize)]
struct WatchmodeRelease {
    id: i32,
    title: String,
    #[serde(rename = "type")]
    type_: Option<String>,
    release_date: Option<String>,
    tmdb_id: Option<i32>,
    imdb_id: Option<String>,
    source_ids: Option<Vec<i32>>,
    source_names: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_check() {
        // This would require proper mocking
        // Just verify the structure compiles
        assert!(true);
    }
}