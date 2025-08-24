# Radarr Indexers

Torrent and NZB indexer integrations for the Radarr movie automation system. Provides unified access to multiple indexers through the Prowlarr API and direct indexer implementations with advanced features like rate limiting, health monitoring, and circuit breaker patterns.

## Features

- **Prowlarr Integration**: Unified access to 500+ indexers through Prowlarr API
- **Direct HDBits Integration**: Specialized high-quality private tracker integration
- **Rate Limiting**: Intelligent rate limiting to respect indexer limits
- **Circuit Breaker**: Automatic failure detection and recovery
- **Health Monitoring**: Real-time service health tracking and metrics
- **Retry Logic**: Configurable retry policies with exponential backoff
- **Search Capabilities**: Advanced movie search with quality and year filtering
- **Release Parsing**: Intelligent release name parsing and quality detection
- **Multi-Indexer Support**: Aggregate results from multiple indexers
- **Caching**: Response caching to reduce API calls

## Key Dependencies

- **reqwest**: HTTP client for indexer API communications
- **serde/serde_json**: JSON serialization for API requests/responses
- **url**: URL parsing and manipulation for API endpoints
- **chrono**: Date/time handling for release dates and search filters
- **regex**: Pattern matching for release name parsing
- **uuid**: Unique identifier generation for requests
- **md5**: Hash generation for authentication and caching
- **scraper**: HTML parsing for web scraping capabilities
- **async-trait**: Async trait implementations

## Prowlarr Integration

### Basic Setup

```rust
use radarr_indexers::{ProwlarrClient, ProwlarrConfig, IndexerClient};

// Create client from environment variables
let client = radarr_indexers::client_from_env()?;

// Or configure manually
let config = ProwlarrConfig::builder()
    .base_url("http://localhost:9696")
    .api_key("your-prowlarr-api-key")
    .timeout_seconds(30)
    .rate_limit_per_minute(60)
    .build()?;

let client = ProwlarrClient::new(config);
```

### Movie Search

```rust
use radarr_indexers::{MovieSearchRequest, IndexerResult};

// Search for movies across all configured indexers
let search_request = MovieSearchRequest {
    title: "The Matrix".to_string(),
    year: Some(1999),
    imdb_id: Some("tt0133093".to_string()),
    tmdb_id: Some(603),
    categories: vec![2000, 2010], // Movie categories
    limit: Some(100),
};

let results = client.search_movies(&search_request).await?;

for result in results {
    println!("Found: {} - {} MB", result.title, result.size_mb);
    println!("Indexer: {}, Seeds: {}", result.indexer, result.seeders);
}
```

### RSS Feed Monitoring

```rust
// Monitor RSS feeds for new releases
let rss_results = client.get_rss_feed(None).await?;

for release in rss_results {
    if release.matches_movie_criteria(&search_criteria) {
        println!("New release: {}", release.title);
    }
}
```

## HDBits Direct Integration

### Specialized Private Tracker Support

```rust
use radarr_indexers::{HDBitsClient, HDBitsConfig, MovieSearchRequest};

// Create HDBits client
let client = radarr_indexers::hdbits_client_from_env()?;

// Or configure manually
let config = HDBitsConfig {
    base_url: "https://hdbits.org".to_string(),
    username: "your_username".to_string(),
    passkey: "your_passkey".to_string(),
    timeout_seconds: 30,
};

let client = HDBitsClient::new(config);

// Search with HDBits-specific parameters
let search = MovieSearchRequest {
    title: "The Matrix".to_string(),
    year: Some(1999),
    // HDBits supports additional quality filters
    quality_filters: Some(vec!["1080p", "Remux"]),
    codec_filters: Some(vec!["x264", "x265"]),
};

let results = client.search_movies(&search).await?;
```

### Authentication & Session Management

```rust
// HDBits handles session management automatically
// Login is performed transparently when needed
let authenticated = client.ensure_authenticated().await?;

if authenticated {
    let results = client.search_movies(&search).await?;
}
```

## Health Monitoring & Service Management

### Service Health Tracking

```rust
use radarr_indexers::{ServiceHealth, HealthStatus, ServiceMetrics};

// Check individual indexer health
let health = client.check_health().await?;

match health.status {
    HealthStatus::Healthy => {
        println!("Service operational - {}ms average response", 
                 health.metrics.avg_response_time_ms);
    }
    HealthStatus::Degraded => {
        println!("Service degraded - {} recent failures", 
                 health.metrics.failure_count_24h);
    }
    HealthStatus::Unhealthy => {
        println!("Service unavailable - last success: {}", 
                 health.metrics.last_success);
    }
}

// Get detailed metrics
let metrics = client.get_service_metrics().await?;
println!("Success rate: {:.2}%", metrics.success_rate_percentage);
println!("Rate limit usage: {}/{}", metrics.requests_this_hour, metrics.rate_limit_per_hour);
```

### Circuit Breaker Pattern

```rust
use radarr_indexers::CircuitBreakerState;

// Circuit breaker automatically handles failures
let results = client.search_movies(&search).await;

match results {
    Ok(releases) => println!("Found {} releases", releases.len()),
    Err(e) if e.is_circuit_breaker_open() => {
        println!("Circuit breaker open, service temporarily disabled");
        // Fallback to other indexers or cached results
    }
    Err(e) => println!("Search failed: {}", e),
}
```

## Multi-Indexer Aggregation

### Combining Multiple Sources

```rust
use radarr_indexers::{MultiIndexerService, MultiIndexerConfig, IndexerSearchResult};

let config = MultiIndexerConfig {
    prowlarr: Some(prowlarr_config),
    hdbits: Some(hdbits_config),
    // Future: support for additional direct integrations
    max_concurrent_searches: 5,
    timeout_seconds: 45,
};

let multi_indexer = MultiIndexerService::new(config);

// Search across all configured indexers
let aggregated_results = multi_indexer.search_all_indexers(&search).await?;

// Results are automatically deduplicated and ranked
for result in aggregated_results.ranked_results {
    println!("{} - Quality Score: {:.2}", result.title, result.quality_score);
}
```

## Release Models

### Core Release Information

```rust
use radarr_indexers::{IndexerResult, Quality, ReleaseType};

pub struct IndexerResult {
    pub title: String,                    // Release title
    pub size_bytes: u64,                  // File size in bytes  
    pub download_url: String,             // Download link
    pub info_url: Option<String>,         // Info page URL
    pub indexer: String,                  // Source indexer name
    pub seeders: Option<u32>,             // Seeder count
    pub leechers: Option<u32>,            // Leecher count
    pub published: chrono::DateTime<chrono::Utc>, // Publish time
    pub quality: Quality,                 // Detected quality
    pub release_type: ReleaseType,        // Movie/TV classification
    pub categories: Vec<u32>,             // Indexer categories
}
```

### Quality Detection

```rust
use radarr_indexers::{Quality, parse_quality_from_title};

// Automatic quality detection from release titles
let quality = parse_quality_from_title("The.Matrix.1999.1080p.BluRay.x264-GROUP")?;
assert_eq!(quality, Quality::Bluray1080p);

// Supported qualities include:
// - WebDL480p, WebDL720p, WebDL1080p, WebDL4K
// - Bluray480p, Bluray720p, Bluray1080p, Bluray4K
// - HDTV720p, HDTV1080p
// - DVD, SDTV, Cam, Telesync, Telecine
```

## Configuration

### Environment Variables

```bash
# Prowlarr configuration
PROWLARR_URL=http://localhost:9696
PROWLARR_API_KEY=your-api-key

# HDBits configuration  
HDBITS_USERNAME=your_username
HDBITS_PASSKEY=your_passkey

# Optional settings
INDEXER_TIMEOUT_SECONDS=30
INDEXER_RATE_LIMIT_PER_MINUTE=60
INDEXER_MAX_RETRIES=3
```

### Configuration Builder

```rust
use radarr_indexers::ProwlarrConfigBuilder;

let config = ProwlarrConfigBuilder::new()
    .base_url("http://prowlarr:9696")
    .api_key("api-key")
    .timeout_seconds(45)
    .rate_limit_per_minute(100)
    .max_retries(5)
    .retry_delay_seconds(2)
    .user_agent("Radarr/1.0")
    .build()?;
```

## Testing

### Unit Tests

```bash
# Run all indexer tests
cargo test -p radarr-indexers

# Run specific test modules
cargo test -p radarr-indexers hdbits::tests
cargo test -p radarr-indexers prowlarr::tests
```

### Mock Indexer Client

```rust
use radarr_indexers::MockIndexerClient;
use mockall::predicate::eq;

let mut mock_client = MockIndexerClient::new();

mock_client
    .expect_search_movies()
    .with(eq(search_request))
    .times(1)
    .returning(|_| Ok(mock_results()));

// Use mock client in tests
let results = mock_client.search_movies(&search).await?;
```

### Integration Tests

```rust
#[tokio::test]
async fn test_prowlarr_integration() {
    let client = radarr_indexers::client_from_env()
        .expect("Prowlarr configuration required");
        
    let search = MovieSearchRequest {
        title: "The Matrix".to_string(),
        year: Some(1999),
        ..Default::default()
    };
    
    let results = client.search_movies(&search).await
        .expect("Search should succeed");
        
    assert!(!results.is_empty());
    
    // Verify result structure
    let first_result = &results[0];
    assert!(!first_result.title.is_empty());
    assert!(first_result.size_bytes > 0);
}
```

## Error Handling

### Comprehensive Error Types

```rust
use radarr_indexers::IndexerError;

pub enum IndexerError {
    Authentication(String),           // Auth failures
    RateLimit(String),               // Rate limit exceeded
    ServiceUnavailable(String),      // Indexer down/unreachable
    InvalidRequest(String),          // Bad request parameters
    ParseError(String),              // Response parsing failed
    CircuitBreakerOpen(String),      // Circuit breaker protection
    Configuration(String),           // Invalid configuration
    Network(reqwest::Error),         // Network/HTTP errors
}

// Error handling in application code
match client.search_movies(&search).await {
    Ok(results) => process_results(results),
    Err(IndexerError::RateLimit(_)) => {
        // Back off and retry later
        tokio::time::sleep(Duration::from_secs(60)).await;
        retry_search().await?
    }
    Err(IndexerError::CircuitBreakerOpen(_)) => {
        // Use fallback indexer or cached results
        use_fallback_results()
    }
    Err(e) => return Err(e),
}
```

## Performance & Reliability

- **Rate Limiting**: Automatic rate limit detection and backoff
- **Circuit Breaker**: Fail-fast behavior for unhealthy services
- **Concurrent Searches**: Parallel search across multiple indexers
- **Response Caching**: Intelligent caching to reduce API calls
- **Connection Pooling**: Reuse HTTP connections for efficiency
- **Request Deduplication**: Avoid duplicate concurrent requests
- **Retry Logic**: Exponential backoff for transient failures
- **Health Monitoring**: Proactive service health tracking