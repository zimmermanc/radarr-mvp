use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use radarr_core::{
    streaming::{
        traits::TmdbAdapter, Availability, AvailabilityItem, ComingSoon, MediaType, ServiceType,
        TimeWindow, TrendingEntry, TrendingSource, VideoQuality,
    },
    RadarrError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info};

use super::client::TmdbClient;

/// Extended TMDB client with streaming service features
pub struct TmdbStreamingClient {
    client: Client,
    api_key: String,
    base_url: String,
    tmdb_client: TmdbClient,
}

impl TmdbStreamingClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.clone(),
            base_url: "https://api.themoviedb.org/3".to_string(),
            tmdb_client: TmdbClient::new(api_key.clone()),
        }
    }
}

#[async_trait]
impl TmdbAdapter for TmdbStreamingClient {
    async fn trending_movies(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError> {
        let time_window = match window {
            TimeWindow::Day => "day",
            TimeWindow::Week => "week",
        };
        
        let url = format!("{}/trending/movie/{}", self.base_url, time_window);
        
        info!("Fetching TMDB trending movies for {}", time_window);
        
        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.api_key)])
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let tmdb_response: TmdbTrendingResponse = response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;
        
        debug!("TMDB trending returned {} movies", tmdb_response.results.len());
        
        // Convert to TrendingEntry
        let entries: Vec<TrendingEntry> = tmdb_response.results
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let mut entry = TrendingEntry::new(
                    item.id,
                    MediaType::Movie,
                    item.title.clone(),
                    TrendingSource::Tmdb,
                    window.clone(),
                );
                
                entry.rank = Some((index + 1) as i32);
                entry.release_date = item.release_date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
                entry.poster_path = item.poster_path;
                entry.backdrop_path = item.backdrop_path;
                entry.overview = item.overview;
                entry.vote_average = item.vote_average.map(|v| v as f32);
                entry.vote_count = item.vote_count;
                entry.popularity = item.popularity.map(|p| p as f32);
                
                entry
            })
            .collect();

        Ok(entries)
    }

    async fn trending_tv(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError> {
        let time_window = match window {
            TimeWindow::Day => "day",
            TimeWindow::Week => "week",
        };
        
        let url = format!("{}/trending/tv/{}", self.base_url, time_window);
        
        info!("Fetching TMDB trending TV shows for {}", time_window);
        
        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.api_key)])
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let tmdb_response: TmdbTrendingTvResponse = response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;
        
        debug!("TMDB trending returned {} TV shows", tmdb_response.results.len());
        
        // Convert to TrendingEntry
        let entries: Vec<TrendingEntry> = tmdb_response.results
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let mut entry = TrendingEntry::new(
                    item.id,
                    MediaType::Tv,
                    item.name.clone(),
                    TrendingSource::Tmdb,
                    window.clone(),
                );
                
                entry.rank = Some((index + 1) as i32);
                entry.release_date = item.first_air_date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
                entry.poster_path = item.poster_path;
                entry.backdrop_path = item.backdrop_path;
                entry.overview = item.overview;
                entry.vote_average = item.vote_average.map(|v| v as f32);
                entry.vote_count = item.vote_count;
                entry.popularity = item.popularity.map(|p| p as f32);
                
                entry
            })
            .collect();

        Ok(entries)
    }

    async fn upcoming_movies(&self) -> Result<Vec<ComingSoon>, RadarrError> {
        let url = format!("{}/movie/upcoming", self.base_url);
        
        info!("Fetching TMDB upcoming movies");
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("region", &"US".to_string()),
            ])
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let tmdb_response: TmdbUpcomingResponse = response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;
        
        debug!("TMDB upcoming returned {} movies", tmdb_response.results.len());
        
        // Convert to ComingSoon
        let entries: Vec<ComingSoon> = tmdb_response.results
            .into_iter()
            .filter_map(|item| {
                item.release_date.and_then(|date_str| {
                    NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok().map(|date| {
                        let mut entry = ComingSoon::new(
                            item.id,
                            MediaType::Movie,
                            item.title.clone(),
                            date,
                            "tmdb".to_string(),
                        );
                        
                        entry.poster_path = item.poster_path;
                        entry.backdrop_path = item.backdrop_path;
                        entry.overview = item.overview;
                        
                        entry
                    })
                })
            })
            .collect();

        Ok(entries)
    }

    async fn on_the_air(&self) -> Result<Vec<ComingSoon>, RadarrError> {
        let url = format!("{}/tv/on_the_air", self.base_url);
        
        info!("Fetching TMDB on the air TV shows");
        
        let response = self.client
            .get(&url)
            .query(&[
                ("api_key", &self.api_key),
                ("region", &"US".to_string()),
            ])
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("TMDB API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let tmdb_response: TmdbOnTheAirResponse = response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;
        
        debug!("TMDB on the air returned {} TV shows", tmdb_response.results.len());
        
        // Convert to ComingSoon
        let entries: Vec<ComingSoon> = tmdb_response.results
            .into_iter()
            .filter_map(|item| {
                item.first_air_date.and_then(|date_str| {
                    NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok().map(|date| {
                        let mut entry = ComingSoon::new(
                            item.id,
                            MediaType::Tv,
                            item.name.clone(),
                            date,
                            "tmdb".to_string(),
                        );
                        
                        entry.poster_path = item.poster_path;
                        entry.backdrop_path = item.backdrop_path;
                        entry.overview = item.overview;
                        
                        entry
                    })
                })
            })
            .collect();

        Ok(entries)
    }

    async fn watch_providers(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        region: &str,
    ) -> Result<Availability, RadarrError> {
        let media_type_str = match media_type {
            MediaType::Movie => "movie",
            MediaType::Tv => "tv",
        };
        
        let url = format!("{}/{}/{}/watch/providers", self.base_url, media_type_str, tmdb_id);
        
        info!("Fetching TMDB watch providers for {} {} in {}", media_type_str, tmdb_id, region);
        
        let response = self.client
            .get(&url)
            .query(&[("api_key", &self.api_key)])
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            
            // If 404, return empty availability
            if status == 404 {
                debug!("No watch providers found for {} {}", media_type_str, tmdb_id);
                return Ok(Availability {
                    tmdb_id,
                    media_type,
                    region: region.to_string(),
                    items: Vec::new(),
                    fetched_at: Utc::now(),
                    expires_at: Utc::now() + chrono::Duration::hours(24),
                });
            }
            
            error!("TMDB API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        let providers_response: TmdbProvidersResponse = response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "tmdb".to_string(),
                error: e.to_string(),
            })?;
        
        // Get providers for the requested region
        let region_providers = providers_response.results.get(region);
        
        let mut items = Vec::new();
        
        if let Some(providers) = region_providers {
            // Add subscription services
            if let Some(flatrate) = &providers.flatrate {
                for provider in flatrate {
                    let mut item = AvailabilityItem::new(
                        tmdb_id,
                        media_type.clone(),
                        region.to_string(),
                        provider.provider_name.clone(),
                        ServiceType::Subscription,
                    );
                    item.service_logo_url = provider.logo_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w92{}", p));
                    items.push(item);
                }
            }
            
            // Add rental services
            if let Some(rent) = &providers.rent {
                for provider in rent {
                    let mut item = AvailabilityItem::new(
                        tmdb_id,
                        media_type.clone(),
                        region.to_string(),
                        provider.provider_name.clone(),
                        ServiceType::Rent,
                    );
                    item.service_logo_url = provider.logo_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w92{}", p));
                    items.push(item);
                }
            }
            
            // Add purchase services
            if let Some(buy) = &providers.buy {
                for provider in buy {
                    let mut item = AvailabilityItem::new(
                        tmdb_id,
                        media_type.clone(),
                        region.to_string(),
                        provider.provider_name.clone(),
                        ServiceType::Buy,
                    );
                    item.service_logo_url = provider.logo_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w92{}", p));
                    items.push(item);
                }
            }
            
            // Add free services
            if let Some(free) = &providers.free {
                for provider in free {
                    let mut item = AvailabilityItem::new(
                        tmdb_id,
                        media_type.clone(),
                        region.to_string(),
                        provider.provider_name.clone(),
                        ServiceType::Free,
                    );
                    item.service_logo_url = provider.logo_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w92{}", p));
                    items.push(item);
                }
            }
            
            // Add ad-supported services
            if let Some(ads) = &providers.ads {
                for provider in ads {
                    let mut item = AvailabilityItem::new(
                        tmdb_id,
                        media_type.clone(),
                        region.to_string(),
                        provider.provider_name.clone(),
                        ServiceType::Ads,
                    );
                    item.service_logo_url = provider.logo_path.as_ref().map(|p| format!("https://image.tmdb.org/t/p/w92{}", p));
                    items.push(item);
                }
            }
        }
        
        debug!("Found {} watch providers for {} {} in {}", items.len(), media_type_str, tmdb_id, region);
        
        Ok(Availability {
            tmdb_id,
            media_type,
            region: region.to_string(),
            items,
            fetched_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        })
    }
}

// TMDB Response Models
#[derive(Debug, Deserialize)]
struct TmdbTrendingResponse {
    page: i32,
    results: Vec<TmdbTrendingMovie>,
    total_pages: i32,
    total_results: i32,
}

#[derive(Debug, Deserialize)]
struct TmdbTrendingMovie {
    id: i32,
    title: String,
    original_title: String,
    overview: Option<String>,
    release_date: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    vote_count: Option<i32>,
    popularity: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TmdbTrendingTvResponse {
    page: i32,
    results: Vec<TmdbTrendingTv>,
    total_pages: i32,
    total_results: i32,
}

#[derive(Debug, Deserialize)]
struct TmdbTrendingTv {
    id: i32,
    name: String,
    original_name: String,
    overview: Option<String>,
    first_air_date: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    vote_average: Option<f64>,
    vote_count: Option<i32>,
    popularity: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TmdbUpcomingResponse {
    dates: Option<DateRange>,
    page: i32,
    results: Vec<TmdbUpcomingMovie>,
    total_pages: i32,
    total_results: i32,
}

#[derive(Debug, Deserialize)]
struct DateRange {
    minimum: String,
    maximum: String,
}

#[derive(Debug, Deserialize)]
struct TmdbUpcomingMovie {
    id: i32,
    title: String,
    original_title: String,
    overview: Option<String>,
    release_date: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbOnTheAirResponse {
    page: i32,
    results: Vec<TmdbOnTheAirTv>,
    total_pages: i32,
    total_results: i32,
}

#[derive(Debug, Deserialize)]
struct TmdbOnTheAirTv {
    id: i32,
    name: String,
    original_name: String,
    overview: Option<String>,
    first_air_date: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbProvidersResponse {
    id: i32,
    results: HashMap<String, RegionProviders>,
}

#[derive(Debug, Deserialize)]
struct RegionProviders {
    link: Option<String>,
    flatrate: Option<Vec<Provider>>,
    rent: Option<Vec<Provider>>,
    buy: Option<Vec<Provider>>,
    free: Option<Vec<Provider>>,
    ads: Option<Vec<Provider>>,
}

#[derive(Debug, Deserialize)]
struct Provider {
    logo_path: Option<String>,
    provider_id: i32,
    provider_name: String,
    display_priority: Option<i32>,
}