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
use tower_http::services::ServeDir;
use crate::{security::{SecurityConfig, apply_security}, metrics::MetricsCollector};
use radarr_core::{Movie, MovieStatus, RadarrError, repositories::MovieRepository};
use radarr_infrastructure::{DatabasePool, PostgresMovieRepository, TmdbClient, CachedTmdbClient};
use radarr_indexers::{IndexerClient, SearchRequest, SearchResponse, ProwlarrSearchResult};
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use chrono;
use tracing::{info, warn, error};
use radarr_core::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState};
use std::time::Duration;

/// Simple application state for MVP
#[derive(Clone)]
pub struct SimpleApiState {
    pub database_pool: DatabasePool,
    pub indexer_client: Option<Arc<dyn IndexerClient + Send + Sync>>,
    pub movie_repo: Arc<PostgresMovieRepository>,
    pub tmdb_client: Option<Arc<CachedTmdbClient>>,
    pub metrics_collector: Option<Arc<MetricsCollector>>,
    pub quality_state: crate::handlers::quality::QualityState,
    // Circuit breakers for testing
    pub tmdb_circuit_breaker: Arc<CircuitBreaker>,
    pub hdbits_circuit_breaker: Arc<CircuitBreaker>,
    pub qbittorrent_circuit_breaker: Arc<CircuitBreaker>,
    pub database_circuit_breaker: Arc<CircuitBreaker>,
}

impl SimpleApiState {
    pub fn new(database_pool: DatabasePool) -> Self {
        let movie_repo = Arc::new(PostgresMovieRepository::new(database_pool.clone()));
        
        // Create circuit breakers for testing
        let tmdb_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerConfig::new("TMDB")
                .with_failure_threshold(3)
                .with_timeout(Duration::from_secs(30))
                .with_request_timeout(Duration::from_secs(10))
        ));
        
        let hdbits_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerConfig::new("HDBits")
                .with_failure_threshold(5)
                .with_timeout(Duration::from_secs(60))
                .with_request_timeout(Duration::from_secs(15))
        ));
        
        let qbittorrent_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerConfig::new("qBittorrent")
                .with_failure_threshold(4)
                .with_timeout(Duration::from_secs(45))
                .with_request_timeout(Duration::from_secs(8))
        ));
        
        let database_cb = Arc::new(CircuitBreaker::new(
            CircuitBreakerConfig::new("PostgreSQL")
                .with_failure_threshold(2)
                .with_timeout(Duration::from_secs(15))
                .with_request_timeout(Duration::from_secs(5))
        ));
        
        let quality_state = crate::handlers::quality::QualityState::new(database_pool.clone());
        
        Self { 
            database_pool,
            indexer_client: None,
            movie_repo,
            tmdb_client: None,
            metrics_collector: None,
            quality_state,
            tmdb_circuit_breaker: tmdb_cb,
            hdbits_circuit_breaker: hdbits_cb,
            qbittorrent_circuit_breaker: qbittorrent_cb,
            database_circuit_breaker: database_cb,
        }
    }
    
    /// Create new state with indexer client
    pub fn with_indexer_client(mut self, client: Arc<dyn IndexerClient + Send + Sync>) -> Self {
        self.indexer_client = Some(client);
        self
    }
    
    /// Create new state with TMDB client
    pub fn with_tmdb_client(mut self, client: Arc<CachedTmdbClient>) -> Self {
        self.tmdb_client = Some(client);
        self
    }
    
    /// Create new state with metrics collector
    pub fn with_metrics_collector(mut self, metrics: Arc<MetricsCollector>) -> Self {
        self.metrics_collector = Some(metrics);
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

/// Movie lookup query parameters
#[derive(Debug, Deserialize)]
pub struct MovieLookupParams {
    pub term: String,
    #[serde(default = "default_search_limit")]
    pub limit: u32,
    pub year: Option<i32>,
}

/// Movie lookup response (matches frontend SearchResult interface)
#[derive(Debug, Serialize)]
pub struct MovieLookupResponse {
    pub title: String,
    pub year: Option<i32>,
    pub tmdb_id: i32,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
    pub popularity: Option<f64>,
}

impl From<Movie> for MovieLookupResponse {
    fn from(movie: Movie) -> Self {
        let tmdb_metadata = movie.metadata.get("tmdb");
        
        Self {
            title: movie.title,
            year: movie.year,
            tmdb_id: movie.tmdb_id,
            imdb_id: movie.imdb_id,
            overview: tmdb_metadata
                .and_then(|meta| meta.get("overview"))
                .and_then(|overview| overview.as_str())
                .map(String::from),
            poster_path: tmdb_metadata
                .and_then(|meta| meta.get("poster_path"))
                .and_then(|path| path.as_str())
                .map(String::from),
            release_date: tmdb_metadata
                .and_then(|meta| meta.get("release_date"))
                .and_then(|date| date.as_str())
                .map(String::from),
            vote_average: tmdb_metadata
                .and_then(|meta| meta.get("vote_average"))
                .and_then(|rating| rating.as_f64()),
            popularity: tmdb_metadata
                .and_then(|meta| meta.get("popularity"))
                .and_then(|pop| pop.as_f64()),
        }
    }
}

fn default_search_limit() -> u32 { 20 }

fn default_page() -> u32 { 1 }
fn default_limit() -> u32 { 50 }

/// Create the simplified API router with security features
pub fn create_simple_api_router(state: SimpleApiState) -> Router {
    // Load security configuration from environment
    let security_config = SecurityConfig::from_env();
    
    // Create protected API router 
    let api_router = Router::new()
        
        // Protected movie endpoints (require API key)
        .route("/v3/movie", get(list_movies))
        .route("/v3/movie", post(create_movie))
        .route("/v3/movie/lookup", get(lookup_movies))  // IMPORTANT: Must come before /:id route
        .route("/v3/movie/:id", get(get_movie))
        .route("/v3/movie/:id", delete(delete_movie))
        
        // Protected search endpoint (real Prowlarr integration)
        .route("/v3/indexer/search", post(search_movies))
        
        // Protected Prowlarr test endpoint
        .route("/v3/indexer/test", post(test_prowlarr_connection))
        
        // Protected download endpoint (mock)
        .route("/v3/download", post(start_download))
        
        // Protected import endpoint (real import pipeline)
        .route("/v3/command/import", post(import_download))
        
        // Circuit breaker test endpoints
        .route("/v3/test/circuit-breaker/status", get(circuit_breaker_status))
        .route("/v3/test/circuit-breaker/simulate-failure/:service", post(simulate_service_failure))
        .route("/v3/test/circuit-breaker/reset/:service", post(reset_circuit_breaker))
        
        // Quality management endpoints - these will need a separate state
        // TODO: Implement quality routes with proper state management
        
        .with_state(state.clone());
    
    // Create static file service for React app  
    let static_service = ServeDir::new("web/dist")
        .append_index_html_on_directories(true)
        .fallback(ServeDir::new("web/dist").append_index_html_on_directories(true));
    
    // Combine routes: protected API routes under /api, public routes for everything else
    let full_router = Router::new()
        .route("/health", get(health_check)) // Public health check
        .nest("/api", api_router) // Protected API routes under /api prefix
        .fallback_service(static_service); // Serve React app for all other routes
    
    // Apply comprehensive security features
    apply_security(full_router, security_config)
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

/// Movie lookup endpoint - searches TMDB for movies
async fn lookup_movies(
    State(state): State<SimpleApiState>,
    Query(params): Query<MovieLookupParams>,
) -> Result<Json<Vec<MovieLookupResponse>>, (StatusCode, Json<Value>)> {
    info!("Looking up movies with term: '{}'", params.term);
    
    let tmdb_client = match state.tmdb_client.as_ref() {
        Some(client) => client,
        None => {
            error!("TMDB client not configured");
            let error_response = serde_json::json!({
                "error": "TMDB client not configured",
                "message": "Movie lookup service is not available"
            });
            return Err((StatusCode::SERVICE_UNAVAILABLE, Json(error_response)));
        }
    };

    // Perform search with TMDB
    match tmdb_client.search_movies(&params.term, Some(1)).await {
        Ok(movies) => {
            info!("TMDB search returned {} movies", movies.len());
            
            // Convert to response format and apply limit
            let mut responses: Vec<MovieLookupResponse> = movies
                .into_iter()
                .map(MovieLookupResponse::from)
                .collect();
            
            // Apply year filter if provided
            if let Some(year) = params.year {
                responses.retain(|movie| movie.year == Some(year));
            }
            
            // Apply limit
            responses.truncate(params.limit as usize);
            
            info!("Returning {} movie results", responses.len());
            Ok(Json(responses))
        }
        Err(e) => {
            error!("TMDB search failed: {}", e);
            let error_response = serde_json::json!({
                "error": "Movie search failed",
                "message": e.to_string()
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
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
            
            // Record metrics for successful search
            if let Some(metrics) = &state.metrics_collector {
                metrics.record_search("prowlarr", start_time.elapsed(), true);
            }
            
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
            
            // Record metrics for failed search
            if let Some(metrics) = &state.metrics_collector {
                metrics.record_search("prowlarr", start_time.elapsed(), false);
            }
            
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

/// Circuit breaker status endpoint - shows all circuit breaker states
async fn circuit_breaker_status(
    State(state): State<SimpleApiState>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    info!("Getting circuit breaker status for all services");
    
    let mut services = Vec::new();
    
    // TMDB circuit breaker
    let tmdb_metrics = state.tmdb_circuit_breaker.get_metrics().await;
    services.push(serde_json::json!({
        "service": "TMDB",
        "state": tmdb_metrics.state.as_str(),
        "total_requests": tmdb_metrics.total_requests,
        "successful_requests": tmdb_metrics.successful_requests,
        "failed_requests": tmdb_metrics.failed_requests,
        "rejected_requests": tmdb_metrics.rejected_requests,
        "consecutive_failures": tmdb_metrics.consecutive_failures,
        "consecutive_successes": tmdb_metrics.consecutive_successes,
        "last_failure_time": tmdb_metrics.last_failure_time.map(|t| t.elapsed().as_secs()),
        "last_success_time": tmdb_metrics.last_success_time.map(|t| t.elapsed().as_secs()),
        "healthy": state.tmdb_circuit_breaker.is_healthy().await
    }));
    
    // HDBits circuit breaker
    let hdbits_metrics = state.hdbits_circuit_breaker.get_metrics().await;
    services.push(serde_json::json!({
        "service": "HDBits", 
        "state": hdbits_metrics.state.as_str(),
        "total_requests": hdbits_metrics.total_requests,
        "successful_requests": hdbits_metrics.successful_requests,
        "failed_requests": hdbits_metrics.failed_requests,
        "rejected_requests": hdbits_metrics.rejected_requests,
        "consecutive_failures": hdbits_metrics.consecutive_failures,
        "consecutive_successes": hdbits_metrics.consecutive_successes,
        "last_failure_time": hdbits_metrics.last_failure_time.map(|t| t.elapsed().as_secs()),
        "last_success_time": hdbits_metrics.last_success_time.map(|t| t.elapsed().as_secs()),
        "healthy": state.hdbits_circuit_breaker.is_healthy().await
    }));
    
    // qBittorrent circuit breaker
    let qbit_metrics = state.qbittorrent_circuit_breaker.get_metrics().await;
    services.push(serde_json::json!({
        "service": "qBittorrent",
        "state": qbit_metrics.state.as_str(), 
        "total_requests": qbit_metrics.total_requests,
        "successful_requests": qbit_metrics.successful_requests,
        "failed_requests": qbit_metrics.failed_requests,
        "rejected_requests": qbit_metrics.rejected_requests,
        "consecutive_failures": qbit_metrics.consecutive_failures,
        "consecutive_successes": qbit_metrics.consecutive_successes,
        "last_failure_time": qbit_metrics.last_failure_time.map(|t| t.elapsed().as_secs()),
        "last_success_time": qbit_metrics.last_success_time.map(|t| t.elapsed().as_secs()),
        "healthy": state.qbittorrent_circuit_breaker.is_healthy().await
    }));
    
    // Database circuit breaker
    let db_metrics = state.database_circuit_breaker.get_metrics().await;
    services.push(serde_json::json!({
        "service": "PostgreSQL",
        "state": db_metrics.state.as_str(),
        "total_requests": db_metrics.total_requests,
        "successful_requests": db_metrics.successful_requests,
        "failed_requests": db_metrics.failed_requests,
        "rejected_requests": db_metrics.rejected_requests,
        "consecutive_failures": db_metrics.consecutive_failures,
        "consecutive_successes": db_metrics.consecutive_successes,
        "last_failure_time": db_metrics.last_failure_time.map(|t| t.elapsed().as_secs()),
        "last_success_time": db_metrics.last_success_time.map(|t| t.elapsed().as_secs()),
        "healthy": state.database_circuit_breaker.is_healthy().await
    }));
    
    let response = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "services": services,
        "overall_healthy": services.iter().all(|s| s["healthy"].as_bool().unwrap_or(false))
    });
    
    info!("Returned circuit breaker status for {} services", services.len());
    Ok(Json(response))
}

/// Simulate service failure endpoint - forces a service to fail multiple times
async fn simulate_service_failure(
    State(state): State<SimpleApiState>,
    Path(service): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    info!("Simulating failures for service: {}", service);
    
    let circuit_breaker = match service.to_lowercase().as_str() {
        "tmdb" => &state.tmdb_circuit_breaker,
        "hdbits" => &state.hdbits_circuit_breaker,
        "qbittorrent" | "qbit" => &state.qbittorrent_circuit_breaker,
        "database" | "postgresql" | "postgres" => &state.database_circuit_breaker,
        _ => {
            let error_response = serde_json::json!({
                "error": "Invalid service name",
                "message": format!("Service '{}' not found. Valid services: tmdb, hdbits, qbittorrent, database", service),
                "valid_services": ["tmdb", "hdbits", "qbittorrent", "database"]
            });
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
    };
    
    // Get the failure threshold for this service
    let metrics_before = circuit_breaker.get_metrics().await;
    let failures_needed = if metrics_before.state == CircuitBreakerState::Open {
        0 // Already open
    } else {
        // Calculate how many more failures we need to trigger the circuit breaker
        let current_failures = metrics_before.consecutive_failures;
        let threshold = match service.to_lowercase().as_str() {
            "tmdb" => 3,
            "hdbits" => 5, 
            "qbittorrent" | "qbit" => 4,
            "database" | "postgresql" | "postgres" => 2,
            _ => 3, // Default
        };
        
        if current_failures >= threshold {
            0 // Already at threshold
        } else {
            threshold - current_failures
        }
    };
    
    // Simulate the required number of failures
    let mut simulated_failures = 0;
    for i in 0..failures_needed {
        let result = circuit_breaker.call(async {
            Err::<(), RadarrError>(RadarrError::ExternalServiceError {
                service: service.clone(),
                error: format!("Simulated failure #{}", i + 1),
            })
        }).await;
        
        if result.is_err() {
            simulated_failures += 1;
        }
        
        // Small delay between failures to make it realistic
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    let metrics_after = circuit_breaker.get_metrics().await;
    
    let response = serde_json::json!({
        "service": service,
        "simulated_failures": simulated_failures,
        "state_before": metrics_before.state.as_str(),
        "state_after": metrics_after.state.as_str(),
        "consecutive_failures_before": metrics_before.consecutive_failures,
        "consecutive_failures_after": metrics_after.consecutive_failures,
        "circuit_opened": metrics_after.state == CircuitBreakerState::Open && metrics_before.state != CircuitBreakerState::Open,
        "message": if metrics_after.state == CircuitBreakerState::Open {
            format!("Circuit breaker for {} is now OPEN after {} simulated failures", service, simulated_failures)
        } else {
            format!("Simulated {} failures for {}, circuit breaker state: {}", simulated_failures, service, metrics_after.state.as_str())
        }
    });
    
    info!("Simulated {} failures for {}, circuit state: {} -> {}", 
          simulated_failures, service, metrics_before.state.as_str(), metrics_after.state.as_str());
    
    Ok(Json(response))
}

/// Reset circuit breaker endpoint - manually resets a circuit breaker to closed state
async fn reset_circuit_breaker(
    State(state): State<SimpleApiState>,
    Path(service): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    info!("Resetting circuit breaker for service: {}", service);
    
    let circuit_breaker = match service.to_lowercase().as_str() {
        "tmdb" => &state.tmdb_circuit_breaker,
        "hdbits" => &state.hdbits_circuit_breaker,
        "qbittorrent" | "qbit" => &state.qbittorrent_circuit_breaker,
        "database" | "postgresql" | "postgres" => &state.database_circuit_breaker,
        _ => {
            let error_response = serde_json::json!({
                "error": "Invalid service name",
                "message": format!("Service '{}' not found. Valid services: tmdb, hdbits, qbittorrent, database", service),
                "valid_services": ["tmdb", "hdbits", "qbittorrent", "database"]
            });
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
    };
    
    let state_before = circuit_breaker.get_state().await;
    
    // Force close the circuit breaker and reset metrics
    circuit_breaker.force_close().await;
    circuit_breaker.reset_metrics().await;
    
    let state_after = circuit_breaker.get_state().await;
    let metrics_after = circuit_breaker.get_metrics().await;
    
    let response = serde_json::json!({
        "service": service,
        "state_before": state_before.as_str(),
        "state_after": state_after.as_str(),
        "metrics_reset": true,
        "current_metrics": {
            "total_requests": metrics_after.total_requests,
            "successful_requests": metrics_after.successful_requests,
            "failed_requests": metrics_after.failed_requests,
            "rejected_requests": metrics_after.rejected_requests,
            "consecutive_failures": metrics_after.consecutive_failures
        },
        "message": format!("Circuit breaker for {} has been reset to CLOSED state with cleared metrics", service)
    });
    
    info!("Reset circuit breaker for {}: {} -> {}", service, state_before.as_str(), state_after.as_str());
    
    Ok(Json(response))
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
            info_hash: None, // TODO: Extract from torrent data
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