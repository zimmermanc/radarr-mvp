//! Queue repository implementation for PostgreSQL

use async_trait::async_trait;
use radarr_core::{
    QueueItem, QueuePriority, QueueRepository, QueueStats, QueueStatus, RadarrError, Result,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// PostgreSQL implementation of QueueRepository
pub struct PostgresQueueRepository {
    pool: PgPool,
}

impl PostgresQueueRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert database row to QueueItem
    fn row_to_queue_item(&self, row: &sqlx::postgres::PgRow) -> Result<QueueItem> {
        let status_str: String = row.try_get("status")?;
        let priority_str: String = row.try_get("priority")?;

        let status = match status_str.as_str() {
            "queued" => QueueStatus::Queued,
            "downloading" => QueueStatus::Downloading,
            "completed" => QueueStatus::Completed,
            "failed" => QueueStatus::Failed,
            "cancelled" => QueueStatus::Cancelled,
            "paused" => QueueStatus::Paused,
            "stalled" => QueueStatus::Stalled,
            "seeding" => QueueStatus::Seeding,
            _ => {
                return Err(RadarrError::DatabaseError {
                    message: format!("Invalid queue status: {}", status_str),
                })
            }
        };

        let priority = match priority_str.as_str() {
            "low" => QueuePriority::Low,
            "normal" => QueuePriority::Normal,
            "high" => QueuePriority::High,
            "very_high" => QueuePriority::VeryHigh,
            _ => {
                return Err(RadarrError::DatabaseError {
                    message: format!("Invalid queue priority: {}", priority_str),
                })
            }
        };

        // Convert NUMERIC to f64
        let progress: rust_decimal::Decimal = row.try_get("progress")?;
        let progress_f64 =
            progress
                .to_string()
                .parse::<f64>()
                .map_err(|_| RadarrError::DatabaseError {
                    message: "Failed to parse progress value".to_string(),
                })?;

        Ok(QueueItem {
            id: row.try_get("id")?,
            movie_id: row.try_get("movie_id")?,
            release_id: row.try_get("release_id")?,
            title: row.try_get("title")?,
            download_url: row.try_get("download_url")?,
            magnet_url: row.try_get("magnet_url")?,
            size_bytes: row.try_get("size_bytes")?,
            status,
            priority,
            progress: progress_f64,
            download_client_id: row.try_get("download_client_id")?,
            download_path: row.try_get("download_path")?,
            category: row.try_get("category")?,
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            upload_bytes: row.try_get("upload_bytes")?,
            download_speed: row
                .try_get::<Option<i64>, _>("download_speed")?
                .map(|v| v as u64),
            upload_speed: row
                .try_get::<Option<i64>, _>("upload_speed")?
                .map(|v| v as u64),
            eta_seconds: row.try_get("eta_seconds")?,
            seeders: row.try_get("seeders")?,
            leechers: row.try_get("leechers")?,
            error_message: row.try_get("error_message")?,
            retry_count: row.try_get("retry_count")?,
            max_retries: row.try_get("max_retries")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            started_at: row.try_get("started_at")?,
            completed_at: row.try_get("completed_at")?,
        })
    }
}

#[async_trait]
impl QueueRepository for PostgresQueueRepository {
    async fn add_queue_item(&self, item: &QueueItem) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO queue (
                id, movie_id, release_id, title, download_url, magnet_url, size_bytes,
                status, priority, progress, download_client_id, download_path, category,
                downloaded_bytes, upload_bytes, download_speed, upload_speed, eta_seconds,
                seeders, leechers, error_message, retry_count, max_retries,
                created_at, updated_at, started_at, completed_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27
            )
            "#,
        )
        .bind(item.id)
        .bind(item.movie_id)
        .bind(item.release_id)
        .bind(&item.title)
        .bind(&item.download_url)
        .bind(&item.magnet_url)
        .bind(item.size_bytes)
        .bind(item.status.to_string())
        .bind(item.priority.to_string())
        .bind(rust_decimal::Decimal::from_f64_retain(item.progress).unwrap_or_default())
        .bind(&item.download_client_id)
        .bind(&item.download_path)
        .bind(&item.category)
        .bind(item.downloaded_bytes)
        .bind(item.upload_bytes)
        .bind(item.download_speed.map(|v| v as i64))
        .bind(item.upload_speed.map(|v| v as i64))
        .bind(item.eta_seconds)
        .bind(item.seeders)
        .bind(item.leechers)
        .bind(&item.error_message)
        .bind(item.retry_count)
        .bind(item.max_retries)
        .bind(item.created_at)
        .bind(item.updated_at)
        .bind(item.started_at)
        .bind(item.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to insert queue item: {}", e),
        })?;

        Ok(())
    }

    async fn get_queue_item(&self, id: Uuid) -> Result<Option<QueueItem>> {
        let row = sqlx::query("SELECT * FROM queue WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to fetch queue item: {}", e),
            })?;

        match row {
            Some(row) => Ok(Some(self.row_to_queue_item(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_queue_item_by_client_id(&self, client_id: &str) -> Result<Option<QueueItem>> {
        let row = sqlx::query("SELECT * FROM queue WHERE download_client_id = $1")
            .bind(client_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to fetch queue item by client ID: {}", e),
            })?;

        match row {
            Some(row) => Ok(Some(self.row_to_queue_item(&row)?)),
            None => Ok(None),
        }
    }

    async fn get_queue_items(&self, status_filter: Option<QueueStatus>) -> Result<Vec<QueueItem>> {
        let query = match status_filter {
            Some(status) => sqlx::query(
                "SELECT * FROM queue WHERE status = $1 ORDER BY priority DESC, created_at ASC",
            )
            .bind(status.to_string()),
            None => sqlx::query("SELECT * FROM queue ORDER BY priority DESC, created_at ASC"),
        };

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to fetch queue items: {}", e),
            })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_queue_item(&row)?);
        }

        Ok(items)
    }

    async fn get_queue_items_for_movie(&self, movie_id: Uuid) -> Result<Vec<QueueItem>> {
        let rows = sqlx::query("SELECT * FROM queue WHERE movie_id = $1 ORDER BY created_at DESC")
            .bind(movie_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to fetch queue items for movie: {}", e),
            })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_queue_item(&row)?);
        }

        Ok(items)
    }

    async fn update_queue_item(&self, item: &QueueItem) -> Result<()> {
        let affected = sqlx::query(
            r#"
            UPDATE queue SET
                title = $2, download_url = $3, magnet_url = $4, size_bytes = $5,
                status = $6, priority = $7, progress = $8, download_client_id = $9,
                download_path = $10, category = $11, downloaded_bytes = $12,
                upload_bytes = $13, download_speed = $14, upload_speed = $15,
                eta_seconds = $16, seeders = $17, leechers = $18, error_message = $19,
                retry_count = $20, max_retries = $21, updated_at = $22,
                started_at = $23, completed_at = $24
            WHERE id = $1
            "#,
        )
        .bind(item.id)
        .bind(&item.title)
        .bind(&item.download_url)
        .bind(&item.magnet_url)
        .bind(item.size_bytes)
        .bind(item.status.to_string())
        .bind(item.priority.to_string())
        .bind(rust_decimal::Decimal::from_f64_retain(item.progress).unwrap_or_default())
        .bind(&item.download_client_id)
        .bind(&item.download_path)
        .bind(&item.category)
        .bind(item.downloaded_bytes)
        .bind(item.upload_bytes)
        .bind(item.download_speed.map(|v| v as i64))
        .bind(item.upload_speed.map(|v| v as i64))
        .bind(item.eta_seconds)
        .bind(item.seeders)
        .bind(item.leechers)
        .bind(&item.error_message)
        .bind(item.retry_count)
        .bind(item.max_retries)
        .bind(item.updated_at)
        .bind(item.started_at)
        .bind(item.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to update queue item: {}", e),
        })?;

        if affected.rows_affected() == 0 {
            return Err(RadarrError::NotFoundError {
                entity: "queue_item".to_string(),
                id: item.id.to_string(),
            });
        }

        Ok(())
    }

    async fn delete_queue_item(&self, id: Uuid) -> Result<()> {
        let affected = sqlx::query("DELETE FROM queue WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| RadarrError::DatabaseError {
                message: format!("Failed to delete queue item: {}", e),
            })?;

        if affected.rows_affected() == 0 {
            return Err(RadarrError::NotFoundError {
                entity: "queue_item".to_string(),
                id: id.to_string(),
            });
        }

        Ok(())
    }

    async fn get_queue_stats(&self) -> Result<QueueStats> {
        let stats_row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_count,
                SUM(CASE WHEN status = 'queued' THEN 1 ELSE 0 END) as queued_count,
                SUM(CASE WHEN status = 'downloading' THEN 1 ELSE 0 END) as downloading_count,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed_count,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed_count,
                SUM(CASE WHEN status = 'paused' THEN 1 ELSE 0 END) as paused_count,
                COALESCE(SUM(CASE WHEN download_speed IS NOT NULL THEN download_speed ELSE 0 END), 0) as total_download_speed,
                COALESCE(SUM(CASE WHEN upload_speed IS NOT NULL THEN upload_speed ELSE 0 END), 0) as total_upload_speed,
                COALESCE(SUM(CASE WHEN size_bytes IS NOT NULL THEN size_bytes ELSE 0 END), 0) as total_size_bytes,
                COALESCE(SUM(CASE WHEN downloaded_bytes IS NOT NULL THEN downloaded_bytes ELSE 0 END), 0) as total_downloaded_bytes
            FROM queue
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to fetch queue stats: {}", e),
        })?;

        Ok(QueueStats {
            total_count: stats_row.try_get::<i64, _>("total_count").unwrap_or(0),
            downloading_count: stats_row
                .try_get::<Option<i64>, _>("downloading_count")
                .unwrap_or(None)
                .unwrap_or(0),
            queued_count: stats_row
                .try_get::<Option<i64>, _>("queued_count")
                .unwrap_or(None)
                .unwrap_or(0),
            completed_count: stats_row
                .try_get::<Option<i64>, _>("completed_count")
                .unwrap_or(None)
                .unwrap_or(0),
            failed_count: stats_row
                .try_get::<Option<i64>, _>("failed_count")
                .unwrap_or(None)
                .unwrap_or(0),
            paused_count: stats_row
                .try_get::<Option<i64>, _>("paused_count")
                .unwrap_or(None)
                .unwrap_or(0),
            total_download_speed: stats_row
                .try_get::<Option<i64>, _>("total_download_speed")
                .unwrap_or(None)
                .unwrap_or(0) as u64,
            total_upload_speed: stats_row
                .try_get::<Option<i64>, _>("total_upload_speed")
                .unwrap_or(None)
                .unwrap_or(0) as u64,
            total_size_bytes: stats_row
                .try_get::<Option<i64>, _>("total_size_bytes")
                .unwrap_or(None)
                .unwrap_or(0),
            total_downloaded_bytes: stats_row
                .try_get::<Option<i64>, _>("total_downloaded_bytes")
                .unwrap_or(None)
                .unwrap_or(0),
        })
    }

    async fn get_retry_items(&self) -> Result<Vec<QueueItem>> {
        let rows = sqlx::query(
            "SELECT * FROM queue WHERE status = 'failed' AND retry_count < max_retries ORDER BY updated_at ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RadarrError::DatabaseError {
            message: format!("Failed to fetch retry items: {}", e),
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(self.row_to_queue_item(&row)?);
        }

        Ok(items)
    }
}
