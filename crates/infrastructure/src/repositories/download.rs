//! Simplified PostgreSQL implementation of DownloadRepository

use crate::database::DatabasePool;
use async_trait::async_trait;
use radarr_core::{
    domain::repositories::DownloadRepository,
    models::{Download, DownloadStatus},
    Result,
};
use sqlx::Row;
use uuid::Uuid;

/// PostgreSQL implementation of DownloadRepository
pub struct PostgresDownloadRepository {
    pool: DatabasePool,
}

impl PostgresDownloadRepository {
    /// Create a new PostgreSQL download repository
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DownloadRepository for PostgresDownloadRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Download>> {
        let row = sqlx::query(
            "SELECT id, movie_id, download_client_id, indexer_id, download_id,
             title, category, status, size_bytes, size_left, quality,
             download_time, completion_time, error_message, imported,
             import_time, created_at, updated_at FROM downloads WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let download = Download {
                    id: row.try_get("id")?,
                    movie_id: row.try_get("movie_id")?,
                    download_client_id: row.try_get("download_client_id")?,
                    indexer_id: row.try_get("indexer_id")?,
                    download_id: row.try_get("download_id")?,
                    title: row.try_get("title")?,
                    category: row.try_get("category")?,
                    status: parse_download_status(&row.try_get::<String, _>("status")?)?,
                    size_bytes: row.try_get("size_bytes")?,
                    size_left: row.try_get("size_left")?,
                    quality: row.try_get("quality")?,
                    download_time: row.try_get("download_time")?,
                    completion_time: row.try_get("completion_time")?,
                    error_message: row.try_get("error_message")?,
                    imported: row.try_get("imported")?,
                    import_time: row.try_get("import_time")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(download))
            }
            None => Ok(None),
        }
    }

    async fn find_by_movie_id(&self, movie_id: Uuid) -> Result<Vec<Download>> {
        let rows = sqlx::query(
            "SELECT id, movie_id, download_client_id, indexer_id, download_id,
             title, category, status, size_bytes, size_left, quality,
             download_time, completion_time, error_message, imported,
             import_time, created_at, updated_at FROM downloads WHERE movie_id = $1
             ORDER BY created_at DESC",
        )
        .bind(movie_id)
        .fetch_all(&self.pool)
        .await?;

        let mut downloads = Vec::new();
        for row in rows {
            let download = Download {
                id: row.try_get("id")?,
                movie_id: row.try_get("movie_id")?,
                download_client_id: row.try_get("download_client_id")?,
                indexer_id: row.try_get("indexer_id")?,
                download_id: row.try_get("download_id")?,
                title: row.try_get("title")?,
                category: row.try_get("category")?,
                status: parse_download_status(&row.try_get::<String, _>("status")?)?,
                size_bytes: row.try_get("size_bytes")?,
                size_left: row.try_get("size_left")?,
                quality: row.try_get("quality")?,
                download_time: row.try_get("download_time")?,
                completion_time: row.try_get("completion_time")?,
                error_message: row.try_get("error_message")?,
                imported: row.try_get("imported")?,
                import_time: row.try_get("import_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            downloads.push(download);
        }

        Ok(downloads)
    }

    async fn find_by_status(&self, _status: DownloadStatus) -> Result<Vec<Download>> {
        Ok(Vec::new())
    }

    async fn find_active(&self) -> Result<Vec<Download>> {
        Ok(Vec::new())
    }

    async fn find_completed_not_imported(&self) -> Result<Vec<Download>> {
        Ok(Vec::new())
    }

    async fn create(&self, download: &Download) -> Result<Download> {
        let _result = sqlx::query(
            "INSERT INTO downloads (id, movie_id, download_client_id, indexer_id, download_id,
             title, category, status, size_bytes, size_left, quality,
             download_time, completion_time, error_message, imported,
             import_time, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)"
        )
        .bind(download.id)
        .bind(download.movie_id)
        .bind(download.download_client_id)
        .bind(download.indexer_id)
        .bind(&download.download_id)
        .bind(&download.title)
        .bind(&download.category)
        .bind(download.status.to_string())
        .bind(download.size_bytes)
        .bind(download.size_left)
        .bind(&download.quality)
        .bind(download.download_time)
        .bind(download.completion_time)
        .bind(&download.error_message)
        .bind(download.imported)
        .bind(download.import_time)
        .bind(download.created_at)
        .bind(download.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(download.clone())
    }

    async fn update(&self, download: &Download) -> Result<Download> {
        let _result = sqlx::query(
            "UPDATE downloads SET movie_id = $2, download_client_id = $3, indexer_id = $4,
             download_id = $5, title = $6, category = $7, status = $8,
             size_bytes = $9, size_left = $10, quality = $11,
             download_time = $12, completion_time = $13, error_message = $14,
             imported = $15, import_time = $16, updated_at = $17 WHERE id = $1",
        )
        .bind(download.id)
        .bind(download.movie_id)
        .bind(download.download_client_id)
        .bind(download.indexer_id)
        .bind(&download.download_id)
        .bind(&download.title)
        .bind(&download.category)
        .bind(download.status.to_string())
        .bind(download.size_bytes)
        .bind(download.size_left)
        .bind(&download.quality)
        .bind(download.download_time)
        .bind(download.completion_time)
        .bind(&download.error_message)
        .bind(download.imported)
        .bind(download.import_time)
        .bind(download.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(download.clone())
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM downloads WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list(&self, offset: i64, limit: i32) -> Result<Vec<Download>> {
        let rows = sqlx::query(
            "SELECT id, movie_id, download_client_id, indexer_id, download_id,
             title, category, status, size_bytes, size_left, quality,
             download_time, completion_time, error_message, imported,
             import_time, created_at, updated_at FROM downloads
             ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut downloads = Vec::new();
        for row in rows {
            let download = Download {
                id: row.try_get("id")?,
                movie_id: row.try_get("movie_id")?,
                download_client_id: row.try_get("download_client_id")?,
                indexer_id: row.try_get("indexer_id")?,
                download_id: row.try_get("download_id")?,
                title: row.try_get("title")?,
                category: row.try_get("category")?,
                status: parse_download_status(&row.try_get::<String, _>("status")?)?,
                size_bytes: row.try_get("size_bytes")?,
                size_left: row.try_get("size_left")?,
                quality: row.try_get("quality")?,
                download_time: row.try_get("download_time")?,
                completion_time: row.try_get("completion_time")?,
                error_message: row.try_get("error_message")?,
                imported: row.try_get("imported")?,
                import_time: row.try_get("import_time")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            };
            downloads.push(download);
        }

        Ok(downloads)
    }

    async fn cleanup_old(&self, _days: i32) -> Result<i64> {
        Ok(0)
    }
}

fn parse_download_status(status_str: &str) -> Result<DownloadStatus> {
    match status_str {
        "queued" => Ok(DownloadStatus::Queued),
        "downloading" => Ok(DownloadStatus::Downloading),
        "completed" => Ok(DownloadStatus::Completed),
        "failed" => Ok(DownloadStatus::Failed),
        "warning" => Ok(DownloadStatus::Warning),
        "paused" => Ok(DownloadStatus::Paused),
        "importing" => Ok(DownloadStatus::Importing),
        "imported" => Ok(DownloadStatus::Imported),
        "removed" => Ok(DownloadStatus::Removed),
        _ => Err(radarr_core::RadarrError::ValidationError {
            field: "status".to_string(),
            message: format!("Invalid download status: {}", status_str),
        }),
    }
}
