use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub mod memory;
pub mod redis;

pub use memory::MemoryCache;
#[cfg(feature = "redis")]
pub use redis::RedisCache;

#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a value from the cache
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    
    /// Set a value in the cache with a TTL
    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<(), CacheError>;
    
    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> Result<(), CacheError>;
    
    /// Clear all values from the cache
    async fn clear(&self) -> Result<(), CacheError>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> bool;
    
    /// Get the remaining TTL for a key
    async fn ttl(&self, key: &str) -> Option<Duration>;
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    #[error("Cache backend error: {0}")]
    Backend(String),
    
    #[error("Key not found")]
    KeyNotFound,
}

/// Cache manager that handles multiple cache layers
pub struct CacheManager {
    layers: Vec<Arc<dyn Cache>>,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            layers: vec![Arc::new(MemoryCache::new())],
        }
    }
    
    pub fn with_layer(mut self, cache: Arc<dyn Cache>) -> Self {
        self.layers.push(cache);
        self
    }
    
    /// Get from the first cache layer that has the value
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        for layer in &self.layers {
            if let Some(value) = layer.get(key).await {
                // Write-through to earlier layers
                for earlier_layer in &self.layers {
                    if Arc::ptr_eq(earlier_layer, layer) {
                        break;
                    }
                    let _ = earlier_layer.set(&key, &value, Duration::from_secs(3600)).await;
                }
                return Some(value);
            }
        }
        None
    }
    
    /// Set in all cache layers
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<(), CacheError> {
        for layer in &self.layers {
            layer.set(key, value, ttl).await?;
        }
        Ok(())
    }
    
    /// Delete from all cache layers
    pub async fn delete(&self, key: &str) -> Result<(), CacheError> {
        for layer in &self.layers {
            layer.delete(key).await?;
        }
        Ok(())
    }
}

/// Cache key builder for consistent key generation
pub struct CacheKey;

impl CacheKey {
    pub fn tmdb_movie(id: i32) -> String {
        format!("tmdb:movie:{}", id)
    }
    
    pub fn tmdb_search(query: &str, page: i32) -> String {
        format!("tmdb:search:{}:{}", query.to_lowercase().replace(' ', "_"), page)
    }
    
    pub fn tmdb_popular(page: i32) -> String {
        format!("tmdb:popular:{}", page)
    }
    
    pub fn tmdb_upcoming(page: i32) -> String {
        format!("tmdb:upcoming:{}", page)
    }
    
    pub fn hdbits_scene_group(name: &str) -> String {
        format!("hdbits:scene_group:{}", name.to_uppercase())
    }
    
    pub fn quality_score(release_title: &str) -> String {
        format!("quality:score:{}", release_title)
    }
    
    pub fn indexer_search(indexer: &str, query: &str) -> String {
        format!("indexer:{}:search:{}", indexer, query.to_lowercase().replace(' ', "_"))
    }
}