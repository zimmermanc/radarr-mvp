use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Movie,
    Tv,
}

impl MediaType {
    pub fn as_str(&self) -> &str {
        match self {
            MediaType::Movie => "movie",
            MediaType::Tv => "tv",
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeWindow {
    Day,
    Week,
}

impl TimeWindow {
    pub fn as_str(&self) -> &str {
        match self {
            TimeWindow::Day => "day",
            TimeWindow::Week => "week",
        }
    }
}

impl std::fmt::Display for TimeWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrendingSource {
    Tmdb,
    Trakt,
    Aggregated,
}

impl TrendingSource {
    pub fn as_str(&self) -> &str {
        match self {
            TrendingSource::Tmdb => "tmdb",
            TrendingSource::Trakt => "trakt",
            TrendingSource::Aggregated => "aggregated",
        }
    }
}

impl std::fmt::Display for TrendingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Title {
    pub tmdb_id: i32,
    pub media_type: MediaType,
    pub title: String,
    pub original_title: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingEntry {
    pub id: Option<Uuid>,
    pub tmdb_id: i32,
    pub media_type: MediaType,
    pub title: String,
    pub release_date: Option<NaiveDate>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub source: TrendingSource,
    pub time_window: TimeWindow,
    pub rank: Option<i32>,
    pub score: Option<f32>,
    pub vote_average: Option<f32>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f32>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl TrendingEntry {
    pub fn new(
        tmdb_id: i32,
        media_type: MediaType,
        title: String,
        source: TrendingSource,
        time_window: TimeWindow,
    ) -> Self {
        let now = Utc::now();
        let expires_at = match source {
            TrendingSource::Tmdb => now + chrono::Duration::hours(3),
            TrendingSource::Trakt => now + chrono::Duration::hours(1),
            TrendingSource::Aggregated => now + chrono::Duration::hours(1),
        };

        Self {
            id: None,
            tmdb_id,
            media_type,
            title,
            release_date: None,
            poster_path: None,
            backdrop_path: None,
            overview: None,
            source,
            time_window,
            rank: None,
            score: None,
            vote_average: None,
            vote_count: None,
            popularity: None,
            fetched_at: now,
            expires_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    Subscription,
    Rent,
    Buy,
    Free,
    Ads,
}

impl ServiceType {
    pub fn as_str(&self) -> &str {
        match self {
            ServiceType::Subscription => "subscription",
            ServiceType::Rent => "rent",
            ServiceType::Buy => "buy",
            ServiceType::Free => "free",
            ServiceType::Ads => "ads",
        }
    }
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum VideoQuality {
    SD,
    HD,
    #[serde(rename = "4K")]
    UHD4K,
    HDR,
}

impl VideoQuality {
    pub fn as_str(&self) -> &str {
        match self {
            VideoQuality::SD => "SD",
            VideoQuality::HD => "HD",
            VideoQuality::UHD4K => "4K",
            VideoQuality::HDR => "HDR",
        }
    }
}

impl std::fmt::Display for VideoQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityItem {
    pub id: Option<Uuid>,
    pub tmdb_id: i32,
    pub media_type: MediaType,
    pub region: String,
    pub service_name: String,
    pub service_type: ServiceType,
    pub service_logo_url: Option<String>,
    pub deep_link: Option<String>,
    pub price_amount: Option<f32>,
    pub price_currency: String,
    pub quality: Option<VideoQuality>,
    pub leaving_date: Option<NaiveDate>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AvailabilityItem {
    pub fn new(
        tmdb_id: i32,
        media_type: MediaType,
        region: String,
        service_name: String,
        service_type: ServiceType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            tmdb_id,
            media_type,
            region,
            service_name,
            service_type,
            service_logo_url: None,
            deep_link: None,
            price_amount: None,
            price_currency: "USD".to_string(),
            quality: None,
            leaving_date: None,
            fetched_at: now,
            expires_at: now + chrono::Duration::hours(24),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Availability {
    pub tmdb_id: i32,
    pub media_type: MediaType,
    pub region: String,
    pub items: Vec<AvailabilityItem>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComingSoon {
    pub id: Option<Uuid>,
    pub tmdb_id: i32,
    pub media_type: MediaType,
    pub title: String,
    pub release_date: NaiveDate,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub overview: Option<String>,
    pub source: String,
    pub region: String,
    pub streaming_services: Vec<String>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl ComingSoon {
    pub fn new(
        tmdb_id: i32,
        media_type: MediaType,
        title: String,
        release_date: NaiveDate,
        source: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            tmdb_id,
            media_type,
            title,
            release_date,
            poster_path: None,
            backdrop_path: None,
            overview: None,
            source,
            region: "US".to_string(),
            streaming_services: Vec::new(),
            fetched_at: now,
            expires_at: now + chrono::Duration::hours(24),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdMapping {
    pub tmdb_id: i32,
    pub watchmode_id: Option<i32>,
    pub media_type: MediaType,
    pub last_verified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub id: Option<Uuid>,
    pub service: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub scope: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    pub fn needs_refresh(&self) -> bool {
        // Refresh if token expires in the next 5 minutes
        Utc::now() + chrono::Duration::minutes(5) >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub key: String,
    pub data: T,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    pub fn new(key: String, data: T, ttl_hours: i64) -> Self {
        let now = Utc::now();
        Self {
            key,
            data,
            expires_at: now + chrono::Duration::hours(ttl_hours),
            created_at: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingProvider {
    pub name: String,
    pub display_name: String,
    pub logo_url: Option<String>,
    pub supported_regions: Vec<String>,
    pub service_types: Vec<ServiceType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingResponse {
    pub media_type: MediaType,
    pub time_window: TimeWindow,
    pub source: TrendingSource,
    pub entries: Vec<TrendingEntry>,
    pub total_results: usize,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResponse {
    pub tmdb_id: i32,
    pub media_type: MediaType,
    pub title: Option<String>,
    pub region: String,
    pub availability: HashMap<String, Vec<AvailabilityItem>>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComingSoonResponse {
    pub media_type: MediaType,
    pub region: String,
    pub entries: Vec<ComingSoon>,
    pub total_results: usize,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraktDeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: i32,
    pub interval: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraktTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub refresh_token: String,
    pub scope: String,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_display() {
        assert_eq!(MediaType::Movie.as_str(), "movie");
        assert_eq!(MediaType::Tv.as_str(), "tv");
        assert_eq!(format!("{}", MediaType::Movie), "movie");
    }

    #[test]
    fn test_time_window_display() {
        assert_eq!(TimeWindow::Day.as_str(), "day");
        assert_eq!(TimeWindow::Week.as_str(), "week");
        assert_eq!(format!("{}", TimeWindow::Day), "day");
    }

    #[test]
    fn test_trending_entry_creation() {
        let entry = TrendingEntry::new(
            550,
            MediaType::Movie,
            "Fight Club".to_string(),
            TrendingSource::Tmdb,
            TimeWindow::Day,
        );

        assert_eq!(entry.tmdb_id, 550);
        assert_eq!(entry.media_type, MediaType::Movie);
        assert_eq!(entry.title, "Fight Club");
        assert_eq!(entry.source, TrendingSource::Tmdb);
        assert_eq!(entry.time_window, TimeWindow::Day);
        assert!(entry.expires_at > entry.fetched_at);
    }

    #[test]
    fn test_oauth_token_expiry() {
        let mut token = OAuthToken {
            id: None,
            service: "trakt".to_string(),
            access_token: "test_token".to_string(),
            refresh_token: Some("refresh_token".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            scope: Some("public".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!token.is_expired());
        assert!(!token.needs_refresh());

        // Token expiring soon
        token.expires_at = Utc::now() + chrono::Duration::minutes(3);
        assert!(token.needs_refresh());

        // Token expired
        token.expires_at = Utc::now() - chrono::Duration::minutes(1);
        assert!(token.is_expired());
        assert!(token.needs_refresh());
    }

    #[test]
    fn test_cache_entry() {
        let data = vec!["test".to_string()];
        let cache = CacheEntry::new("test_key".to_string(), data.clone(), 24);

        assert_eq!(cache.key, "test_key");
        assert_eq!(cache.data, data);
        assert!(!cache.is_expired());
        assert!(cache.expires_at > cache.created_at);
    }
}