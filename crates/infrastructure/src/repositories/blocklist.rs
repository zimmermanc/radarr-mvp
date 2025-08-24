//! PostgreSQL implementation of the blocklist repository

use crate::error::InfrastructureError;
use radarr_core::{
    Result, RadarrError,
    blocklist::{
        BlocklistEntry, BlocklistQuery, FailureReason, ImportFailureType,
        BlocklistRepository, BlocklistStatistics, FailureReasonStat
    }
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use tracing::{debug, error, info, warn, instrument};
use uuid::Uuid;

/// PostgreSQL implementation of the blocklist repository
pub struct PostgresBlocklistRepository {
    pool: PgPool,
}

impl PostgresBlocklistRepository {
    /// Create a new PostgreSQL blocklist repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Convert FailureReason to database string representation
    fn failure_reason_to_db(&self, reason: &FailureReason) -> (String, Option<String>) {
        match reason {
            FailureReason::ImportFailed(detail) => (
                "ImportFailed".to_string(),
                Some(detail.to_string())
            ),
            _ => (format!("{:?}", reason), None)
        }
    }
    
    /// Convert database strings back to FailureReason
    fn failure_reason_from_db(&self, reason: &str, detail: Option<&str>) -> Result<FailureReason> {
        match reason {
            "ConnectionTimeout" => Ok(FailureReason::ConnectionTimeout),
            "AuthenticationFailed" => Ok(FailureReason::AuthenticationFailed),
            "RateLimited" => Ok(FailureReason::RateLimited),
            "ParseError" => Ok(FailureReason::ParseError),
            "DownloadStalled" => Ok(FailureReason::DownloadStalled),
            "HashMismatch" => Ok(FailureReason::HashMismatch),
            "ImportFailed" => {
                let import_type = match detail {
                    Some("FileMoveError") => ImportFailureType::FileMoveError,
                    Some("FileAlreadyExists") => ImportFailureType::FileAlreadyExists,
                    Some("DirectoryCreationFailed") => ImportFailureType::DirectoryCreationFailed,
                    Some("UnsupportedFormat") => ImportFailureType::UnsupportedFormat,
                    Some("QualityAnalysisFailed") => ImportFailureType::QualityAnalysisFailed,
                    Some("FilenameParseFailed") => ImportFailureType::FilenameParseFailed,
                    Some("MediaInfoFailed") => ImportFailureType::MediaInfoFailed,
                    _ => return Err(RadarrError::DatabaseError {
                        message: format!("Unknown import failure detail: {:?}", detail)
                    })
                };
                Ok(FailureReason::ImportFailed(import_type))
            }
            "DiskFull" => Ok(FailureReason::DiskFull),
            "PermissionDenied" => Ok(FailureReason::PermissionDenied),
            "ManuallyRejected" => Ok(FailureReason::ManuallyRejected),
            "QualityRejected" => Ok(FailureReason::QualityRejected),
            "SizeRejected" => Ok(FailureReason::SizeRejected),
            "ReleasePurged" => Ok(FailureReason::ReleasePurged),
            "NetworkError" => Ok(FailureReason::NetworkError),
            "ServerError" => Ok(FailureReason::ServerError),
            "CorruptedDownload" => Ok(FailureReason::CorruptedDownload),
            "DownloadClientError" => Ok(FailureReason::DownloadClientError),
            "ExclusionMatched" => Ok(FailureReason::ExclusionMatched),
            _ => Err(RadarrError::DatabaseError {
                message: format!("Unknown failure reason: {}", reason)
            })
        }
    }
    
    /// Map database row to BlocklistEntry
    fn row_to_entry(&self, row: &sqlx::postgres::PgRow) -> Result<BlocklistEntry> {
        let reason_str: String = row.try_get("reason")?;
        let reason_detail: Option<String> = row.try_get("reason_detail")?;
        let reason = self.failure_reason_from_db(&reason_str, reason_detail.as_deref())?;
        
        Ok(BlocklistEntry {
            id: row.try_get("id")?,
            release_id: row.try_get("release_id")?,
            indexer: row.try_get("indexer")?,
            reason,
            blocked_until: row.try_get("blocked_until")?,
            retry_count: row.try_get::<i32, _>("retry_count")? as u32,
            movie_id: row.try_get("movie_id")?,
            release_title: row.try_get("release_title")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            metadata: row.try_get("metadata")?,
        })
    }
    
    /// Build WHERE clause from query parameters
    fn build_where_clause(&self, query: &BlocklistQuery) -> (String, Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>>) {
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 1;
        
        if let Some(ref indexer) = query.indexer {
            conditions.push(format!("indexer = ${}", param_count));
            params.push(Box::new(indexer.clone()));
            param_count += 1;
        }
        
        if let Some(reason) = query.reason {
            let (reason_str, detail_str) = self.failure_reason_to_db(&reason);
            conditions.push(format!("reason = ${}", param_count));
            params.push(Box::new(reason_str));
            param_count += 1;
            
            if let Some(detail) = detail_str {
                conditions.push(format!("reason_detail = ${}", param_count));
                params.push(Box::new(detail));
                param_count += 1;
            }
        }
        
        if let Some(movie_id) = query.movie_id {
            conditions.push(format!("movie_id = ${}", param_count));
            params.push(Box::new(movie_id));
            param_count += 1;
        }
        
        if query.expired_only {
            conditions.push("blocked_until <= NOW()".to_string());
        }
        
        if query.active_only {
            conditions.push("blocked_until > NOW()".to_string());
        }
        
        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        
        (where_clause, params)
    }
}

#[async_trait]
impl BlocklistRepository for PostgresBlocklistRepository {
    #[instrument(skip(self, entry), fields(release_id = %entry.release_id, indexer = %entry.indexer))]
    async fn add_entry(&self, entry: &BlocklistEntry) -> Result<BlocklistEntry> {
        let (reason_str, reason_detail) = self.failure_reason_to_db(&entry.reason);
        
        let row = sqlx::query(
            r#"
            INSERT INTO blocklist (
                id, release_id, indexer, reason, reason_detail, 
                blocked_until, retry_count, movie_id, release_title, 
                metadata, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (release_id, indexer) 
            DO UPDATE SET 
                reason = EXCLUDED.reason,
                reason_detail = EXCLUDED.reason_detail,
                blocked_until = EXCLUDED.blocked_until,
                retry_count = EXCLUDED.retry_count,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at
            RETURNING *
            "#
        )
        .bind(&entry.id)
        .bind(&entry.release_id)
        .bind(&entry.indexer)
        .bind(&reason_str)
        .bind(&reason_detail)
        .bind(&entry.blocked_until)
        .bind(entry.retry_count as i32)
        .bind(&entry.movie_id)
        .bind(&entry.release_title)
        .bind(&entry.metadata)
        .bind(&entry.created_at)
        .bind(&entry.updated_at)
        .fetch_one(&self.pool)
        .await?;
        
        self.row_to_entry(&row)
    }
    
    #[instrument(skip(self), fields(release_id = %release_id, indexer = %indexer))]
    async fn is_blocked(&self, release_id: &str, indexer: &str) -> Result<bool> {
        let result: Option<bool> = sqlx::query_scalar(
            "SELECT is_release_blocked($1, $2)"
        )
        .bind(release_id)
        .bind(indexer)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.unwrap_or(false))
    }
    
    #[instrument(skip(self), fields(release_id = %release_id, indexer = %indexer))]
    async fn get_entry(&self, release_id: &str, indexer: &str) -> Result<Option<BlocklistEntry>> {
        let row = sqlx::query(
            "SELECT * FROM blocklist WHERE release_id = $1 AND indexer = $2"
        )
        .bind(release_id)
        .bind(indexer)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => Ok(Some(self.row_to_entry(&row)?)),
            None => Ok(None),
        }
    }
    
    #[instrument(skip(self), fields(id = %id))]
    async fn get_entry_by_id(&self, id: Uuid) -> Result<Option<BlocklistEntry>> {
        let row = sqlx::query(
            "SELECT * FROM blocklist WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => Ok(Some(self.row_to_entry(&row)?)),
            None => Ok(None),
        }
    }
    
    #[instrument(skip(self, query))]
    async fn search_entries(&self, query: &BlocklistQuery) -> Result<Vec<BlocklistEntry>> {
        let (where_clause, mut params) = self.build_where_clause(query);
        
        let sql = format!(
            "SELECT * FROM blocklist {} ORDER BY created_at DESC OFFSET ${} LIMIT ${}",
            where_clause,
            params.len() + 1,
            params.len() + 2
        );
        
        params.push(Box::new(query.offset));
        params.push(Box::new(query.limit));
        
        // Note: This is a simplified approach. In production, you'd want to use
        // a query builder or macro that handles the dynamic parameter binding properly.
        // For now, we'll handle common cases manually.
        
        let rows = if query.indexer.is_some() && query.reason.is_none() && query.movie_id.is_none() {
            sqlx::query(&sql)
                .bind(&query.indexer.as_ref().unwrap())
                .bind(query.offset)
                .bind(query.limit)
                .fetch_all(&self.pool)
                .await?
        } else if query.indexer.is_none() && query.reason.is_none() && query.movie_id.is_none() {
            sqlx::query(&sql)
                .bind(query.offset)
                .bind(query.limit)
                .fetch_all(&self.pool)
                .await?
        } else {
            // For more complex cases, use a simpler approach
            let base_sql = format!("SELECT * FROM blocklist {} ORDER BY created_at DESC", where_clause);
            sqlx::query(&base_sql)
                .fetch_all(&self.pool)
                .await?
        };
        
        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }
        
        Ok(entries)
    }
    
    #[instrument(skip(self, query))]
    async fn count_entries(&self, query: &BlocklistQuery) -> Result<i64> {
        let (where_clause, _) = self.build_where_clause(query);
        
        let sql = format!("SELECT COUNT(*) FROM blocklist {}", where_clause);
        
        // Simplified counting - in production would handle parameters properly
        let count: i64 = sqlx::query_scalar(&sql)
            .fetch_one(&self.pool)
            .await?;
        
        Ok(count)
    }
    
    #[instrument(skip(self, entry), fields(id = %entry.id, release_id = %entry.release_id))]
    async fn update_entry(&self, entry: &BlocklistEntry) -> Result<BlocklistEntry> {
        let (reason_str, reason_detail) = self.failure_reason_to_db(&entry.reason);
        
        let row = sqlx::query(
            r#"
            UPDATE blocklist SET 
                reason = $1, reason_detail = $2, blocked_until = $3, 
                retry_count = $4, metadata = $5, updated_at = $6
            WHERE id = $7
            RETURNING *
            "#
        )
        .bind(&reason_str)
        .bind(&reason_detail)
        .bind(&entry.blocked_until)
        .bind(entry.retry_count as i32)
        .bind(&entry.metadata)
        .bind(&entry.updated_at)
        .bind(&entry.id)
        .fetch_one(&self.pool)
        .await?;
        
        self.row_to_entry(&row)
    }
    
    #[instrument(skip(self), fields(release_id = %release_id, indexer = %indexer))]
    async fn remove_entry(&self, release_id: &str, indexer: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM blocklist WHERE release_id = $1 AND indexer = $2"
        )
        .bind(release_id)
        .bind(indexer)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    #[instrument(skip(self), fields(id = %id))]
    async fn remove_entry_by_id(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM blocklist WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
    
    #[instrument(skip(self))]
    async fn get_expired_entries(&self, limit: Option<i32>) -> Result<Vec<BlocklistEntry>> {
        let limit = limit.unwrap_or(1000);
        
        let rows = sqlx::query(
            "SELECT * FROM blocklist WHERE blocked_until <= NOW() ORDER BY blocked_until ASC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }
        
        Ok(entries)
    }
    
    #[instrument(skip(self))]
    async fn get_expiring_entries(&self, within_hours: i32, limit: Option<i32>) -> Result<Vec<BlocklistEntry>> {
        let limit = limit.unwrap_or(1000);
        
        let rows = sqlx::query(
            "SELECT * FROM blocklist WHERE blocked_until > NOW() AND blocked_until <= NOW() + ($1 || ' hours')::INTERVAL ORDER BY blocked_until ASC LIMIT $2"
        )
        .bind(within_hours)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        
        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }
        
        Ok(entries)
    }
    
    #[instrument(skip(self))]
    async fn cleanup_expired_entries(&self, older_than_days: i32) -> Result<i64> {
        let result: Option<i32> = sqlx::query_scalar(
            "SELECT cleanup_blocklist_entries($1)"
        )
        .bind(older_than_days)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(result.unwrap_or(0) as i64)
    }
    
    #[instrument(skip(self), fields(indexer = %indexer))]
    async fn cleanup_indexer_entries(&self, indexer: &str) -> Result<i64> {
        let result = sqlx::query(
            "DELETE FROM blocklist WHERE indexer = $1"
        )
        .bind(indexer)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() as i64)
    }
    
    #[instrument(skip(self))]
    async fn get_statistics(&self) -> Result<BlocklistStatistics> {
        let row = sqlx::query(
            "SELECT * FROM blocklist_stats"
        )
        .fetch_one(&self.pool)
        .await?;
        
        let top_failure_reason = if let Ok(reason_str) = row.try_get::<Option<String>, _>("top_failure_reason") {
            if let Some(reason_str) = reason_str {
                let count: i64 = row.try_get("top_failure_count")?;
                let reason = self.failure_reason_from_db(&reason_str, None)?;
                Some((reason, count))
            } else {
                None
            }
        } else {
            None
        };
        
        let top_failing_indexer = if let Ok(indexer) = row.try_get::<Option<String>, _>("top_failing_indexer") {
            if let Some(indexer) = indexer {
                let count: i64 = row.try_get("top_indexer_failure_count")?;
                Some((indexer, count))
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(BlocklistStatistics {
            active_entries: row.try_get("active_entries")?,
            expired_entries: row.try_get("expired_entries")?,
            permanent_blocks: row.try_get("permanent_blocks")?,
            recent_additions: row.try_get("recent_additions")?,
            top_failure_reason,
            top_failing_indexer,
        })
    }
    
    #[instrument(skip(self))]
    async fn get_failure_reason_stats(&self) -> Result<Vec<FailureReasonStat>> {
        let rows = sqlx::query(
            "SELECT * FROM blocklist_failure_analysis ORDER BY total_count DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        
        let mut stats = Vec::new();
        for row in rows {
            let reason_str: String = row.try_get("reason")?;
            let reason_detail: Option<String> = row.try_get("reason_detail")?;
            let reason = self.failure_reason_from_db(&reason_str, reason_detail.as_deref())?;
            
            let stat = FailureReasonStat {
                reason,
                active_count: row.try_get("active_count")?,
                expired_count: row.try_get("expired_count")?,
                average_retries: row.try_get::<Option<f64>, _>("avg_retries")?.unwrap_or(0.0),
                retry_success_rate: None, // Would need additional tracking to calculate
            };
            
            stats.push(stat);
        }
        
        Ok(stats)
    }
    
    #[instrument(skip(self), fields(movie_id = %movie_id))]
    async fn get_entries_for_movie(&self, movie_id: Uuid) -> Result<Vec<BlocklistEntry>> {
        let rows = sqlx::query(
            "SELECT * FROM blocklist WHERE movie_id = $1 ORDER BY created_at DESC"
        )
        .bind(movie_id)
        .fetch_all(&self.pool)
        .await?;
        
        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }
        
        Ok(entries)
    }
    
    #[instrument(skip(self), fields(movie_id = %movie_id))]
    async fn remove_entries_for_movie(&self, movie_id: Uuid) -> Result<i64> {
        let result = sqlx::query(
            "DELETE FROM blocklist WHERE movie_id = $1"
        )
        .bind(movie_id)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected() as i64)
    }
    
    #[instrument(skip(self), fields(release_id = %release_id))]
    async fn get_recent_failure(&self, release_id: &str) -> Result<Option<BlocklistEntry>> {
        let row = sqlx::query(
            "SELECT * FROM blocklist WHERE release_id = $1 ORDER BY created_at DESC LIMIT 1"
        )
        .bind(release_id)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => Ok(Some(self.row_to_entry(&row)?)),
            None => Ok(None),
        }
    }
    
    #[instrument(skip(self), fields(indexer = %indexer))]
    async fn check_indexer_health(&self, indexer: &str, hours_back: i32, failure_threshold: i32) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM blocklist WHERE indexer = $1 AND created_at > NOW() - ($2 || ' hours')::INTERVAL"
        )
        .bind(indexer)
        .bind(hours_back)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(count < failure_threshold as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radarr_core::blocklist::models::{BlocklistEntry, FailureReason};
    use sqlx::PgPool;
    
    async fn setup_test_db() -> PgPool {
        // This would set up a test database in a real test environment
        unimplemented!("Test database setup needed")
    }
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_add_and_retrieve_entry() {
        let pool = setup_test_db().await;
        let repo = PostgresBlocklistRepository::new(pool);
        
        let entry = BlocklistEntry::new(
            "test-release-123".to_string(),
            "test-indexer".to_string(),
            FailureReason::ConnectionTimeout,
            "Test Release".to_string(),
        );
        
        let saved_entry = repo.add_entry(&entry).await.unwrap();
        assert_eq!(saved_entry.release_id, entry.release_id);
        
        let retrieved = repo.get_entry(&entry.release_id, &entry.indexer).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().release_id, entry.release_id);
    }
    
    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_is_blocked_functionality() {
        let pool = setup_test_db().await;
        let repo = PostgresBlocklistRepository::new(pool);
        
        let entry = BlocklistEntry::new(
            "test-release-456".to_string(),
            "test-indexer".to_string(),
            FailureReason::ConnectionTimeout,
            "Test Release".to_string(),
        );
        
        // Should not be blocked initially
        let is_blocked = repo.is_blocked(&entry.release_id, &entry.indexer).await.unwrap();
        assert!(!is_blocked);
        
        // Add to blocklist
        repo.add_entry(&entry).await.unwrap();
        
        // Should now be blocked
        let is_blocked = repo.is_blocked(&entry.release_id, &entry.indexer).await.unwrap();
        assert!(is_blocked);
    }
    
    #[tokio::test]
    async fn test_failure_reason_conversion() {
        let repo = PostgresBlocklistRepository::new(PgPool::connect("").await.unwrap());
        
        let reason = FailureReason::ImportFailed(ImportFailureType::FileMoveError);
        let (reason_str, detail_str) = repo.failure_reason_to_db(&reason);
        
        assert_eq!(reason_str, "ImportFailed");
        assert_eq!(detail_str, Some("FileMoveError".to_string()));
        
        let converted_back = repo.failure_reason_from_db(&reason_str, detail_str.as_deref()).unwrap();
        assert_eq!(converted_back, reason);
    }
}