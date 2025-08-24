use crate::services::AppServices;
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::Json,
};
use radarr_core::{domain::repositories::MovieRepository, models::Movie};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, warn};

#[derive(Deserialize, Debug)]
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
        match services
            .movie_repository
            .search_by_title(search_term, limit)
            .await
        {
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

        debug!(
            "Fetching movies with pagination: page={}, limit={}, offset={}",
            page, limit, offset
        );

        // Apply filtering based on params
        if params.monitored == Some(true) {
            debug!("Fetching only monitored movies");
            match services.movie_repository.find_monitored().await {
                Ok(movies) => {
                    // Apply manual pagination to monitored movies
                    movies
                        .into_iter()
                        .skip(offset as usize)
                        .take(limit as usize)
                        .collect()
                }
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
                    movies
                        .into_iter()
                        .skip(offset as usize)
                        .take(limit as usize)
                        .collect()
                }
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
    let filtered_movies: Vec<Movie> = movies
        .into_iter()
        .filter(|movie| {
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
        })
        .collect();

    // Convert domain models to API response models
    let response_movies: Vec<MovieResponse> = filtered_movies
        .into_iter()
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
        }
        Ok(None) => {
            debug!("Movie with ID {} not found", id);
            Err(StatusCode::NOT_FOUND)
        }
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
    debug!("Updating movie with ID: {} with payload: {:?}", id, payload);

    // Find the movie first
    let mut movie = match services.movie_repository.find_by_tmdb_id(id).await {
        Ok(Some(movie)) => movie,
        Ok(None) => {
            debug!("Movie with ID {} not found", id);
            return Err(StatusCode::NOT_FOUND);
        }
        Err(e) => {
            error!("Failed to fetch movie with ID {}: {}", id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Apply updates from payload
    if let Some(monitored) = payload.get("monitored").and_then(|v| v.as_bool()) {
        movie.monitored = monitored;
        debug!("Updated movie monitored status to: {}", monitored);
    }

    if let Some(quality_profile_id) = payload
        .get("quality_profile_id")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
    {
        movie.quality_profile_id = Some(quality_profile_id);
        debug!("Updated movie quality profile to: {}", quality_profile_id);
    }

    // Update the movie in repository
    match services.movie_repository.update(&movie).await {
        Ok(updated_movie) => {
            debug!("Successfully updated movie: {}", updated_movie.title);
            Ok(Json(convert_movie_to_response(updated_movie)))
        }
        Err(e) => {
            error!("Failed to update movie: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
    Ok(Json(vec![json!({
        "id": 1,
        "name": "HD-1080p",
        "cutoff": 7,
        "items": [],
        "minFormatScore": 0,
        "cutoffFormatScore": 0,
        "formatItems": [],
        "language": {"id": 1, "name": "English"},
        "upgradeAllowed": true
    })]))
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
    let poster_url = movie
        .metadata
        .get("tmdb")
        .and_then(|tmdb| tmdb.get("poster_path"))
        .and_then(|path| path.as_str())
        .map(|path| {
            if path.starts_with("/") {
                format!("https://image.tmdb.org/t/p/w500{}", path)
            } else {
                path.to_string()
            }
        });

    let backdrop_url = movie
        .metadata
        .get("tmdb")
        .and_then(|tmdb| tmdb.get("backdrop_path"))
        .and_then(|path| path.as_str())
        .map(|path| {
            if path.starts_with("/") {
                format!("https://image.tmdb.org/t/p/w1280{}", path)
            } else {
                path.to_string()
            }
        });

    let genres = movie
        .metadata
        .get("tmdb")
        .and_then(|tmdb| tmdb.get("genres"))
        .and_then(|genres| genres.as_array())
        .map(|genres| {
            genres
                .iter()
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

    let release_date = movie
        .metadata
        .get("tmdb")
        .and_then(|tmdb| tmdb.get("release_date"))
        .and_then(|date| date.as_str())
        .map(|s| s.to_string());

    // Generate sort title (lowercase, remove articles)
    let sort_title = generate_sort_title(&movie.title);

    // Use tmdb_id as the API id since the web UI expects integer IDs
    MovieResponse {
        id: movie.tmdb_id,
        title: movie.title.clone(),
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

// Additional movie action endpoints

#[derive(Deserialize, Debug)]
pub struct SearchReleasesQuery {
    indexers: Option<Vec<String>>,
}

#[derive(Serialize, Debug)]
pub struct SearchReleaseResponse {
    pub id: String,
    pub title: String,
    pub indexer: String,
    #[serde(rename = "indexerId")]
    pub indexer_id: String,
    pub size: i64,
    pub quality: String,
    pub resolution: String,
    pub source: String,
    pub codec: String,
    pub seeders: i32,
    pub leechers: i32,
    #[serde(rename = "sceneGroup")]
    pub scene_group: Option<String>,
    #[serde(rename = "releaseGroup")]
    pub release_group: Option<String>,
    pub languages: Vec<String>,
    #[serde(rename = "publishDate")]
    pub publish_date: String,
    pub score: Option<i32>,
    #[serde(rename = "matchType")]
    pub match_type: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
}

/// GET /api/v3/movies/{id}/search - Search releases for a movie
pub async fn search_movie_releases(
    Extension(_services): Extension<Arc<AppServices>>,
    Path(id): Path<i32>,
    Query(_params): Query<SearchReleasesQuery>,
) -> Result<Json<Vec<SearchReleaseResponse>>, StatusCode> {
    debug!("Searching releases for movie ID: {}", id);

    // For MVP, return mock search results
    // In production, this would:
    // 1. Look up movie details
    // 2. Query indexers (Prowlarr, HDBits, etc.)
    // 3. Parse and score releases
    // 4. Return sorted results

    let mock_releases = vec![
        SearchReleaseResponse {
            id: format!("rel_{}", uuid::Uuid::new_v4().to_string()),
            title: format!("Movie.{}.1080p.BluRay.x264-SPARKS", id),
            indexer: "HDBits".to_string(),
            indexer_id: "hdb_12345".to_string(),
            size: 10737418240, // 10GB
            quality: "1080p BluRay".to_string(),
            resolution: "1080p".to_string(),
            source: "BluRay".to_string(),
            codec: "x264".to_string(),
            seeders: 45,
            leechers: 2,
            scene_group: Some("SPARKS".to_string()),
            release_group: Some("SPARKS".to_string()),
            languages: vec!["English".to_string()],
            publish_date: chrono::Utc::now().to_rfc3339(),
            score: Some(95),
            match_type: "exact".to_string(),
            download_url: Some(format!("magnet:?xt=urn:btih:movie{}", id)),
        },
        SearchReleaseResponse {
            id: format!("rel_{}", uuid::Uuid::new_v4().to_string()),
            title: format!("Movie.{}.2160p.WEB-DL.DDP5.1.Atmos.H.265-FLUX", id),
            indexer: "HDBits".to_string(),
            indexer_id: "hdb_12346".to_string(),
            size: 21474836480, // 20GB
            quality: "2160p WEB-DL".to_string(),
            resolution: "2160p".to_string(),
            source: "WEB-DL".to_string(),
            codec: "H.265".to_string(),
            seeders: 32,
            leechers: 5,
            scene_group: Some("FLUX".to_string()),
            release_group: Some("FLUX".to_string()),
            languages: vec!["English".to_string()],
            publish_date: chrono::Utc::now().to_rfc3339(),
            score: Some(90),
            match_type: "exact".to_string(),
            download_url: Some(format!("magnet:?xt=urn:btih:movie{}_4k", id)),
        },
    ];

    debug!(
        "Returning {} mock releases for movie {}",
        mock_releases.len(),
        id
    );
    Ok(Json(mock_releases))
}

#[derive(Deserialize, Debug)]
pub struct DownloadReleaseRequest {
    #[serde(rename = "movieId")]
    pub movie_id: i32,
    #[serde(rename = "releaseId")]
    pub release_id: String,
    #[serde(rename = "indexerId")]
    pub indexer_id: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
}

/// POST /api/v3/movies/download - Download a release
pub async fn download_release(
    Extension(_services): Extension<Arc<AppServices>>,
    Json(request): Json<DownloadReleaseRequest>,
) -> Result<StatusCode, StatusCode> {
    debug!(
        "Starting download for release: {} from movie: {}",
        request.release_id, request.movie_id
    );

    // For MVP, just simulate successful download initiation
    // In production, this would:
    // 1. Validate the release and movie
    // 2. Add to qBittorrent download client
    // 3. Create queue entry for tracking
    // 4. Return success/error status

    debug!(
        "Download initiated successfully for release: {}",
        request.release_id
    );
    Ok(StatusCode::OK)
}

#[derive(Deserialize, Debug)]
pub struct BulkUpdateRequest {
    #[serde(rename = "movieIds")]
    pub movie_ids: Vec<i32>,
    pub updates: BulkUpdateData,
}

#[derive(Deserialize, Debug)]
pub struct BulkUpdateData {
    pub monitored: Option<bool>,
    pub quality_profile_id: Option<i32>,
}

/// PUT /api/v3/movies/bulk - Bulk update movies
pub async fn bulk_update_movies(
    Extension(services): Extension<Arc<AppServices>>,
    Json(request): Json<BulkUpdateRequest>,
) -> Result<Json<Vec<MovieResponse>>, StatusCode> {
    debug!("Bulk updating {} movies", request.movie_ids.len());

    let mut updated_movies = Vec::new();

    // Update each movie individually
    for movie_id in &request.movie_ids {
        match services.movie_repository.find_by_tmdb_id(*movie_id).await {
            Ok(Some(mut movie)) => {
                let mut has_changes = false;

                // Apply bulk updates
                if let Some(monitored) = request.updates.monitored {
                    movie.monitored = monitored;
                    has_changes = true;
                }

                if let Some(quality_profile_id) = request.updates.quality_profile_id {
                    movie.quality_profile_id = Some(quality_profile_id);
                    has_changes = true;
                }

                // Save changes if any were made
                if has_changes {
                    match services.movie_repository.update(&movie).await {
                        Ok(updated_movie) => {
                            debug!("Updated movie: {}", updated_movie.title);
                            updated_movies.push(convert_movie_to_response(updated_movie));
                        }
                        Err(e) => {
                            error!("Failed to update movie {}: {}", movie_id, e);
                            // Continue with other movies instead of failing completely
                        }
                    }
                }
            }
            Ok(None) => {
                warn!("Movie with ID {} not found during bulk update", movie_id);
                // Continue with other movies
            }
            Err(e) => {
                error!(
                    "Failed to fetch movie {} during bulk update: {}",
                    movie_id, e
                );
                // Continue with other movies
            }
        }
    }

    debug!(
        "Successfully updated {} out of {} movies",
        updated_movies.len(),
        request.movie_ids.len()
    );
    Ok(Json(updated_movies))
}
