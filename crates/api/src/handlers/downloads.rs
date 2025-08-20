//! Download API handlers

use crate::{
    error::{ApiError, ApiResult},
    models::DownloadResponse,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use radarr_core::{
    models::Download,
    repositories::DownloadRepository,
};
use radarr_infrastructure::{DatabasePool, repositories::PostgresDownloadRepository};
use serde::Deserialize;
use std::{sync::Arc, collections::HashMap};
use tracing::{info, instrument};
use uuid::Uuid;

/// Download query parameters for filtering and pagination
#[derive(Debug, Deserialize)]
pub struct DownloadQueryParams {
    /// Filter by status
    pub status: Option<String>,
    /// Filter by movie ID
    pub movie_id: Option<Uuid>,
    /// Number of items to return (max 100)
    pub limit: Option<i32>,
    /// Number of items to skip
    pub offset: Option<i64>,
}

/// Download application state containing repositories and external clients
#[derive(Clone)]
pub struct DownloadAppState {
    pub database_pool: DatabasePool,
    pub download_repo: Arc<PostgresDownloadRepository>,
}

impl DownloadAppState {
    pub fn new(database_pool: DatabasePool) -> Self {
        let download_repo = Arc::new(PostgresDownloadRepository::new(database_pool.clone()));
        Self {
            database_pool,
            download_repo,
        }
    }
}

/// GET /api/v3/download - Get all downloads with optional filtering
#[instrument(skip(_state))]
pub async fn list_downloads(
    State(_state): State<DownloadAppState>,
    Query(_params): Query<DownloadQueryParams>,
) -> ApiResult<Json<Vec<DownloadResponse>>> {
    info!("Listing downloads - mock implementation");
    
    // Mock implementation for tests
    Ok(Json(vec![]))
}

/// DELETE /api/v3/download/{id} - Delete download
#[instrument(skip(_state), fields(download_id = %download_id))]
pub async fn delete_download(
    State(_state): State<DownloadAppState>,
    Path(download_id): Path<Uuid>,
    Query(_params): Query<HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    info!("Deleting download: {} - mock implementation", download_id);
    
    // Mock implementation for tests
    Ok(StatusCode::NO_CONTENT)
}