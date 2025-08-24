use async_trait::async_trait;
use chrono::{DateTime, Utc};
use radarr_core::{
    streaming::{
        traits::{
            AvailabilityRepository, ComingSoonRepository, OAuthTokenRepository,
            StreamingCacheRepository, TrendingRepository,
        },
        AvailabilityItem, ComingSoon, IdMapping, MediaType, OAuthToken, TimeWindow, TrendingEntry,
        TrendingSource,
    },
    RadarrError,
};
use rust_decimal::Decimal;
use serde_json::Value as JsonValue;
use sqlx::{PgPool, Row};
use tracing::{debug, error, info};
use uuid::Uuid;

pub struct PostgresStreamingCache {
    pool: PgPool,
}

impl PostgresStreamingCache {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StreamingCacheRepository for PostgresStreamingCache {
    async fn get_raw(&self, key: &str) -> Result<Option<JsonValue>, RadarrError> {
        let result = sqlx::query!(
            r#"
            SELECT data, expires_at
            FROM streaming_cache
            WHERE cache_key = $1
            "#,
            key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to fetch cache entry: {}", e),
        })?;

        match result {
            Some(row) if row.expires_at > Utc::now() => Ok(Some(row.data)),
            Some(_) => {
                // Entry expired, delete it
                let _ = self.delete(key).await;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn set_raw(&self, key: &str, data: JsonValue, ttl_hours: i64) -> Result<(), RadarrError> {
        let expires_at = Utc::now() + chrono::Duration::hours(ttl_hours);

        sqlx::query!(
            r#"
            INSERT INTO streaming_cache (cache_key, data, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (cache_key)
            DO UPDATE SET 
                data = EXCLUDED.data,
                expires_at = EXCLUDED.expires_at,
                updated_at = CURRENT_TIMESTAMP
            "#,
            key,
            data,
            expires_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to set cache entry: {}", e),
        })?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), RadarrError> {
        sqlx::query!("DELETE FROM streaming_cache WHERE cache_key = $1", key)
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to delete cache entry: {}", e),
            })?;

        Ok(())
    }

    async fn clear_expired(&self) -> Result<usize, RadarrError> {
        let result =
            sqlx::query!("DELETE FROM streaming_cache WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&self.pool)
                .await
                .map_err(|e| RadarrError::DatabaseError {
                    message: format!("Failed to clear expired cache: {}", e),
                })?;

        Ok(result.rows_affected() as usize)
    }

    async fn store_id_mappings(&self, mappings: Vec<IdMapping>) -> Result<usize, RadarrError> {
        let mut count = 0;

        for mapping in mappings {
            let media_type_str = mapping.media_type.as_str();

            sqlx::query!(
                r#"
                INSERT INTO streaming_id_mappings (tmdb_id, watchmode_id, media_type)
                VALUES ($1, $2, $3)
                ON CONFLICT (tmdb_id)
                DO UPDATE SET 
                    watchmode_id = EXCLUDED.watchmode_id,
                    media_type = EXCLUDED.media_type,
                    last_verified = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                "#,
                mapping.tmdb_id,
                mapping.watchmode_id,
                media_type_str
            )
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to store id mapping: {}", e),
            })?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_watchmode_id(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
    ) -> Result<Option<i32>, RadarrError> {
        let media_type_str = media_type.as_str();

        let result = sqlx::query!(
            r#"
            SELECT watchmode_id
            FROM streaming_id_mappings
            WHERE tmdb_id = $1 AND media_type = $2
            "#,
            tmdb_id,
            media_type_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to get watchmode id: {}", e),
        })?;

        Ok(result.and_then(|r| r.watchmode_id))
    }

    async fn get_id_mapping(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
    ) -> Result<Option<IdMapping>, RadarrError> {
        let media_type_str = media_type.as_str();

        let result = sqlx::query!(
            r#"
            SELECT tmdb_id, watchmode_id, media_type, last_verified
            FROM streaming_id_mappings
            WHERE tmdb_id = $1 AND media_type = $2
            "#,
            tmdb_id,
            media_type_str
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to get id mapping: {}", e),
        })?;

        match result {
            Some(row) => {
                let media_type = match row.media_type.as_str() {
                    "movie" => MediaType::Movie,
                    "tv" => MediaType::Tv,
                    _ => return Ok(None),
                };

                Ok(Some(IdMapping {
                    tmdb_id: row.tmdb_id,
                    watchmode_id: row.watchmode_id,
                    media_type,
                    last_verified: row.last_verified.unwrap_or_else(|| Utc::now()),
                }))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl TrendingRepository for PostgresStreamingCache {
    async fn store_trending(&self, entries: Vec<TrendingEntry>) -> Result<usize, RadarrError> {
        let mut count = 0;

        for entry in entries {
            let media_type_str = entry.media_type.as_str();
            let source_str = entry.source.as_str();
            let window_str = entry.time_window.as_str();

            sqlx::query!(
                r#"
                INSERT INTO trending_entries (
                    tmdb_id, media_type, title, release_date, poster_path, backdrop_path,
                    overview, source, time_window, rank, score, vote_average, vote_count,
                    popularity, fetched_at, expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                ON CONFLICT (tmdb_id, media_type, source, time_window)
                DO UPDATE SET 
                    title = EXCLUDED.title,
                    release_date = EXCLUDED.release_date,
                    poster_path = EXCLUDED.poster_path,
                    backdrop_path = EXCLUDED.backdrop_path,
                    overview = EXCLUDED.overview,
                    rank = EXCLUDED.rank,
                    score = EXCLUDED.score,
                    vote_average = EXCLUDED.vote_average,
                    vote_count = EXCLUDED.vote_count,
                    popularity = EXCLUDED.popularity,
                    fetched_at = EXCLUDED.fetched_at,
                    expires_at = EXCLUDED.expires_at
                "#,
                entry.tmdb_id,
                media_type_str,
                entry.title,
                entry.release_date,
                entry.poster_path,
                entry.backdrop_path,
                entry.overview,
                source_str,
                window_str,
                entry.rank,
                entry
                    .score
                    .and_then(|s| rust_decimal::Decimal::from_f32_retain(s)),
                entry
                    .vote_average
                    .and_then(|v| rust_decimal::Decimal::from_f32_retain(v)),
                entry.vote_count,
                entry
                    .popularity
                    .and_then(|p| rust_decimal::Decimal::from_f32_retain(p)),
                entry.fetched_at,
                entry.expires_at
            )
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to store trending entry: {}", e),
            })?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_trending(
        &self,
        media_type: MediaType,
        source: TrendingSource,
        window: TimeWindow,
    ) -> Result<Vec<TrendingEntry>, RadarrError> {
        let media_type_str = media_type.as_str();
        let source_str = source.as_str();
        let window_str = window.as_str();

        let rows = sqlx::query!(
            r#"
            SELECT 
                id, tmdb_id, media_type, title, release_date, poster_path, backdrop_path,
                overview, source, time_window, rank, score, vote_average, vote_count,
                popularity, fetched_at, expires_at
            FROM trending_entries
            WHERE media_type = $1 AND source = $2 AND time_window = $3 AND expires_at > CURRENT_TIMESTAMP
            ORDER BY rank ASC NULLS LAST, score DESC NULLS LAST
            "#,
            media_type_str,
            source_str,
            window_str
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to get trending entries: {}", e),
        })?;

        let entries: Vec<TrendingEntry> = rows
            .into_iter()
            .map(|row| {
                let media_type = match row.media_type.as_str() {
                    "movie" => MediaType::Movie,
                    "tv" => MediaType::Tv,
                    _ => MediaType::Movie,
                };

                let source = match row.source.as_str() {
                    "tmdb" => TrendingSource::Tmdb,
                    "trakt" => TrendingSource::Trakt,
                    "aggregated" => TrendingSource::Aggregated,
                    _ => TrendingSource::Aggregated,
                };

                let time_window = match row.time_window.as_str() {
                    "day" => TimeWindow::Day,
                    "week" => TimeWindow::Week,
                    _ => TimeWindow::Day,
                };

                TrendingEntry {
                    id: Some(row.id),
                    tmdb_id: row.tmdb_id,
                    media_type,
                    title: row.title,
                    release_date: row.release_date,
                    poster_path: row.poster_path,
                    backdrop_path: row.backdrop_path,
                    overview: row.overview,
                    source,
                    time_window,
                    rank: row.rank,
                    score: row.score.and_then(|s| s.to_string().parse::<f32>().ok()),
                    vote_average: row
                        .vote_average
                        .and_then(|v| v.to_string().parse::<f32>().ok()),
                    vote_count: row.vote_count,
                    popularity: row
                        .popularity
                        .and_then(|p| p.to_string().parse::<f32>().ok()),
                    fetched_at: row.fetched_at.unwrap_or_else(|| Utc::now()),
                    expires_at: row.expires_at,
                }
            })
            .collect();

        Ok(entries)
    }

    async fn clear_expired_trending(&self) -> Result<usize, RadarrError> {
        let result =
            sqlx::query!("DELETE FROM trending_entries WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&self.pool)
                .await
                .map_err(|e| RadarrError::DatabaseError {
                    message: format!("Failed to clear expired trending: {}", e),
                })?;

        Ok(result.rows_affected() as usize)
    }
}

#[async_trait]
impl AvailabilityRepository for PostgresStreamingCache {
    async fn store_availability(&self, items: Vec<AvailabilityItem>) -> Result<usize, RadarrError> {
        let mut count = 0;

        for item in items {
            let media_type_str = item.media_type.as_str();
            let service_type_str = item.service_type.as_str();
            let quality_str = item.quality.as_ref().map(|q| q.as_str());

            sqlx::query!(
                r#"
                INSERT INTO streaming_availability (
                    tmdb_id, media_type, region, service_name, service_type, service_logo_url,
                    deep_link, price_amount, price_currency, quality, leaving_date,
                    fetched_at, expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ON CONFLICT (tmdb_id, media_type, region, service_name, service_type)
                DO UPDATE SET 
                    service_logo_url = EXCLUDED.service_logo_url,
                    deep_link = EXCLUDED.deep_link,
                    price_amount = EXCLUDED.price_amount,
                    price_currency = EXCLUDED.price_currency,
                    quality = EXCLUDED.quality,
                    leaving_date = EXCLUDED.leaving_date,
                    fetched_at = EXCLUDED.fetched_at,
                    expires_at = EXCLUDED.expires_at
                "#,
                item.tmdb_id,
                media_type_str,
                item.region,
                item.service_name,
                service_type_str,
                item.service_logo_url,
                item.deep_link,
                item.price_amount.and_then(|p| Decimal::from_f32_retain(p)),
                item.price_currency,
                quality_str,
                item.leaving_date,
                item.fetched_at,
                item.expires_at
            )
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to store availability item: {}", e),
            })?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_availability(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        region: &str,
    ) -> Result<Vec<AvailabilityItem>, RadarrError> {
        let media_type_str = media_type.as_str();

        let rows = sqlx::query!(
            r#"
            SELECT 
                id, tmdb_id, media_type, region, service_name, service_type, service_logo_url,
                deep_link, price_amount, price_currency, quality, leaving_date,
                fetched_at, expires_at
            FROM streaming_availability
            WHERE tmdb_id = $1 AND media_type = $2 AND region = $3 AND expires_at > CURRENT_TIMESTAMP
            ORDER BY service_type, service_name
            "#,
            tmdb_id,
            media_type_str,
            region
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to get availability items: {}", e),
        })?;

        let items: Vec<AvailabilityItem> = rows
            .into_iter()
            .map(|row| {
                use radarr_core::streaming::ServiceType;
                use radarr_core::streaming::VideoQuality;

                let media_type = match row.media_type.as_str() {
                    "movie" => MediaType::Movie,
                    "tv" => MediaType::Tv,
                    _ => MediaType::Movie,
                };

                let service_type = match row.service_type.as_deref() {
                    Some("subscription") => ServiceType::Subscription,
                    Some("rent") => ServiceType::Rent,
                    Some("buy") => ServiceType::Buy,
                    Some("free") => ServiceType::Free,
                    Some("ads") => ServiceType::Ads,
                    _ => ServiceType::Subscription,
                };

                let quality = row.quality.and_then(|q| match q.as_str() {
                    "SD" => Some(VideoQuality::SD),
                    "HD" => Some(VideoQuality::HD),
                    "4K" => Some(VideoQuality::UHD4K),
                    "HDR" => Some(VideoQuality::HDR),
                    _ => None,
                });

                AvailabilityItem {
                    id: Some(row.id),
                    tmdb_id: row.tmdb_id,
                    media_type,
                    region: row.region.unwrap_or_else(|| "US".to_string()),
                    service_name: row.service_name,
                    service_type,
                    service_logo_url: row.service_logo_url,
                    deep_link: row.deep_link,
                    price_amount: row
                        .price_amount
                        .and_then(|p| p.to_string().parse::<f32>().ok()),
                    price_currency: row.price_currency.unwrap_or_else(|| "USD".to_string()),
                    quality,
                    leaving_date: row.leaving_date,
                    fetched_at: row.fetched_at.unwrap_or_else(|| Utc::now()),
                    expires_at: row.expires_at,
                }
            })
            .collect();

        Ok(items)
    }

    async fn clear_expired_availability(&self) -> Result<usize, RadarrError> {
        let result =
            sqlx::query!("DELETE FROM streaming_availability WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&self.pool)
                .await
                .map_err(|e| RadarrError::DatabaseError {
                    message: format!("Failed to clear expired availability: {}", e),
                })?;

        Ok(result.rows_affected() as usize)
    }
}

#[async_trait]
impl ComingSoonRepository for PostgresStreamingCache {
    async fn store_coming_soon(&self, entries: Vec<ComingSoon>) -> Result<usize, RadarrError> {
        let mut count = 0;

        for entry in entries {
            let media_type_str = entry.media_type.as_str();
            let streaming_services = serde_json::to_value(&entry.streaming_services)
                .unwrap_or(serde_json::Value::Array(vec![]));

            sqlx::query!(
                r#"
                INSERT INTO coming_soon_releases (
                    tmdb_id, media_type, title, release_date, poster_path, backdrop_path,
                    overview, source, region, streaming_services, fetched_at, expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                ON CONFLICT (tmdb_id, media_type, source, region)
                DO UPDATE SET 
                    title = EXCLUDED.title,
                    release_date = EXCLUDED.release_date,
                    poster_path = EXCLUDED.poster_path,
                    backdrop_path = EXCLUDED.backdrop_path,
                    overview = EXCLUDED.overview,
                    streaming_services = EXCLUDED.streaming_services,
                    fetched_at = EXCLUDED.fetched_at,
                    expires_at = EXCLUDED.expires_at
                "#,
                entry.tmdb_id,
                media_type_str,
                entry.title,
                entry.release_date,
                entry.poster_path,
                entry.backdrop_path,
                entry.overview,
                entry.source,
                entry.region,
                streaming_services,
                entry.fetched_at,
                entry.expires_at
            )
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to store coming soon entry: {}", e),
            })?;

            count += 1;
        }

        Ok(count)
    }

    async fn get_coming_soon(
        &self,
        media_type: MediaType,
        region: &str,
    ) -> Result<Vec<ComingSoon>, RadarrError> {
        let media_type_str = media_type.as_str();

        let rows = sqlx::query!(
            r#"
            SELECT 
                id, tmdb_id, media_type, title, release_date, poster_path, backdrop_path,
                overview, source, region, streaming_services, fetched_at, expires_at
            FROM coming_soon_releases
            WHERE media_type = $1 AND region = $2 AND expires_at > CURRENT_TIMESTAMP
            ORDER BY release_date ASC
            "#,
            media_type_str,
            region
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to get coming soon entries: {}", e),
        })?;

        let entries: Vec<ComingSoon> = rows
            .into_iter()
            .map(|row| {
                let media_type = match row.media_type.as_str() {
                    "movie" => MediaType::Movie,
                    "tv" => MediaType::Tv,
                    _ => MediaType::Movie,
                };

                let streaming_services: Vec<String> = row
                    .streaming_services
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                ComingSoon {
                    id: Some(row.id),
                    tmdb_id: row.tmdb_id,
                    media_type,
                    title: row.title,
                    release_date: row.release_date,
                    poster_path: row.poster_path,
                    backdrop_path: row.backdrop_path,
                    overview: row.overview,
                    source: row.source,
                    region: row.region.unwrap_or_else(|| "US".to_string()),
                    streaming_services,
                    fetched_at: row.fetched_at.unwrap_or_else(|| Utc::now()),
                    expires_at: row.expires_at,
                }
            })
            .collect();

        Ok(entries)
    }

    async fn clear_expired_coming_soon(&self) -> Result<usize, RadarrError> {
        let result =
            sqlx::query!("DELETE FROM coming_soon_releases WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&self.pool)
                .await
                .map_err(|e| RadarrError::DatabaseError {
                    message: format!("Failed to clear expired coming soon: {}", e),
                })?;

        Ok(result.rows_affected() as usize)
    }
}

#[async_trait]
impl OAuthTokenRepository for PostgresStreamingCache {
    async fn get_token(&self, service: &str) -> Result<Option<OAuthToken>, RadarrError> {
        let result = sqlx::query!(
            r#"
            SELECT 
                id, service, access_token, refresh_token, token_type, 
                expires_at, scope, created_at, updated_at
            FROM oauth_tokens
            WHERE service = $1
            "#,
            service
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to get oauth token: {}", e),
        })?;

        match result {
            Some(row) => Ok(Some(OAuthToken {
                id: Some(row.id),
                service: row.service,
                access_token: row.access_token,
                refresh_token: row.refresh_token,
                token_type: row.token_type.unwrap_or_else(|| "Bearer".to_string()),
                expires_at: row.expires_at,
                scope: row.scope,
                created_at: row.created_at.unwrap_or_else(|| Utc::now()),
                updated_at: row.updated_at.unwrap_or_else(|| Utc::now()),
            })),
            None => Ok(None),
        }
    }

    async fn store_token(&self, token: OAuthToken) -> Result<(), RadarrError> {
        sqlx::query!(
            r#"
            INSERT INTO oauth_tokens (
                service, access_token, refresh_token, token_type, 
                expires_at, scope
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (service)
            DO UPDATE SET 
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token,
                token_type = EXCLUDED.token_type,
                expires_at = EXCLUDED.expires_at,
                scope = EXCLUDED.scope,
                updated_at = CURRENT_TIMESTAMP
            "#,
            token.service,
            token.access_token,
            token.refresh_token,
            token.token_type,
            token.expires_at,
            token.scope
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to store oauth token: {}", e),
        })?;

        Ok(())
    }

    async fn update_token(&self, service: &str, token: OAuthToken) -> Result<(), RadarrError> {
        sqlx::query!(
            r#"
            UPDATE oauth_tokens
            SET 
                access_token = $2,
                refresh_token = $3,
                token_type = $4,
                expires_at = $5,
                scope = $6,
                updated_at = CURRENT_TIMESTAMP
            WHERE service = $1
            "#,
            service,
            token.access_token,
            token.refresh_token,
            token.token_type,
            token.expires_at,
            token.scope
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to update oauth token: {}", e),
        })?;

        Ok(())
    }

    async fn delete_token(&self, service: &str) -> Result<(), RadarrError> {
        sqlx::query!("DELETE FROM oauth_tokens WHERE service = $1", service)
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to delete oauth token: {}", e),
            })?;

        Ok(())
    }
}
