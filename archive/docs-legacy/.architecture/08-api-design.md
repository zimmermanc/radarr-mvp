# API Design & Endpoint Implementation

**Last Updated**: August 20, 2025  
**API Status**: 🟡 15% Complete - Basic CRUD only  
**Compatibility**: Partial Radarr v3 API compatibility  
**Build Status**: ❌ Cannot test due to 164 compilation errors  

## API Implementation Status

### ✅ Working Endpoints (Basic CRUD)

| Method | Endpoint | Status | Functionality |
|--------|----------|--------|--------------|
| GET | `/api/v3/movie` | ✅ Working | List all movies |
| POST | `/api/v3/movie` | ✅ Working | Add new movie |
| GET | `/api/v3/movie/{id}` | ✅ Working | Get movie by ID |
| PUT | `/api/v3/movie/{id}` | ✅ Working | Update movie |
| DELETE | `/api/v3/movie/{id}` | ✅ Working | Delete movie |

### ❌ Missing Critical Endpoints (85% of functionality)

| Method | Endpoint | Status | Priority |
|--------|----------|--------|----------|
| GET | `/api/v3/movie/lookup` | ❌ Missing | Critical |
| GET | `/api/v3/indexer` | ❌ Missing | Critical |
| GET | `/api/v3/downloadclient` | ❌ Missing | Critical |
| GET | `/api/v3/qualityprofile` | ❌ Missing | High |
| GET | `/api/v3/customformat` | ❌ Missing | High |
| GET | `/api/v3/queue` | ❌ Missing | Critical |
| GET | `/api/v3/calendar` | ❌ Missing | Medium |
| GET | `/api/v3/system/status` | ❌ Missing | Medium |
| POST | `/api/v3/command` | ❌ Missing | Critical |
| GET | `/api/v3/history` | ❌ Missing | Medium |
| GET | `/api/v3/config` | ❌ Missing | High |

## API Architecture Overview

### Current Implementation (Limited)

```rust
// Current working API structure
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub tmdb_client: Arc<TmdbClient>,
}

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/api/v3/movie", get(list_movies).post(create_movie))
        .route(
            "/api/v3/movie/:id",
            get(get_movie).put(update_movie).delete(delete_movie),
        )
        .with_state(state)
}

// Working endpoint implementations
pub async fn list_movies(
    State(state): State<AppState>,
) -> Result<Json<Vec<Movie>>, ApiError> {
    let movies = state
        .movie_repository
        .list(ListCriteria::default())
        .await
        .map_err(ApiError::from)?;
    
    Ok(Json(movies))
}

pub async fn create_movie(
    State(state): State<AppState>,
    Json(request): Json<CreateMovieRequest>,
) -> Result<(StatusCode, Json<Movie>), ApiError> {
    // 1. Validate request
    request.validate()?;
    
    // 2. Fetch metadata from TMDB
    let tmdb_movie = state
        .tmdb_client
        .get_movie_details(request.tmdb_id)
        .await
        .map_err(ApiError::from)?;
    
    // 3. Create domain movie
    let movie = Movie::from_tmdb(tmdb_movie, request.quality_profile_id)?;
    
    // 4. Save to database
    let saved_movie = state
        .movie_repository
        .save(movie)
        .await
        .map_err(ApiError::from)?;
    
    Ok((StatusCode::CREATED, Json(saved_movie)))
}

pub async fn get_movie(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<Movie>, ApiError> {
    let movie = state
        .movie_repository
        .get_by_id(id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound(format!("Movie with id {} not found", id)))?;
    
    Ok(Json(movie))
}
```

### ❌ Compilation Issues Blocking Full Implementation

```bash
# Current build failure prevents testing API
$ cargo build --workspace
   Compiling infrastructure v0.1.0
error[E0432]: unresolved import `crate::error::InfrastructureError`
   --> infrastructure/src/lib.rs:12:5
    |
12 | use crate::error::InfrastructureError;
    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

[... 163 more compilation errors preventing API server startup ...]
```

## Radarr v3 API Compatibility Analysis

### Official Radarr API Structure

```yaml
# Official Radarr v3 API endpoints (for reference)
Movies:
  - GET    /api/v3/movie                    # List movies
  - POST   /api/v3/movie                    # Add movie
  - GET    /api/v3/movie/{id}               # Get movie
  - PUT    /api/v3/movie/{id}               # Update movie
  - DELETE /api/v3/movie/{id}               # Delete movie
  - GET    /api/v3/movie/lookup             # Search TMDB
  - PUT    /api/v3/movie/editor             # Bulk edit
  - DELETE /api/v3/movie/editor             # Bulk delete

Queue:
  - GET    /api/v3/queue                    # Download queue
  - DELETE /api/v3/queue/{id}               # Remove from queue
  - POST   /api/v3/queue/grab/{id}          # Manual grab
  
Indexers:
  - GET    /api/v3/indexer                  # List indexers
  - POST   /api/v3/indexer                  # Add indexer
  - GET    /api/v3/indexer/{id}             # Get indexer
  - PUT    /api/v3/indexer/{id}             # Update indexer
  - DELETE /api/v3/indexer/{id}             # Delete indexer
  - POST   /api/v3/indexer/test             # Test indexer

Download Clients:
  - GET    /api/v3/downloadclient           # List clients
  - POST   /api/v3/downloadclient           # Add client
  - GET    /api/v3/downloadclient/{id}      # Get client
  - PUT    /api/v3/downloadclient/{id}      # Update client
  - DELETE /api/v3/downloadclient/{id}      # Delete client
  - POST   /api/v3/downloadclient/test      # Test client

System:
  - GET    /api/v3/system/status            # System status
  - GET    /api/v3/system/health            # Health check
  - GET    /api/v3/config                   # Configuration
  - GET    /api/v3/calendar                 # Calendar events
  - GET    /api/v3/history                  # Download history
```

### Compatibility Matrix

| Endpoint Category | Official Radarr | Rust MVP | Compatibility |
|------------------|----------------|----------|---------------|
| **Basic Movie CRUD** | ✅ Complete | ✅ Working | 100% |
| **Movie Search** | ✅ Complete | ❌ Missing | 0% |
| **Queue Management** | ✅ Complete | ❌ Missing | 0% |
| **Indexer Management** | ✅ Complete | ❌ Missing | 0% |
| **Download Clients** | ✅ Complete | ❌ Missing | 0% |
| **System Information** | ✅ Complete | ❌ Missing | 0% |
| **Configuration** | ✅ Complete | ❌ Missing | 0% |
| **Calendar** | ✅ Complete | ❌ Missing | 0% |
| **History** | ✅ Complete | ❌ Missing | 0% |
| **Commands** | ✅ Complete | ❌ Missing | 0% |

**Overall API Compatibility**: ~15%

## Request/Response Models

### ✅ Working Models (Movie CRUD)

```rust
// Request models for working endpoints
#[derive(Debug, Deserialize, Validate)]
pub struct CreateMovieRequest {
    #[validate(range(min = 1))]
    pub tmdb_id: u32,
    
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    
    pub year: Option<u16>,
    
    pub quality_profile_id: u32,
    
    pub monitoring: bool,
    
    pub minimum_availability: MinimumAvailability,
    
    pub root_folder_path: String,
    
    pub tags: Vec<u32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMovieRequest {
    pub title: Option<String>,
    pub year: Option<u16>,
    pub quality_profile_id: Option<u32>,
    pub monitoring: Option<bool>,
    pub minimum_availability: Option<MinimumAvailability>,
    pub tags: Option<Vec<u32>>,
}

#[derive(Debug, Serialize)]
pub struct MovieResponse {
    pub id: u32,
    pub tmdb_id: u32,
    pub title: String,
    pub year: Option<u16>,
    pub runtime: Option<u32>,
    pub overview: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub genres: Vec<String>,
    pub certification: Option<String>,
    pub quality_profile_id: u32,
    pub monitoring: bool,
    pub minimum_availability: MinimumAvailability,
    pub has_file: bool,
    pub movie_file: Option<MovieFileResponse>,
    pub images: Vec<ImageResponse>,
    pub tags: Vec<u32>,
    pub added: DateTime<Utc>,
    pub path: String,
    pub size_on_disk: u64,
}

#[derive(Debug, Serialize)]
pub struct MovieFileResponse {
    pub id: u32,
    pub movie_id: u32,
    pub relative_path: String,
    pub size: u64,
    pub date_added: DateTime<Utc>,
    pub scene_name: Option<String>,
    pub quality: QualityResponse,
    pub custom_formats: Vec<CustomFormatResponse>,
    pub edition: Option<String>,
}
```

### ❌ Missing Critical Models

```rust
// Models that need to be implemented

#[derive(Debug, Serialize, Deserialize)]
pub struct MovieLookupRequest {
    pub term: String,
    pub year: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct MovieLookupResponse {
    pub tmdb_id: u32,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub images: Vec<ImageResponse>,
    pub genres: Vec<String>,
    pub ratings: RatingResponse,
    pub runtime: Option<u32>,
    pub certification: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub website: Option<String>,
    pub youtube_trailer_id: Option<String>,
    pub status: MovieStatus,
    pub recommendations: Vec<MovieRecommendation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueueResponse {
    pub page: u32,
    pub page_size: u32,
    pub sort_key: String,
    pub sort_direction: SortDirection,
    pub total_records: u32,
    pub records: Vec<QueueItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct QueueItemResponse {
    pub id: u32,
    pub movie: MovieResponse,
    pub quality: QualityResponse,
    pub custom_formats: Vec<CustomFormatResponse>,
    pub size: u64,
    pub title: String,
    pub sizel: u64,
    pub status: DownloadStatus,
    pub tracking_id: String,
    pub status_messages: Vec<TrackedDownloadStatusMessage>,
    pub error_message: Option<String>,
    pub download_id: String,
    pub estimatedCompletionTime: Option<DateTime<Utc>>,
    pub timeleft: Option<Duration>,
    pub added: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexerResponse {
    pub id: u32,
    pub name: String,
    pub implementation: String,
    pub implementation_name: String,
    pub config_contract: String,
    pub info_link: Option<String>,
    pub tags: Vec<u32>,
    pub fields: Vec<FieldResponse>,
    pub enable_rss: bool,
    pub enable_automatic_search: bool,
    pub enable_interactive_search: bool,
    pub supports_rss: bool,
    pub supports_search: bool,
    pub protocol: IndexerProtocol,
    pub priority: u32,
    pub download_client_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadClientResponse {
    pub id: u32,
    pub name: String,
    pub implementation: String,
    pub implementation_name: String,
    pub config_contract: String,
    pub info_link: Option<String>,
    pub tags: Vec<u32>,
    pub fields: Vec<FieldResponse>,
    pub enable: bool,
    pub protocol: DownloadProtocol,
    pub priority: u32,
    pub remove_completed_downloads: bool,
    pub remove_failed_downloads: bool,
}

#[derive(Debug, Serialize)]
pub struct SystemStatusResponse {
    pub app_name: String,
    pub instance_name: String,
    pub version: String,
    pub build_time: DateTime<Utc>,
    pub is_debug: bool,
    pub is_production: bool,
    pub is_admin: bool,
    pub is_user_interactive: bool,
    pub startup_path: String,
    pub app_data: String,
    pub os_name: String,
    pub os_version: String,
    pub is_mono_runtime: bool,
    pub is_mono: bool,
    pub is_linux: bool,
    pub is_osx: bool,
    pub is_windows: bool,
    pub mode: RuntimeMode,
    pub branch: String,
    pub authentication: AuthenticationType,
    pub sqlite_version: String,
    pub migration_version: u32,
    pub url_base: String,
    pub runtime_version: String,
}
```

## Error Handling & HTTP Status Codes

### ✅ Current Error Handling (Working)

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Conflict: {0}")]
    Conflict(String),
    
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::InternalServerError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg)
            }
            ApiError::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, msg)
            }
        };
        
        let body = Json(serde_json::json!({
            "error": error_message,
            "status": status.as_u16()
        }));
        
        (status, body).into_response()
    }
}

// Conversion from various error types
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                ApiError::NotFound("Resource not found".to_string())
            }
            sqlx::Error::Database(db_err) => {
                if db_err.constraint().is_some() {
                    ApiError::Conflict("Constraint violation".to_string())
                } else {
                    ApiError::InternalServerError(db_err.to_string())
                }
            }
            _ => ApiError::InternalServerError(err.to_string()),
        }
    }
}

impl From<TmdbError> for ApiError {
    fn from(err: TmdbError) -> Self {
        match err {
            TmdbError::NotFound => ApiError::NotFound("Movie not found on TMDB".to_string()),
            TmdbError::Unauthorized => {
                ApiError::InternalServerError("TMDB API key invalid".to_string())
            }
            TmdbError::RateLimited { .. } => {
                ApiError::ServiceUnavailable("TMDB rate limit exceeded".to_string())
            }
            _ => ApiError::InternalServerError(err.to_string()),
        }
    }
}
```

### HTTP Status Code Usage

| Status Code | Usage | Implementation |
|-------------|-------|----------------|
| **200 OK** | Successful GET, PUT | ✅ Implemented |
| **201 Created** | Successful POST | ✅ Implemented |
| **204 No Content** | Successful DELETE | ✅ Implemented |
| **400 Bad Request** | Validation errors | ✅ Implemented |
| **401 Unauthorized** | Authentication failure | ❌ Missing (no auth) |
| **403 Forbidden** | Authorization failure | ❌ Missing (no auth) |
| **404 Not Found** | Resource not found | ✅ Implemented |
| **409 Conflict** | Duplicate resource | ✅ Implemented |
| **422 Unprocessable Entity** | Validation failure | 🟡 Partial |
| **429 Too Many Requests** | Rate limiting | ❌ Missing |
| **500 Internal Server Error** | Server errors | ✅ Implemented |
| **503 Service Unavailable** | External service down | ✅ Implemented |

## API Testing Strategy

### ❌ Current Status: Cannot Test Due to Build Failures

```rust
// Planned API testing framework
#[cfg(test)]
mod api_tests {
    use super::*;
    use axum_test::TestServer;
    use serde_json::json;
    
    async fn setup_test_server() -> TestServer {
        let app_state = AppState {
            db_pool: create_test_db_pool().await,
            tmdb_client: Arc::new(create_mock_tmdb_client()),
        };
        
        let app = create_app(app_state);
        TestServer::new(app).unwrap()
    }
    
    #[tokio::test]
    async fn test_create_movie_success() {
        let server = setup_test_server().await;
        
        let response = server
            .post("/api/v3/movie")
            .json(&json!({
                "tmdb_id": 603,
                "title": "The Matrix",
                "year": 1999,
                "quality_profile_id": 1,
                "monitoring": true,
                "minimum_availability": "announced",
                "root_folder_path": "/movies",
                "tags": []
            }))
            .await;
        
        response.assert_status_success();
        response.assert_status(StatusCode::CREATED);
        
        let movie: MovieResponse = response.json();
        assert_eq!(movie.tmdb_id, 603);
        assert_eq!(movie.title, "The Matrix");
        assert_eq!(movie.year, Some(1999));
    }
    
    #[tokio::test]
    async fn test_create_movie_validation_error() {
        let server = setup_test_server().await;
        
        let response = server
            .post("/api/v3/movie")
            .json(&json!({
                "tmdb_id": 0,  // Invalid TMDB ID
                "title": "",   // Empty title
            }))
            .await;
        
        response.assert_status(StatusCode::BAD_REQUEST);
        
        let error: serde_json::Value = response.json();
        assert!(error["error"].as_str().unwrap().contains("validation"));
    }
    
    #[tokio::test]
    async fn test_get_movie_not_found() {
        let server = setup_test_server().await;
        
        let response = server
            .get("/api/v3/movie/999999")
            .await;
        
        response.assert_status(StatusCode::NOT_FOUND);
        
        let error: serde_json::Value = response.json();
        assert!(error["error"].as_str().unwrap().contains("not found"));
    }
    
    #[tokio::test]
    async fn test_list_movies_pagination() {
        let server = setup_test_server().await;
        
        // Create test movies
        for i in 1..=25 {
            server
                .post("/api/v3/movie")
                .json(&json!({
                    "tmdb_id": i,
                    "title": format!("Test Movie {}", i),
                    "quality_profile_id": 1,
                    "monitoring": true,
                    "minimum_availability": "announced",
                    "root_folder_path": "/movies",
                    "tags": []
                }))
                .await
                .assert_status_success();
        }
        
        // Test pagination
        let response = server
            .get("/api/v3/movie?page=1&pageSize=10&sortKey=title&sortDir=asc")
            .await;
        
        response.assert_status_success();
        
        let page: PaginatedResponse<MovieResponse> = response.json();
        assert_eq!(page.records.len(), 10);
        assert_eq!(page.total_records, 25);
        assert_eq!(page.page, 1);
    }
}
```

## Missing Endpoint Implementation Plans

### Critical Missing Endpoints (Priority 1)

#### 1. Movie Lookup Endpoint
```rust
// GET /api/v3/movie/lookup
pub async fn lookup_movies(
    Query(params): Query<MovieLookupQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<MovieLookupResponse>>, ApiError> {
    let tmdb_results = state
        .tmdb_client
        .search_movies(&params.term, params.year)
        .await
        .map_err(ApiError::from)?;
    
    let lookup_results: Vec<MovieLookupResponse> = tmdb_results
        .into_iter()
        .map(|tmdb_movie| MovieLookupResponse::from(tmdb_movie))
        .collect();
    
    Ok(Json(lookup_results))
}
```

#### 2. Queue Management Endpoints
```rust
// GET /api/v3/queue
pub async fn get_queue(
    Query(params): Query<QueueQuery>,
    State(state): State<AppState>,
) -> Result<Json<QueueResponse>, ApiError> {
    let queue_items = state
        .download_queue
        .list(params.into())
        .await
        .map_err(ApiError::from)?;
    
    let response = QueueResponse {
        page: params.page.unwrap_or(1),
        page_size: params.page_size.unwrap_or(20),
        sort_key: params.sort_key.unwrap_or("added".to_string()),
        sort_direction: params.sort_dir.unwrap_or(SortDirection::Desc),
        total_records: queue_items.total as u32,
        records: queue_items.items.into_iter().map(Into::into).collect(),
    };
    
    Ok(Json(response))
}

// DELETE /api/v3/queue/{id}
pub async fn remove_from_queue(
    Path(id): Path<u32>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state
        .download_queue
        .remove(id)
        .await
        .map_err(ApiError::from)?;
    
    Ok(StatusCode::NO_CONTENT)
}
```

#### 3. System Status Endpoint
```rust
// GET /api/v3/system/status
pub async fn get_system_status(
    State(state): State<AppState>,
) -> Result<Json<SystemStatusResponse>, ApiError> {
    let status = SystemStatusResponse {
        app_name: "Radarr MVP".to_string(),
        instance_name: state.config.instance_name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_time: build_info::BUILD_TIME.parse().unwrap(),
        is_debug: cfg!(debug_assertions),
        is_production: !cfg!(debug_assertions),
        is_admin: true, // TODO: Implement proper auth
        is_user_interactive: false,
        startup_path: env!("PWD").to_string(),
        app_data: state.config.data_directory.clone(),
        os_name: std::env::consts::OS.to_string(),
        os_version: get_os_version(),
        is_mono_runtime: false,
        is_mono: false,
        is_linux: cfg!(target_os = "linux"),
        is_osx: cfg!(target_os = "macos"),
        is_windows: cfg!(target_os = "windows"),
        mode: RuntimeMode::Normal,
        branch: "main".to_string(),
        authentication: AuthenticationType::None, // TODO: Implement auth
        sqlite_version: "N/A".to_string(), // Using PostgreSQL
        migration_version: get_current_migration_version(&state.db_pool).await?,
        url_base: state.config.url_base.clone(),
        runtime_version: rustc_version_runtime::version().to_string(),
    };
    
    Ok(Json(status))
}
```

### High Priority Endpoints (Priority 2)

#### Indexer Management
```rust
// GET /api/v3/indexer
pub async fn list_indexers(
    State(state): State<AppState>,
) -> Result<Json<Vec<IndexerResponse>>, ApiError> {
    let indexers = state
        .indexer_service
        .list_all()
        .await
        .map_err(ApiError::from)?;
    
    let responses: Vec<IndexerResponse> = indexers
        .into_iter()
        .map(Into::into)
        .collect();
    
    Ok(Json(responses))
}

// POST /api/v3/indexer/test
pub async fn test_indexer(
    Json(request): Json<TestIndexerRequest>,
    State(state): State<AppState>,
) -> Result<Json<TestResult>, ApiError> {
    let result = state
        .indexer_service
        .test_indexer(request.into())
        .await
        .map_err(ApiError::from)?;
    
    Ok(Json(result.into()))
}
```

#### Download Client Management
```rust
// GET /api/v3/downloadclient
pub async fn list_download_clients(
    State(state): State<AppState>,
) -> Result<Json<Vec<DownloadClientResponse>>, ApiError> {
    let clients = state
        .download_client_service
        .list_all()
        .await
        .map_err(ApiError::from)?;
    
    let responses: Vec<DownloadClientResponse> = clients
        .into_iter()
        .map(Into::into)
        .collect();
    
    Ok(Json(responses))
}

// POST /api/v3/downloadclient/test
pub async fn test_download_client(
    Json(request): Json<TestDownloadClientRequest>,
    State(state): State<AppState>,
) -> Result<Json<TestResult>, ApiError> {
    let result = state
        .download_client_service
        .test_client(request.into())
        .await
        .map_err(ApiError::from)?;
    
    Ok(Json(result.into()))
}
```

## API Documentation Strategy

### ❌ Missing: OpenAPI/Swagger Documentation

```rust
// Planned OpenAPI integration
use utoipa::{OpenApi, ToSchema};

#[derive(OpenApi)]
#[openapi(
    paths(
        list_movies,
        create_movie,
        get_movie,
        update_movie,
        delete_movie,
        lookup_movies,
        get_queue,
        get_system_status
    ),
    components(
        schemas(
            MovieResponse,
            CreateMovieRequest,
            UpdateMovieRequest,
            MovieLookupResponse,
            QueueResponse,
            SystemStatusResponse,
            ApiError
        )
    ),
    tags(
        (name = "movies", description = "Movie management endpoints"),
        (name = "queue", description = "Download queue management"),
        (name = "system", description = "System information and status")
    ),
    info(
        title = "Radarr MVP API",
        version = "0.1.0",
        description = "A modern Rust implementation of the Radarr API",
        contact(
            name = "Radarr MVP Team",
            email = "contact@radarr-mvp.com"
        )
    )
)]
pub struct ApiDoc;

// Auto-generate OpenAPI documentation
pub fn create_app_with_docs(state: AppState) -> Router {
    Router::new()
        .merge(create_app(state))
        .route("/docs/openapi.json", get(|| async {
            Json(ApiDoc::openapi())
        }))
        .route("/docs", get(|| async {
            Html(include_str!("../static/swagger-ui.html"))
        }))
}
```

## Performance & Optimization

### API Performance Targets

| Endpoint | Target Latency | Current Status |
|----------|---------------|----------------|
| `GET /api/v3/movie` | <50ms | ❌ Cannot measure |
| `POST /api/v3/movie` | <300ms | ❌ Cannot measure |
| `GET /api/v3/movie/{id}` | <20ms | ❌ Cannot measure |
| `GET /api/v3/movie/lookup` | <400ms | ❌ Not implemented |
| `GET /api/v3/queue` | <100ms | ❌ Not implemented |
| `GET /api/v3/system/status` | <10ms | ❌ Not implemented |

### ❌ Planned Performance Optimizations

```rust
// Response caching middleware
pub async fn cache_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let cache_key = generate_cache_key(&req);
    
    // Check cache first
    if let Some(cached_response) = CACHE.get(&cache_key).await {
        return Ok(cached_response);
    }
    
    // Execute request
    let response = next.run(req).await?;
    
    // Cache successful responses
    if response.status().is_success() {
        CACHE.insert(cache_key, response.clone(), Duration::from_secs(300)).await;
    }
    
    Ok(response)
}

// Request rate limiting
pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = get_client_ip(&req);
    
    if !RATE_LIMITER.check_rate_limit(client_ip).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    
    next.run(req).await
}

// Request/response compression
pub fn create_optimized_app(state: AppState) -> Router {
    Router::new()
        .layer(CompressionLayer::new())
        .layer(middleware::from_fn(cache_middleware))
        .layer(middleware::from_fn(rate_limit_middleware))
        .layer(CorsLayer::permissive())
        .merge(create_app(state))
}
```

## API Development Roadmap

### Phase 1: Fix Build Issues & Core API (Week 1-2)
1. **Resolve Compilation Errors**: Fix 164 infrastructure errors blocking API
2. **Basic API Testing**: Set up API test infrastructure
3. **Error Handling**: Complete error handling and HTTP status codes
4. **Documentation**: Basic API documentation

### Phase 2: Critical Endpoints (Week 3-4)
1. **Movie Lookup**: Implement TMDB search endpoint
2. **System Status**: System information and health endpoints
3. **Basic Queue**: Download queue viewing (read-only)
4. **Authentication**: Basic API key authentication

### Phase 3: Management Endpoints (Week 5-6)
1. **Indexer Management**: Full CRUD for indexers
2. **Download Client Management**: Full CRUD for download clients
3. **Queue Management**: Add/remove queue items
4. **Configuration**: System configuration endpoints

### Phase 4: Advanced Features (Week 7-8)
1. **Calendar Endpoints**: Movie release calendar
2. **History Endpoints**: Download and import history
3. **Command System**: Background task management
4. **Bulk Operations**: Bulk movie operations

### Success Criteria

- ✅ All critical endpoints implemented and tested
- ✅ >90% compatibility with official Radarr v3 API
- ✅ <100ms average response time for core endpoints
- ✅ Comprehensive error handling and validation
- ✅ OpenAPI documentation complete
- ✅ API test coverage >85%
- ✅ Authentication and authorization working
- ✅ Rate limiting and security measures active

**Critical Blocker**: API development is completely blocked by 164 compilation errors in the infrastructure layer. All API work must begin with resolving these build issues.