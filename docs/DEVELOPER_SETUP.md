# Developer Setup Guide

**PostgreSQL-Only Architecture** | **5-Minute Setup** | **Simplified Development**

This guide provides a streamlined setup process for the Radarr MVP development environment, now using a simplified PostgreSQL-only architecture.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Prerequisites](#prerequisites)
3. [Database Setup](#database-setup)
4. [Application Setup](#application-setup)
5. [Development Workflow](#development-workflow)
6. [Testing](#testing)
7. [Performance Monitoring](#performance-monitoring)
8. [Troubleshooting](#troubleshooting)

## Quick Start

**5-Minute Setup for New Developers**

```bash
# 1. Clone and enter directory
git clone <repository-url>
cd radarr-mvp

# 2. Start PostgreSQL with Docker
docker-compose up -d postgres

# 3. Setup environment
cp .env.example .env
# Edit .env with your configuration

# 4. Run migrations
cargo install sqlx-cli --features postgres
sqlx migrate run

# 5. Start development server
cargo run

# 6. Run tests
cargo test
```

**That's it!** Your development environment is ready in under 5 minutes.

## Prerequisites

### Required Software

1. **Rust** (1.70+): Latest stable Rust toolchain
```bash
# Install via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. **Docker & Docker Compose**: For PostgreSQL development database
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install docker.io docker-compose

# macOS
brew install docker docker-compose

# Or use Docker Desktop
```

3. **SQLx CLI**: For database migrations
```bash
cargo install sqlx-cli --features postgres
```

### Optional Tools

1. **PostgreSQL Client**: For direct database access
```bash
# Ubuntu/Debian
sudo apt install postgresql-client

# macOS
brew install postgresql
```

2. **VS Code Extensions**:
   - rust-analyzer
   - SQLx Language Server
   - PostgreSQL Explorer

## Database Setup

### 1. PostgreSQL with Docker

**Development Database** (Recommended):
```bash
# Start PostgreSQL container
docker-compose up -d postgres

# Check status
docker-compose ps
```

**Docker Compose Configuration** (`docker-compose.yml`):
```yaml
version: '3.8'
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: radarr_dev
      POSTGRES_USER: radarr
      POSTGRES_PASSWORD: radarr_dev_pass
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./scripts/postgres-init.sql:/docker-entrypoint-initdb.d/01-init.sql
    command: >
      postgres 
      -c shared_preload_libraries=pg_stat_statements
      -c pg_stat_statements.track=all
      -c max_connections=100
      -c shared_buffers=256MB
      -c effective_cache_size=1GB

volumes:
  postgres_data:
```

### 2. Local PostgreSQL Installation

**Alternative for Native Installation**:
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql

# macOS
brew install postgresql
brew services start postgresql

# Create development database
sudo -u postgres psql
CREATE DATABASE radarr_dev;
CREATE USER radarr WITH PASSWORD 'radarr_dev_pass';
GRANT ALL PRIVILEGES ON DATABASE radarr_dev TO radarr;
\q
```

### 3. Environment Configuration

**Environment File** (`.env`):
```bash
# Copy from template
cp .env.example .env

# Edit with your settings
# .env contents:
DATABASE_URL=postgresql://radarr:radarr_dev_pass@localhost:5432/radarr_dev
RUST_LOG=debug
TMDB_API_KEY=your_tmdb_api_key_here
SERVER_PORT=3000
SERVER_HOST=127.0.0.1
```

### 4. Database Migrations

**Run Migrations**:
```bash
# Ensure SQLx CLI is installed
cargo install sqlx-cli --features postgres

# Run all migrations
sqlx migrate run

# Check migration status
sqlx migrate info

# Create new migration (if needed)
sqlx migrate add description_of_migration
```

**Migration Files Location**: `migrations/`
- `001_initial_schema.sql` - Core database schema
- `002_add_indexes.sql` - Performance indexes
- `003_add_full_text_search.sql` - Search capabilities

## Application Setup

### 1. Rust Environment

**Install Dependencies**:
```bash
# Verify Rust installation
rustc --version
cargo --version

# Install project dependencies
cargo build

# Install development tools
cargo install cargo-watch  # For auto-recompiling
cargo install cargo-audit  # For security audits
cargo install cargo-tarpaulin  # For code coverage
```

### 2. Configuration

**Application Configuration** (`src/config.rs`):
```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub tmdb_api_key: String,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            tmdb_api_key: env::var("TMDB_API_KEY")?,
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        })
    }
}
```

### 3. Database Connection

**Connection Pool Setup** (`src/database.rs`):
```rust
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .test_before_acquire(true)
        .connect(database_url)
        .await
}
```

## Development Workflow

### 1. Starting Development

**Development Server**:
```bash
# Start with auto-reload
cargo watch -x run

# Or start normally
cargo run

# With specific log level
RUST_LOG=debug cargo run

# Background service
nohup cargo run > app.log 2>&1 &
```

**Check Application Health**:
```bash
# Health check endpoint
curl http://localhost:3000/health

# API documentation
curl http://localhost:3000/api/docs
```

### 2. Database Development

**Common Database Tasks**:
```bash
# Connect to development database
psql postgresql://radarr:radarr_dev_pass@localhost:5432/radarr_dev

# Run specific migration
sqlx migrate run --target-version 20250818000001

# Rollback migration
sqlx migrate revert

# Reset database (development only)
sqlx database drop
sqlx database create
sqlx migrate run
```

**Database Schema Inspection**:
```sql
-- List all tables
\dt

-- Describe table structure
\d movies

-- Check indexes
\di

-- View recent queries (if pg_stat_statements enabled)
SELECT query, calls, total_time, mean_time 
FROM pg_stat_statements 
ORDER BY total_time DESC 
LIMIT 10;
```

### 3. API Development

**Testing API Endpoints**:
```bash
# Get all movies
curl -X GET http://localhost:3000/api/movies

# Search movies
curl -X GET "http://localhost:3000/api/movies/search?q=action"

# Get specific movie
curl -X GET http://localhost:3000/api/movies/123

# Create movie
curl -X POST http://localhost:3000/api/movies \
  -H "Content-Type: application/json" \
  -d '{
    "tmdb_id": 12345,
    "title": "Test Movie",
    "year": 2025,
    "monitored": true
  }'
```

**API Documentation**: Available at `http://localhost:3000/api/docs` when server is running.

## Testing

### 1. Unit Tests

**Run All Tests**:
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_movie_creation

# Run tests for specific module
cargo test movies::

# Run tests with coverage
cargo tarpaulin --out Html
```

### 2. Integration Tests

**Database Integration Tests**:
```bash
# Run integration tests (requires test database)
TEST_DATABASE_URL=postgresql://radarr:radarr_test_pass@localhost:5432/radarr_test cargo test integration

# Setup test database
sqlx database create --database-url postgresql://radarr:radarr_test_pass@localhost:5432/radarr_test
sqlx migrate run --database-url postgresql://radarr:radarr_test_pass@localhost:5432/radarr_test
```

### 3. Performance Tests

**Benchmarks**:
```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench movie_creation

# With output
cargo bench -- --verbose
```

## Performance Monitoring

### 1. Application Metrics

**Enable Metrics** (in development):
```rust
// Add to main.rs for development monitoring
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    
    // Application startup
    let config = Config::from_env()?;
    let pool = create_pool(&config.database_url).await?;
    
    println!("Application started in {:?}", start.elapsed());
    
    // Start server...
}
```

### 2. Database Performance

**Query Performance Monitoring**:
```sql
-- Enable query statistics (in development)
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

-- Check slow queries
SELECT 
    query,
    calls,
    total_time,
    mean_time,
    rows
FROM pg_stat_statements 
WHERE query LIKE '%movies%'
ORDER BY total_time DESC
LIMIT 10;

-- Check database size
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables 
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### 3. Connection Pool Monitoring

**Pool Metrics** (add to your application):
```rust
use sqlx::Pool;

async fn log_pool_status(pool: &PgPool) {
    println!("Pool status:");
    println!("  Size: {}", pool.size());
    println!("  Idle: {}", pool.num_idle());
    println!("  Connections: {}", pool.size() - pool.num_idle());
}

// Call periodically in development
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        log_pool_status(&pool).await;
    }
});
```

## Troubleshooting

### Common Issues

#### 1. Database Connection Errors

**Problem**: `connection refused` or `authentication failed`

**Solutions**:
```bash
# Check PostgreSQL is running
docker-compose ps postgres
# or
sudo systemctl status postgresql

# Check connection string
echo $DATABASE_URL

# Test connection manually
psql $DATABASE_URL

# Reset Docker volume if needed
docker-compose down
docker volume rm radarr-mvp_postgres_data
docker-compose up -d postgres
```

#### 2. Migration Errors

**Problem**: Migration fails or is out of sync

**Solutions**:
```bash
# Check migration status
sqlx migrate info

# Force migration (development only)
sqlx migrate revert
sqlx migrate run

# Reset database (development only)
sqlx database drop && sqlx database create
sqlx migrate run
```

#### 3. Compilation Errors

**Problem**: Rust compilation fails

**Solutions**:
```bash
# Clean and rebuild
cargo clean
cargo build

# Update dependencies
cargo update

# Check for specific errors
cargo check

# Update Rust toolchain
rustup update
```

#### 4. Performance Issues

**Problem**: Slow queries or high memory usage

**Solutions**:
```sql
-- Check for missing indexes
SELECT schemaname, tablename, attname, n_distinct, correlation
FROM pg_stats
WHERE tablename = 'movies' AND n_distinct > 100;

-- Analyze query performance
EXPLAIN (ANALYZE, BUFFERS) 
SELECT * FROM movies WHERE title LIKE '%action%';

-- Update table statistics
ANALYZE movies;
```

### Development Tools

#### 1. Useful Cargo Commands

```bash
# Code formatting
cargo fmt

# Linting
cargo clippy

# Security audit
cargo audit

# Documentation generation
cargo doc --open

# Dependency tree
cargo tree

# License checking
cargo license
```

#### 2. Database Tools

```bash
# Database backup (development)
pg_dump postgresql://radarr:radarr_dev_pass@localhost:5432/radarr_dev > backup.sql

# Database restore
psql postgresql://radarr:radarr_dev_pass@localhost:5432/radarr_dev < backup.sql

# Database size
psql -c "SELECT pg_size_pretty(pg_database_size('radarr_dev'))" postgresql://radarr:radarr_dev_pass@localhost:5432/radarr_dev
```

#### 3. VS Code Configuration

**`.vscode/settings.json`**:
```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy",
    "sqlx.database": "postgresql://radarr:radarr_dev_pass@localhost:5432/radarr_dev"
}
```

**`.vscode/launch.json`** (for debugging):
```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "lldb",
            "request": "launch",
            "name": "Debug Radarr MVP",
            "cargo": {
                "args": ["build", "--bin=radarr-rust"],
                "filter": {
                    "name": "radarr-rust",
                    "kind": "bin"
                }
            },
            "args": [],
            "cwd": "${workspaceFolder}",
            "env": {
                "RUST_LOG": "debug"
            }
        }
    ]
}
```

## Next Steps

After completing the setup:

1. **Explore the API**: Visit `http://localhost:3000/api/docs`
2. **Run the test suite**: `cargo test`
3. **Check the database**: Connect and explore the schema
4. **Read the architecture docs**: Understand the PostgreSQL consolidation
5. **Start contributing**: Check the feature development guides

## Support

- **Documentation**: Check the `docs/` directory
- **Architecture Decisions**: See `radarr-rust-plans/00-Architecture/`
- **Feature Guides**: Explore `features/` directory
- **Performance**: Review `docs/POSTGRESQL_CONSOLIDATION.md`

The simplified PostgreSQL-only architecture provides a much faster and easier development experience compared to the previous dual-database setup!

---

**Setup Time**: ~5 minutes  
**Dependencies**: PostgreSQL only (simplified from dual-database)  
**Status**: Production Ready ✅