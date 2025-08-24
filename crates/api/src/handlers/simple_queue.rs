//! Simplified queue management handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::ApiResponse;
use radarr_infrastructure::DatabasePool;

/// Simple queue state
#[derive(Clone)]
pub struct SimpleQueueState {
    pub pool: DatabasePool,
}

impl SimpleQueueState {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
}

/// Simple queue item response
#[derive(Debug, Serialize)]
pub struct SimpleQueueItem {
    pub id: String,
    #[serde(rename = "movieId")]
    pub movie_id: i32,
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
}

/// Simple queue response
#[derive(Debug, Serialize)]
pub struct SimpleQueueResponse {
    pub page: i32,
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    #[serde(rename = "totalRecords")]
    pub total_records: i64,
    pub items: Vec<SimpleQueueItem>,
}

/// Query parameters for queue listing
#[derive(Debug, Deserialize)]
pub struct QueueQueryParams {
    #[serde(default = "default_page")]
    pub page: i32,
    #[serde(default = "default_page_size")]
    #[serde(rename = "pageSize")]
    pub page_size: i32,
}

fn default_page() -> i32 { 1 }
fn default_page_size() -> i32 { 50 }

/// GET /api/v3/queue - List queue items
pub async fn list_queue_simple(
    State(_state): State<SimpleQueueState>,
    Query(_query): Query<QueueQueryParams>,
) -> Json<ApiResponse<SimpleQueueResponse>> {
    // Return empty queue for now
    let response = SimpleQueueResponse {
        page: 1,
        page_size: 50,
        total_records: 0,
        items: vec![],
    };

    Json(ApiResponse::success(response))
}

/// PUT /api/v3/queue/{id}/pause - Pause queue item
pub async fn pause_queue_item_simple(
    State(_state): State<SimpleQueueState>,
    Path(_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Mock implementation - return success
    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/v3/queue/{id}/resume - Resume queue item
pub async fn resume_queue_item_simple(
    State(_state): State<SimpleQueueState>,
    Path(_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Mock implementation - return success
    Ok(Json(ApiResponse::success(())))
}

/// DELETE /api/v3/queue/{id} - Remove queue item
pub async fn remove_queue_item_simple(
    State(_state): State<SimpleQueueState>,
    Path(_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Mock implementation - return success
    Ok(Json(ApiResponse::success(())))
}

/// PUT /api/v3/queue/{id}/priority - Update queue item priority
pub async fn update_queue_priority_simple(
    State(_state): State<SimpleQueueState>,
    Path(_id): Path<String>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Mock implementation - return success
    Ok(Json(ApiResponse::success(())))
}