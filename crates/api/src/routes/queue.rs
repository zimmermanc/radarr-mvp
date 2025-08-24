//! Queue management API routes

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::queue;

/// Create queue-related routes
pub fn create_queue_routes() -> Router {
    Router::new()
        .route("/queue", get(queue::list_queue))
        .route("/queue/grab", post(queue::grab_release))
        .route("/queue/grab/:release_id", post(queue::grab_release_by_id))
        .route("/queue/:id", delete(queue::remove_queue_item))
        .route("/queue/:id/pause", put(queue::pause_queue_item))
        .route("/queue/:id/resume", put(queue::resume_queue_item))
        .route("/queue/:id/priority", put(queue::update_queue_priority))
        .route("/queue/status", get(queue::get_queue_status))
        .route("/queue/retry", post(queue::retry_failed_downloads))
        .route("/queue/process", post(queue::process_queue))
        .route("/queue/cleanup", delete(queue::cleanup_completed))
}

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

/// Queue item response with additional metadata
#[derive(Debug, Serialize)]
pub struct QueueItemResponse {
    #[serde(flatten)]
    pub queue_item: QueueItem,
    /// Human readable size
    pub size_display: Option<String>,
    /// Human readable download speed
    pub download_speed_display: Option<String>,
    /// Human readable ETA
    pub eta_display: Option<String>,
    /// Can this item be retried
    pub can_retry: bool,
}

impl From<QueueItem> for QueueItemResponse {
    fn from(queue_item: QueueItem) -> Self {
        Self {
            size_display: queue_item.human_readable_size(),
            download_speed_display: queue_item.human_readable_download_speed(),
            eta_display: queue_item.human_readable_eta(),
            can_retry: queue_item.can_retry(),
            queue_item,
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

/// Queue service state
#[derive(Clone)]
pub struct QueueServiceState {
    // This would be injected as a dependency
    // For now, we'll define it as a trait object
    // In practice, this would be the concrete implementation
}

// Placeholder for the actual service implementation
// This would be injected via dependency injection
pub async fn grab_release(
    State(_state): State<QueueServiceState>,
    ValidatedJson(request): ValidatedJson<GrabReleaseRequest>,
) -> Result<Json<ApiResponse<QueueItemResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // This is a placeholder implementation
    // In the real implementation, this would:
    // 1. Validate the release exists
    // 2. Get the movie details
    // 3. Create a Release object from the request
    // 4. Call queue_service.grab_release()
    // 5. Return the created queue item
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// GET /api/v3/queue - List queue items
pub async fn list_queue(
    State(_state): State<QueueServiceState>,
    Query(query): Query<QueueQuery>,
) -> Result<Json<ApiResponse<Vec<QueueItemResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Placeholder implementation
    // In the real implementation, this would:
    // 1. Call queue_service.get_queue_items() with filters
    // 2. Convert to response format
    // 3. Return paginated results
    
    let _ = query; // Suppress unused warning
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// POST /api/v3/queue/grab/{release_id} - Add release to queue
pub async fn grab_release_by_id(
    State(_state): State<QueueServiceState>,
    Path(release_id): Path<Uuid>,
) -> Result<Json<ApiResponse<QueueItemResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
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
    State(_state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract deleteFiles parameter
    let delete_files = query.get("deleteFiles")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
        
    let _ = (id, delete_files); // Suppress unused warning
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// PUT /api/v3/queue/{id}/pause - Pause queue item
pub async fn pause_queue_item(
    State(_state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let _ = id; // Suppress unused warning
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// PUT /api/v3/queue/{id}/resume - Resume queue item
pub async fn resume_queue_item(
    State(_state): State<QueueServiceState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let _ = id; // Suppress unused warning
    
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// GET /api/v3/queue/status - Queue statistics
pub async fn get_queue_status(
    State(_state): State<QueueServiceState>,
) -> Result<Json<ApiResponse<QueueStatsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// POST /api/v3/queue/retry - Retry failed downloads
pub async fn retry_failed_downloads(
    State(_state): State<QueueServiceState>,
) -> Result<Json<ApiResponse<Vec<Uuid>>>, (StatusCode, Json<ApiResponse<()>>)> {
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// POST /api/v3/queue/process - Manually trigger queue processing
pub async fn process_queue(
    State(_state): State<QueueServiceState>,
) -> Result<Json<ApiResponse<Vec<Uuid>>>, (StatusCode, Json<ApiResponse<()>>)> {
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error("Queue service not yet wired".to_string())),
    ))
}

/// DELETE /api/v3/queue/cleanup - Clean up completed items
pub async fn cleanup_completed(
    State(_state): State<QueueServiceState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<usize>>, (StatusCode, Json<ApiResponse<()>>)> {
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
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if size >= 100.0 {
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