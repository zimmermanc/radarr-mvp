use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::services::AppServices;

#[derive(Deserialize)]
pub struct MovieListQuery {
    #[serde(default)]
    sort_by: Option<String>,
    #[serde(default)]
    sort_direction: Option<String>,
    #[serde(default)]
    page: Option<u32>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    monitored: Option<bool>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    has_file: Option<bool>,
    #[serde(default)]
    quality_profile_id: Option<i32>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct MovieResponse {
    id: i32,
    title: String,
    original_title: Option<String>,
    sort_title: String,
    status: String,
    overview: Option<String>,
    release_date: Option<String>,
    year: Option<i32>,
    runtime: i32,
    tmdb_id: Option<i32>,
    imdb_id: Option<String>,
    path: String,
    monitored: bool,
    quality_profile_id: i32,
    has_file: bool,
    size_on_disk: i64,
    added: String,
    updated: String,
    poster_url: Option<String>,
    backdrop_url: Option<String>,
    trailer_url: Option<String>,
    genres: Vec<String>,
    ratings: Value,
}

/// GET /api/v3/movie - List all movies with filtering
pub async fn list_movies(
    Extension(services): Extension<Arc<AppServices>>,
    Query(params): Query<MovieListQuery>,
) -> Result<Json<Vec<MovieResponse>>, StatusCode> {
    // For now, return empty array to satisfy the web UI
    // TODO: Implement actual database query
    Ok(Json(vec![]))
}

/// GET /api/v3/movie/{id} - Get single movie
pub async fn get_movie(
    Extension(services): Extension<Arc<AppServices>>,
    Path(id): Path<i32>,
) -> Result<Json<MovieResponse>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

/// POST /api/v3/movie - Add new movie
pub async fn add_movie(
    Extension(services): Extension<Arc<AppServices>>,
    Json(payload): Json<Value>,
) -> Result<Json<MovieResponse>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// PUT /api/v3/movie/{id} - Update movie
pub async fn update_movie(
    Extension(services): Extension<Arc<AppServices>>,
    Path(id): Path<i32>,
    Json(payload): Json<Value>,
) -> Result<Json<MovieResponse>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// DELETE /api/v3/movie/{id} - Delete movie
pub async fn delete_movie(
    Extension(services): Extension<Arc<AppServices>>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// GET /api/v3/movie/lookup - Search for movies
pub async fn lookup_movies(
    Extension(services): Extension<Arc<AppServices>>,
    Query(params): Query<Value>,
) -> Result<Json<Vec<Value>>, StatusCode> {
    Ok(Json(vec![]))
}

/// GET /api/v3/qualityprofile - List quality profiles
pub async fn list_quality_profiles(
    Extension(services): Extension<Arc<AppServices>>,
) -> Result<Json<Vec<Value>>, StatusCode> {
    // Return default quality profile for now
    Ok(Json(vec![
        json!({
            "id": 1,
            "name": "HD-1080p",
            "cutoff": 7,
            "items": [],
            "minFormatScore": 0,
            "cutoffFormatScore": 0,
            "formatItems": [],
            "language": {"id": 1, "name": "English"},
            "upgradeAllowed": true
        })
    ]))
}

/// GET /api/v3/qualityprofile/{id} - Get single quality profile
pub async fn get_quality_profile(
    Extension(services): Extension<Arc<AppServices>>,
    Path(id): Path<i32>,
) -> Result<Json<Value>, StatusCode> {
    if id == 1 {
        Ok(Json(json!({
            "id": 1,
            "name": "HD-1080p",
            "cutoff": 7,
            "items": [],
            "minFormatScore": 0,
            "cutoffFormatScore": 0,
            "formatItems": [],
            "language": {"id": 1, "name": "English"},
            "upgradeAllowed": true
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}