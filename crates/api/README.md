# Radarr API

HTTP API layer providing REST endpoints for the Radarr movie automation system. Built with Axum for high-performance async HTTP handling with comprehensive observability, security, and validation features.

## Features

- **REST API**: Complete Radarr v3 API specification compatibility
- **Async HTTP**: High-performance async request handling with Axum
- **OpenTelemetry**: Distributed tracing and metrics collection
- **Security**: CORS, rate limiting, and security headers
- **Validation**: Request/response validation with detailed error messages
- **Middleware**: Custom middleware for authentication, logging, and error handling
- **Web UI**: Template-based web interface with Askama
- **Metrics**: Prometheus metrics integration
- **Error Handling**: Structured error responses with proper HTTP status codes

## Key Dependencies

- **axum**: Modern async web framework with excellent performance
- **tower/tower-http**: Middleware ecosystem for HTTP services
- **serde**: JSON serialization for request/response bodies
- **uuid**: Unique identifier handling in API paths
- **tracing**: Structured logging and request tracing
- **validator**: Request validation with derive macros
- **askama**: Type-safe HTML templating engine
- **opentelemetry**: Observability and distributed tracing

## API Endpoints

### Movie Management

```bash
# Get all movies with pagination
GET /api/v3/movie?page=1&pageSize=50&sortKey=title&sortDir=asc

# Get movie by ID
GET /api/v3/movie/{id}

# Add new movie
POST /api/v3/movie
Content-Type: application/json
{
  "title": "The Matrix",
  "year": 1999,
  "tmdbId": 603
}

# Update movie
PUT /api/v3/movie/{id}

# Delete movie
DELETE /api/v3/movie/{id}
```

### Queue Management

```bash
# Get download queue
GET /api/v3/queue

# Remove item from queue
DELETE /api/v3/queue/{id}

# Grab release (add to queue)
POST /api/v3/queue/grab
```

### System Endpoints

```bash
# Health check
GET /api/v3/health

# System status
GET /api/v3/system/status

# API documentation
GET /api/v3/docs
```

## Request/Response Models

### Movie Resource

```rust
use radarr_api::models::{MovieResource, MovieStatus, MinimumAvailability};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct MovieResource {
    pub id: Uuid,
    pub title: String,
    pub year: i32,
    pub status: MovieStatus,
    pub minimum_availability: MinimumAvailability,
    pub monitored: bool,
    pub overview: Option<String>,
    pub added: DateTime<Utc>,
    // ... other fields
}
```

### Pagination

```rust
use radarr_api::models::PaginationParams;

// Automatic query parameter parsing
#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: u32,           // default: 1
    pub page_size: u32,      // default: 50, max: 1000
    pub sort_key: Option<String>,
    pub sort_dir: Option<SortDirection>,
}
```

## Error Handling

Structured error responses with appropriate HTTP status codes:

```rust
use radarr_api::{ApiError, ApiResult};

// Comprehensive error types
pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    Forbidden,
    NotFound(String),
    Conflict(String),
    ValidationError(Vec<ValidationError>),
    InternalServerError(String),
}

// JSON error response format
{
  "error": "ValidationError",
  "message": "Invalid input data",
  "details": [
    {
      "field": "title",
      "message": "Title cannot be empty"
    }
  ]
}
```

## Middleware Stack

### Security Middleware

```rust
use radarr_api::security::{SecurityConfig, apply_security};

let security_config = SecurityConfig {
    cors_origins: vec!["http://localhost:3000".to_string()],
    api_keys: vec!["your-api-key".to_string()],
    rate_limit_rpm: Some(100),
};

let app = apply_security(router, security_config);
```

### Observability

```rust
use radarr_api::telemetry::{TelemetryConfig, init_telemetry};

let telemetry_config = TelemetryConfig {
    service_name: "radarr-api".to_string(),
    jaeger_endpoint: Some("http://localhost:14268".to_string()),
    metrics_enabled: true,
};

init_telemetry(telemetry_config).await?;
```

## Configuration

### Simple API Setup

```rust
use radarr_api::{create_simple_api_router, SimpleApiState};
use radarr_infrastructure::DatabasePool;
use std::sync::Arc;

// Create API state with dependencies
let state = SimpleApiState {
    database_pool: Arc::new(database_pool),
    // ... other dependencies
};

// Create router with all endpoints
let app = create_simple_api_router(state);

// Start server
let listener = tokio::net::TcpListener::bind("0.0.0.0:7878").await?;
axum::serve(listener, app).await?;
```

### Full API with Dependencies

```rust
use radarr_api::{ApiDependencies, create_api_router_with_deps};

let deps = ApiDependencies {
    database_pool,
    indexer_client,
    download_client,
    cors_origins: Some(vec!["*".to_string()]),
};

let app = create_api_router_with_deps(deps);
```

## Web UI

Built-in web interface using Askama templates:

```html
<!-- templates/movies.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Radarr - Movies</title>
    <link rel="stylesheet" href="/static/style.css">
</head>
<body>
    <h1>Movies ({{ movies.len() }})</h1>
    {% for movie in movies %}
        <div class="movie-card">
            <h3>{{ movie.title }} ({{ movie.year }})</h3>
            <p>Status: {{ movie.status }}</p>
        </div>
    {% endfor %}
</body>
</html>
```

## Testing

### Unit Tests

```bash
# Run API tests
cargo test -p radarr-api

# Run with integration tests
cargo test -p radarr-api --features integration-tests
```

### Integration Testing

```rust
use axum_test::TestServer;
use radarr_api::create_simple_api_router;

#[tokio::test]
async fn test_get_movies() {
    let app = create_simple_api_router(test_state());
    let server = TestServer::new(app).unwrap();
    
    let response = server
        .get("/api/v3/movie")
        .await;
        
    assert_eq!(response.status_code(), 200);
}
```

## Metrics and Monitoring

### Prometheus Metrics

```rust
use radarr_api::metrics::MetricsCollector;

// Built-in metrics
// - HTTP request duration histogram
// - Request count by endpoint and status
// - Active connections gauge
// - Database connection pool metrics

// Custom metrics
let metrics = MetricsCollector::new();
metrics.increment_counter("custom.events", &[("type", "movie_added")]);
```

### Tracing

```rust
use tracing::{info, instrument};

#[instrument(skip(state), fields(movie_id = %id))]
async fn get_movie(
    Path(id): Path<Uuid>,
    State(state): State<ApiState>,
) -> ApiResult<Json<MovieResource>> {
    info!("Getting movie");
    // ... implementation
}
```

## Performance

- **Async I/O**: Non-blocking request handling
- **Connection Pooling**: Efficient database connections
- **Compression**: Gzip/Brotli response compression  
- **Static File Serving**: Efficient static asset delivery
- **Request Timeouts**: Configurable timeout middleware
- **Rate Limiting**: Per-IP and per-API-key rate limiting

## Security Features

- **CORS**: Configurable Cross-Origin Resource Sharing
- **API Keys**: Header-based authentication
- **Input Validation**: Comprehensive request validation
- **Security Headers**: HSTS, CSP, X-Frame-Options
- **Request Size Limits**: Protection against large payloads
- **SQL Injection Protection**: Parameterized queries via SQLx