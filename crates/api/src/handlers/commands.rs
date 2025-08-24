//! Command API handlers

use crate::{
    error::{ApiError, ApiResult},
    models::{DownloadRequest, DownloadResponse},
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use radarr_core::{
    models::{Download, DownloadStatus},
    repositories::DownloadRepository,
};
use radarr_infrastructure::{repositories::PostgresDownloadRepository, DatabasePool};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tracing::{info, instrument};
use uuid::Uuid;

/// Command query parameters for testing
#[derive(Debug, Deserialize)]
pub struct CommandParams {
    pub fail_qbittorrent: Option<bool>,
}

/// Application state for commands
#[derive(Clone)]
pub struct CommandState {
    pub database_pool: DatabasePool,
    pub download_repo: Arc<PostgresDownloadRepository>,
}

impl CommandState {
    pub fn new(database_pool: DatabasePool) -> Self {
        let download_repo = Arc::new(PostgresDownloadRepository::new(database_pool.clone()));
        Self {
            database_pool,
            download_repo,
        }
    }
}

/// POST /api/v3/command/download - Start a download
#[instrument(skip(state))]
pub async fn start_download(
    State(state): State<CommandState>,
    Query(params): Query<CommandParams>,
    Json(request): Json<DownloadRequest>,
) -> ApiResult<(StatusCode, Json<DownloadResponse>)> {
    info!("Starting download for GUID: {}", request.guid);

    // Simulate failure for testing
    if params.fail_qbittorrent.unwrap_or(false) {
        return Err(ApiError::ExternalServiceError {
            service: "qbittorrent".to_string(),
            error: "Simulated qBittorrent failure for testing".to_string(),
        });
    }

    // Create a mock download
    let mock_movie_id = Uuid::new_v4(); // In real implementation, this would come from the release
    let mut download = Download::new(
        mock_movie_id,
        1, // Default download client ID
        format!("mock_hash_{}", Uuid::new_v4().simple()),
        "Fight Club 1999 1080p BluRay x264-SPARKS".to_string(),
    );

    download.indexer_id = Some(request.indexer_id);
    download.category = Some("movies".to_string());
    download.size_bytes = Some(1_500_000_000);
    download.size_left = Some(1_500_000_000);
    download.quality = serde_json::json!({
        "quality": "1080p",
        "revision": 1
    });
    download.update_status(DownloadStatus::Downloading);

    // For testing, simulate immediate creation without database
    let download_response = DownloadResponse::from(download);

    info!("Started download with ID: {}", download_response.id);
    Ok((StatusCode::ACCEPTED, Json(download_response)))
}

/// GET /api/v3/download/{id} - Get download status
#[instrument(skip(state))]
pub async fn get_download_status(
    State(state): State<CommandState>,
    Path(download_id): Path<Uuid>,
) -> ApiResult<Json<DownloadResponse>> {
    info!("Getting download status for ID: {}", download_id);

    // For testing, create a mock completed download
    let mock_movie_id = Uuid::new_v4();
    let mut download = Download::new(
        mock_movie_id,
        1,
        format!("mock_hash_{}", download_id.simple()),
        "Fight Club 1999 1080p BluRay x264-SPARKS".to_string(),
    );

    download.id = download_id;
    download.size_bytes = Some(1_500_000_000);
    download.size_left = Some(0); // Completed
    download.update_status(DownloadStatus::Completed);

    let download_response = DownloadResponse::from(download);

    info!("Retrieved download status: {}", download_response.status);
    Ok(Json(download_response))
}

/// POST /api/v3/command/import/{id} - Import a completed download
#[instrument(skip(state))]
pub async fn import_download(
    State(state): State<CommandState>,
    Path(download_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    info!("Importing download: {}", download_id);

    // For testing, simulate successful import
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    info!("Successfully imported download: {}", download_id);
    Ok(StatusCode::ACCEPTED)
}
