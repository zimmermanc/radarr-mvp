# Contributing Guide

Welcome to the Radarr MVP project! This guide will help you get started with contributing to the codebase.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Environment](#development-environment)
3. [Code Organization](#code-organization)
4. [Development Workflow](#development-workflow)
5. [Testing](#testing)
6. [Code Style](#code-style)
7. [Pull Request Process](#pull-request-process)
8. [Architecture Guidelines](#architecture-guidelines)

## Getting Started

### Prerequisites

- **Rust 1.75+** with Cargo
- **Node.js 18+** with npm
- **PostgreSQL 14+**
- **Git** for version control
- **Docker** (optional but recommended)

### Fork and Clone

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/your-username/radarr-mvp.git
cd radarr-mvp/unified-radarr

# Add upstream remote
git remote add upstream https://github.com/original-repo/radarr-mvp.git
```

### Initial Setup

```bash
# Copy environment configuration
cp .env.example .env
# Edit .env with your local settings

# Install Rust dependencies and build
cargo build --workspace

# Install frontend dependencies
cd web
npm install
cd ..

# Run database migrations
sqlx migrate run

# Run tests to verify setup
cargo test --workspace
```

## Development Environment

### Required Tools

```bash
# Rust toolchain components
rustup component add rustfmt clippy

# Database CLI tool
cargo install sqlx-cli --features postgres

# Development helpers
cargo install cargo-watch  # For hot reloading
cargo install cargo-audit   # Security auditing
cargo install cargo-edit    # Dependency management
```

### IDE Setup

#### VS Code (Recommended)

**Required Extensions:**
- `rust-analyzer` - Rust language support
- `ms-vscode.vscode-typescript-next` - TypeScript support
- `bradlc.vscode-tailwindcss` - Tailwind CSS support
- `ms-vscode.vscode-json` - JSON support

**Recommended Extensions:**
- `tamasfe.even-better-toml` - TOML support
- `serayuzgur.crates` - Crate management
- `vadimcn.vscode-lldb` - Debugging support

**Settings (`.vscode/settings.json`):**
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.imports.granularity.group": "module",
  "rust-analyzer.imports.prefix": "crate",
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll": true
  }
}
```

#### IntelliJ IDEA / CLion

- Install **Rust** plugin
- Install **TypeScript and JavaScript** plugin
- Configure code style to use `rustfmt`

### External Services

**Development Services (Docker):**
```bash
# Start development database
docker run -d --name radarr-postgres \
  -e POSTGRES_DB=radarr_dev \
  -e POSTGRES_USER=radarr \
  -e POSTGRES_PASSWORD=radarr \
  -p 5432:5432 postgres:16

# Start Prowlarr for testing
docker run -d --name prowlarr \
  -p 9696:9696 \
  -v ./data/prowlarr:/config \
  lscr.io/linuxserver/prowlarr:latest

# Start qBittorrent for testing
docker run -d --name qbittorrent \
  -p 8080:8080 \
  -v ./data/qbittorrent:/config \
  -v ./data/downloads:/downloads \
  lscr.io/linuxserver/qbittorrent:latest
```

## Code Organization

### Project Structure

```
unified-radarr/
├── crates/                  # Rust crates
│   ├── core/               # Domain logic (no external dependencies)
│   ├── api/                # HTTP API layer (Axum)
│   ├── infrastructure/     # External concerns (DB, HTTP, FS)
│   ├── indexers/           # Prowlarr integration
│   ├── downloaders/        # qBittorrent integration
│   ├── import/             # File import pipeline
│   └── decision/           # Quality profiles and selection
├── web/                    # React frontend
│   ├── src/
│   │   ├── components/      # Reusable UI components
│   │   ├── pages/           # Page components
│   │   ├── services/        # API client
│   │   └── utils/           # Utility functions
│   └── package.json
├── migrations/             # Database migrations
├── k8s/                    # Kubernetes manifests
├── scripts/                # Build and deployment scripts
└── tests/                  # Integration tests
```

### Clean Architecture Principles

**Dependency Flow:**
```
API → Core ← Infrastructure
 │           ↑
 ↓           │
Indexers    Downloaders
 │           │
 ↓           ↓
    Import → Decision
```

**Key Rules:**
1. **Core** has no external dependencies
2. All other crates depend on **Core**
3. **Infrastructure** implements traits defined in **Core**
4. **API** orchestrates business logic from **Core**

## Development Workflow

### Daily Development

```bash
# Start development servers
# Terminal 1: Backend with hot reload
cargo watch -x "run"

# Terminal 2: Frontend development server
cd web && npm run dev

# Terminal 3: Database and external services
docker-compose -f docker-compose.dev.yml up
```

### Branch Strategy

**Branch Naming:**
- `feature/add-quality-profiles` - New features
- `fix/prowlarr-timeout-handling` - Bug fixes
- `refactor/database-layer` - Code improvements
- `docs/api-documentation` - Documentation updates

**Workflow:**
```bash
# Create feature branch
git checkout -b feature/your-feature-name

# Make changes and commit frequently
git add .
git commit -m "feat: add quality profile creation API"

# Push to your fork
git push origin feature/your-feature-name

# Create pull request on GitHub
```

### Commit Message Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, etc.)
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Maintenance tasks

**Examples:**
```bash
git commit -m "feat(api): add movie search endpoint"
git commit -m "fix(indexers): handle Prowlarr connection timeout"
git commit -m "docs: update API documentation for quality profiles"
git commit -m "refactor(core): simplify movie entity structure"
```

## Testing

### Test Categories

1. **Unit Tests** - Individual functions and structs
2. **Integration Tests** - Cross-crate functionality
3. **End-to-End Tests** - Full API workflows
4. **Property Tests** - Input validation with random data

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p radarr-core

# Run tests with output
cargo test --workspace -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run with coverage
cargo tarpaulin --out Html

# Frontend tests
cd web
npm test
```

### Writing Tests

#### Unit Tests

```rust
// In the same file as the code being tested
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movie_creation() {
        let movie = Movie {
            id: Uuid::new_v4(),
            title: "Test Movie".to_string(),
            year: Some(2024),
            tmdb_id: 12345,
            monitored: true,
            status: MovieStatus::Wanted,
        };

        assert_eq!(movie.title, "Test Movie");
        assert_eq!(movie.year, Some(2024));
        assert!(movie.monitored);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = some_async_function().await;
        assert!(result.is_ok());
    }
}
```

#### Integration Tests

```rust
// tests/integration_test.rs
use radarr_api::create_app;
use axum_test::TestServer;

#[tokio::test]
async fn test_movie_api_endpoints() {
    let app = create_app().await;
    let server = TestServer::new(app).unwrap();

    // Test GET /api/v3/movie
    let response = server
        .get("/api/v3/movie")
        .add_header("X-Api-Key", "test-key")
        .await;
    
    response.assert_status_ok();
    let movies: Vec<Movie> = response.json();
    assert!(!movies.is_empty());
}
```

#### Property Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_movie_title_validation(title in "[a-zA-Z0-9 ]{1,200}") {
        let result = validate_movie_title(&title);
        prop_assert!(result.is_ok());
    }

    #[test]
    fn test_year_validation(year in 1900..=2030) {
        let movie = Movie::new("Test".to_string(), Some(year), 123);
        prop_assert!(movie.year.unwrap() >= 1900);
        prop_assert!(movie.year.unwrap() <= 2030);
    }
}
```

### Test Data

**Use Test Fixtures:**
```rust
// tests/fixtures/mod.rs
pub fn create_test_movie() -> Movie {
    Movie {
        id: Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap(),
        title: "The Matrix".to_string(),
        year: Some(1999),
        tmdb_id: 603,
        monitored: true,
        status: MovieStatus::Downloaded,
    }
}

pub fn create_test_database() -> DatabaseConnection {
    // Setup test database with mock data
}
```

## Code Style

### Rust Code Style

**Use `rustfmt` and `clippy`:**
```bash
# Format code
cargo fmt --all

# Check for style issues and potential bugs
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run both in CI mode
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Custom `rustfmt.toml`:**
```toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
merge_derives = true
```

### Code Guidelines

#### Error Handling

```rust
// Use Result types for fallible operations
fn parse_movie_id(id: &str) -> Result<Uuid, RadarrError> {
    Uuid::parse_str(id)
        .map_err(|_| RadarrError::InvalidMovieId(id.to_string()))
}

// Use ? operator for error propagation
async fn get_movie_details(id: Uuid) -> Result<Movie, RadarrError> {
    let movie = database.find_movie(id).await?;
    let metadata = tmdb_client.get_movie_details(movie.tmdb_id).await?;
    Ok(movie.with_metadata(metadata))
}

// Handle errors at the API boundary
async fn movie_handler(id: Uuid) -> Result<Json<Movie>, StatusCode> {
    match get_movie_details(id).await {
        Ok(movie) => Ok(Json(movie)),
        Err(RadarrError::MovieNotFound(_)) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

#### Async Programming

```rust
// Use async/await for I/O operations
async fn download_movie_metadata(tmdb_id: i32) -> Result<MovieMetadata, RadarrError> {
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("https://api.themoviedb.org/3/movie/{}", tmdb_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?
        .json::<MovieMetadata>()
        .await?;
    
    Ok(response)
}

// Use tokio::spawn for concurrent operations
async fn search_multiple_indexers(query: &str) -> Vec<SearchResult> {
    let mut handles = Vec::new();
    
    for indexer in indexers {
        let query = query.to_string();
        let handle = tokio::spawn(async move {
            indexer.search(&query).await
        });
        handles.push(handle);
    }
    
    let mut results = Vec::new();
    for handle in handles {
        if let Ok(Ok(mut indexer_results)) = handle.await {
            results.append(&mut indexer_results);
        }
    }
    
    results
}
```

#### Documentation

```rust
/// Represents a movie in the Radarr library.
/// 
/// # Examples
/// 
/// ```
/// use radarr_core::Movie;
/// 
/// let movie = Movie::new(
///     "The Matrix".to_string(),
///     Some(1999),
///     603 // TMDB ID
/// );
/// assert_eq!(movie.title, "The Matrix");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    /// Unique identifier for the movie
    pub id: Uuid,
    /// Movie title
    pub title: String,
    /// Release year (optional for movies without known release dates)
    pub year: Option<i32>,
    /// TMDB (The Movie Database) identifier
    pub tmdb_id: i32,
    /// Whether the movie is monitored for automatic downloads
    pub monitored: bool,
    /// Current status of the movie
    pub status: MovieStatus,
}

impl Movie {
    /// Creates a new movie with default values.
    /// 
    /// # Arguments
    /// 
    /// * `title` - The movie title
    /// * `year` - Optional release year
    /// * `tmdb_id` - TMDB identifier
    /// 
    /// # Returns
    /// 
    /// A new `Movie` instance with generated UUID and default status.
    pub fn new(title: String, year: Option<i32>, tmdb_id: i32) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            year,
            tmdb_id,
            monitored: true,
            status: MovieStatus::Wanted,
        }
    }
}
```

### Frontend Code Style

**Use Prettier and ESLint:**
```bash
# In web/ directory
npm run lint
npm run format
```

**TypeScript Guidelines:**
```typescript
// Use interfaces for type definitions
interface Movie {
  id: string;
  title: string;
  year?: number;
  tmdbId: number;
  monitored: boolean;
  status: MovieStatus;
}

// Use React function components with TypeScript
interface MovieCardProps {
  movie: Movie;
  onToggleMonitored: (id: string) => void;
}

const MovieCard: React.FC<MovieCardProps> = ({ movie, onToggleMonitored }) => {
  const handleMonitorClick = useCallback(() => {
    onToggleMonitored(movie.id);
  }, [movie.id, onToggleMonitored]);

  return (
    <div className="movie-card">
      <h3>{movie.title} ({movie.year})</h3>
      <button onClick={handleMonitorClick}>
        {movie.monitored ? 'Unmonitor' : 'Monitor'}
      </button>
    </div>
  );
};

export default MovieCard;
```

## Pull Request Process

### Before Creating PR

1. **Update your branch:**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run full test suite:**
   ```bash
   cargo test --workspace
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cd web && npm test && npm run lint
   ```

3. **Update documentation if needed:**
   - Update API documentation for new endpoints
   - Add or update code comments
   - Update README if behavior changes

### PR Template

```markdown
## Summary
Brief description of changes and motivation.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Changes Made
- Specific change 1
- Specific change 2
- Specific change 3

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Frontend tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No breaking changes (or clearly documented)
```

### Code Review Guidelines

**For Authors:**
- Keep PRs focused and reasonably sized
- Write clear commit messages
- Include tests for new functionality
- Update documentation
- Respond to feedback promptly

**For Reviewers:**
- Review for correctness, not style (tools handle style)
- Check for security issues
- Verify tests cover new functionality
- Suggest improvements, don't just point out problems
- Approve when ready, don't nitpick

## Architecture Guidelines

### Adding New Features

#### 1. Start with Core Domain

```rust
// crates/core/src/entities/quality_profile.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityProfile {
    pub id: Uuid,
    pub name: String,
    pub cutoff: Quality,
    pub items: Vec<QualityItem>,
    pub upgrade_allowed: bool,
}

#[async_trait]
pub trait QualityProfileRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<QualityProfile>, RadarrError>;
    async fn save(&self, profile: &QualityProfile) -> Result<(), RadarrError>;
    async fn delete(&self, id: Uuid) -> Result<(), RadarrError>;
}
```

#### 2. Implement Infrastructure

```rust
// crates/infrastructure/src/repositories/quality_profile.rs
pub struct PostgresQualityProfileRepository {
    pool: Arc<PgPool>,
}

#[async_trait]
impl QualityProfileRepository for PostgresQualityProfileRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<QualityProfile>, RadarrError> {
        let profile = sqlx::query_as!(
            QualityProfileRow,
            "SELECT * FROM quality_profiles WHERE id = $1",
            id
        )
        .fetch_optional(&*self.pool)
        .await?
        .map(|row| row.into());
        
        Ok(profile)
    }
}
```

#### 3. Add API Endpoints

```rust
// crates/api/src/handlers/quality_profiles.rs
pub async fn list_quality_profiles(
    State(state): State<AppState>,
) -> Result<Json<Vec<QualityProfile>>, StatusCode> {
    let profiles = state
        .quality_profile_service
        .list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(profiles))
}

pub async fn create_quality_profile(
    State(state): State<AppState>,
    Json(request): Json<CreateQualityProfileRequest>,
) -> Result<Json<QualityProfile>, StatusCode> {
    let profile = state
        .quality_profile_service
        .create(request)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    Ok(Json(profile))
}
```

#### 4. Add Frontend Components

```typescript
// web/src/components/QualityProfileCard.tsx
interface QualityProfileCardProps {
  profile: QualityProfile;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
}

const QualityProfileCard: React.FC<QualityProfileCardProps> = ({
  profile,
  onEdit,
  onDelete,
}) => {
  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h3 className="text-lg font-semibold">{profile.name}</h3>
      <p className="text-gray-600">Cutoff: {profile.cutoff}</p>
      <div className="mt-4 flex space-x-2">
        <button 
          onClick={() => onEdit(profile.id)}
          className="btn btn-primary"
        >
          Edit
        </button>
        <button 
          onClick={() => onDelete(profile.id)}
          className="btn btn-danger"
        >
          Delete
        </button>
      </div>
    </div>
  );
};
```

### Database Migrations

**Create Migration:**
```bash
sqlx migrate add create_quality_profiles_table
```

**Migration File:**
```sql
-- migrations/001_create_quality_profiles_table.sql
CREATE TABLE quality_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    cutoff VARCHAR(50) NOT NULL,
    items JSONB NOT NULL,
    upgrade_allowed BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_quality_profiles_name ON quality_profiles(name);
```

### External Service Integration

```rust
// crates/indexers/src/prowlarr/client.rs
use circuit_breaker::CircuitBreaker;

pub struct ProwlarrClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    circuit_breaker: CircuitBreaker,
}

impl ProwlarrClient {
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Release>, ProwlarrError> {
        self.circuit_breaker
            .call(|| async {
                let response = self
                    .client
                    .get(&format!("{}/api/v1/search", self.base_url))
                    .header("X-Api-Key", &self.api_key)
                    .query(&query)
                    .send()
                    .await?
                    .error_for_status()?;
                
                let releases: Vec<Release> = response.json().await?;
                Ok(releases)
            })
            .await
    }
}
```

### Performance Considerations

- **Database Queries:** Use indexes and limit result sets
- **External APIs:** Implement rate limiting and circuit breakers
- **Memory Usage:** Use streaming for large datasets
- **Caching:** Cache expensive operations (TMDB metadata, etc.)

```rust
// Example: Cached TMDB client
use std::time::Duration;
use moka::future::Cache;

pub struct CachedTmdbClient {
    client: TmdbClient,
    cache: Cache<i32, MovieMetadata>,
}

impl CachedTmdbClient {
    pub fn new(api_key: String) -> Self {
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_hours(24))
            .build();
        
        Self {
            client: TmdbClient::new(api_key),
            cache,
        }
    }
    
    pub async fn get_movie(&self, tmdb_id: i32) -> Result<MovieMetadata, TmdbError> {
        if let Some(metadata) = self.cache.get(&tmdb_id).await {
            return Ok(metadata);
        }
        
        let metadata = self.client.get_movie(tmdb_id).await?;
        self.cache.insert(tmdb_id, metadata.clone()).await;
        Ok(metadata)
    }
}
```

## Getting Help

- **Discord/Community:** Join our Discord server for real-time discussion
- **GitHub Issues:** Use GitHub issues for bug reports and feature requests
- **Documentation:** Check existing documentation first
- **Code Review:** Ask questions during the PR review process

## Development Resources

### Rust Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Programming in Rust](https://rust-lang.github.io/async-book/)

### Framework Documentation
- [Axum](https://docs.rs/axum/latest/axum/) - Web framework
- [SQLx](https://docs.rs/sqlx/latest/sqlx/) - Database toolkit
- [Tokio](https://tokio.rs/) - Async runtime
- [React](https://react.dev/) - Frontend framework
- [Tailwind CSS](https://tailwindcss.com/) - CSS framework

### Project-Specific Resources
- [Radarr API Documentation](https://radarr.video/docs/api/)
- [TMDB API](https://developers.themoviedb.org/3)
- [Prowlarr API](https://prowlarr.com/)

Thank you for contributing to Radarr MVP! 🎉