//! Blocklist repository trait

use crate::error::Result;
use crate::blocklist::models::{BlocklistEntry, BlocklistQuery, FailureReason};
use async_trait::async_trait;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Repository trait for blocklist operations
#[async_trait]
pub trait BlocklistRepository: Send + Sync {
    /// Add a new entry to the blocklist
    async fn add_entry(&self, entry: &BlocklistEntry) -> Result<BlocklistEntry>;
    
    /// Check if a release is currently blocked
    async fn is_blocked(&self, release_id: &str, indexer: &str) -> Result<bool>;
    
    /// Get a specific blocklist entry by release and indexer
    async fn get_entry(&self, release_id: &str, indexer: &str) -> Result<Option<BlocklistEntry>>;
    
    /// Get a blocklist entry by its ID
    async fn get_entry_by_id(&self, id: Uuid) -> Result<Option<BlocklistEntry>>;
    
    /// Search blocklist entries with query parameters
    async fn search_entries(&self, query: &BlocklistQuery) -> Result<Vec<BlocklistEntry>>;
    
    /// Count entries matching a query
    async fn count_entries(&self, query: &BlocklistQuery) -> Result<i64>;
    
    /// Update an existing blocklist entry
    async fn update_entry(&self, entry: &BlocklistEntry) -> Result<BlocklistEntry>;
    
    /// Remove a specific entry from the blocklist (manual unblock)
    async fn remove_entry(&self, release_id: &str, indexer: &str) -> Result<bool>;
    
    /// Remove an entry by its ID
    async fn remove_entry_by_id(&self, id: Uuid) -> Result<bool>;
    
    /// Get all expired entries that can be retried
    async fn get_expired_entries(&self, limit: Option<i32>) -> Result<Vec<BlocklistEntry>>;
    
    /// Get entries that are approaching expiration (for proactive management)
    async fn get_expiring_entries(&self, within_hours: i32, limit: Option<i32>) -> Result<Vec<BlocklistEntry>>;
    
    /// Clean up expired entries (removes permanently blocked entries older than threshold)
    async fn cleanup_expired_entries(&self, older_than_days: i32) -> Result<i64>;
    
    /// Clean up entries for a specific indexer (when indexer is removed)
    async fn cleanup_indexer_entries(&self, indexer: &str) -> Result<i64>;
    
    /// Get statistics about blocklist entries
    async fn get_statistics(&self) -> Result<BlocklistStatistics>;
    
    /// Get failure reason distribution for monitoring
    async fn get_failure_reason_stats(&self) -> Result<Vec<FailureReasonStat>>;
    
    /// Get entries blocked by a specific movie (for movie deletion cleanup)
    async fn get_entries_for_movie(&self, movie_id: Uuid) -> Result<Vec<BlocklistEntry>>;
    
    /// Bulk remove entries for a movie (when movie is deleted)
    async fn remove_entries_for_movie(&self, movie_id: Uuid) -> Result<i64>;
    
    /// Get the most recent failure for a specific release (across all indexers)
    async fn get_recent_failure(&self, release_id: &str) -> Result<Option<BlocklistEntry>>;
    
    /// Check if an indexer has too many recent failures (circuit breaker support)
    async fn check_indexer_health(&self, indexer: &str, hours_back: i32, failure_threshold: i32) -> Result<bool>;
}

/// Statistics about blocklist entries
#[derive(Debug, Clone)]
pub struct BlocklistStatistics {
    /// Total number of active (non-expired) blocked entries
    pub active_entries: i64,
    /// Total number of expired entries awaiting cleanup
    pub expired_entries: i64,
    /// Total number of permanent blocks (will never retry)
    pub permanent_blocks: i64,
    /// Total number of entries added in the last 24 hours
    pub recent_additions: i64,
    /// Most common failure reason
    pub top_failure_reason: Option<(FailureReason, i64)>,
    /// Most problematic indexer (highest failure count)
    pub top_failing_indexer: Option<(String, i64)>,
}

/// Statistics about a specific failure reason
#[derive(Debug, Clone)]
pub struct FailureReasonStat {
    /// The failure reason
    pub reason: FailureReason,
    /// Number of active blocks with this reason
    pub active_count: i64,
    /// Number of expired blocks with this reason
    pub expired_count: i64,
    /// Average retry count for this reason
    pub average_retries: f64,
    /// Success rate after retry for this reason (if available)
    pub retry_success_rate: Option<f64>,
}

/// Indexer health check result
#[derive(Debug, Clone)]
pub struct IndexerHealthStatus {
    /// The indexer name
    pub indexer: String,
    /// Number of failures in the checked time window
    pub failure_count: i64,
    /// Whether the failure count exceeds the threshold
    pub is_healthy: bool,
    /// Most common failure reason for this indexer
    pub primary_failure_reason: Option<FailureReason>,
    /// Time window checked (in hours)
    pub time_window_hours: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocklist::models::FailureReason;

    #[tokio::test]
    async fn test_blocklist_query_defaults() {
        let query = BlocklistQuery::default();
        
        assert!(query.indexer.is_none());
        assert!(query.reason.is_none());
        assert!(query.movie_id.is_none());
        assert!(!query.expired_only);
        assert!(!query.active_only);
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, 0);
    }
    
    #[test]
    fn test_failure_reason_stat_creation() {
        let stat = FailureReasonStat {
            reason: FailureReason::ConnectionTimeout,
            active_count: 10,
            expired_count: 5,
            average_retries: 2.5,
            retry_success_rate: Some(0.8),
        };
        
        assert_eq!(stat.reason, FailureReason::ConnectionTimeout);
        assert_eq!(stat.active_count, 10);
        assert_eq!(stat.expired_count, 5);
        assert_eq!(stat.average_retries, 2.5);
        assert_eq!(stat.retry_success_rate, Some(0.8));
    }
}