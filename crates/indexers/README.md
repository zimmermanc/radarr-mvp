# Radarr Indexers

Production-ready Prowlarr API client for torrent and NZB indexer integration.

## Features

- **Full Prowlarr API Support**: Search, indexer management, health checking
- **Rate Limiting**: Built-in protection against indexer bans
- **Async/Await**: Non-blocking operations with proper error handling
- **Type Safety**: Strongly typed models for all API responses
- **Production Ready**: Comprehensive logging, timeouts, and resilience
- **Testing Support**: Mock client for unit testing
- **Flexible Configuration**: Environment variables or builder pattern

## Quick Start

### Basic Usage

```rust
use radarr_indexers::{ProwlarrClient, ProwlarrConfigBuilder, SearchRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the client
    let config = ProwlarrConfigBuilder::new()
        .base_url("http://localhost:9696")
        .api_key("your-api-key-here")
        .timeout(30)
        .rate_limit(60) // 60 requests per minute
        .build();
    
    let client = ProwlarrClient::new(config)?;
    
    // Search for a movie by IMDB ID
    let request = SearchRequest::for_movie_imdb("tt0111161")
        .with_min_seeders(5)
        .with_limit(25);
    
    let results = client.search(&request).await?;
    
    println!("Found {} releases", results.total);
    for release in results.results {
        println!("  - {}", release.title);
    }
    
    Ok(())
}
```

### Environment Configuration

```rust
use radarr_indexers::client_from_env;

// Set environment variables:
// PROWLARR_BASE_URL=http://localhost:9696
// PROWLARR_API_KEY=your-api-key

let client = client_from_env()?;
```

## Search Methods

### By IMDB ID (Recommended)
```rust
let request = SearchRequest::for_movie_imdb("tt0111161");
```

### By TMDB ID
```rust
let request = SearchRequest::for_movie_tmdb(278);
```

### By Title
```rust
let request = SearchRequest::for_title("The Shawshank Redemption");
```

### Advanced Search Options
```rust
let request = SearchRequest::for_movie_imdb("tt0111161")
    .with_min_seeders(10)           // Minimum seeders
    .with_limit(50)                 // Result limit
    .with_indexers(vec![1, 2, 3]);  // Specific indexers
```

## Indexer Management

### List All Indexers
```rust
let indexers = client.get_indexers().await?;
for indexer in indexers {
    println!("{}: {} (enabled: {})", 
        indexer.id, indexer.name, indexer.enable);
}
```

### Test Indexer Connectivity
```rust
let is_healthy = client.test_indexer(1).await?;
if is_healthy {
    println!("Indexer is working correctly");
}
```

### Health Checks
```rust
let is_healthy = client.health_check().await?;
```

## Configuration Options

| Option | Default | Description |
|--------|---------|-------------|
| `base_url` | `http://localhost:9696` | Prowlarr instance URL |
| `api_key` | (required) | Prowlarr API key |
| `timeout` | 30 seconds | Request timeout |
| `max_requests_per_minute` | 60 | Rate limiting |
| `user_agent` | `Radarr-Rust/1.0` | HTTP User-Agent |
| `verify_ssl` | `true` | SSL certificate verification |

## Error Handling

All methods return `Result<T, RadarrError>` for consistent error handling:

```rust
match client.search(&request).await {
    Ok(results) => {
        // Handle successful search
    }
    Err(RadarrError::ExternalServiceError { service, error }) => {
        eprintln!("Prowlarr error: {}", error);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Rate Limiting

The client automatically enforces rate limits to prevent indexer bans:

- Default: 60 requests per minute
- Configurable per client instance
- Automatic backoff when limits are reached
- Per-instance rate limiting (create multiple clients for parallel access)

## Testing

### Unit Tests with Mock Client
```rust
use radarr_indexers::tests::MockIndexerClient;

let mock_response = SearchResponse {
    total: 1,
    results: vec![/* test data */],
    indexers_searched: 1,
    indexers_with_errors: 0,
    errors: Vec::new(),
};

let client = MockIndexerClient::new()
    .with_search_response(mock_response);

// Test your code with the mock client
```

### Integration Tests
```rust
// Run with: cargo test test_real_prowlarr_integration -- --ignored
// Requires PROWLARR_BASE_URL and PROWLARR_API_KEY environment variables
```

## Examples

Run the included example:

```bash
# Set environment variables
export PROWLARR_BASE_URL="http://localhost:9696"
export PROWLARR_API_KEY="your-api-key"

# Run the example
cargo run --example search_example
```

## Response Models

### Search Results
- `SearchResponse`: Complete search response with metadata
- `ProwlarrSearchResult`: Individual torrent/NZB release
- `Category`: Release category information
- `SearchError`: Error details from failed indexers

### Indexer Information
- `ProwlarrIndexer`: Complete indexer configuration
- `IndexerStatus`: Health and error status
- `IndexerCapabilities`: Supported features and limits
- `IndexerStats`: Performance statistics

## Production Considerations

### Logging
The client uses `tracing` for structured logging:
```rust
use tracing::{info, warn, error};
use tracing_subscriber;

// Initialize logging in your application
tracing_subscriber::fmt().init();
```

### Monitoring
Monitor these metrics for production health:
- Request success/failure rates
- Response times
- Rate limit hits
- Indexer availability

### Security
- Store API keys securely (environment variables, secrets management)
- Use HTTPS URLs when possible
- Enable SSL verification in production
- Rotate API keys regularly

## Dependencies

- `reqwest`: HTTP client with async support
- `serde`: JSON serialization/deserialization  
- `tokio`: Async runtime
- `tracing`: Structured logging
- `url`: URL parsing and manipulation
- `chrono`: Date/time handling

## License

Part of the unified-radarr project.