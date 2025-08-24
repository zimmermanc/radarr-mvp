# Radarr Core

The core domain logic and business rules for the Radarr movie automation system. This crate contains fundamental domain models, value objects, and business rules that define the Radarr application's core functionality.

## Features

- **Domain Models**: Core entities including Movie, Release, Quality, Download, and Queue
- **Business Logic**: Domain services and business rules enforcement
- **Error Handling**: Comprehensive error types with detailed context
- **Event System**: Domain events for decoupled communication between components  
- **Notification System**: Event-driven notification infrastructure
- **Retry Logic**: Configurable retry policies with exponential backoff
- **Circuit Breaker**: Fault tolerance patterns for external service calls
- **Progress Tracking**: Real-time progress reporting for long-running operations
- **Job System**: Background job orchestration with list synchronization
- **Streaming Integration**: Support for streaming service metadata and availability
- **RSS Support**: RSS feed parsing and processing capabilities
- **Correlation IDs**: Request tracing and correlation across service boundaries
- **Blocklist Management**: Automatic blocklist handling for failed releases

## Key Dependencies

- **serde**: JSON serialization/deserialization
- **chrono**: Date and time handling
- **uuid**: Unique identifier generation
- **thiserror**: Error handling and propagation
- **async-trait**: Async trait support
- **reqwest**: HTTP client for external service integration
- **tracing**: Structured logging and observability
- **tokio**: Async runtime support
- **regex**: Pattern matching for release parsing

## Core Types

### Domain Models

```rust
use radarr_core::{Movie, MovieStatus, MinimumAvailability, Quality, Release, Download};

// Movie entity with metadata and status
let movie = Movie {
    id: uuid::Uuid::new_v4(),
    title: "The Matrix".to_string(),
    year: 1999,
    status: MovieStatus::Wanted,
    minimum_availability: MinimumAvailability::InCinemas,
    // ... other fields
};

// Release with quality information
let release = Release {
    title: "The.Matrix.1999.1080p.BluRay.x264-GROUP".to_string(),
    quality: Quality::Bluray1080p,
    size_bytes: 8_000_000_000,
    // ... other fields
};
```

### Services and Repositories

```rust
use radarr_core::{MovieRepository, ReleaseRepository, DownloadService};

// Repository traits define data access contracts
#[async_trait]
pub trait MovieRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Movie>>;
    async fn save(&self, movie: &Movie) -> Result<()>;
    // ... other methods
}

// Services encapsulate business logic
pub struct DownloadService {
    // Implementation details
}
```

### Error Handling

```rust
use radarr_core::{RadarrError, Result};

// Comprehensive error types with context
pub enum RadarrError {
    Database(String),
    Network(String),
    Validation(String),
    NotFound(String),
    Configuration(String),
    // ... other variants
}

// Result type alias
pub type Result<T> = std::result::Result<T, RadarrError>;
```

## Configuration

The core crate supports optional features:

```toml
[dependencies]
radarr-core = { version = "0.1.0", features = ["postgres"] }
```

### Available Features

- **postgres**: Enable PostgreSQL-specific functionality via SQLx integration
- **default**: Base functionality without database-specific features

## Testing

Run the test suite with:

```bash
cargo test -p radarr-core
```

The crate includes comprehensive unit tests and property-based tests using:

- **proptest**: Property-based testing for domain models
- **tokio-test**: Async testing utilities

## Architecture

The core crate follows Domain-Driven Design (DDD) principles:

- **Entities**: Rich domain objects with identity (Movie, Release, Download)
- **Value Objects**: Immutable objects without identity (Quality, MovieStatus)
- **Services**: Stateless objects that perform domain operations
- **Repositories**: Abstractions for data persistence
- **Events**: Domain events for cross-cutting concerns

## Error Recovery

Built-in resilience patterns:

- **Retry Logic**: Configurable exponential backoff for transient failures
- **Circuit Breaker**: Automatic fallback when services are degraded
- **Correlation IDs**: Request tracing across service boundaries
- **Progress Tracking**: Real-time status updates for long operations

## Integration

This crate is designed to be consumed by:

- **radarr-api**: HTTP API layer
- **radarr-infrastructure**: Data persistence implementations
- **radarr-indexers**: Search and indexer integrations
- **radarr-decision**: Release selection logic
- **radarr-downloaders**: Download client integrations
- **radarr-import**: Media import pipeline
- **radarr-notifications**: Notification providers

## Example Usage

```rust
use radarr_core::{Movie, MovieStatus, Quality, RadarrError};
use uuid::Uuid;

// Create a new movie
let movie = Movie::new(
    "The Matrix".to_string(),
    1999,
    MovieStatus::Wanted,
)?;

// Validate movie data
movie.validate()?;

// Work with quality profiles
let quality = Quality::from_string("Bluray-1080p")?;
println!("Quality: {:?}", quality);
```