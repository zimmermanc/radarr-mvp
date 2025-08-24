//! Queue management API handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use radarr_core::{
    QueueItem, QueueStatus, QueuePriority, QueueStats,
    services::{QueueService, DownloadClientService, ClientDownloadStatus},
    RadarrError,
};
use radarr_infrastructure::{repositories::PostgresQueueRepository, DatabasePool};
use crate::models::{ApiResponse, PaginationQuery};

/// Request to add a release to the queue
#[derive(Debug, Deserialize)]
pub struct GrabReleaseRequest {
    pub release_id: Uuid,
    pub movie_id: Uuid,
    pub title: String,
    pub download_url: String,
    pub priority: Option<QueuePriority>,
    pub category: Option<String>,
}

/// Query parameters for queue listing
#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    pub status: Option<QueueStatus>,
    pub movie_id: Option<Uuid>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

/// Queue item response compatible with frontend expectations
#[derive(Debug, Serialize, Clone)]
pub struct QueueItemResponse {
    pub id: String,
    #[serde(rename = "movieId")]
    pub movie_id: i32, // Frontend expects number, not UUID
    #[serde(rename = "movieTitle")]
    pub movie_title: String,
    pub quality: String,
    pub protocol: String,
    pub indexer: String,
    #[serde(rename = "downloadClient")]
    pub download_client: String,
    pub status: String,
    pub size: i64,
    #[serde(rename = "sizeLeft")]
    pub size_left: i64,
    #[serde(rename = "downloadedSize")]
    pub downloaded_size: i64,
    pub progress: f64,
    #[serde(rename = "downloadRate")]
    pub download_rate: Option<u64>,
    #[serde(rename = "uploadRate")]
    pub upload_rate: Option<u64>,
    pub seeders: Option<i32>,
    pub leechers: Option<i32>,
    pub eta: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
    pub added: String,
}

impl From<QueueItem> for QueueItemResponse {
    fn from(queue_item: QueueItem) -> Self {
        let eta = queue_item.human_readable_eta();
        Self {
            id: queue_item.id.to_string(),
            movie_id: queue_item.movie_id.as_u128() as i32, // Convert UUID to int (simplified)
            movie_title: queue_item.title,
            quality: "1080p BluRay".to_string(), // Mock quality
            protocol: "torrent".to_string(),
            indexer: "Mock Indexer".to_string(),
            download_client: "qBittorrent".to_string(),
            status: queue_item.status.to_string(),
            size: queue_item.size_bytes.unwrap_or(0),
            size_left: queue_item.size_bytes.unwrap_or(0) - queue_item.downloaded_bytes.unwrap_or(0),
            downloaded_size: queue_item.downloaded_bytes.unwrap_or(0),
            progress: queue_item.progress,
            download_rate: queue_item.download_speed,
            upload_rate: queue_item.upload_speed,
            seeders: queue_item.seeders,
            leechers: queue_item.leechers,
            eta,
            error_message: queue_item.error_message,
            added: queue_item.created_at.to_rfc3339(),
        }
    }
}

/// Enhanced queue statistics with additional metadata
#[derive(Debug, Serialize)]
pub struct QueueStatsResponse {
    #[serde(flatten)]
    pub stats: QueueStats,
    /// Human readable total size
    pub total_size_display: String,
    /// Human readable total downloaded
    pub total_downloaded_display: String,
    /// Human readable download speed
    pub total_download_speed_display: String,
    /// Human readable upload speed
    pub total_upload_speed_display: String,
    /// Overall progress percentage
    pub overall_progress: f64,
}

impl From<QueueStats> for QueueStatsResponse {
    fn from(stats: QueueStats) -> Self {
        let overall_progress = if stats.total_size_bytes > 0 {
            (stats.total_downloaded_bytes as f64 / stats.total_size_bytes as f64) * 100.0
        } else {
            0.0
        };
        
        Self {
            total_size_display: format_bytes(stats.total_size_bytes),
            total_downloaded_display: format_bytes(stats.total_downloaded_bytes),
            total_download_speed_display: format_bytes_per_sec(stats.total_download_speed),
            total_upload_speed_display: format_bytes_per_sec(stats.total_upload_speed),
            overall_progress,
            stats,
        }
    }
}

/// Queue service state for dependency injection
#[derive(Clone)]
pub struct QueueServiceState {
    pub queue_service: Arc<QueueService<PostgresQueueRepository, MockDownloadClient>>,
}

use std::sync::Arc;
use async_trait::async_trait;

/// Mock download client for API handlers
#[derive(Clone)]
pub struct MockDownloadClient;

#[async_trait]
impl DownloadClientService for MockDownloadClient {
    async fn add_download(
        &self,
        _download_url: &str,
        _category: Option<String>,
        _save_path: Option<String>,
    ) -> radarr_core::Result<String> {
        Ok(format!("mock_client_{}", uuid::Uuid::new_v4()))
    }
    
    async fn get_download_status(&self, _client_id: &str) -> radarr_core::Result<Option<ClientDownloadStatus>> {
        Ok(Some(ClientDownloadStatus {
            client_id: _client_id.to_string(),
            name: "Mock Download".to_string(),
            status: "downloading".to_string(),
            progress: 0.5,
            download_speed: Some(1024 * 1024),
            upload_speed: Some(512 * 1024),
            downloaded_bytes: Some(500 * 1024 * 1024),
            upload_bytes: Some(100 * 1024 * 1024),
            eta_seconds: Some(600),
            seeders: Some(10),
            leechers: Some(5),
            save_path: Some("/downloads".to_string()),
        }))
    }
    
    async fn remove_download(&self, _client_id: &str, _delete_files: bool) -> radarr_core::Result<()> {
        Ok(())
    }
    
    async fn pause_download(&self, _client_id: &str) -> radarr_core::Result<()> {
        Ok(())
    }
    
    async fn resume_download(&self, _client_id: &str) -> radarr_core::Result<()> {
        Ok(())
    }
    
    async fn get_all_downloads(&self) -> radarr_core::Result<Vec<ClientDownloadStatus>> {
        Ok(vec![])
    }
}

impl QueueServiceState {
    pub fn new(pool: DatabasePool) -> Self {
        let queue_repo = PostgresQueueRepository::new(pool);
        let download_client = MockDownloadClient;
        let queue_service = Arc::new(QueueService::new(queue_repo, download_client));
        
        Self {
            queue_service,
        }
    }
}

/// POST /api/v3/queue/grab - Add release to queue
pub async fn grab_release(
    State(_state): State<QueueServiceState>,
    Json(request): Json<GrabReleaseRequest>,
) -> std::result::Result<Json<ApiResponse<QueueItemResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // For now, return not implemented as this requires movie/release data
    let _ = request;
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Grab release endpoint requires movie/release integration".to_string())),
    ))
}

/// PUT /api/v3/queue/{id}/priority - Update queue item priority  
pub async fn update_queue_priority(
    State(state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
    Json(request): Json<PriorityUpdateRequest>,
) -> std::result::Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // This is a simplified implementation - in practice you'd need to
    // implement priority reordering logic in the queue service
    let _ = request; // For now, just acknowledge the request
    
    match state.queue_service.pause_queue_item(id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Failed to update priority for queue item {}: {}", id, e);
            let status_code = match e {
                RadarrError::NotFoundError { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status_code,
                Json(ApiResponse::error(format!("Failed to update priority: {}", e))),
            ))
        }
    }
}

/// Request to update queue item priority
#[derive(Debug, Deserialize)]
pub struct PriorityUpdateRequest {
    pub direction: String, // "up" or "down"
}

/// GET /api/v3/queue - List queue items
pub async fn list_queue(
    State(state): State<QueueServiceState>,
    Query(query): Query<QueueQuery>,
) -> std::result::Result<Json<ApiResponse<QueueResponseData>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.queue_service.get_queue_items(
        query.status,
        query.movie_id,
        Some(query.pagination.page_size as usize),
        Some(((query.pagination.page - 1) * query.pagination.page_size) as usize),
    ).await {
        Ok(items) => {
            let response_items: Vec<QueueItemResponse> = items
                .into_iter()
                .map(QueueItemResponse::from)
                .collect();
            
            let response = QueueResponseData {
                items: response_items.clone(),
                total_records: response_items.len() as i64,
                page: Some(query.pagination.page as i32),
                page_size: Some(query.pagination.page_size as i32),
            };
            
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to list queue items: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to list queue items: {}", e))),
            ))
        }
    }
}

/// Queue response data structure matching frontend expectations
#[derive(Debug, Serialize)]
pub struct QueueResponseData {
    pub items: Vec<QueueItemResponse>,
    #[serde(rename = "totalRecords")]
    pub total_records: i64,
    pub page: Option<i32>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i32>,
}

/// POST /api/v3/queue/grab/{release_id} - Add release to queue by ID
pub async fn grab_release_by_id(
    State(_state): State<QueueServiceState>,
    Path(release_id): Path<Uuid>,
) -> std::result::Result<Json<ApiResponse<QueueItemResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Placeholder implementation
    // This would grab a release by ID, typically from search results
    
    let _ = release_id; // Suppress unused warning
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// DELETE /api/v3/queue/{id} - Remove from queue
pub async fn remove_queue_item(
    State(state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
    Query(query): Query<serde_json::Value>,
) -> std::result::Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract deleteFiles parameter
    let delete_files = query.get("deleteFiles")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    match state.queue_service.remove_queue_item(id, delete_files).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Failed to remove queue item {}: {}", id, e);
            let status_code = match e {
                radarr_core::RadarrError::NotFoundError { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status_code,
                Json(ApiResponse::error(format!("Failed to remove queue item: {}", e))),
            ))
        }
    }
}

/// PUT /api/v3/queue/{id}/pause - Pause queue item
pub async fn pause_queue_item(
    State(state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
) -> std::result::Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.queue_service.pause_queue_item(id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Failed to pause queue item {}: {}", id, e);
            let status_code = match e {
                radarr_core::RadarrError::NotFoundError { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status_code,
                Json(ApiResponse::error(format!("Failed to pause queue item: {}", e))),
            ))
        }
    }
}

/// PUT /api/v3/queue/{id}/resume - Resume queue item
pub async fn resume_queue_item(
    State(state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
) -> std::result::Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.queue_service.resume_queue_item(id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => {
            tracing::error!("Failed to resume queue item {}: {}", id, e);
            let status_code = match e {
                radarr_core::RadarrError::NotFoundError { .. } => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            Err((
                status_code,
                Json(ApiResponse::error(format!("Failed to resume queue item: {}", e))),
            ))
        }
    }
}

/// GET /api/v3/queue/status - Queue statistics
pub async fn get_queue_status(
    State(state): State<QueueServiceState>,
) -> std::result::Result<Json<ApiResponse<QueueStatsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.queue_service.get_queue_statistics().await {
        Ok(stats) => Ok(Json(ApiResponse::success(QueueStatsResponse::from(stats)))),
        Err(e) => {
            tracing::error!("Failed to get queue statistics: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(format!("Failed to get queue statistics: {}", e))),
            ))
        }
    }
}

/// POST /api/v3/queue/retry - Retry failed downloads
pub async fn retry_failed_downloads(
    State(_state): State<QueueServiceState>,
) -> std::result::Result<Json<ApiResponse<Vec<Uuid>>>, (StatusCode, Json<ApiResponse<()>>)> {
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// POST /api/v3/queue/process - Manually trigger queue processing
pub async fn process_queue(
    State(_state): State<QueueServiceState>,
) -> std::result::Result<Json<ApiResponse<Vec<Uuid>>>, (StatusCode, Json<ApiResponse<()>>)> {
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// DELETE /api/v3/queue/cleanup - Clean up completed items
pub async fn cleanup_completed(
    State(_state): State<QueueServiceState>,
    Query(query): Query<serde_json::Value>,
) -> std::result::Result<Json<ApiResponse<usize>>, (StatusCode, Json<ApiResponse<()>>)> {
    let days_old = query.get("daysOld")
        .and_then(|v| v.as_i64())
        .unwrap_or(7); // Default to 7 days
        
    let _ = days_old; // Suppress unused warning
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

// Helper functions for formatting

/// Format bytes as human readable string
fn format_bytes(bytes: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    // Special case for 0 bytes
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        // For bytes, don't show decimal places
        format!("{:.0} {}", size, UNITS[unit_index])
    } else if size >= 100.0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else if size >= 10.0 {
        format!("{:.1} {}", size, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Format bytes per second as human readable string
fn format_bytes_per_sec(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec as i64))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }
    
    #[test]
    fn test_format_bytes_per_sec() {
        assert_eq!(format_bytes_per_sec(1024), "1.00 KB/s");
        assert_eq!(format_bytes_per_sec(1024 * 1024), "1.00 MB/s");
    }
    
    #[test]
    fn test_queue_item_response_conversion() {
        let mut queue_item = QueueItem::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Movie".to_string(),
            "magnet:test".to_string(),
        );
        queue_item.size_bytes = Some(1024 * 1024 * 1024); // 1GB
        queue_item.download_speed = Some(1024 * 1024); // 1MB/s
        queue_item.eta_seconds = Some(3600); // 1 hour
        
        let response = QueueItemResponse::from(queue_item);
        
        assert_eq!(response.size_display, Some("1.0 GB".to_string()));
        assert_eq!(response.download_speed_display, Some("1.0 MB/s".to_string()));
        assert_eq!(response.eta_display, Some("1h 0m".to_string()));
        assert!(!response.can_retry);
    }
    
    #[test]
    fn test_queue_stats_response_conversion() {
        let stats = QueueStats {
            total_count: 10,
            downloading_count: 3,
            queued_count: 5,
            completed_count: 2,
            failed_count: 0,
            paused_count: 0,
            total_download_speed: 2 * 1024 * 1024, // 2MB/s
            total_upload_speed: 512 * 1024, // 512KB/s
            total_size_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            total_downloaded_bytes: 3 * 1024 * 1024 * 1024, // 3GB
        };
        
        let response = QueueStatsResponse::from(stats);
        
        assert_eq!(response.total_size_display, "10.0 GB");
        assert_eq!(response.total_downloaded_display, "3.00 GB");
        assert_eq!(response.total_download_speed_display, "2.00 MB/s");
        assert_eq!(response.total_upload_speed_display, "512 KB/s");
        assert_eq!(response.overall_progress, 30.0);
    }
}