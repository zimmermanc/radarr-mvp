# Radarr Infrastructure

Infrastructure layer providing concrete implementations of repository traits and external service integrations for the Radarr movie automation system. This crate handles data persistence, third-party API integrations, and external service communications.

## Features

- **PostgreSQL Integration**: High-performance database operations with SQLx
- **Repository Implementations**: Concrete implementations of core domain repository traits
- **External Service Clients**: TMDB, Trakt, WatchMode API integrations
- **Download Client Support**: Integration with various download clients
- **Streaming Service Integration**: Real-time streaming availability data
- **List Management**: Movie list synchronization and management
- **Database Migrations**: Schema management and version control
- **Connection Pooling**: Efficient database connection management
- **Error Handling**: Comprehensive error mapping and recovery
- **Monitoring**: Health checks and service monitoring

## Key Dependencies

- **sqlx**: Async PostgreSQL driver with compile-time query verification
- **reqwest**: HTTP client for external API integrations
- **serde/serde_json**: JSON serialization for API responses
- **chrono**: Date/time handling for timestamps and scheduling
- **uuid**: Unique identifier handling
- **async-trait**: Async trait implementations for repositories
- **rust_decimal**: Precise decimal arithmetic for ratings and scores
- **scraper**: HTML parsing for web scraping tasks
- **csv**: CSV file processing for data imports

## Repository Implementations

### Movie Repository

```rust
use radarr_infrastructure::PostgresMovieRepository;
use radarr_core::{Movie, MovieRepository};
use sqlx::PgPool;

let pool = PgPool::connect(&database_url).await?;
let repo = PostgresMovieRepository::new(pool);

// Find movie by ID
let movie = repo.find_by_id(movie_id).await?;

// Save new movie
repo.save(&movie).await?;

// Search movies with filters
let movies = repo.find_by_criteria(search_criteria).await?;
```

### Download Repository

```rust
use radarr_infrastructure::PostgresDownloadRepository;
use radarr_core::{Download, DownloadStatus};

let repo = PostgresDownloadRepository::new(pool);

// Get active downloads
let active_downloads = repo.find_by_status(DownloadStatus::Downloading).await?;

// Update download progress
repo.update_progress(download_id, progress_info).await?;
```

## External Service Integrations

### TMDB (The Movie Database)

```rust
use radarr_infrastructure::tmdb::{TMDBClient, TMDBConfig, MovieDetails};

let config = TMDBConfig {
    api_key: "your_api_key".to_string(),
    base_url: "https://api.themoviedb.org/3".to_string(),
    timeout_seconds: 30,
};

let client = TMDBClient::new(config);

// Search for movies
let search_results = client.search_movies("The Matrix", Some(1999)).await?;

// Get detailed movie information
let movie_details = client.get_movie_details(603).await?;

// Get movie images
let images = client.get_movie_images(603).await?;
```

### Trakt Integration

```rust
use radarr_infrastructure::trakt::{TraktClient, TraktConfig};

let config = TraktConfig {
    client_id: "your_client_id".to_string(),
    client_secret: "your_client_secret".to_string(),
    redirect_uri: "http://localhost:8080/trakt/callback".to_string(),
};

let client = TraktClient::new(config);

// Get trending movies
let trending = client.get_trending_movies(10).await?;

// Get user's watchlist
let watchlist = client.get_user_watchlist("username").await?;

// Sync collection
client.sync_collection(&movies).await?;
```

### WatchMode Streaming Data

```rust
use radarr_infrastructure::watchmode::{WatchModeClient, StreamingProvider};

let client = WatchModeClient::new(api_key);

// Get streaming availability
let availability = client.get_streaming_availability(tmdb_id).await?;

// Get supported providers
let providers = client.get_streaming_providers().await?;
```

## Database Schema

### Core Tables

```sql
-- Movies table
CREATE TABLE movies (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    year INTEGER NOT NULL,
    tmdb_id INTEGER UNIQUE,
    status VARCHAR NOT NULL,
    monitored BOOLEAN NOT NULL DEFAULT true,
    minimum_availability VARCHAR NOT NULL,
    overview TEXT,
    poster_url VARCHAR,
    added TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Downloads table  
CREATE TABLE downloads (
    id UUID PRIMARY KEY,
    movie_id UUID NOT NULL REFERENCES movies(id),
    release_title VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    progress DECIMAL(5,2) DEFAULT 0.0,
    download_client VARCHAR NOT NULL,
    download_id VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Quality profiles
CREATE TABLE quality_profiles (
    id UUID PRIMARY KEY,
    name VARCHAR NOT NULL,
    cutoff_quality VARCHAR NOT NULL,
    allowed_qualities JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Migrations

```rust
use sqlx::migrate::Migrator;

// Embedded migrations
static MIGRATOR: Migrator = sqlx::migrate!();

// Run migrations
MIGRATOR.run(&pool).await?;
```

## Configuration

### Database Connection

```rust
use radarr_infrastructure::{DatabaseConfig, create_database_pool};

let config = DatabaseConfig {
    url: "postgresql://user:password@localhost/radarr".to_string(),
    max_connections: 10,
    min_connections: 5,
    connect_timeout: Duration::from_secs(30),
    idle_timeout: Some(Duration::from_secs(600)),
};

let pool = create_database_pool(config).await?;
```

### Service Configuration

```rust
use radarr_infrastructure::ServiceConfig;

let config = ServiceConfig {
    tmdb: TMDBConfig {
        api_key: env::var("TMDB_API_KEY")?,
        // ... other settings
    },
    trakt: TraktConfig {
        client_id: env::var("TRAKT_CLIENT_ID")?,
        // ... other settings
    },
    database: DatabaseConfig::from_env()?,
};
```

## Error Handling

```rust
use radarr_infrastructure::InfrastructureError;

pub enum InfrastructureError {
    Database(sqlx::Error),
    ExternalService {
        service: String,
        error: String,
    },
    Configuration(String),
    Network(reqwest::Error),
    Serialization(serde_json::Error),
}

// Error conversion from external crates
impl From<sqlx::Error> for InfrastructureError {
    fn from(err: sqlx::Error) -> Self {
        InfrastructureError::Database(err)
    }
}
```

## Download Client Integration

### qBittorrent Client

```rust
use radarr_infrastructure::download_clients::QBittorrentClient;

let client = QBittorrentClient::new(
    "http://localhost:8080".to_string(),
    "admin".to_string(),
    "password".to_string(),
);

// Add torrent
let torrent_data = std::fs::read("movie.torrent")?;
client.add_torrent(torrent_data, "/downloads/movies").await?;

// Get torrent info
let torrents = client.get_torrents().await?;

// Remove completed torrent
client.remove_torrent(&torrent_hash, false).await?;
```

## Streaming Integration

### Real-time Availability Updates

```rust
use radarr_infrastructure::streaming::{StreamingService, AvailabilityUpdate};

let service = StreamingService::new(watchmode_client);

// Check availability for movie
let availability = service.check_movie_availability(tmdb_id).await?;

// Subscribe to availability updates
let mut updates = service.availability_updates().await;
while let Some(update) = updates.recv().await {
    println!("Availability changed: {:?}", update);
}
```

## List Management

### Movie List Synchronization

```rust
use radarr_infrastructure::lists::{ListSyncService, ListProvider};

let sync_service = ListSyncService::new(database_pool);

// Sync IMDB list
let imdb_list = ListProvider::imdb("ls123456789".to_string());
let sync_result = sync_service.sync_list(imdb_list).await?;

println!("Added {} new movies", sync_result.added_count);
```

## Testing

### Database Testing

```rust
use radarr_infrastructure::testing::TestDatabase;

#[tokio::test]
async fn test_movie_repository() {
    let test_db = TestDatabase::new().await;
    let repo = PostgresMovieRepository::new(test_db.pool());
    
    // Test repository methods
    let movie = Movie::new("Test Movie".to_string(), 2023);
    repo.save(&movie).await.unwrap();
    
    let found = repo.find_by_id(movie.id).await.unwrap();
    assert_eq!(found.unwrap().title, "Test Movie");
}
```

### Mock External Services

```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use radarr_infrastructure::tmdb::TMDBClient;

#[tokio::test]
async fn test_tmdb_integration() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/3/search/movie"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(&mock_response))
        .mount(&mock_server)
        .await;
    
    let client = TMDBClient::new_with_base_url(mock_server.uri());
    let results = client.search_movies("The Matrix", None).await.unwrap();
    
    assert!(!results.is_empty());
}
```

## Monitoring and Health Checks

```rust
use radarr_infrastructure::monitoring::{HealthChecker, ServiceHealth};

let health_checker = HealthChecker::new()
    .with_database_check(pool.clone())
    .with_external_service_check("tmdb", tmdb_client)
    .with_external_service_check("trakt", trakt_client);

// Get overall health status
let health = health_checker.check_health().await;
match health.status {
    ServiceHealth::Healthy => println!("All systems operational"),
    ServiceHealth::Degraded => println!("Some services unavailable"),
    ServiceHealth::Unhealthy => println!("Critical systems down"),
}
```

## Performance Optimization

- **Connection Pooling**: Efficient database connection reuse
- **Query Optimization**: Indexed queries with proper EXPLAIN analysis  
- **Caching**: Redis integration for frequently accessed data
- **Batch Operations**: Bulk insert/update operations
- **Async I/O**: Non-blocking operations for external API calls
- **Circuit Breaker**: Automatic fallback for failing services