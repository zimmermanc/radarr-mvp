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
use radarr_core::repositories::DownloadRepository;
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
#[instrument(skip(state))]
pub async fn list_downloads(
    State(state): State<DownloadAppState>,
    Query(params): Query<DownloadQueryParams>,
) -> ApiResult<Json<Vec<DownloadResponse>>> {
    info!("Listing downloads with filters: {:?}", params);
    
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    
    let downloads = if let Some(movie_id) = params.movie_id {
        // Filter by movie ID
        state.download_repo.find_by_movie_id(movie_id).await
            .map_err(|e| ApiError::InternalError { message: format!("Failed to fetch downloads by movie: {}", e) })?
    } else {
        // Get all downloads with pagination
        state.download_repo.list(offset, limit).await
            .map_err(|e| ApiError::InternalError { message: format!("Failed to fetch downloads: {}", e) })?
    };
    
    let responses: Vec<DownloadResponse> = downloads.into_iter()
        .map(DownloadResponse::from)
        .collect();
    
    info!("Retrieved {} downloads", responses.len());
    Ok(Json(responses))
}

// Moved delete_download logic to cancel_download below

/// POST /api/v3/download - Start a new download
#[instrument(skip(state))]
pub async fn start_download(
    State(state): State<DownloadAppState>,
    Json(request): Json<crate::models::DownloadRequest>,
) -> ApiResult<Json<DownloadResponse>> {
    info!("Starting download for GUID: {} from indexer: {}", request.guid, request.indexer_id);
    
    // Implement actual download logic:
    // 1. Look up release details from indexer would require indexer integration
    // 2. For MVP, create Download entity with available information
    // 3. In production, would integrate with qBittorrent client using radarr_downloaders::QBittorrentClient
    // 4. Save to database and return response
    
    // MVP implementation - creates download record for tracking
    // In production, this would:
    // - Query indexer for release details using request.indexer_id and request.guid
    // - Extract movie information and download URL from release
    // - Initialize qBittorrent client and add torrent
    // - Track download progress and status
    let download = radarr_core::models::Download::new(
        uuid::Uuid::new_v4(), // movie_id - would be extracted from release metadata
        1, // download_client_id - would be configurable qBittorrent client
        request.guid.clone(),
        format!("Download for GUID: {} (indexer: {})", request.guid, request.indexer_id),
    );
    
    // Save to database
    let saved_download = state.download_repo.create(&download).await
        .map_err(|e| ApiError::InternalError { message: format!("Failed to create download: {}", e) })?;
    
    info!("Successfully created download with ID: {}", saved_download.id);
    Ok(Json(DownloadResponse::from(saved_download)))
}

/// GET /api/v3/download/{id} - Get download details
#[instrument(skip(state), fields(download_id = %download_id))]
pub async fn get_download(
    State(state): State<DownloadAppState>,
    Path(download_id): Path<Uuid>,
) -> ApiResult<Json<DownloadResponse>> {
    info!("Getting download details for ID: {}", download_id);
    
    let download = state.download_repo.find_by_id(download_id).await
        .map_err(|e| ApiError::InternalError { message: format!("Failed to fetch download: {}", e) })?
        .ok_or_else(|| ApiError::NotFound { resource: format!("Download with ID: {}", download_id) })?;
    
    Ok(Json(DownloadResponse::from(download)))
}

/// Cancel download handler using proper parameter name
#[instrument(skip(state), fields(download_id = %download_id))]
pub async fn cancel_download(
    State(state): State<DownloadAppState>,
    Path(download_id): Path<Uuid>,
    Query(_params): Query<HashMap<String, String>>,
) -> ApiResult<StatusCode> {
    info!("Cancelling download: {}", download_id);
    
    // Find the download first
    let download = state.download_repo.find_by_id(download_id).await
        .map_err(|e| ApiError::InternalError { message: format!("Failed to fetch download: {}", e) })?
        .ok_or_else(|| ApiError::NotFound { resource: format!("Download with ID: {}", download_id) })?;
    
    // Implement actual cancellation:
    // In production, this would:
    // 1. Initialize qBittorrent client with configuration
    // 2. Cancel/delete torrent from qBittorrent using torrent hash
    // 3. Update download status to cancelled in database
    // 4. Clean up any temporary files
    
    // MVP implementation - removes download record
    // Production would use:
    // let qb_client = radarr_downloaders::QBittorrentClient::new(qb_config)?;
    // qb_client.delete_torrent(&download.torrent_hash, true).await?;
    // download_repo.update_status(download_id, DownloadStatus::Cancelled).await?;
    
    state.download_repo.delete(download_id).await
        .map_err(|e| ApiError::InternalError { message: format!("Failed to cancel download: {}", e) })?;
    
    info!("Successfully cancelled download: {}", download_id);
    Ok(StatusCode::NO_CONTENT)
}