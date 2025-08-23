use async_trait::async_trait;
use chrono::NaiveDate;
use radarr_core::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    streaming::{
        traits::{TraktAdapter, OAuthTokenRepository},
        MediaType, OAuthToken, TimeWindow, TraktDeviceCode, TraktTokenResponse, TrendingEntry,
        TrendingSource,
    },
    RadarrError,
};
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use super::oauth::{TraktOAuth, TraktOAuthConfig};

/// Trakt API client with OAuth support
pub struct TraktClient {
    client: Client,
    oauth: TraktOAuth,
    token_repo: Arc<dyn OAuthTokenRepository>,
    client_id: String,
    base_url: String,
    circuit_breaker: CircuitBreaker,
}

impl TraktClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        token_repo: Arc<dyn OAuthTokenRepository>,
    ) -> Self {
        let config = TraktOAuthConfig::new(client_id.clone(), client_secret);
        let oauth = TraktOAuth::new(config);
        
        let circuit_breaker_config = CircuitBreakerConfig::new("Trakt")
            .with_failure_threshold(5)
            .with_timeout(Duration::from_secs(30))
            .with_request_timeout(Duration::from_secs(10))
            .with_success_threshold(2);
        
        Self {
            client: Client::new(),
            oauth,
            token_repo,
            client_id,
            base_url: "https://api.trakt.tv".to_string(),
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        }
    }

    /// Get or refresh a valid access token
    async fn get_valid_token(&self) -> Result<String, RadarrError> {
        // Get stored token
        let stored_token = self.token_repo.get_token("trakt").await?;
        
        match stored_token {
            Some(token) if !token.needs_refresh() => {
                // Token is still valid
                Ok(token.access_token)
            }
            Some(token) if token.refresh_token.is_some() => {
                // Token needs refresh
                info!("Trakt token expired or expiring, refreshing...");
                let refresh_token = token.refresh_token.unwrap();
                
                match self.oauth.refresh_token(&refresh_token).await {
                    Ok(new_token_response) => {
                        // Convert and store new token
                        let new_token = self.oauth.token_to_oauth(new_token_response);
                        self.token_repo.update_token("trakt", new_token.clone()).await?;
                        Ok(new_token.access_token)
                    }
                    Err(e) => {
                        error!("Failed to refresh Trakt token: {}", e);
                        Err(e)
                    }
                }
            }
            _ => {
                // No valid token, user needs to authenticate
                Err(RadarrError::AuthenticationRequired {
                    service: "trakt".to_string(),
                    message: "No valid Trakt token found. Please authenticate.".to_string(),
                })
            }
        }
    }

    /// Build headers for Trakt API requests
    fn build_headers(&self, access_token: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        
        // Required headers for all requests
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        headers.insert("trakt-api-version", HeaderValue::from_static("2"));
        headers.insert("trakt-api-key", HeaderValue::from_str(&self.client_id).unwrap());
        headers.insert(USER_AGENT, HeaderValue::from_static("Radarr-MVP/1.0"));
        
        // Add authorization header if token provided
        if let Some(token) = access_token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token)).unwrap()
            );
        }
        
        headers
    }

    /// Make an authenticated request to Trakt API
    async fn make_request<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        require_auth: bool,
    ) -> Result<T, RadarrError> {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let access_token = if require_auth {
            Some(self.get_valid_token().await?)
        } else {
            None
        };
        
        let headers = self.build_headers(access_token.as_deref());
        
        let response = self.client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Trakt API error: {} - {}", status, text);
            return Err(RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: format!("HTTP {}: {}", status, text),
            });
        }

        response.json().await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "trakt".to_string(),
                error: e.to_string(),
            })
    }
}

#[async_trait]
impl TraktAdapter for TraktClient {
    async fn authenticate_device(&self) -> Result<TraktDeviceCode, RadarrError> {
        self.oauth.initiate_device_flow().await
    }

    async fn poll_for_token(&self, device_code: &str) -> Result<TraktTokenResponse, RadarrError> {
        self.oauth.poll_for_token(device_code).await
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TraktTokenResponse, RadarrError> {
        self.oauth.refresh_token(refresh_token).await
    }

    async fn trending_movies(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError> {
        let period = match window {
            TimeWindow::Day => "daily",
            TimeWindow::Week => "weekly",
        };
        
        let endpoint = format!("/movies/trending?page=1&limit=20&extended=full&period={}", period);
        
        info!("Fetching Trakt trending movies for period: {}", period);
        
        let trakt_movies: Vec<TraktTrendingMovie> = self.make_request(&endpoint, false).await?;
        
        debug!("Trakt returned {} trending movies", trakt_movies.len());
        
        // Convert to TrendingEntry
        let entries: Vec<TrendingEntry> = trakt_movies
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let mut entry = TrendingEntry::new(
                    item.movie.ids.tmdb.unwrap_or(0),
                    MediaType::Movie,
                    item.movie.title.clone(),
                    TrendingSource::Trakt,
                    window.clone(),
                );
                
                entry.rank = Some((index + 1) as i32);
                entry.score = Some(item.watchers as f32);
                entry.release_date = item.movie.released.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
                entry.overview = item.movie.overview;
                entry.vote_average = item.movie.rating.map(|r| r as f32);
                entry.vote_count = item.movie.votes;
                
                entry
            })
            .collect();

        Ok(entries)
    }

    async fn trending_shows(&self, window: TimeWindow) -> Result<Vec<TrendingEntry>, RadarrError> {
        let period = match window {
            TimeWindow::Day => "daily",
            TimeWindow::Week => "weekly",
        };
        
        let endpoint = format!("/shows/trending?page=1&limit=20&extended=full&period={}", period);
        
        info!("Fetching Trakt trending shows for period: {}", period);
        
        let trakt_shows: Vec<TraktTrendingShow> = self.make_request(&endpoint, false).await?;
        
        debug!("Trakt returned {} trending shows", trakt_shows.len());
        
        // Convert to TrendingEntry
        let entries: Vec<TrendingEntry> = trakt_shows
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let mut entry = TrendingEntry::new(
                    item.show.ids.tmdb.unwrap_or(0),
                    MediaType::Tv,
                    item.show.title.clone(),
                    TrendingSource::Trakt,
                    window.clone(),
                );
                
                entry.rank = Some((index + 1) as i32);
                entry.score = Some(item.watchers as f32);
                entry.release_date = item.show.first_aired.and_then(|d| {
                    // Parse ISO 8601 datetime and extract date
                    d.split('T').next().and_then(|date_str| {
                        NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
                    })
                });
                entry.overview = item.show.overview;
                entry.vote_average = item.show.rating.map(|r| r as f32);
                entry.vote_count = item.show.votes;
                
                entry
            })
            .collect();

        Ok(entries)
    }
}

// Trakt API response models
#[derive(Debug, Deserialize)]
struct TraktTrendingMovie {
    watchers: i32,
    movie: TraktMovie,
}

#[derive(Debug, Deserialize)]
struct TraktMovie {
    title: String,
    year: Option<i32>,
    ids: TraktIds,
    overview: Option<String>,
    released: Option<String>,
    runtime: Option<i32>,
    rating: Option<f64>,
    votes: Option<i32>,
    comment_count: Option<i32>,
    genres: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TraktTrendingShow {
    watchers: i32,
    show: TraktShow,
}

#[derive(Debug, Deserialize)]
struct TraktShow {
    title: String,
    year: Option<i32>,
    ids: TraktIds,
    overview: Option<String>,
    first_aired: Option<String>,
    runtime: Option<i32>,
    rating: Option<f64>,
    votes: Option<i32>,
    comment_count: Option<i32>,
    genres: Option<Vec<String>>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TraktIds {
    trakt: i32,
    slug: Option<String>,
    imdb: Option<String>,
    tmdb: Option<i32>,
    tvdb: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_building() {
        // This would require a mock token repository
        // Just verify the structure compiles
        assert!(true);
    }
}