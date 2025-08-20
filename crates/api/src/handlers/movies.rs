//! Movie API handlers

use crate::{
    error::{ApiError, ApiResult},
    extractors::{ValidatedPath, ValidatedPagination, validate_tmdb_id, validate_movie_title},
    models::{CreateMovieRequest, UpdateMovieRequest, MovieResponse, PaginatedResponse},
};
use axum::{extract::State, http::StatusCode, Json};
use radarr_core::{
    Movie, RadarrError,
    repositories::MovieRepository,
};
use radarr_infrastructure::{DatabasePool, PostgresMovieRepository};
use std::sync::Arc;
use tracing::{info, warn, error, instrument};
use uuid::Uuid;

/// Application state containing database pool and repositories
#[derive(Clone)]
pub struct AppState {
    pub database_pool: DatabasePool,
    pub movie_repo: Arc<PostgresMovieRepository>,
}

impl AppState {
    pub fn new(database_pool: DatabasePool) -> Self {
        let movie_repo = Arc::new(PostgresMovieRepository::new(database_pool.clone()));
        Self {
            database_pool,
            movie_repo,
        }
    }
}

/// GET /api/v3/movie - List all movies with pagination
#[instrument(skip(state))]
pub async fn list_movies(
    State(state): State<AppState>,
    ValidatedPagination(pagination): ValidatedPagination,
) -> ApiResult<Json<PaginatedResponse<MovieResponse>>> {
    info!("Listing movies with pagination: page={}, page_size={}", pagination.page, pagination.page_size);
    
    let (limit, offset) = pagination.to_sql_params();
    
    // Get movies and total count in parallel
    let (movies, total_count) = tokio::try_join!(
        state.movie_repo.list(offset, limit),
        state.movie_repo.count()
    ).map_err(ApiError::CoreError)?;
    
    let movie_responses: Vec<MovieResponse> = movies.into_iter().map(MovieResponse::from).collect();
    
    let response = PaginatedResponse::new(
        pagination.page,
        pagination.page_size,
        total_count,
        movie_responses,
    );
    
    info!("Retrieved {} movies, total: {}", response.records.len(), total_count);
    Ok(Json(response))
}

/// GET /api/v3/movie/{id} - Get movie by ID
#[instrument(skip(state), fields(movie_id = %movie_id))]
pub async fn get_movie(
    State(state): State<AppState>,
    ValidatedPath(movie_id): ValidatedPath<Uuid>,
) -> ApiResult<Json<MovieResponse>> {
    info!("Getting movie by ID: {}", movie_id);
    
    let movie = state
        .movie_repo
        .find_by_id(movie_id)
        .await
        .map_err(ApiError::CoreError)?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("Movie with ID {}", movie_id),
        })?;
    
    info!("Found movie: {}", movie.title);
    Ok(Json(MovieResponse::from(movie)))
}

/// POST /api/v3/movie - Add new movie (from TMDB ID)
#[instrument(skip(state))]
pub async fn create_movie(
    State(state): State<AppState>,
    Json(request): Json<CreateMovieRequest>,
) -> ApiResult<(StatusCode, Json<MovieResponse>)> {
    info!("Creating movie from TMDB ID: {}", request.tmdb_id);
    
    // Validate TMDB ID
    validate_tmdb_id(request.tmdb_id)?;
    
    // Check if movie already exists
    if let Ok(Some(_)) = state.movie_repo.find_by_tmdb_id(request.tmdb_id).await {
        return Err(ApiError::Conflict {
            resource: format!("Movie with TMDB ID {}", request.tmdb_id),
        });
    }
    
    // For MVP, we'll create a basic movie record
    // In production, this would fetch metadata from TMDB
    let title = request.title.unwrap_or_else(|| format!("Movie {}", request.tmdb_id));
    validate_movie_title(&title)?;
    
    let mut movie = Movie::new(request.tmdb_id, title);
    movie.monitored = request.monitored;
    movie.quality_profile_id = request.quality_profile_id;
    
    if let Some(min_availability) = request.minimum_availability {
        movie.minimum_availability = min_availability;
    }
    
    // Merge additional metadata if provided
    if let Some(metadata) = request.metadata {
        movie.update_metadata(metadata);
    }
    
    let created_movie = state
        .movie_repo
        .create(&movie)
        .await
        .map_err(ApiError::CoreError)?;
    
    info!("Created movie: {} (ID: {})", created_movie.title, created_movie.id);
    Ok((StatusCode::CREATED, Json(MovieResponse::from(created_movie))))
}

/// PUT /api/v3/movie/{id} - Update movie
#[instrument(skip(state), fields(movie_id = %movie_id))]
pub async fn update_movie(
    State(state): State<AppState>,
    ValidatedPath(movie_id): ValidatedPath<Uuid>,
    Json(request): Json<UpdateMovieRequest>,
) -> ApiResult<Json<MovieResponse>> {
    info!("Updating movie: {}", movie_id);
    
    // Get existing movie
    let mut movie = state
        .movie_repo
        .find_by_id(movie_id)
        .await
        .map_err(ApiError::CoreError)?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("Movie with ID {}", movie_id),
        })?;
    
    // Apply updates
    if let Some(monitored) = request.monitored {
        movie.monitored = monitored;
    }
    
    if let Some(quality_profile_id) = request.quality_profile_id {
        movie.quality_profile_id = Some(quality_profile_id);
    }
    
    if let Some(minimum_availability) = request.minimum_availability {
        movie.minimum_availability = minimum_availability;
    }
    
    if let Some(metadata) = request.metadata {
        movie.update_metadata(metadata);
    }
    
    let updated_movie = state
        .movie_repo
        .update(&movie)
        .await
        .map_err(ApiError::CoreError)?;
    
    info!("Updated movie: {}", updated_movie.title);
    Ok(Json(MovieResponse::from(updated_movie)))
}

/// DELETE /api/v3/movie/{id} - Delete movie
#[instrument(skip(state), fields(movie_id = %movie_id))]
pub async fn delete_movie(
    State(state): State<AppState>,
    ValidatedPath(movie_id): ValidatedPath<Uuid>,
) -> ApiResult<StatusCode> {
    info!("Deleting movie: {}", movie_id);
    
    // Check if movie exists first
    let movie = state
        .movie_repo
        .find_by_id(movie_id)
        .await
        .map_err(ApiError::CoreError)?
        .ok_or_else(|| ApiError::NotFound {
            resource: format!("Movie with ID {}", movie_id),
        })?;
    
    // Delete the movie
    state
        .movie_repo
        .delete(movie_id)
        .await
        .map_err(ApiError::CoreError)?;
    
    info!("Deleted movie: {} (ID: {})", movie.title, movie_id);
    Ok(StatusCode::NO_CONTENT)
}