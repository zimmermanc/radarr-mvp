use super::{Cache, CacheError};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, trace};

/// In-memory cache implementation with TTL support
pub struct MemoryCache {
    store: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    cleanup_interval: Duration,
}

struct CacheEntry {
    data: Vec<u8>,
    expires_at: Instant,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self::with_config(10000, Duration::from_secs(300))
    }
    
    pub fn with_config(max_size: usize, cleanup_interval: Duration) -> Self {
        let cache = Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            cleanup_interval,
        };
        
        // Start background cleanup task
        let store_clone = cache.store.clone();
        let cleanup_interval = cache.cleanup_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                Self::cleanup_expired(&store_clone).await;
            }
        });
        
        cache
    }
    
    async fn cleanup_expired(store: &Arc<RwLock<HashMap<String, CacheEntry>>>) {
        let mut store = store.write().await;
        let now = Instant::now();
        let before_size = store.len();
        
        store.retain(|_key, entry| entry.expires_at > now);
        
        let removed = before_size - store.len();
        if removed > 0 {
            debug!("Cleaned up {} expired cache entries", removed);
        }
    }
    
    async fn evict_if_needed(&self, store: &mut HashMap<String, CacheEntry>) {
        if store.len() >= self.max_size {
            // Simple eviction: remove oldest entries (LRU would be better)
            let to_remove = store.len() - (self.max_size * 9 / 10); // Remove 10%
            let mut keys_to_remove: Vec<String> = Vec::new();
            
            for (key, _) in store.iter().take(to_remove) {
                keys_to_remove.push(key.clone());
            }
            
            for key in keys_to_remove {
                store.remove(&key);
            }
            
            debug!("Evicted {} cache entries", to_remove);
        }
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let store = self.store.read().await;
        
        if let Some(entry) = store.get(key) {
            if entry.expires_at > Instant::now() {
                trace!("Cache hit for key: {}", key);
                match serde_json::from_slice(&entry.data) {
                    Ok(value) => return Some(value),
                    Err(e) => {
                        debug!("Failed to deserialize cache entry for key {}: {}", key, e);
                        return None;
                    }
                }
            } else {
                trace!("Cache entry expired for key: {}", key);
            }
        } else {
            trace!("Cache miss for key: {}", key);
        }
        
        None
    }
    
    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<(), CacheError> {
        let data = serde_json::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;
        
        let entry = CacheEntry {
            data,
            expires_at: Instant::now() + ttl,
        };
        
        let mut store = self.store.write().await;
        self.evict_if_needed(&mut store).await;
        store.insert(key.to_string(), entry);
        
        trace!("Cached value for key: {} with TTL: {:?}", key, ttl);
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut store = self.store.write().await;
        store.remove(key);
        trace!("Deleted cache entry for key: {}", key);
        Ok(())
    }
    
    async fn clear(&self) -> Result<(), CacheError> {
        let mut store = self.store.write().await;
        let count = store.len();
        store.clear();
        debug!("Cleared {} cache entries", count);
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> bool {
        let store = self.store.read().await;
        if let Some(entry) = store.get(key) {
            entry.expires_at > Instant::now()
        } else {
            false
        }
    }
    
    async fn ttl(&self, key: &str) -> Option<Duration> {
        let store = self.store.read().await;
        if let Some(entry) = store.get(key) {
            let now = Instant::now();
            if entry.expires_at > now {
                Some(entry.expires_at - now)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        id: i32,
        name: String,
    }
    
    #[tokio::test]
    async fn test_memory_cache_basic() {
        let cache = MemoryCache::new();
        
        let data = TestData {
            id: 1,
            name: "Test".to_string(),
        };
        
        // Set value
        cache.set("test_key", &data, Duration::from_secs(60))
            .await
            .unwrap();
        
        // Get value
        let retrieved: Option<TestData> = cache.get("test_key").await;
        assert_eq!(retrieved, Some(data.clone()));
        
        // Check exists
        assert!(cache.exists("test_key").await);
        assert!(!cache.exists("non_existent").await);
        
        // Delete value
        cache.delete("test_key").await.unwrap();
        let retrieved: Option<TestData> = cache.get("test_key").await;
        assert_eq!(retrieved, None);
    }
    
    #[tokio::test]
    async fn test_memory_cache_expiration() {
        let cache = MemoryCache::new();
        
        let data = TestData {
            id: 2,
            name: "Expires".to_string(),
        };
        
        // Set with short TTL
        cache.set("expires", &data, Duration::from_millis(100))
            .await
            .unwrap();
        
        // Should exist immediately
        assert!(cache.exists("expires").await);
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        assert!(!cache.exists("expires").await);
        let retrieved: Option<TestData> = cache.get("expires").await;
        assert_eq!(retrieved, None);
    }
}