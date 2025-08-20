//! Simplified API implementation for MVP
//! 
//! This provides basic REST API endpoints without complex state management
//! for demonstration purposes.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use radarr_core::{Movie, MovieStatus, MinimumAvailability};
use radarr_infrastructure::DatabasePool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
use chrono;

/// Simple application state for MVP
#[derive(Clone)]
pub struct SimpleApiState {
    pub database_pool: DatabasePool,
}

impl SimpleApiState {
    pub fn new(database_pool: DatabasePool) -> Self {
        Self { database_pool }
    }
}

/// Simple movie response for MVP
#[derive(Debug, Serialize)]
pub struct SimpleMovieResponse {
    pub id: Uuid,
    pub tmdb_id: i32,
    pub title: String,
    pub year: Option<i32>,
    pub status: MovieStatus,
    pub monitored: bool,
    pub created_at: String,
}

impl From<Movie> for SimpleMovieResponse {
    fn from(movie: Movie) -> Self {
        Self {
            id: movie.id,
            tmdb_id: movie.tmdb_id,
            title: movie.title,
            year: movie.year,
            status: movie.status,
            monitored: movie.monitored,
            created_at: movie.created_at.to_rfc3339(),
        }
    }
}

/// Simple movie creation request
#[derive(Debug, Deserialize)]
pub struct SimpleCreateMovieRequest {
    pub tmdb_id: i32,
    pub title: String,
    #[serde(default)]
    pub monitored: bool,
}

/// Simple query parameters
#[derive(Debug, Deserialize)]
pub struct SimpleQueryParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 { 1 }
fn default_limit() -> u32 { 50 }

/// Create the simplified API router
pub fn create_simple_api_router(state: SimpleApiState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(health_check))
        
        // Movie endpoints
        .route("/api/v3/movie", get(list_movies))
        .route("/api/v3/movie", post(create_movie))
        .route("/api/v3/movie/:id", get(get_movie))
        .route("/api/v3/movie/:id", delete(delete_movie))
        
        // Search endpoint (mock)
        .route("/api/v3/indexer/search", post(search_movies))
        
        // Download endpoint (mock)
        .route("/api/v3/download", post(start_download))
        
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "radarr-api",
        "version": "1.0.0"
    }))
}

/// List movies endpoint
async fn list_movies(
    State(_state): State<SimpleApiState>,
    Query(params): Query<SimpleQueryParams>,
) -> Json<Value> {
    // For MVP, return mock data
    let movies = vec![
        serde_json::json!({
            "id": Uuid::new_v4(),
            "tmdbId": 603,
            "title": "The Matrix",
            "year": 1999,
            "status": "released",
            "monitored": true,
            "createdAt": "2024-01-01T00:00:00Z"
        }),
        serde_json::json!({
            "id": Uuid::new_v4(),
            "tmdbId": 13,
            "title": "Forrest Gump", 
            "year": 1994,
            "status": "released",
            "monitored": true,
            "createdAt": "2024-01-01T00:00:00Z"
        })
    ];
    
    Json(serde_json::json!({
        "page": params.page,
        "pageSize": params.limit,
        "totalCount": movies.len(),
        "records": movies
    }))
}

/// Get movie by ID endpoint
async fn get_movie(
    State(_state): State<SimpleApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // For MVP, return mock data
    Ok(Json(serde_json::json!({
        "id": id,
        "tmdbId": 603,
        "title": "The Matrix",
        "year": 1999,
        "status": "released",
        "monitored": true,
        "createdAt": "2024-01-01T00:00:00Z"
    })))
}

/// Create movie endpoint
async fn create_movie(
    State(_state): State<SimpleApiState>,
    Json(request): Json<SimpleCreateMovieRequest>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    // For MVP, return mock created movie
    let movie_id = Uuid::new_v4();
    
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": movie_id,
        "tmdbId": request.tmdb_id,
        "title": request.title,
        "status": "announced",
        "monitored": request.monitored,
        "createdAt": chrono::Utc::now().to_rfc3339()
    }))))
}

/// Delete movie endpoint
async fn delete_movie(
    State(_state): State<SimpleApiState>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    // For MVP, always return success
    StatusCode::NO_CONTENT
}

/// Search movies endpoint (mock)
async fn search_movies(
    State(_state): State<SimpleApiState>,
    Json(request): Json<Value>,
) -> Json<Value> {
    // For MVP, return mock search results
    Json(serde_json::json!({
        "total": 2,
        "releases": [
            {
                "title": "The.Matrix.1999.1080p.BluRay.x264-GROUP",
                "downloadUrl": "magnet:?xt=urn:btih:example1",
                "indexer": "Example Indexer",
                "size": 8000000000i64,
                "seeders": 50,
                "qualityScore": 85
            },
            {
                "title": "The.Matrix.1999.720p.WEB-DL.x264-GROUP",
                "downloadUrl": "magnet:?xt=urn:btih:example2", 
                "indexer": "Example Indexer",
                "size": 4000000000i64,
                "seeders": 25,
                "qualityScore": 70
            }
        ],
        "indexersSearched": 1,
        "executionTimeMs": 250
    }))
}

/// Start download endpoint (mock)
async fn start_download(
    State(_state): State<SimpleApiState>,
    Json(request): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    // For MVP, return mock download started
    let download_id = Uuid::new_v4();
    
    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": download_id,
        "status": "queued",
        "progress": 0,
        "createdAt": chrono::Utc::now().to_rfc3339()
    }))))
}