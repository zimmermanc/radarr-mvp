use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, debug, warn};
use radarr_core::{models::Movie, domain::repositories::MovieRepository};
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
    debug!("Fetching movies from database with params: {:?}", params);
    
    // Handle search query if provided
    let movies = if let Some(search_term) = &params.search {
        debug!("Searching movies by title: {}", search_term);
        let limit = params.limit.unwrap_or(100).min(1000) as i32; // Cap at 1000
        match services.movie_repository.search_by_title(search_term, limit).await {
            Ok(movies) => movies,
            Err(e) => {
                error!("Failed to search movies by title: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        // Handle pagination
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(50).min(1000) as i32; // Default 50, cap at 1000
        let offset = ((page - 1) * limit as u32) as i64;
        
        debug!("Fetching movies with pagination: page={}, limit={}, offset={}", page, limit, offset);
        
        // Apply filtering based on params
        let movies = if params.monitored == Some(true) {
            debug!("Fetching only monitored movies");
            match services.movie_repository.find_monitored().await {
                Ok(movies) => {
                    // Apply manual pagination to monitored movies
                    movies.into_iter()
                        .skip(offset as usize)
                        .take(limit as usize)
                        .collect()
                },
                Err(e) => {
                    error!("Failed to fetch monitored movies: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        } else if params.has_file == Some(false) {
            debug!("Fetching movies without files");
            match services.movie_repository.find_missing_files().await {
                Ok(movies) => {
                    // Apply manual pagination to movies without files
                    movies.into_iter()
                        .skip(offset as usize)
                        .take(limit as usize)
                        .collect()
                },
                Err(e) => {
                    error!("Failed to fetch movies without files: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        } else {
            // Default: list all movies with pagination
            match services.movie_repository.list(offset, limit).await {
                Ok(movies) => movies,
                Err(e) => {
                    error!("Failed to fetch movies from database: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
    };
    
    // Apply additional filtering if needed
    let filtered_movies: Vec<Movie> = movies.into_iter().filter(|movie| {
        // Filter by status if provided
        if let Some(ref status) = params.status {
            if movie.status.to_string() != *status {
                return false;
            }
        }
        
        // Filter by quality profile if provided
        if let Some(quality_profile_id) = params.quality_profile_id {
            if movie.quality_profile_id != Some(quality_profile_id) {
                return false;
            }
        }
        
        true
    }).collect();
    
    // Convert domain models to API response models
    let response_movies: Vec<MovieResponse> = filtered_movies.into_iter()
        .map(convert_movie_to_response)
        .collect();
    
    debug!("Returning {} movies", response_movies.len());
    Ok(Json(response_movies))
}

/// GET /api/v3/movie/{id} - Get single movie
pub async fn get_movie(
    Extension(services): Extension<Arc<AppServices>>,
    Path(id): Path<i32>,
) -> Result<Json<MovieResponse>, StatusCode> {
    debug!("Fetching movie with ID: {}", id);
    
    // Since the API uses integer IDs but our domain uses UUIDs,
    // we need to search by tmdb_id which is the integer ID
    match services.movie_repository.find_by_tmdb_id(id).await {
        Ok(Some(movie)) => {
            debug!("Found movie: {}", movie.title);
            Ok(Json(convert_movie_to_response(movie)))
        },
        Ok(None) => {
            debug!("Movie with ID {} not found", id);
            Err(StatusCode::NOT_FOUND)
        },
        Err(e) => {
            error!("Failed to fetch movie with ID {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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

/// Convert a domain Movie model to an API MovieResponse
fn convert_movie_to_response(movie: Movie) -> MovieResponse {
    // Extract additional metadata from the movie's metadata field
    let overview = movie.overview().map(|s| s.to_string());
    let poster_url = movie.metadata.get("tmdb")
        .and_then(|tmdb| tmdb.get("poster_path"))
        .and_then(|path| path.as_str())
        .map(|path| {
            if path.starts_with("/") {
                format!("https://image.tmdb.org/t/p/w500{}", path)
            } else {
                path.to_string()
            }
        });
    
    let backdrop_url = movie.metadata.get("tmdb")
        .and_then(|tmdb| tmdb.get("backdrop_path"))
        .and_then(|path| path.as_str())
        .map(|path| {
            if path.starts_with("/") {
                format!("https://image.tmdb.org/t/p/w1280{}", path)
            } else {
                path.to_string()
            }
        });
    
    let genres = movie.metadata.get("tmdb")
        .and_then(|tmdb| tmdb.get("genres"))
        .and_then(|genres| genres.as_array())
        .map(|genres| {
            genres.iter()
                .filter_map(|genre| genre.get("name"))
                .filter_map(|name| name.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    
    let rating = movie.rating().unwrap_or(0.0);
    let ratings = json!({
        "tmdb": {
            "value": rating,
            "votes": movie.metadata.get("tmdb")
                .and_then(|tmdb| tmdb.get("vote_count"))
                .and_then(|count| count.as_i64())
                .unwrap_or(0)
        }
    });
    
    let release_date = movie.metadata.get("tmdb")
        .and_then(|tmdb| tmdb.get("release_date"))
        .and_then(|date| date.as_str())
        .map(|s| s.to_string());
    
    // Generate sort title (lowercase, remove articles)
    let sort_title = generate_sort_title(&movie.title);
    
    // Use tmdb_id as the API id since the web UI expects integer IDs
    MovieResponse {
        id: movie.tmdb_id,
        title: movie.title,
        original_title: movie.original_title,
        sort_title,
        status: movie.status.to_string(),
        overview,
        release_date,
        year: movie.year,
        runtime: movie.runtime.unwrap_or(0),
        tmdb_id: Some(movie.tmdb_id),
        imdb_id: movie.imdb_id,
        path: format!("/movies/{}", movie.title.replace(" ", ".")), // Generate a default path
        monitored: movie.monitored,
        quality_profile_id: movie.quality_profile_id.unwrap_or(1), // Default quality profile
        has_file: movie.has_file,
        size_on_disk: 0, // TODO: Calculate actual size when file tracking is implemented
        added: movie.created_at.to_rfc3339(),
        updated: movie.updated_at.to_rfc3339(),
        poster_url,
        backdrop_url,
        trailer_url: None, // TODO: Add trailer URL support
        genres,
        ratings,
    }
}

/// Generate a sort title by removing common articles and converting to lowercase
fn generate_sort_title(title: &str) -> String {
    let lower = title.to_lowercase();
    
    // Remove common articles from the beginning
    if lower.starts_with("the ") {
        lower[4..].to_string()
    } else if lower.starts_with("a ") {
        lower[2..].to_string()
    } else if lower.starts_with("an ") {
        lower[3..].to_string()
    } else {
        lower
    }
}