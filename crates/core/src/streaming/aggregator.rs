use async_trait::async_trait;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::models::*;
use super::traits::*;
use crate::error::RadarrError;

pub struct TrendingAggregator {
    tmdb: Arc<dyn TmdbAdapter>,
    trakt: Arc<dyn TraktAdapter>,
    watchmode: Option<Arc<dyn WatchmodeAdapter>>,
    cache: Arc<dyn StreamingCacheRepository>,
    trending_repo: Arc<dyn TrendingRepository>,
    availability_repo: Arc<dyn AvailabilityRepository>,
    config: StreamingConfig,
}

impl TrendingAggregator {
    pub fn new(
        tmdb: Arc<dyn TmdbAdapter>,
        trakt: Arc<dyn TraktAdapter>,
        watchmode: Option<Arc<dyn WatchmodeAdapter>>,
        cache: Arc<dyn StreamingCacheRepository>,
        trending_repo: Arc<dyn TrendingRepository>,
        availability_repo: Arc<dyn AvailabilityRepository>,
        config: StreamingConfig,
    ) -> Self {
        Self {
            tmdb,
            trakt,
            watchmode,
            cache,
            trending_repo,
            availability_repo,
            config,
        }
    }

    async fn fetch_and_merge_trending(
        &self,
        media_type: MediaType,
        window: TimeWindow,
    ) -> Result<Vec<TrendingEntry>, RadarrError> {
        let cache_key = format!("aggregated_trending:{}:{}", media_type, window);
        
        // Check cache first
        if let Some(cached_value) = self.cache.get_raw(&cache_key).await? {
            if let Ok(cache_entry) = serde_json::from_value::<CacheEntry<Vec<TrendingEntry>>>(cached_value) {
                if !cache_entry.is_expired() {
                    debug!("Using cached aggregated trending data");
                    return Ok(cache_entry.data);
                }
            }
        }

        // Fetch from both sources in parallel
        let (tmdb_result, trakt_result) = tokio::join!(
            self.fetch_tmdb_trending(media_type.clone(), window.clone()),
            self.fetch_trakt_trending(media_type.clone(), window.clone())
        );

        let mut tmdb_entries = tmdb_result.unwrap_or_else(|e| {
            warn!("Failed to fetch TMDB trending: {}", e);
            Vec::new()
        });

        let trakt_entries = trakt_result.unwrap_or_else(|e| {
            warn!("Failed to fetch Trakt trending: {}", e);
            Vec::new()
        });

        // Merge and deduplicate by TMDB ID
        let mut merged = self.merge_trending_entries(tmdb_entries, trakt_entries);
        
        // Sort by aggregated score
        merged.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Update ranks
        for (index, entry) in merged.iter_mut().enumerate() {
            entry.rank = Some((index + 1) as i32);
        }

        // Cache the aggregated results
        let ttl = self.config.cache_ttl_hours.get("aggregated_trending").copied().unwrap_or(1);
        let cache_entry = CacheEntry::new(cache_key.clone(), merged.clone(), ttl);
        self.cache.set_raw(&cache_key, serde_json::to_value(cache_entry)?, ttl).await?;

        // Store in database
        self.trending_repo.store_trending(merged.clone()).await?;

        Ok(merged)
    }

    async fn fetch_tmdb_trending(
        &self,
        media_type: MediaType,
        window: TimeWindow,
    ) -> Result<Vec<TrendingEntry>, RadarrError> {
        let cache_key = format!("tmdb_trending:{}:{}", media_type, window);
        
        // Check cache
        if let Some(cached_value) = self.cache.get_raw(&cache_key).await? {
            if let Ok(cache_entry) = serde_json::from_value::<CacheEntry<Vec<TrendingEntry>>>(cached_value) {
                if !cache_entry.is_expired() {
                    return Ok(cache_entry.data);
                }
            }
        }

        // Fetch from TMDB
        let entries = match media_type {
            MediaType::Movie => self.tmdb.trending_movies(window).await?,
            MediaType::Tv => self.tmdb.trending_tv(window).await?,
        };

        // Cache the results
        let ttl = self.config.cache_ttl_hours.get("tmdb_trending").copied().unwrap_or(3);
        let cache_entry = CacheEntry::new(cache_key.clone(), entries.clone(), ttl);
        self.cache.set_raw(&cache_key, serde_json::to_value(cache_entry)?, ttl).await?;

        Ok(entries)
    }

    async fn fetch_trakt_trending(
        &self,
        media_type: MediaType,
        window: TimeWindow,
    ) -> Result<Vec<TrendingEntry>, RadarrError> {
        let cache_key = format!("trakt_trending:{}:{}", media_type, window);
        
        // Check cache
        if let Some(cached_value) = self.cache.get_raw(&cache_key).await? {
            if let Ok(cache_entry) = serde_json::from_value::<CacheEntry<Vec<TrendingEntry>>>(cached_value) {
                if !cache_entry.is_expired() {
                    return Ok(cache_entry.data);
                }
            }
        }

        // Fetch from Trakt
        let entries = match media_type {
            MediaType::Movie => self.trakt.trending_movies(window).await?,
            MediaType::Tv => self.trakt.trending_shows(window).await?,
        };

        // Cache the results
        let ttl = self.config.cache_ttl_hours.get("trakt_trending").copied().unwrap_or(1);
        let cache_entry = CacheEntry::new(cache_key.clone(), entries.clone(), ttl);
        self.cache.set_raw(&cache_key, serde_json::to_value(cache_entry)?, ttl).await?;

        Ok(entries)
    }

    fn merge_trending_entries(
        &self,
        tmdb_entries: Vec<TrendingEntry>,
        trakt_entries: Vec<TrendingEntry>,
    ) -> Vec<TrendingEntry> {
        let mut merged_map: HashMap<i32, TrendingEntry> = HashMap::new();
        
        // Process TMDB entries
        for (index, entry) in tmdb_entries.into_iter().enumerate() {
            let mut aggregated = entry.clone();
            aggregated.source = TrendingSource::Aggregated;
            
            // Calculate TMDB score (0-50 based on rank)
            let tmdb_score = 50.0 - (index as f32 * 50.0 / 20.0).min(50.0);
            aggregated.score = Some(tmdb_score);
            
            merged_map.insert(entry.tmdb_id, aggregated);
        }
        
        // Process Trakt entries
        for (index, entry) in trakt_entries.into_iter().enumerate() {
            let trakt_score = 50.0 - (index as f32 * 50.0 / 20.0).min(50.0);
            
            merged_map
                .entry(entry.tmdb_id)
                .and_modify(|e| {
                    // Combine scores if entry exists in both
                    if let Some(existing_score) = e.score {
                        e.score = Some(existing_score + trakt_score);
                    }
                })
                .or_insert_with(|| {
                    let mut aggregated = entry.clone();
                    aggregated.source = TrendingSource::Aggregated;
                    aggregated.score = Some(trakt_score);
                    aggregated
                });
        }
        
        merged_map.into_values().collect()
    }

    async fn enrich_with_availability(
        &self,
        entries: &mut Vec<TrendingEntry>,
        region: &str,
    ) -> Result<(), RadarrError> {
        if self.watchmode.is_none() {
            debug!("Watchmode not configured, skipping availability enrichment");
            return Ok(());
        }
        
        let watchmode = self.watchmode.as_ref().unwrap();
        
        for entry in entries.iter_mut() {
            // Get availability data (this will be cached internally)
            match watchmode.sources_by_tmdb(entry.tmdb_id, entry.media_type.clone()).await {
                Ok(availability) => {
                    // Store availability in repository
                    self.availability_repo.store_availability(availability.items).await?;
                }
                Err(e) => {
                    debug!("Failed to get availability for TMDB {}: {}", entry.tmdb_id, e);
                }
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl StreamingAggregator for TrendingAggregator {
    async fn get_trending(
        &self,
        media_type: MediaType,
        window: TimeWindow,
    ) -> Result<TrendingResponse, RadarrError> {
        info!("Fetching trending {} for {}", media_type, window);
        
        let mut entries = self.fetch_and_merge_trending(media_type.clone(), window.clone()).await?;
        
        // Enrich with availability if configured
        self.enrich_with_availability(&mut entries, &self.config.default_region).await?;
        
        let now = Utc::now();
        Ok(TrendingResponse {
            media_type,
            time_window: window,
            source: TrendingSource::Aggregated,
            total_results: entries.len(),
            entries,
            fetched_at: now,
            expires_at: now + chrono::Duration::hours(1),
        })
    }
    
    async fn get_availability(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        region: &str,
    ) -> Result<AvailabilityResponse, RadarrError> {
        info!("Fetching availability for TMDB {} in {}", tmdb_id, region);
        
        let cache_key = format!("availability:{}:{}:{}", tmdb_id, media_type, region);
        
        // Check cache
        if let Some(cached_value) = self.cache.get_raw(&cache_key).await? {
            if let Ok(cache_entry) = serde_json::from_value::<CacheEntry<AvailabilityResponse>>(cached_value) {
                if !cache_entry.is_expired() {
                    return Ok(cache_entry.data);
                }
            }
        }
        
        // Get from TMDB watch providers
        let tmdb_availability = self.tmdb.watch_providers(tmdb_id, media_type.clone(), region).await?;
        
        // Get from Watchmode if available
        let mut all_items = tmdb_availability.items;
        if let Some(watchmode) = &self.watchmode {
            match watchmode.sources_by_tmdb(tmdb_id, media_type.clone()).await {
                Ok(wm_availability) => {
                    all_items.extend(wm_availability.items);
                }
                Err(e) => {
                    debug!("Watchmode availability fetch failed: {}", e);
                }
            }
        }
        
        // Group by service type
        let mut grouped: HashMap<String, Vec<AvailabilityItem>> = HashMap::new();
        for item in all_items {
            grouped
                .entry(item.service_type.as_str().to_string())
                .or_insert_with(Vec::new)
                .push(item);
        }
        
        // Deduplicate within each group
        for items in grouped.values_mut() {
            items.sort_by_key(|i| i.service_name.clone());
            items.dedup_by_key(|i| i.service_name.clone());
        }
        
        let now = Utc::now();
        let response = AvailabilityResponse {
            tmdb_id,
            media_type,
            title: None, // Will be populated by the API layer
            region: region.to_string(),
            availability: grouped,
            fetched_at: now,
            expires_at: now + chrono::Duration::hours(24),
        };
        
        // Cache the response
        let ttl = self.config.cache_ttl_hours.get("tmdb_providers").copied().unwrap_or(24);
        let cache_entry = CacheEntry::new(cache_key.clone(), response.clone(), ttl);
        self.cache.set_raw(&cache_key, serde_json::to_value(cache_entry)?, ttl).await?;
        
        Ok(response)
    }
    
    async fn get_coming_soon(
        &self,
        media_type: MediaType,
        region: &str,
    ) -> Result<ComingSoonResponse, RadarrError> {
        info!("Fetching coming soon {} for {}", media_type, region);
        
        let cache_key = format!("coming_soon:{}:{}", media_type, region);
        
        // Check cache
        if let Some(cached_value) = self.cache.get_raw(&cache_key).await? {
            if let Ok(cache_entry) = serde_json::from_value::<CacheEntry<ComingSoonResponse>>(cached_value) {
                if !cache_entry.is_expired() {
                    return Ok(cache_entry.data);
                }
            }
        }
        
        // Fetch from TMDB
        let mut entries = match media_type {
            MediaType::Movie => self.tmdb.upcoming_movies().await?,
            MediaType::Tv => self.tmdb.on_the_air().await?,
        };
        
        // Add Watchmode coming soon if available
        if let Some(watchmode) = &self.watchmode {
            match watchmode.streaming_releases(region).await {
                Ok(wm_entries) => {
                    // Filter by media type and merge
                    let filtered: Vec<ComingSoon> = wm_entries
                        .into_iter()
                        .filter(|e| e.media_type == media_type)
                        .collect();
                    entries.extend(filtered);
                }
                Err(e) => {
                    debug!("Watchmode coming soon fetch failed: {}", e);
                }
            }
        }
        
        // Deduplicate by TMDB ID
        let mut seen = HashSet::new();
        entries.retain(|e| seen.insert(e.tmdb_id));
        
        // Sort by release date
        entries.sort_by_key(|e| e.release_date);
        
        let now = Utc::now();
        let response = ComingSoonResponse {
            media_type,
            region: region.to_string(),
            total_results: entries.len(),
            entries,
            fetched_at: now,
            expires_at: now + chrono::Duration::hours(24),
        };
        
        // Cache the response
        let ttl = self.config.cache_ttl_hours.get("coming_soon").copied().unwrap_or(24);
        let cache_entry = CacheEntry::new(cache_key.clone(), response.clone(), ttl);
        self.cache.set_raw(&cache_key, serde_json::to_value(cache_entry)?, ttl).await?;
        
        Ok(response)
    }
    
    async fn refresh_cache(&self) -> Result<(), RadarrError> {
        info!("Refreshing streaming cache");
        
        // Clear expired entries
        let expired_cache = self.cache.clear_expired().await?;
        let expired_trending = self.trending_repo.clear_expired_trending().await?;
        let expired_availability = self.availability_repo.clear_expired_availability().await?;
        
        info!(
            "Cleared {} expired cache entries, {} trending, {} availability",
            expired_cache, expired_trending, expired_availability
        );
        
        // Refresh ID mappings if Watchmode is configured
        if let Some(watchmode) = &self.watchmode {
            match watchmode.refresh_id_mappings().await {
                Ok(mappings) => {
                    let count = self.cache.store_id_mappings(mappings).await?;
                    info!("Refreshed {} ID mappings", count);
                }
                Err(e) => {
                    warn!("Failed to refresh ID mappings: {}", e);
                }
            }
        }
        
        Ok(())
    }
}