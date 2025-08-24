//! List Sync Repository Implementation
//!
//! Provides PostgreSQL-backed persistence for list synchronization operations,
//! including comprehensive audit logging and performance tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::error::InfrastructureError;

/// Repository for managing list sync history and operations
#[derive(Clone)]
pub struct PostgresListSyncRepository {
    pool: PgPool,
}

/// Detailed sync history entry with comprehensive audit information
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncHistoryEntry {
    pub id: Uuid,
    pub import_list_id: Uuid,
    pub sync_status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i32>,
    pub items_found: i32,
    pub items_added: i32,
    pub items_updated: i32,
    pub items_removed: i32,
    pub items_excluded: i32,
    pub error_message: Option<String>,
    pub error_details: Option<serde_json::Value>,
    pub sync_metadata: Option<serde_json::Value>,
}

/// Performance metrics for a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPerformanceMetrics {
    pub list_id: Uuid,
    pub duration_ms: i64,
    pub items_per_second: f64,
    pub memory_peak_mb: Option<f64>,
    pub network_requests: i32,
    pub cache_hit_rate: Option<f64>,
    pub error_rate: f64,
    pub timestamp: DateTime<Utc>,
}

/// Conflict resolution details for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionDetail {
    pub movie_tmdb_id: Option<i32>,
    pub movie_imdb_id: Option<String>,
    pub movie_title: String,
    pub conflict_type: String,
    pub resolution_strategy: String,
    pub existing_data: serde_json::Value,
    pub new_data: serde_json::Value,
    pub final_data: serde_json::Value,
    pub resolved_at: DateTime<Utc>,
}

/// Comprehensive sync statistics over time periods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatistics {
    pub list_id: Option<Uuid>,
    pub time_period: String,
    pub total_syncs: i64,
    pub successful_syncs: i64,
    pub failed_syncs: i64,
    pub average_duration_ms: f64,
    pub total_items_processed: i64,
    pub average_items_per_sync: f64,
    pub peak_items_per_second: f64,
    pub error_rate_percent: f64,
    pub most_common_errors: Vec<(String, i64)>,
}

impl PostgresListSyncRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Start a new sync operation and return the history ID
    pub async fn start_sync(&self, list_id: Uuid, metadata: Option<serde_json::Value>) -> Result<Uuid, InfrastructureError> {
        let sync_id = Uuid::new_v4();
        
        sqlx::query!(
            r#"
            INSERT INTO list_sync_history 
            (id, import_list_id, sync_status, started_at, items_found, items_added, 
             items_updated, items_removed, items_excluded, sync_metadata)
            VALUES ($1, $2, 'started', $3, 0, 0, 0, 0, 0, $4)
            "#,
            sync_id,
            list_id,
            Utc::now(),
            metadata
        )
        .execute(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        info!("Started sync operation {} for list {}", sync_id, list_id);
        Ok(sync_id)
    }

    /// Complete a sync operation with results
    pub async fn complete_sync(
        &self,
        sync_id: Uuid,
        status: &str,
        items_found: i32,
        items_added: i32,
        items_updated: i32,
        items_removed: i32,
        items_excluded: i32,
        error_message: Option<String>,
        error_details: Option<serde_json::Value>,
    ) -> Result<(), InfrastructureError> {
        let completed_at = Utc::now();
        
        let duration_ms = sqlx::query!(
            "SELECT started_at FROM list_sync_history WHERE id = $1",
            sync_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?
        .started_at;

        let duration = (completed_at.signed_duration_since(duration_ms)).num_milliseconds() as i32;

        sqlx::query!(
            r#"
            UPDATE list_sync_history 
            SET sync_status = $2, completed_at = $3, duration_ms = $4,
                items_found = $5, items_added = $6, items_updated = $7,
                items_removed = $8, items_excluded = $9, error_message = $10,
                error_details = $11
            WHERE id = $1
            "#,
            sync_id,
            status,
            completed_at,
            duration,
            items_found,
            items_added,
            items_updated,
            items_removed,
            items_excluded,
            error_message,
            error_details
        )
        .execute(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        info!(
            "Completed sync {} with status: {} ({} items in {}ms)", 
            sync_id, status, items_found, duration
        );
        Ok(())
    }

    /// Record performance metrics for a sync operation
    pub async fn record_performance_metrics(
        &self,
        metrics: &SyncPerformanceMetrics,
    ) -> Result<(), InfrastructureError> {
        let metrics_json = serde_json::to_value(metrics)
            .map_err(InfrastructureError::Serialization)?;

        // Store in sync_metadata of the most recent sync for this list
        sqlx::query!(
            r#"
            UPDATE list_sync_history 
            SET sync_metadata = COALESCE(sync_metadata, '{}'::jsonb) || jsonb_build_object('performance_metrics', $2)
            WHERE import_list_id = $1 
            AND started_at >= $3 - INTERVAL '1 hour'
            ORDER BY started_at DESC 
            LIMIT 1
            "#,
            metrics.list_id,
            metrics_json,
            metrics.timestamp
        )
        .execute(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        debug!("Recorded performance metrics for list {}", metrics.list_id);
        Ok(())
    }

    /// Record conflict resolution details
    pub async fn record_conflict_resolution(
        &self,
        sync_id: Uuid,
        conflicts: &[ConflictResolutionDetail],
    ) -> Result<(), InfrastructureError> {
        if conflicts.is_empty() {
            return Ok(());
        }

        let conflicts_json = serde_json::to_value(conflicts)
            .map_err(InfrastructureError::Serialization)?;

        sqlx::query!(
            r#"
            UPDATE list_sync_history 
            SET sync_metadata = COALESCE(sync_metadata, '{}'::jsonb) || jsonb_build_object('conflict_resolutions', $2)
            WHERE id = $1
            "#,
            sync_id,
            conflicts_json
        )
        .execute(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        info!("Recorded {} conflict resolutions for sync {}", conflicts.len(), sync_id);
        Ok(())
    }

    /// Get sync history for a specific list with pagination
    pub async fn get_sync_history(
        &self,
        list_id: Option<Uuid>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SyncHistoryEntry>, InfrastructureError> {
        let entries = if let Some(list_id) = list_id {
            sqlx::query_as!(
                SyncHistoryEntry,
                r#"
                SELECT id, import_list_id, sync_status, started_at, completed_at,
                       duration_ms, 
                       items_found, items_added, items_updated,
                       items_removed, items_excluded, error_message, 
                       error_details,
                       sync_metadata
                FROM list_sync_history 
                WHERE import_list_id = $1
                ORDER BY started_at DESC 
                LIMIT $2 OFFSET $3
                "#,
                list_id,
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(InfrastructureError::Database)?
        } else {
            sqlx::query_as!(
                SyncHistoryEntry,
                r#"
                SELECT id, import_list_id, sync_status, started_at, completed_at,
                       duration_ms, 
                       items_found, items_added, items_updated,
                       items_removed, items_excluded, error_message, 
                       error_details,
                       sync_metadata
                FROM list_sync_history 
                ORDER BY started_at DESC 
                LIMIT $1 OFFSET $2
                "#,
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(InfrastructureError::Database)?
        };

        debug!("Retrieved {} sync history entries", entries.len());
        Ok(entries)
    }

    /// Get comprehensive statistics for sync operations
    pub async fn get_sync_statistics(
        &self,
        list_id: Option<Uuid>,
        days_back: i32,
    ) -> Result<SyncStatistics, InfrastructureError> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days_back as i64);

        let base_query = if let Some(list_id) = list_id {
            sqlx::query!(
                r#"
                SELECT 
                    COUNT(*) as "total_syncs!",
                    COUNT(*) FILTER (WHERE sync_status = 'success') as "successful_syncs!",
                    COUNT(*) FILTER (WHERE sync_status = 'failed') as "failed_syncs!",
                    AVG(duration_ms::float) as "avg_duration_ms?",
                    SUM(items_found) as "total_items?",
                    AVG(items_found::float) as "avg_items_per_sync?",
                    MAX(CASE 
                        WHEN duration_ms > 0 THEN items_found::float / (duration_ms::float / 1000.0)
                        ELSE 0 
                    END) as "peak_items_per_second?"
                FROM list_sync_history 
                WHERE import_list_id = $1 AND started_at >= $2
                "#,
                list_id,
                cutoff_date
            )
            .fetch_one(&self.pool)
            .await
            .map_err(InfrastructureError::Database)?
        } else {
            sqlx::query!(
                r#"
                SELECT 
                    COUNT(*) as "total_syncs!",
                    COUNT(*) FILTER (WHERE sync_status = 'success') as "successful_syncs!",
                    COUNT(*) FILTER (WHERE sync_status = 'failed') as "failed_syncs!",
                    AVG(duration_ms::float) as "avg_duration_ms?",
                    SUM(items_found) as "total_items?",
                    AVG(items_found::float) as "avg_items_per_sync?",
                    MAX(CASE 
                        WHEN duration_ms > 0 THEN items_found::float / (duration_ms::float / 1000.0)
                        ELSE 0 
                    END) as "peak_items_per_second?"
                FROM list_sync_history 
                WHERE started_at >= $1
                "#,
                cutoff_date
            )
            .fetch_one(&self.pool)
            .await
            .map_err(InfrastructureError::Database)?
        };

        // Get most common errors
        let error_query = if let Some(list_id) = list_id {
            sqlx::query!(
                r#"
                SELECT error_message, COUNT(*) as error_count
                FROM list_sync_history 
                WHERE import_list_id = $1 AND started_at >= $2 AND error_message IS NOT NULL
                GROUP BY error_message 
                ORDER BY error_count DESC 
                LIMIT 5
                "#,
                list_id,
                cutoff_date
            )
        } else {
            sqlx::query!(
                r#"
                SELECT error_message, COUNT(*) as error_count
                FROM list_sync_history 
                WHERE started_at >= $1 AND error_message IS NOT NULL
                GROUP BY error_message 
                ORDER BY error_count DESC 
                LIMIT 5
                "#,
                cutoff_date
            )
        }
        .fetch_all(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        let most_common_errors: Vec<(String, i64)> = error_query
            .into_iter()
            .map(|row| (row.error_message.unwrap_or_default(), row.error_count.unwrap_or(0)))
            .collect();

        let total_syncs = base_query.total_syncs.unwrap_or(0);
        let failed_syncs = base_query.failed_syncs.unwrap_or(0);
        let error_rate = if total_syncs > 0 {
            (failed_syncs as f64 / total_syncs as f64) * 100.0
        } else {
            0.0
        };

        Ok(SyncStatistics {
            list_id,
            time_period: format!("{} days", days_back),
            total_syncs,
            successful_syncs: base_query.successful_syncs.unwrap_or(0),
            failed_syncs,
            average_duration_ms: base_query.avg_duration_ms.unwrap_or(0.0),
            total_items_processed: base_query.total_items.unwrap_or(0),
            average_items_per_sync: base_query.avg_items_per_sync.unwrap_or(0.0),
            peak_items_per_second: base_query.peak_items_per_second.unwrap_or(0.0),
            error_rate_percent: error_rate,
            most_common_errors,
        })
    }

    /// Get recent sync performance trends
    pub async fn get_performance_trends(
        &self,
        list_id: Option<Uuid>,
        hours_back: i32,
    ) -> Result<Vec<(DateTime<Utc>, f64, f64)>, InfrastructureError> {
        let cutoff_date = Utc::now() - chrono::Duration::hours(hours_back as i64);

        let query = if let Some(list_id) = list_id {
            sqlx::query!(
                r#"
                SELECT 
                    DATE_TRUNC('hour', started_at) as hour_bucket,
                    AVG(duration_ms) as avg_duration,
                    AVG(CASE 
                        WHEN duration_ms > 0 THEN items_found::float / (duration_ms::float / 1000.0)
                        ELSE 0 
                    END) as avg_items_per_second
                FROM list_sync_history 
                WHERE import_list_id = $1 AND started_at >= $2 AND sync_status = 'success'
                GROUP BY DATE_TRUNC('hour', started_at)
                ORDER BY hour_bucket ASC
                "#,
                list_id,
                cutoff_date
            )
        } else {
            sqlx::query!(
                r#"
                SELECT 
                    DATE_TRUNC('hour', started_at) as hour_bucket,
                    AVG(duration_ms) as avg_duration,
                    AVG(CASE 
                        WHEN duration_ms > 0 THEN items_found::float / (duration_ms::float / 1000.0)
                        ELSE 0 
                    END) as avg_items_per_second
                FROM list_sync_history 
                WHERE started_at >= $1 AND sync_status = 'success'
                GROUP BY DATE_TRUNC('hour', started_at)
                ORDER BY hour_bucket ASC
                "#,
                cutoff_date
            )
        }
        .fetch_all(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        let trends = query
            .into_iter()
            .map(|row| {
                (
                    row.hour_bucket.unwrap(),
                    row.avg_duration.unwrap_or(0.0),
                    row.avg_items_per_second.unwrap_or(0.0),
                )
            })
            .collect();

        debug!("Retrieved {} performance trend data points", trends.len());
        Ok(trends)
    }

    /// Clean up old sync history entries
    pub async fn cleanup_old_history(&self, days_to_keep: i32) -> Result<u64, InfrastructureError> {
        let cutoff_date = Utc::now() - chrono::Duration::days(days_to_keep as i64);

        let result = sqlx::query!(
            "DELETE FROM list_sync_history WHERE started_at < $1",
            cutoff_date
        )
        .execute(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        let deleted_count = result.rows_affected();
        if deleted_count > 0 {
            info!("Cleaned up {} old sync history entries", deleted_count);
        }

        Ok(deleted_count)
    }

    /// Get active (running) sync operations
    pub async fn get_active_syncs(&self) -> Result<Vec<SyncHistoryEntry>, InfrastructureError> {
        let entries = sqlx::query_as!(
            SyncHistoryEntry,
            r#"
            SELECT id, import_list_id, sync_status, started_at, completed_at,
                   duration_ms, items_found, items_added, items_updated,
                   items_removed, items_excluded, error_message, error_details,
                   sync_metadata
            FROM list_sync_history 
            WHERE sync_status = 'started' 
            ORDER BY started_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        Ok(entries)
    }

    /// Cancel a running sync operation
    pub async fn cancel_sync(&self, sync_id: Uuid) -> Result<bool, InfrastructureError> {
        let result = sqlx::query!(
            r#"
            UPDATE list_sync_history 
            SET sync_status = 'cancelled', completed_at = $2
            WHERE id = $1 AND sync_status = 'started'
            "#,
            sync_id,
            Utc::now()
        )
        .execute(&self.pool)
        .await
        .map_err(InfrastructureError::Database)?;

        let cancelled = result.rows_affected() > 0;
        if cancelled {
            info!("Cancelled sync operation {}", sync_id);
        }

        Ok(cancelled)
    }
}

// Include comprehensive test module
#[cfg(test)]
#[path = "list_sync_tests.rs"]
mod tests;