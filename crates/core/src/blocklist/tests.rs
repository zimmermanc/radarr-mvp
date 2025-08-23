//! Comprehensive tests for the blocklist system

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::error::RadarrError;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use uuid::Uuid;
    
    /// Mock implementation of BlocklistRepository for testing
    pub struct MockBlocklistRepository {
        entries: Arc<RwLock<HashMap<(String, String), BlocklistEntry>>>,
        statistics: Arc<RwLock<BlocklistStatistics>>,
    }
    
    impl MockBlocklistRepository {
        pub fn new() -> Self {
            Self {
                entries: Arc::new(RwLock::new(HashMap::new())),
                statistics: Arc::new(RwLock::new(BlocklistStatistics {
                    active_entries: 0,
                    expired_entries: 0,
                    permanent_blocks: 0,
                    recent_additions: 0,
                    top_failure_reason: None,
                    top_failing_indexer: None,
                })),
            }
        }
        
        pub async fn get_all_entries(&self) -> Vec<BlocklistEntry> {
            let entries = self.entries.read().await;
            entries.values().cloned().collect()
        }
        
        pub async fn clear(&self) {
            let mut entries = self.entries.write().await;
            entries.clear();
        }
    }
    
    #[async_trait::async_trait]
    impl BlocklistRepository for MockBlocklistRepository {
        async fn add_entry(&self, entry: &BlocklistEntry) -> crate::Result<BlocklistEntry> {
            let mut entries = self.entries.write().await;
            let key = (entry.release_id.clone(), entry.indexer.clone());
            entries.insert(key, entry.clone());
            Ok(entry.clone())
        }
        
        async fn is_blocked(&self, release_id: &str, indexer: &str) -> crate::Result<bool> {
            let entries = self.entries.read().await;
            let key = (release_id.to_string(), indexer.to_string());
            
            if let Some(entry) = entries.get(&key) {
                Ok(!entry.is_expired())
            } else {
                Ok(false)
            }
        }
        
        async fn get_entry(&self, release_id: &str, indexer: &str) -> crate::Result<Option<BlocklistEntry>> {
            let entries = self.entries.read().await;
            let key = (release_id.to_string(), indexer.to_string());
            Ok(entries.get(&key).cloned())
        }
        
        async fn get_entry_by_id(&self, id: Uuid) -> crate::Result<Option<BlocklistEntry>> {
            let entries = self.entries.read().await;
            Ok(entries.values().find(|e| e.id == id).cloned())
        }
        
        async fn search_entries(&self, query: &BlocklistQuery) -> crate::Result<Vec<BlocklistEntry>> {
            let entries = self.entries.read().await;
            let mut results: Vec<BlocklistEntry> = entries.values().cloned().collect();
            
            // Apply filters
            if let Some(ref indexer) = query.indexer {
                results.retain(|e| e.indexer == *indexer);
            }
            
            if let Some(reason) = query.reason {
                results.retain(|e| e.reason == reason);
            }
            
            if let Some(movie_id) = query.movie_id {
                results.retain(|e| e.movie_id == Some(movie_id));
            }
            
            if query.expired_only {
                results.retain(|e| e.is_expired());
            }
            
            if query.active_only {
                results.retain(|e| !e.is_expired());
            }
            
            // Sort by created_at DESC
            results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            
            // Apply pagination
            let start = query.offset as usize;
            let end = if query.limit > 0 {
                start + query.limit as usize
            } else {
                results.len()
            };
            
            if start < results.len() {
                Ok(results[start..end.min(results.len())].to_vec())
            } else {
                Ok(vec![])
            }
        }
        
        async fn count_entries(&self, query: &BlocklistQuery) -> crate::Result<i64> {
            let results = self.search_entries(query).await?;
            Ok(results.len() as i64)
        }
        
        async fn update_entry(&self, entry: &BlocklistEntry) -> crate::Result<BlocklistEntry> {
            let mut entries = self.entries.write().await;
            let key = (entry.release_id.clone(), entry.indexer.clone());
            entries.insert(key, entry.clone());
            Ok(entry.clone())
        }
        
        async fn remove_entry(&self, release_id: &str, indexer: &str) -> crate::Result<bool> {
            let mut entries = self.entries.write().await;
            let key = (release_id.to_string(), indexer.to_string());
            Ok(entries.remove(&key).is_some())
        }
        
        async fn remove_entry_by_id(&self, id: Uuid) -> crate::Result<bool> {
            let mut entries = self.entries.write().await;
            let key_to_remove = entries.values()
                .find(|e| e.id == id)
                .map(|e| (e.release_id.clone(), e.indexer.clone()));
                
            if let Some(key) = key_to_remove {
                Ok(entries.remove(&key).is_some())
            } else {
                Ok(false)
            }
        }
        
        async fn get_expired_entries(&self, limit: Option<i32>) -> crate::Result<Vec<BlocklistEntry>> {
            let entries = self.entries.read().await;
            let mut expired: Vec<BlocklistEntry> = entries.values()
                .filter(|e| e.is_expired())
                .cloned()
                .collect();
            
            expired.sort_by(|a, b| a.blocked_until.cmp(&b.blocked_until));
            
            if let Some(limit) = limit {
                expired.truncate(limit as usize);
            }
            
            Ok(expired)
        }
        
        async fn get_expiring_entries(&self, within_hours: i32, limit: Option<i32>) -> crate::Result<Vec<BlocklistEntry>> {
            let entries = self.entries.read().await;
            let cutoff = Utc::now() + Duration::hours(within_hours as i64);
            
            let mut expiring: Vec<BlocklistEntry> = entries.values()
                .filter(|e| !e.is_expired() && e.blocked_until <= cutoff)
                .cloned()
                .collect();
            
            expiring.sort_by(|a, b| a.blocked_until.cmp(&b.blocked_until));
            
            if let Some(limit) = limit {
                expiring.truncate(limit as usize);
            }
            
            Ok(expiring)
        }
        
        async fn cleanup_expired_entries(&self, older_than_days: i32) -> crate::Result<i64> {
            let mut entries = self.entries.write().await;
            let cutoff = Utc::now() - Duration::days(older_than_days as i64);
            
            let keys_to_remove: Vec<(String, String)> = entries.values()
                .filter(|e| e.created_at < cutoff && e.is_expired() && e.is_permanent_block())
                .map(|e| (e.release_id.clone(), e.indexer.clone()))
                .collect();
            
            let removed_count = keys_to_remove.len() as i64;
            for key in keys_to_remove {
                entries.remove(&key);
            }
            
            Ok(removed_count)
        }
        
        async fn cleanup_indexer_entries(&self, indexer: &str) -> crate::Result<i64> {
            let mut entries = self.entries.write().await;
            let keys_to_remove: Vec<(String, String)> = entries.values()
                .filter(|e| e.indexer == indexer)
                .map(|e| (e.release_id.clone(), e.indexer.clone()))
                .collect();
            
            let removed_count = keys_to_remove.len() as i64;
            for key in keys_to_remove {
                entries.remove(&key);
            }
            
            Ok(removed_count)
        }
        
        async fn get_statistics(&self) -> crate::Result<BlocklistStatistics> {
            let stats = self.statistics.read().await;
            Ok(stats.clone())
        }
        
        async fn get_failure_reason_stats(&self) -> crate::Result<Vec<FailureReasonStat>> {
            let entries = self.entries.read().await;
            let mut reason_counts: HashMap<FailureReason, (i64, i64, Vec<u32>)> = HashMap::new();
            
            for entry in entries.values() {
                let (active, expired, retries) = reason_counts
                    .entry(entry.reason)
                    .or_insert((0, 0, vec![]));
                    
                if entry.is_expired() {
                    *expired += 1;
                } else {
                    *active += 1;
                }
                
                retries.push(entry.retry_count);
            }
            
            let mut stats = Vec::new();
            for (reason, (active_count, expired_count, retries)) in reason_counts {
                let average_retries = if !retries.is_empty() {
                    retries.iter().sum::<u32>() as f64 / retries.len() as f64
                } else {
                    0.0
                };
                
                stats.push(FailureReasonStat {
                    reason,
                    active_count,
                    expired_count,
                    average_retries,
                    retry_success_rate: None, // Not implemented in mock
                });
            }
            
            Ok(stats)
        }
        
        async fn get_entries_for_movie(&self, movie_id: Uuid) -> crate::Result<Vec<BlocklistEntry>> {
            let entries = self.entries.read().await;
            Ok(entries.values()
                .filter(|e| e.movie_id == Some(movie_id))
                .cloned()
                .collect())
        }
        
        async fn remove_entries_for_movie(&self, movie_id: Uuid) -> crate::Result<i64> {
            let mut entries = self.entries.write().await;
            let keys_to_remove: Vec<(String, String)> = entries.values()
                .filter(|e| e.movie_id == Some(movie_id))
                .map(|e| (e.release_id.clone(), e.indexer.clone()))
                .collect();
            
            let removed_count = keys_to_remove.len() as i64;
            for key in keys_to_remove {
                entries.remove(&key);
            }
            
            Ok(removed_count)
        }
        
        async fn get_recent_failure(&self, release_id: &str) -> crate::Result<Option<BlocklistEntry>> {
            let entries = self.entries.read().await;
            let mut matches: Vec<&BlocklistEntry> = entries.values()
                .filter(|e| e.release_id == release_id)
                .collect();
            
            matches.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(matches.first().cloned().cloned())
        }
        
        async fn check_indexer_health(&self, indexer: &str, hours_back: i32, failure_threshold: i32) -> crate::Result<bool> {
            let entries = self.entries.read().await;
            let cutoff = Utc::now() - Duration::hours(hours_back as i64);
            
            let failure_count = entries.values()
                .filter(|e| e.indexer == indexer && e.created_at >= cutoff)
                .count() as i32;
            
            Ok(failure_count < failure_threshold)
        }
    }
    
    fn create_test_service() -> BlocklistService<MockBlocklistRepository> {
        let repo = Arc::new(MockBlocklistRepository::new());
        BlocklistService::new(repo)
    }
    
    #[tokio::test]
    async fn test_block_release_new_entry() {
        let service = create_test_service();
        
        let result = service.block_release(
            "test-release-123",
            "test-indexer",
            FailureReason::ConnectionTimeout,
            "Test Release",
            None,
            None,
        ).await;
        
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.release_id, "test-release-123");
        assert_eq!(entry.indexer, "test-indexer");
        assert_eq!(entry.reason, FailureReason::ConnectionTimeout);
        assert_eq!(entry.retry_count, 0);
        assert!(entry.blocked_until > Utc::now());
    }
    
    #[tokio::test]
    async fn test_block_release_with_metadata() {
        let service = create_test_service();
        let movie_id = Uuid::new_v4();
        let metadata = serde_json::json!({
            "original_error": "Connection timeout",
            "attempt_number": 1
        });
        
        let result = service.block_release(
            "test-release-456",
            "test-indexer",
            FailureReason::ConnectionTimeout,
            "Test Release",
            Some(movie_id),
            Some(metadata.clone()),
        ).await;
        
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.movie_id, Some(movie_id));
        assert_eq!(entry.metadata, Some(metadata));
    }
    
    #[tokio::test]
    async fn test_is_release_blocked() {
        let service = create_test_service();
        
        // Initially should not be blocked
        let blocked = service.is_release_blocked("test-release", "test-indexer").await.unwrap();
        assert!(!blocked);
        
        // Block the release
        service.block_release(
            "test-release",
            "test-indexer",
            FailureReason::ConnectionTimeout,
            "Test Release",
            None,
            None,
        ).await.unwrap();
        
        // Should now be blocked
        let blocked = service.is_release_blocked("test-release", "test-indexer").await.unwrap();
        assert!(blocked);
    }
    
    #[tokio::test]
    async fn test_unblock_release() {
        let service = create_test_service();
        
        // Block a release
        service.block_release(
            "test-release",
            "test-indexer",
            FailureReason::ConnectionTimeout,
            "Test Release",
            None,
            None,
        ).await.unwrap();
        
        // Verify it's blocked
        let blocked = service.is_release_blocked("test-release", "test-indexer").await.unwrap();
        assert!(blocked);
        
        // Unblock it
        let unblocked = service.unblock_release("test-release", "test-indexer", Some("Manual unblock")).await.unwrap();
        assert!(unblocked);
        
        // Should no longer be blocked
        let blocked = service.is_release_blocked("test-release", "test-indexer").await.unwrap();
        assert!(!blocked);
    }
    
    #[tokio::test]
    async fn test_retry_logic() {
        let service = create_test_service();
        
        // Block with a retryable failure
        service.block_release(
            "test-release",
            "test-indexer",
            FailureReason::ConnectionTimeout,
            "Test Release",
            None,
            None,
        ).await.unwrap();
        
        // Get the entry and verify initial state
        let entry = service.get_blocked_release("test-release", "test-indexer").await.unwrap().unwrap();
        assert_eq!(entry.retry_count, 0);
        assert!(entry.can_retry());
        
        // Try to block again (should increment retry count)
        service.block_release(
            "test-release",
            "test-indexer",
            FailureReason::ConnectionTimeout,
            "Test Release",
            None,
            None,
        ).await.unwrap();
        
        // Verify retry count increased
        let entry = service.get_blocked_release("test-release", "test-indexer").await.unwrap().unwrap();
        assert_eq!(entry.retry_count, 1);
    }
    
    #[tokio::test]
    async fn test_permanent_failure_no_retry() {
        let service = create_test_service();
        
        // Block with permanent failure
        service.block_release(
            "test-release",
            "test-indexer",
            FailureReason::ManuallyRejected,
            "Test Release",
            None,
            None,
        ).await.unwrap();
        
        let entry = service.get_blocked_release("test-release", "test-indexer").await.unwrap().unwrap();
        assert!(!entry.can_retry());
        assert!(entry.is_permanent_block());
        
        // Try to block again (should not increment retry count)
        service.block_release(
            "test-release",
            "test-indexer",
            FailureReason::ManuallyRejected,
            "Test Release",
            None,
            None,
        ).await.unwrap();
        
        let entry = service.get_blocked_release("test-release", "test-indexer").await.unwrap().unwrap();
        assert_eq!(entry.retry_count, 0); // Should still be 0
    }
    
    #[tokio::test]
    async fn test_search_blocked_releases() {
        let service = create_test_service();
        
        // Add multiple entries
        service.block_release("release-1", "indexer-1", FailureReason::ConnectionTimeout, "Release 1", None, None).await.unwrap();
        service.block_release("release-2", "indexer-1", FailureReason::AuthenticationFailed, "Release 2", None, None).await.unwrap();
        service.block_release("release-3", "indexer-2", FailureReason::ConnectionTimeout, "Release 3", None, None).await.unwrap();
        
        // Search by indexer
        let query = BlocklistQuery::default().with_indexer("indexer-1").paginate(0, 10);
        let results = service.search_blocked_releases(&query).await.unwrap();
        assert_eq!(results.len(), 2);
        
        // Search by failure reason
        let query = BlocklistQuery::default().with_reason(FailureReason::ConnectionTimeout).paginate(0, 10);
        let results = service.search_blocked_releases(&query).await.unwrap();
        assert_eq!(results.len(), 2);
        
        // Search with pagination
        let query = BlocklistQuery::default().paginate(0, 1);
        let results = service.search_blocked_releases(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }
    
    #[tokio::test]
    async fn test_cleanup_movie_entries() {
        let service = create_test_service();
        let movie_id = Uuid::new_v4();
        
        // Add entries for the movie
        service.block_release("release-1", "indexer-1", FailureReason::ConnectionTimeout, "Release 1", Some(movie_id), None).await.unwrap();
        service.block_release("release-2", "indexer-2", FailureReason::ConnectionTimeout, "Release 2", Some(movie_id), None).await.unwrap();
        service.block_release("release-3", "indexer-3", FailureReason::ConnectionTimeout, "Release 3", None, None).await.unwrap();
        
        // Verify entries exist
        let query = BlocklistQuery::default().paginate(0, 10);
        let results = service.search_blocked_releases(&query).await.unwrap();
        assert_eq!(results.len(), 3);
        
        // Cleanup movie entries
        let removed = service.cleanup_movie(movie_id).await.unwrap();
        assert_eq!(removed, 2);
        
        // Verify only the non-movie entry remains
        let results = service.search_blocked_releases(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].release_id, "release-3");
    }
    
    #[tokio::test]
    async fn test_cleanup_indexer_entries() {
        let service = create_test_service();
        
        // Add entries for different indexers
        service.block_release("release-1", "indexer-1", FailureReason::ConnectionTimeout, "Release 1", None, None).await.unwrap();
        service.block_release("release-2", "indexer-1", FailureReason::ConnectionTimeout, "Release 2", None, None).await.unwrap();
        service.block_release("release-3", "indexer-2", FailureReason::ConnectionTimeout, "Release 3", None, None).await.unwrap();
        
        // Cleanup indexer-1 entries
        let removed = service.cleanup_indexer("indexer-1").await.unwrap();
        assert_eq!(removed, 2);
        
        // Verify only indexer-2 entry remains
        let query = BlocklistQuery::default().paginate(0, 10);
        let results = service.search_blocked_releases(&query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].indexer, "indexer-2");
    }
    
    #[tokio::test]
    async fn test_error_classification() {
        let service = create_test_service();
        
        // Test timeout error
        let timeout_error = RadarrError::Timeout { operation: "test".to_string() };
        assert_eq!(service.classify_error(&timeout_error), FailureReason::ConnectionTimeout);
        
        // Test authentication error
        let auth_error = RadarrError::AuthenticationRequired { 
            service: "test".to_string(), 
            message: "invalid".to_string() 
        };
        assert_eq!(service.classify_error(&auth_error), FailureReason::AuthenticationFailed);
        
        // Test rate limit error
        let rate_error = RadarrError::RateLimited { 
            service: "test".to_string(), 
            retry_after: None 
        };
        assert_eq!(service.classify_error(&rate_error), FailureReason::RateLimited);
        
        // Test external service error with timeout in message
        let external_error = RadarrError::ExternalServiceError {
            service: "test".to_string(),
            error: "connection timeout occurred".to_string(),
        };
        assert_eq!(service.classify_error(&external_error), FailureReason::ConnectionTimeout);
        
        // Test IO error with permission denied
        let io_error = RadarrError::IoError("permission denied".to_string());
        assert_eq!(service.classify_error(&io_error), FailureReason::PermissionDenied);
    }
    
    #[tokio::test]
    async fn test_indexer_health_check() {
        let service = create_test_service();
        
        // Initially should be healthy
        let health = service.check_indexer_health("test-indexer", Some(24), Some(5)).await.unwrap();
        assert!(health.is_healthy);
        assert_eq!(health.failure_count, 0);
        
        // Add some failures
        service.block_release("release-1", "test-indexer", FailureReason::ConnectionTimeout, "Release 1", None, None).await.unwrap();
        service.block_release("release-2", "test-indexer", FailureReason::AuthenticationFailed, "Release 2", None, None).await.unwrap();
        
        // Should still be healthy (below threshold)
        let health = service.check_indexer_health("test-indexer", Some(24), Some(5)).await.unwrap();
        assert!(health.is_healthy);
        assert_eq!(health.failure_count, 2);
        
        // Add more failures to exceed threshold
        service.block_release("release-3", "test-indexer", FailureReason::NetworkError, "Release 3", None, None).await.unwrap();
        service.block_release("release-4", "test-indexer", FailureReason::ServerError, "Release 4", None, None).await.unwrap();
        service.block_release("release-5", "test-indexer", FailureReason::ParseError, "Release 5", None, None).await.unwrap();
        service.block_release("release-6", "test-indexer", FailureReason::DownloadStalled, "Release 6", None, None).await.unwrap();
        
        // Should now be unhealthy (exceeds threshold)
        let health = service.check_indexer_health("test-indexer", Some(24), Some(5)).await.unwrap();
        assert!(!health.is_healthy);
        assert_eq!(health.failure_count, 6);
    }
    
    #[tokio::test]
    async fn test_blocklist_integration_trait() {
        let service = create_test_service();
        
        // Test report_failure
        let error = RadarrError::ConnectionTimeout { operation: "test".to_string() };
        service.report_failure("test-release", "test-indexer", &error, "Test Release", None).await.unwrap();
        
        // Should be blocked now
        let should_skip = service.should_skip_release("test-release", "test-indexer").await.unwrap();
        assert!(should_skip);
        
        // Test failure rate
        let failure_rate = service.get_indexer_failure_rate("test-indexer").await.unwrap();
        assert!(failure_rate > 0.0);
    }
}

// Integration with the main test module structure
pub use tests::MockBlocklistRepository;