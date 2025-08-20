# Repository Implementations

This module contains PostgreSQL implementations of all repository traits defined in the core domain layer.

## Implemented Repositories

### MovieRepository (`movie.rs`)
Complete implementation with all CRUD operations:
- **Find operations**: by ID, TMDB ID, IMDB ID
- **Search operations**: title search with PostgreSQL full-text search
- **List operations**: monitored movies, missing files, paginated listing
- **CRUD operations**: create, update, delete with proper error handling
- **PostgreSQL optimizations**:
  - JSONB field queries for metadata
  - Full-text search with ranking
  - Batch insert operations using UNNEST
  - Alternative title search in JSONB arrays
  - Connection pooling support

### IndexerRepository (`indexer.rs`)
Complete implementation for indexer management:
- **Find operations**: by ID, name, enabled indexers
- **CRUD operations**: create, update, delete, list
- **Connection testing**: validates indexer configuration
- **Ordering**: results ordered by priority and name

### QualityProfileRepository (`quality_profile.rs`)
Complete implementation for quality profile management:
- **Find operations**: by ID, name, default profile detection
- **CRUD operations**: create, update, delete, list
- **Smart defaults**: finds profiles named 'default' or returns first available

### DownloadRepository (`download.rs`)
Partial implementation - basic structure exists but needs completion for:
- Find operations by status
- Cleanup operations
- Additional filtering

## PostgreSQL Features Used

### JSONB Operations
- **Metadata queries**: Using `metadata #> $1 = $2` for nested field access
- **Array searches**: Using `jsonb_array_elements_text()` for alternative titles
- **Efficient indexing**: JSONB fields support GIN indexing

### Full-Text Search
- **Title ranking**: Using `to_tsvector()` and `ts_rank_cd()` for relevance scoring
- **Case-insensitive**: Using `ILIKE` for pattern matching
- **Multi-field search**: Search across title, original_title, and alternative_titles

### Performance Optimizations
- **Prepared statements**: All queries use parameterized statements
- **Connection pooling**: Leverages SQLx connection pool
- **Batch operations**: UNNEST for bulk inserts with conflict resolution
- **Efficient ordering**: Proper indexes assumed for common query patterns

## Error Handling

All repository methods return `Result<T, RadarrError>` with:
- **Database errors**: Automatically converted from SQLx errors
- **Validation errors**: Custom validation for enum parsing
- **Consistent error propagation**: Using `?` operator throughout

## Connection Pooling

Repositories use `DatabasePool` (SQLx Pool<Postgres>) with:
- **Configurable pool size**: Min/max connections configurable
- **Health checks**: Test before acquire enabled
- **Timeout handling**: Acquire timeout and idle timeout configured
- **Connection lifecycle**: Proper connection lifetime management

## Usage Example

```rust
use radarr_infrastructure::database::{create_pool, DatabaseConfig};
use radarr_infrastructure::repositories::PostgresMovieRepository;
use radarr_core::domain::repositories::MovieRepository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create database pool
    let config = DatabaseConfig::from_env();
    let pool = create_pool(config).await?;
    
    // Create repository
    let movie_repo = PostgresMovieRepository::new(pool);
    
    // Use repository
    let movies = movie_repo.find_monitored().await?;
    println!("Found {} monitored movies", movies.len());
    
    // Search for movies
    let search_results = movie_repo.search_by_title("Inception", 10).await?;
    println!("Search returned {} results", search_results.len());
    
    Ok(())
}
```

## Testing

Repository tests require a PostgreSQL database. Set up test environment:

```bash
# Start test database
docker run --name radarr-test-db -e POSTGRES_PASSWORD=test -e POSTGRES_DB=radarr_test -p 5433:5432 -d postgres:16

# Set test database URL
export TEST_DATABASE_URL=postgresql://postgres:test@localhost:5433/radarr_test

# Run migrations
sqlx migrate run --source migrations --database-url $TEST_DATABASE_URL

# Run repository tests
cargo test --package radarr-infrastructure
```

## Performance Notes

- **Query optimization**: All queries use indexes on commonly filtered columns
- **JSONB indexing**: Consider creating GIN indexes on metadata and alternative_titles
- **Connection reuse**: Pool configuration optimized for web application usage
- **Memory efficiency**: Streaming results for large datasets where applicable

## Future Improvements

1. **Query optimization**: Add database-specific query hints
2. **Caching layer**: Add Redis cache for frequently accessed data
3. **Read replicas**: Support for read/write splitting
4. **Bulk operations**: Optimize batch operations further
5. **Database migrations**: Version-controlled schema changes
6. **Monitoring**: Add query performance monitoring