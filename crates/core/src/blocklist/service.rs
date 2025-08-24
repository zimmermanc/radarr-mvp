//! Blocklist service implementation

use crate::blocklist::models::{BlocklistEntry, BlocklistQuery, FailureReason, ImportFailureType};
use crate::blocklist::repository::{
    BlocklistRepository, BlocklistStatistics, FailureReasonStat, IndexerHealthStatus,
};
use crate::error::{RadarrError, Result};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;
// use chrono::DateTime; // Currently unused
use tracing::{debug, info, warn};
// use tracing::error; // Currently unused
use std::collections::HashMap;

/// Service for managing blocked releases and failure handling
pub struct BlocklistService<R: BlocklistRepository> {
    repository: Arc<R>,
}

impl<R: BlocklistRepository> BlocklistService<R> {
    /// Create a new blocklist service
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Block a release due to failure
    pub async fn block_release(
        &self,
        release_id: impl Into<String>,
        indexer: impl Into<String>,
        reason: FailureReason,
        release_title: impl Into<String>,
        movie_id: Option<Uuid>,
        metadata: Option<serde_json::Value>,
    ) -> Result<BlocklistEntry> {
        let release_id = release_id.into();
        let indexer = indexer.into();
        let release_title = release_title.into();

        // Check if already blocked
        if let Some(mut existing) = self.repository.get_entry(&release_id, &indexer).await? {
            // Update existing entry for retry
            if existing.retry() {
                info!(
                    release_id = %release_id,
                    indexer = %indexer,
                    reason = %reason,
                    retry_count = existing.retry_count,
                    blocked_until = %existing.blocked_until,
                    "Updated existing blocklist entry for retry"
                );
                return self.repository.update_entry(&existing).await;
            } else {
                warn!(
                    release_id = %release_id,
                    indexer = %indexer,
                    reason = %reason,
                    retry_count = existing.retry_count,
                    "Release has exceeded maximum retry attempts or is permanently blocked"
                );
                return Ok(existing);
            }
        }

        // Create new blocklist entry
        let mut entry = match movie_id {
            Some(movie_id) => BlocklistEntry::new_for_movie(
                release_id.clone(),
                indexer.clone(),
                reason,
                release_title,
                movie_id,
            ),
            None => BlocklistEntry::new(release_id.clone(), indexer.clone(), reason, release_title),
        };

        if let Some(metadata) = metadata {
            entry = entry.with_metadata(metadata);
        }

        info!(
            release_id = %release_id,
            indexer = %indexer,
            reason = %reason,
            blocked_until = %entry.blocked_until,
            is_permanent = entry.is_permanent_block(),
            "Added new release to blocklist"
        );

        self.repository.add_entry(&entry).await
    }

    /// Check if a release is currently blocked
    pub async fn is_release_blocked(&self, release_id: &str, indexer: &str) -> Result<bool> {
        self.repository.is_blocked(release_id, indexer).await
    }

    /// Get detailed information about a blocked release
    pub async fn get_blocked_release(
        &self,
        release_id: &str,
        indexer: &str,
    ) -> Result<Option<BlocklistEntry>> {
        self.repository.get_entry(release_id, indexer).await
    }

    /// Manually unblock a release (administrative override)
    pub async fn unblock_release(
        &self,
        release_id: &str,
        indexer: &str,
        reason: Option<&str>,
    ) -> Result<bool> {
        let removed = self.repository.remove_entry(release_id, indexer).await?;

        if removed {
            info!(
                release_id = %release_id,
                indexer = %indexer,
                reason = reason.unwrap_or("Manual unblock"),
                "Release manually unblocked"
            );
        } else {
            warn!(
                release_id = %release_id,
                indexer = %indexer,
                "Attempted to unblock release that was not blocked"
            );
        }

        Ok(removed)
    }

    /// Unblock a release by entry ID
    pub async fn unblock_release_by_id(&self, entry_id: Uuid) -> Result<bool> {
        if let Some(entry) = self.repository.get_entry_by_id(entry_id).await? {
            let removed = self.repository.remove_entry_by_id(entry_id).await?;

            if removed {
                info!(
                    entry_id = %entry_id,
                    release_id = %entry.release_id,
                    indexer = %entry.indexer,
                    "Release unblocked by entry ID"
                );
            }

            Ok(removed)
        } else {
            Ok(false)
        }
    }

    /// Get all blocked releases matching query parameters
    pub async fn search_blocked_releases(
        &self,
        query: &BlocklistQuery,
    ) -> Result<Vec<BlocklistEntry>> {
        self.repository.search_entries(query).await
    }

    /// Count blocked releases matching query parameters
    pub async fn count_blocked_releases(&self, query: &BlocklistQuery) -> Result<i64> {
        self.repository.count_entries(query).await
    }

    /// Get expired entries that can potentially be retried
    pub async fn get_retryable_releases(&self, limit: Option<i32>) -> Result<Vec<BlocklistEntry>> {
        let expired = self.repository.get_expired_entries(limit).await?;

        // Filter for entries that can actually be retried
        let retryable: Vec<BlocklistEntry> = expired
            .into_iter()
            .filter(|entry| entry.can_retry())
            .collect();

        debug!(
            count = retryable.len(),
            "Found retryable expired blocklist entries"
        );

        Ok(retryable)
    }

    /// Process expired entries and attempt retries
    pub async fn process_expired_entries(&self, limit: Option<i32>) -> Result<ProcessingResult> {
        let expired = self.repository.get_expired_entries(limit).await?;
        let mut result = ProcessingResult::default();

        for mut entry in expired {
            if entry.can_retry() {
                if entry.retry() {
                    match self.repository.update_entry(&entry).await {
                        Ok(_) => {
                            result.retries_scheduled += 1;
                            debug!(
                                release_id = %entry.release_id,
                                indexer = %entry.indexer,
                                retry_count = entry.retry_count,
                                "Scheduled retry for expired blocklist entry"
                            );
                        }
                        Err(e) => {
                            result
                                .errors
                                .push(format!("Failed to update entry {}: {}", entry.id, e));
                        }
                    }
                } else {
                    result.max_retries_reached += 1;
                    debug!(
                        release_id = %entry.release_id,
                        indexer = %entry.indexer,
                        retry_count = entry.retry_count,
                        "Entry reached maximum retry attempts"
                    );
                }
            } else {
                result.permanently_blocked += 1;
            }
        }

        info!(
            retries_scheduled = result.retries_scheduled,
            max_retries_reached = result.max_retries_reached,
            permanently_blocked = result.permanently_blocked,
            errors = result.errors.len(),
            "Processed expired blocklist entries"
        );

        Ok(result)
    }

    /// Clean up old entries to prevent database bloat
    pub async fn cleanup_old_entries(&self, older_than_days: i32) -> Result<i64> {
        let removed = self
            .repository
            .cleanup_expired_entries(older_than_days)
            .await?;

        if removed > 0 {
            info!(
                removed_count = removed,
                older_than_days = older_than_days,
                "Cleaned up old blocklist entries"
            );
        }

        Ok(removed)
    }

    /// Clean up all entries for a specific indexer (when indexer is removed)
    pub async fn cleanup_indexer(&self, indexer: &str) -> Result<i64> {
        let removed = self.repository.cleanup_indexer_entries(indexer).await?;

        if removed > 0 {
            info!(
                indexer = %indexer,
                removed_count = removed,
                "Cleaned up blocklist entries for removed indexer"
            );
        }

        Ok(removed)
    }

    /// Clean up entries for a deleted movie
    pub async fn cleanup_movie(&self, movie_id: Uuid) -> Result<i64> {
        let removed = self.repository.remove_entries_for_movie(movie_id).await?;

        if removed > 0 {
            debug!(
                movie_id = %movie_id,
                removed_count = removed,
                "Cleaned up blocklist entries for deleted movie"
            );
        }

        Ok(removed)
    }

    /// Get comprehensive statistics about blocklist usage
    pub async fn get_statistics(&self) -> Result<BlocklistStatistics> {
        self.repository.get_statistics().await
    }

    /// Get failure reason distribution for monitoring and analysis
    pub async fn get_failure_analysis(&self) -> Result<Vec<FailureReasonStat>> {
        self.repository.get_failure_reason_stats().await
    }

    /// Check health of an indexer based on recent failures
    pub async fn check_indexer_health(
        &self,
        indexer: &str,
        hours_back: Option<i32>,
        failure_threshold: Option<i32>,
    ) -> Result<IndexerHealthStatus> {
        let hours_back = hours_back.unwrap_or(24);
        let failure_threshold = failure_threshold.unwrap_or(10);

        let is_healthy = self
            .repository
            .check_indexer_health(indexer, hours_back, failure_threshold)
            .await?;

        // Get recent failures for this indexer to find primary failure reason
        let query = BlocklistQuery::default()
            .with_indexer(indexer)
            .paginate(0, 100);

        let recent_entries = self.repository.search_entries(&query).await?;

        // Count failures by reason
        let mut failure_counts: HashMap<FailureReason, i64> = HashMap::new();
        let mut total_failures = 0i64;

        let cutoff_time = Utc::now() - Duration::hours(hours_back as i64);

        for entry in recent_entries {
            if entry.created_at >= cutoff_time {
                *failure_counts.entry(entry.reason).or_insert(0) += 1;
                total_failures += 1;
            }
        }

        let primary_failure_reason = failure_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(reason, _)| reason);

        Ok(IndexerHealthStatus {
            indexer: indexer.to_string(),
            failure_count: total_failures,
            is_healthy,
            primary_failure_reason,
            time_window_hours: hours_back,
        })
    }

    /// Convert various error types to appropriate failure reasons
    pub fn classify_error(&self, error: &RadarrError) -> FailureReason {
        match error {
            RadarrError::Timeout { .. } => FailureReason::ConnectionTimeout,
            RadarrError::NetworkError { .. } => FailureReason::NetworkError,
            RadarrError::AuthenticationRequired { .. } => FailureReason::AuthenticationFailed,
            RadarrError::RateLimited { .. } => FailureReason::RateLimited,
            RadarrError::SerializationError(_) => FailureReason::ParseError,
            RadarrError::ExternalServiceError { service: _, error } => {
                // Try to classify based on service and error message
                let error_lower = error.to_lowercase();

                if error_lower.contains("timeout") || error_lower.contains("timed out") {
                    FailureReason::ConnectionTimeout
                } else if error_lower.contains("unauthorized") || error_lower.contains("auth") {
                    FailureReason::AuthenticationFailed
                } else if error_lower.contains("rate") || error_lower.contains("429") {
                    FailureReason::RateLimited
                } else if error_lower.contains("parse") || error_lower.contains("json") {
                    FailureReason::ParseError
                } else if error_lower.contains("space") || error_lower.contains("disk") {
                    FailureReason::DiskFull
                } else if error_lower.contains("permission") || error_lower.contains("access") {
                    FailureReason::PermissionDenied
                } else if error_lower.contains("5")
                    && (error_lower.contains("00")
                        || error_lower.contains("02")
                        || error_lower.contains("03"))
                {
                    FailureReason::ServerError
                } else {
                    // Default to server error for unknown external service errors
                    FailureReason::ServerError
                }
            }
            RadarrError::IoError(msg) => {
                let msg_lower = msg.to_lowercase();
                if msg_lower.contains("permission") {
                    FailureReason::PermissionDenied
                } else if msg_lower.contains("space") || msg_lower.contains("disk") {
                    FailureReason::DiskFull
                } else {
                    FailureReason::ImportFailed(ImportFailureType::FileMoveError)
                }
            }
            RadarrError::DatabaseError { .. } => FailureReason::ServerError,
            RadarrError::CircuitBreakerOpen { .. } => FailureReason::ServerError,
            _ => FailureReason::ServerError, // Default fallback
        }
    }

    /// Get entries that are approaching expiration (for proactive management)
    pub async fn get_expiring_entries(
        &self,
        within_hours: i32,
        limit: Option<i32>,
    ) -> Result<Vec<BlocklistEntry>> {
        self.repository
            .get_expiring_entries(within_hours, limit)
            .await
    }
}

/// Result of processing expired entries
#[derive(Debug, Default)]
pub struct ProcessingResult {
    /// Number of entries that had their retry scheduled
    pub retries_scheduled: i64,
    /// Number of entries that reached maximum retry attempts
    pub max_retries_reached: i64,
    /// Number of entries that are permanently blocked
    pub permanently_blocked: i64,
    /// Any errors encountered during processing
    pub errors: Vec<String>,
}

impl ProcessingResult {
    /// Total number of entries processed
    pub fn total_processed(&self) -> i64 {
        self.retries_scheduled + self.max_retries_reached + self.permanently_blocked
    }

    /// Check if processing was completely successful
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Trait for integrating with circuit breakers and health checks
#[async_trait]
pub trait BlocklistIntegration {
    /// Report a failure for potential blocking
    async fn report_failure(
        &self,
        release_id: &str,
        indexer: &str,
        error: &RadarrError,
        release_title: &str,
        movie_id: Option<Uuid>,
    ) -> Result<()>;

    /// Check if a release should be skipped due to blocking
    async fn should_skip_release(&self, release_id: &str, indexer: &str) -> Result<bool>;

    /// Get indexer health for circuit breaker decisions
    async fn get_indexer_failure_rate(&self, indexer: &str) -> Result<f64>;
}

#[async_trait]
impl<R: BlocklistRepository> BlocklistIntegration for BlocklistService<R> {
    async fn report_failure(
        &self,
        release_id: &str,
        indexer: &str,
        error: &RadarrError,
        release_title: &str,
        movie_id: Option<Uuid>,
    ) -> Result<()> {
        let reason = self.classify_error(error);

        // Create error metadata
        let metadata = serde_json::json!({
            "original_error": error.to_string(),
            "error_type": format!("{:?}", error),
            "timestamp": Utc::now().to_rfc3339(),
        });

        self.block_release(
            release_id,
            indexer,
            reason,
            release_title,
            movie_id,
            Some(metadata),
        )
        .await?;

        Ok(())
    }

    async fn should_skip_release(&self, release_id: &str, indexer: &str) -> Result<bool> {
        self.is_release_blocked(release_id, indexer).await
    }

    async fn get_indexer_failure_rate(&self, indexer: &str) -> Result<f64> {
        let health = self
            .check_indexer_health(indexer, Some(24), Some(10))
            .await?;

        // Calculate failure rate as failures per hour
        if health.time_window_hours > 0 {
            Ok(health.failure_count as f64 / health.time_window_hours as f64)
        } else {
            Ok(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocklist::models::FailureReason;
    use crate::error::RadarrError;

    #[test]
    fn test_error_classification() {
        let service = BlocklistService::new(Arc::new(MockRepository::new()));

        let timeout_error = RadarrError::Timeout {
            operation: "test".to_string(),
        };
        assert_eq!(
            service.classify_error(&timeout_error),
            FailureReason::ConnectionTimeout
        );

        let auth_error = RadarrError::AuthenticationRequired {
            service: "test".to_string(),
            message: "invalid key".to_string(),
        };
        assert_eq!(
            service.classify_error(&auth_error),
            FailureReason::AuthenticationFailed
        );

        let rate_limit_error = RadarrError::RateLimited {
            service: "test".to_string(),
            retry_after: Some(60),
        };
        assert_eq!(
            service.classify_error(&rate_limit_error),
            FailureReason::RateLimited
        );
    }

    #[test]
    fn test_processing_result() {
        let mut result = ProcessingResult::default();
        result.retries_scheduled = 5;
        result.max_retries_reached = 2;
        result.permanently_blocked = 1;

        assert_eq!(result.total_processed(), 8);
        assert!(result.is_success());

        result.errors.push("test error".to_string());
        assert!(!result.is_success());
    }

    // Mock repository for testing
    struct MockRepository;

    impl MockRepository {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait]
    impl BlocklistRepository for MockRepository {
        async fn add_entry(&self, _entry: &BlocklistEntry) -> Result<BlocklistEntry> {
            unimplemented!("Mock repository")
        }

        async fn is_blocked(&self, _release_id: &str, _indexer: &str) -> Result<bool> {
            Ok(false)
        }

        async fn get_entry(
            &self,
            _release_id: &str,
            _indexer: &str,
        ) -> Result<Option<BlocklistEntry>> {
            Ok(None)
        }

        async fn get_entry_by_id(&self, _id: Uuid) -> Result<Option<BlocklistEntry>> {
            Ok(None)
        }

        async fn search_entries(&self, _query: &BlocklistQuery) -> Result<Vec<BlocklistEntry>> {
            Ok(vec![])
        }

        async fn count_entries(&self, _query: &BlocklistQuery) -> Result<i64> {
            Ok(0)
        }

        async fn update_entry(&self, entry: &BlocklistEntry) -> Result<BlocklistEntry> {
            Ok(entry.clone())
        }

        async fn remove_entry(&self, _release_id: &str, _indexer: &str) -> Result<bool> {
            Ok(true)
        }

        async fn remove_entry_by_id(&self, _id: Uuid) -> Result<bool> {
            Ok(true)
        }

        async fn get_expired_entries(&self, _limit: Option<i32>) -> Result<Vec<BlocklistEntry>> {
            Ok(vec![])
        }

        async fn get_expiring_entries(
            &self,
            _within_hours: i32,
            _limit: Option<i32>,
        ) -> Result<Vec<BlocklistEntry>> {
            Ok(vec![])
        }

        async fn cleanup_expired_entries(&self, _older_than_days: i32) -> Result<i64> {
            Ok(0)
        }

        async fn cleanup_indexer_entries(&self, _indexer: &str) -> Result<i64> {
            Ok(0)
        }

        async fn get_statistics(&self) -> Result<BlocklistStatistics> {
            Ok(BlocklistStatistics {
                active_entries: 0,
                expired_entries: 0,
                permanent_blocks: 0,
                recent_additions: 0,
                top_failure_reason: None,
                top_failing_indexer: None,
            })
        }

        async fn get_failure_reason_stats(&self) -> Result<Vec<FailureReasonStat>> {
            Ok(vec![])
        }

        async fn get_entries_for_movie(&self, _movie_id: Uuid) -> Result<Vec<BlocklistEntry>> {
            Ok(vec![])
        }

        async fn remove_entries_for_movie(&self, _movie_id: Uuid) -> Result<i64> {
            Ok(0)
        }

        async fn get_recent_failure(&self, _release_id: &str) -> Result<Option<BlocklistEntry>> {
            Ok(None)
        }

        async fn check_indexer_health(
            &self,
            _indexer: &str,
            _hours_back: i32,
            _failure_threshold: i32,
        ) -> Result<bool> {
            Ok(true)
        }
    }
}
