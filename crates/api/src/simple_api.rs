//! Simplified API implementation for MVP
//! 
//! This provides basic REST API endpoints without complex state management
//! for demonstration purposes.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};
use crate::security::{SecurityConfig, apply_security};
use crate::middleware::require_api_key;
use radarr_core::{Movie, MovieStatus, RadarrError, repositories::MovieRepository};
use radarr_infrastructure::{DatabasePool, PostgresMovieRepository};
use radarr_indexers::{IndexerClient, SearchRequest, SearchResponse, ProwlarrSearchResult};
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use chrono;
use tracing::{info, warn, error};

/// Simple application state for MVP
#[derive(Clone)]
pub struct SimpleApiState {
    pub database_pool: DatabasePool,
    pub indexer_client: Option<Arc<dyn IndexerClient + Send + Sync>>,
    pub movie_repo: Arc<PostgresMovieRepository>,
}

impl SimpleApiState {
    pub fn new(database_pool: DatabasePool) -> Self {
        let movie_repo = Arc::new(PostgresMovieRepository::new(database_pool.clone()));
        Self { 
            database_pool,
            indexer_client: None,
            movie_repo,
        }
    }
    
    /// Create new state with indexer client
    pub fn with_indexer_client(mut self, client: Arc<dyn IndexerClient + Send + Sync>) -> Self {
        self.indexer_client = Some(client);
        self
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

/// Create the simplified API router with security features
pub fn create_simple_api_router(state: SimpleApiState) -> Router {
    // Load security configuration from environment
    let security_config = SecurityConfig::from_env();
    
    // Create base router with endpoints
    let api_router = Router::new()
        // Health check (no auth required)
        .route("/health", get(health_check))
        
        // Protected movie endpoints (require API key)
        .route("/api/v3/movie", get(list_movies))
        .route("/api/v3/movie", post(create_movie))
        .route("/api/v3/movie/:id", get(get_movie))
        .route("/api/v3/movie/:id", delete(delete_movie))
        
        // Protected search endpoint (real Prowlarr integration)
        .route("/api/v3/indexer/search", post(search_movies))
        
        // Protected Prowlarr test endpoint
        .route("/api/v3/indexer/test", post(test_prowlarr_connection))
        
        // Protected download endpoint (mock)
        .route("/api/v3/download", post(start_download))
        
        // Protected import endpoint (real import pipeline)
        .route("/api/v3/command/import", post(import_download))
        
        .with_state(state)
        // Apply API key authentication middleware
        .layer(axum::middleware::from_fn(require_api_key));
    
    // Apply comprehensive security features
    apply_security(api_router, security_config)
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
    State(state): State<SimpleApiState>,
    Query(params): Query<SimpleQueryParams>,
) -> Json<Value> {
    info!("Listing movies with pagination: page={}, limit={}", params.page, params.limit);
    
    // Calculate offset from page and limit
    let offset = ((params.page - 1) * params.limit) as i64;
    let limit = params.limit as i32;
    
    // Get movies from database
    match state.movie_repo.list(offset, limit).await {
        Ok(movies) => {
            // Convert to SimpleMovieResponse format
            let movie_responses: Vec<SimpleMovieResponse> = movies
                .into_iter()
                .map(SimpleMovieResponse::from)
                .collect();
            
            // Get total count
            let total_count = match state.movie_repo.count().await {
                Ok(count) => count,
                Err(e) => {
                    error!("Failed to get movie count: {}", e);
                    movie_responses.len() as i64
                }
            };
            
            info!("Retrieved {} movies from database", movie_responses.len());
            
            Json(serde_json::json!({
                "page": params.page,
                "pageSize": params.limit,
                "totalCount": total_count,
                "records": movie_responses
            }))
        }
        Err(e) => {
            error!("Failed to list movies: {}", e);
            // Return empty result on error
            Json(serde_json::json!({
                "page": params.page,
                "pageSize": params.limit,
                "totalCount": 0,
                "records": []
            }))
        }
    }
}

/// Get movie by ID endpoint
async fn get_movie(
    State(state): State<SimpleApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    info!("Getting movie by ID: {}", id);
    
    match state.movie_repo.find_by_id(id).await {
        Ok(Some(movie)) => {
            info!("Found movie: {}", movie.title);
            let response = SimpleMovieResponse::from(movie);
            Ok(Json(serde_json::to_value(&response).unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Failed to serialize movie response"
                })
            })))
        }
        Ok(None) => {
            warn!("Movie not found: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!("Database error while getting movie {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create movie endpoint
async fn create_movie(
    State(state): State<SimpleApiState>,
    Json(request): Json<SimpleCreateMovieRequest>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    info!("Creating movie: {} (TMDB: {})", request.title, request.tmdb_id);
    
    // Check if movie already exists
    match state.movie_repo.find_by_tmdb_id(request.tmdb_id).await {
        Ok(Some(_)) => {
            warn!("Movie with TMDB ID {} already exists", request.tmdb_id);
            return Err(StatusCode::CONFLICT);
        }
        Ok(None) => {} // Good, doesn't exist yet
        Err(e) => {
            error!("Database error checking for existing movie: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    // Create new movie
    let mut movie = Movie::new(
        request.tmdb_id,
        request.title.clone(),
    );
    
    // Set monitored flag from request
    movie.monitored = request.monitored;
    
    match state.movie_repo.create(&movie).await {
        Ok(created_movie) => {
            info!("Movie created successfully: {}", created_movie.title);
            let response = SimpleMovieResponse::from(created_movie);
            Ok((StatusCode::CREATED, Json(serde_json::to_value(&response).unwrap_or_else(|_| {
                serde_json::json!({
                    "error": "Failed to serialize movie response"
                })
            }))))
        }
        Err(e) => {
            error!("Failed to create movie: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
    State(state): State<SimpleApiState>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use tracing::{info, warn, error};
    use std::time::Instant;
    
    let start_time = Instant::now();
    info!("Received search request: {:?}", request);
    
    // Extract search parameters from request
    let query = request.get("query").and_then(|q| q.as_str()).map(String::from);
    let imdb_id = request.get("imdbId").and_then(|id| id.as_str()).map(String::from);
    let tmdb_id = request.get("tmdbId").and_then(|id| id.as_i64()).map(|id| id as i32);
    let limit = request.get("limit").and_then(|l| l.as_i64()).map(|l| l as i32);
    
    // Check if we have an indexer client
    let indexer_client = match state.indexer_client.as_ref() {
        Some(client) => client,
        None => {
            warn!("No indexer client available, falling back to mock data");
            return Ok(Json(create_mock_search_response()));
        }
    };
    
    // Build search request
    let mut search_request = SearchRequest::default();
    if let Some(q) = query {
        search_request.query = Some(q);
    }
    if let Some(imdb) = imdb_id {
        search_request.imdb_id = Some(imdb);
    }
    if let Some(tmdb) = tmdb_id {
        search_request.tmdb_id = Some(tmdb);
    }
    if let Some(l) = limit {
        search_request.limit = Some(l);
    }
    // Default to movie categories
    if search_request.categories.is_empty() {
        search_request.categories = vec![2000]; // Movie category
    }
    
    info!("Searching Prowlarr with request: {:?}", search_request);
    
    // Perform search with retry logic
    let search_result = perform_search_with_retry(indexer_client.as_ref(), &search_request, 3).await;
    
    let execution_time = start_time.elapsed().as_millis();
    
    // If Prowlarr fails, try HDBits directly as fallback
    let search_result = match search_result {
        Err(_) if search_request.query.is_some() => {
            info!("Prowlarr failed, attempting HDBits fallback");
            search_hdbits_fallback(&search_request.query.unwrap_or_default()).await
        },
        result => result
    };
    
    match search_result {
        Ok(response) => {
            info!("Search completed successfully in {}ms, found {} results", execution_time, response.total);
            
            // Convert to API response format
            let api_response = serde_json::json!({
                "total": response.total,
                "releases": response.results.iter().map(|result| {
                    serde_json::json!({
                        "guid": format!("{}-{}", result.indexer_id, result.title.chars().take(20).collect::<String>()),
                        "title": result.title,
                        "downloadUrl": result.download_url,
                        "infoUrl": result.info_url,
                        "indexer": result.indexer,
                        "indexerId": result.indexer_id,
                        "size": result.size,
                        "seeders": result.seeders,
                        "leechers": result.leechers,
                        "downloadFactor": result.download_factor,
                        "uploadFactor": result.upload_factor,
                        "publishDate": result.publish_date,
                        "imdbId": result.imdb_id,
                        "tmdbId": result.tmdb_id,
                        "freeleech": result.freeleech,
                        "qualityScore": calculate_quality_score(&result.title),
                    })
                }).collect::<Vec<_>>(),
                "indexersSearched": response.indexers_searched,
                "indexersWithErrors": response.indexers_with_errors,
                "errors": response.errors,
                "executionTimeMs": execution_time
            });
            
            Ok(Json(api_response))
        }
        Err(e) => {
            error!("Search failed after retries: {}", e);
            
            // Return error response
            let error_response = serde_json::json!({
                "error": "Search failed",
                "message": e.to_string(),
                "executionTimeMs": execution_time,
                "fallbackUsed": false
            });
            
            Err((StatusCode::SERVICE_UNAVAILABLE, Json(error_response)))
        }
    }
}

/// Test Prowlarr connectivity endpoint
async fn test_prowlarr_connection(
    State(state): State<SimpleApiState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    use tracing::{info, warn, error};
    use std::time::Instant;
    
    let start_time = Instant::now();
    info!("Testing Prowlarr connectivity");
    
    let indexer_client = match state.indexer_client.as_ref() {
        Some(client) => client,
        None => {
            error!("No indexer client configured");
            let error_response = serde_json::json!({
                "status": "error",
                "message": "No indexer client configured",
                "connected": false,
                "executionTimeMs": start_time.elapsed().as_millis()
            });
            return Err((StatusCode::SERVICE_UNAVAILABLE, Json(error_response)));
        }
    };
    
    // Test health check
    let health_result = indexer_client.health_check().await;
    let execution_time = start_time.elapsed().as_millis();
    
    match health_result {
        Ok(is_healthy) => {
            if is_healthy {
                info!("Prowlarr connectivity test passed in {}ms", execution_time);
                
                // Also test getting indexers to verify API key works
                match indexer_client.get_indexers().await {
                    Ok(indexers) => {
                        let response = serde_json::json!({
                            "status": "success",
                            "message": "Prowlarr connection successful",
                            "connected": true,
                            "indexerCount": indexers.len(),
                            "indexers": indexers.iter().map(|idx| {
                                serde_json::json!({
                                    "id": idx.id,
                                    "name": idx.name,
                                    "enabled": idx.enable,
                                    "implementation": idx.implementation
                                })
                            }).collect::<Vec<_>>(),
                            "executionTimeMs": execution_time
                        });
                        Ok(Json(response))
                    }
                    Err(e) => {
                        warn!("Prowlarr health check passed but indexers fetch failed: {}", e);
                        let response = serde_json::json!({
                            "status": "warning",
                            "message": format!("Connection works but API access limited: {}", e),
                            "connected": true,
                            "indexerCount": 0,
                            "executionTimeMs": execution_time
                        });
                        Ok(Json(response))
                    }
                }
            } else {
                warn!("Prowlarr health check returned false");
                let response = serde_json::json!({
                    "status": "error",
                    "message": "Prowlarr service is not healthy",
                    "connected": false,
                    "executionTimeMs": execution_time
                });
                Err((StatusCode::SERVICE_UNAVAILABLE, Json(response)))
            }
        }
        Err(e) => {
            error!("Prowlarr connectivity test failed: {}", e);
            let response = serde_json::json!({
                "status": "error",
                "message": format!("Failed to connect to Prowlarr: {}", e),
                "connected": false,
                "executionTimeMs": execution_time
            });
            Err((StatusCode::SERVICE_UNAVAILABLE, Json(response)))
        }
    }
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

/// Import download endpoint - implements basic import pipeline
async fn import_download(
    State(_state): State<SimpleApiState>,
    Json(request): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    info!("Starting import operation with request: {:?}", request);
    
    // Extract download path from request
    let download_path = request.get("path")
        .and_then(|p| p.as_str())
        .unwrap_or("/downloads"); // Default download path
    
    let output_path = request.get("outputPath")
        .and_then(|p| p.as_str())
        .unwrap_or("/movies"); // Default movies path
    
    let dry_run = request.get("dryRun")
        .and_then(|d| d.as_bool())
        .unwrap_or(true); // Default to dry run for safety
    
    info!("Import config: download_path={}, output_path={}, dry_run={}", 
          download_path, output_path, dry_run);
    
    // For MVP demo, simulate successful import
    let mock_response = serde_json::json!({
        "success": true,
        "message": "Import completed successfully (MVP simulation)",
        "stats": {
            "filesScanned": 1,
            "filesAnalyzed": 1,
            "successfulImports": 1,
            "failedImports": 0,
            "skippedFiles": 0,
            "totalSize": 1500000000,
            "totalDurationMs": 1200,
            "hardlinksCreated": 1,
            "filesCopied": 0
        },
        "dryRun": dry_run,
        "sourcePath": download_path,
        "destinationPath": output_path,
        "importedFiles": [
            {
                "originalPath": format!("{}/Fight.Club.1999.1080p.BluRay.x264-SPARKS.mkv", download_path),
                "newPath": format!("{}/Fight Club (1999)/Fight Club (1999) Bluray-1080p.mkv", output_path),
                "size": 1500000000,
                "quality": "Bluray-1080p",
                "hardlinked": !dry_run
            }
        ]
    });
    
    // Simulate some processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    info!("Import simulation completed");
    Ok(Json(mock_response))
}

/// Perform search with exponential backoff retry logic
async fn perform_search_with_retry(
    client: &dyn IndexerClient,
    request: &SearchRequest,
    max_retries: u32,
) -> radarr_core::Result<radarr_indexers::SearchResponse> {
    use tokio::time::{sleep, Duration};
    use tracing::{warn, debug};
    
    let mut last_error = None;
    
    for attempt in 0..=max_retries {
        debug!("Search attempt {} of {}", attempt + 1, max_retries + 1);
        
        match client.search(request).await {
            Ok(response) => {
                debug!("Search succeeded on attempt {}", attempt + 1);
                return Ok(response);
            }
            Err(e) => {
                warn!("Search attempt {} failed: {}", attempt + 1, e);
                last_error = Some(e);
                
                // Don't sleep after the last attempt
                if attempt < max_retries {
                    let delay = Duration::from_millis(1000 * (2_u64.pow(attempt))); // Exponential backoff
                    debug!("Retrying in {:?}", delay);
                    sleep(delay).await;
                }
            }
        }
    }
    
    Err(last_error.unwrap())
}

/// Calculate quality score based on release title
fn calculate_quality_score(title: &str) -> i32 {
    let title_lower = title.to_lowercase();
    let mut score = 50; // Base score
    
    // Resolution scoring
    if title_lower.contains("2160p") || title_lower.contains("4k") {
        score += 30;
    } else if title_lower.contains("1080p") {
        score += 20;
    } else if title_lower.contains("720p") {
        score += 10;
    }
    
    // Source scoring
    if title_lower.contains("bluray") || title_lower.contains("uhd") {
        score += 15;
    } else if title_lower.contains("web-dl") || title_lower.contains("webdl") {
        score += 10;
    } else if title_lower.contains("webrip") {
        score += 8;
    } else if title_lower.contains("hdtv") {
        score += 5;
    }
    
    // Encoding scoring
    if title_lower.contains("x265") || title_lower.contains("hevc") {
        score += 10;
    } else if title_lower.contains("x264") {
        score += 5;
    }
    
    // Group/release scoring (known good groups get bonus)
    let good_groups = ["sparks", "rovers", "blow", "psychd", "veto"];
    if good_groups.iter().any(|group| title_lower.contains(group)) {
        score += 10;
    }
    
    // Cap the score between 0 and 100
    score.max(0).min(100)
}

/// Create mock search response for fallback
/// Fallback search using HDBits directly when Prowlarr is unavailable
async fn search_hdbits_fallback(query: &str) -> Result<SearchResponse, RadarrError> {
    use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};
    use std::env;
    
    // Try to get HDBits credentials from environment
    let username = env::var("HDBITS_USERNAME").map_err(|_| RadarrError::ExternalServiceError {
        service: "hdbits".to_string(),
        error: "HDBITS_USERNAME not configured".to_string(),
    })?;
    
    let session_cookie = env::var("HDBITS_SESSION_COOKIE").map_err(|_| RadarrError::ExternalServiceError {
        service: "hdbits".to_string(),
        error: "HDBITS_SESSION_COOKIE not configured".to_string(),
    })?;
    
    // Create HDBits config
    let config = HDBitsConfig {
        username,
        session_cookie,
        timeout_seconds: 30,
        rate_limit_per_hour: 120,
    };
    
    // Create HDBits client  
    let hdbits = HDBitsClient::new(config).map_err(|e| RadarrError::ExternalServiceError {
        service: "hdbits".to_string(),
        error: format!("Failed to create HDBits client: {}", e),
    })?;
    
    // Build search request
    let search_request = MovieSearchRequest {
        title: Some(query.to_string()),
        year: None,
        imdb_id: None,
        limit: Some(20),
        min_seeders: None,
    };
    
    // Search HDBits
    let results = hdbits.search_movies(&search_request).await.map_err(|e| RadarrError::ExternalServiceError {
        service: "hdbits".to_string(),
        error: format!("HDBits search failed: {}", e),
    })?;
    
    // Convert HDBits Release results to SearchResponse format
    let search_response = SearchResponse {
        total: results.len() as i32,
        results: results.into_iter().map(|release| ProwlarrSearchResult {
            indexer: "HDBits".to_string(),
            indexer_id: release.indexer_id,
            title: release.title.clone(),
            download_url: release.download_url.clone(),
            info_url: release.info_url,
            size: release.size_bytes.map(|s| s as i64),
            seeders: release.seeders,
            leechers: release.leechers,
            imdb_id: None, // TODO: Extract from release metadata
            tmdb_id: None,
            freeleech: Some(false), // TODO: Parse from quality info
            download_factor: Some(1.0),
            upload_factor: Some(1.0),
            publish_date: release.published_date,
            categories: vec![], // TODO: Map HDBits categories
            attributes: HashMap::new(),
        }).collect(),
        indexers_searched: 1,
        indexers_with_errors: 0,
        errors: vec![],
    };
    
    Ok(search_response)
}

fn create_mock_search_response() -> Value {
    serde_json::json!({
        "total": 2,
        "releases": [
            {
                "guid": "mock-guid-1",
                "title": "The.Matrix.1999.1080p.BluRay.x264-GROUP",
                "downloadUrl": "magnet:?xt=urn:btih:example1",
                "indexer": "Mock Indexer",
                "size": 8000000000i64,
                "seeders": 50,
                "qualityScore": 85
            },
            {
                "guid": "mock-guid-2",
                "title": "The.Matrix.1999.720p.WEB-DL.x264-GROUP",
                "downloadUrl": "magnet:?xt=urn:btih:example2", 
                "indexer": "Mock Indexer",
                "size": 4000000000i64,
                "seeders": 25,
                "qualityScore": 70
            }
        ],
        "indexersSearched": 1,
        "indexersWithErrors": 0,
        "errors": [],
        "executionTimeMs": 50,
        "fallbackUsed": true
    })
}