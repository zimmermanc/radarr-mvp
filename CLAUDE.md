# CLAUDE.md - Radarr MVP Complete Project Reference

**Modern, high-performance movie automation and management system built with Rust and React**

**Current Status**: 85% Complete | Production deployment operational on test server (192.168.0.138)

---

## 🚀 Project Overview

### Current State (85% Complete)

**Production Ready Features**:
- ✅ **Complete Search → Download → Import Pipeline**: End-to-end automation working
- ✅ **HDBits Integration**: Scene group analysis, torrent search, rate limiting operational
- ✅ **qBittorrent Client**: Download management, progress tracking, torrent operations
- ✅ **Import Pipeline**: File analysis, hardlinking, renaming, library integration
- ✅ **Quality Engine**: Advanced scoring with custom formats and correlation IDs
- ✅ **Queue Processing**: Background job system with retry logic and event-driven workflows
- ✅ **Database Operations**: PostgreSQL with 15+ tables, full CRUD operations
- ✅ **RSS Calendar**: Feed management and upcoming release tracking
- ✅ **Web Interface**: React UI with real-time updates and progress tracking
- ✅ **API Layer**: 25+ endpoints with authentication, rate limiting, Prometheus metrics
- ✅ **Test Server Deployment**: SSH-based deployment to root@192.168.0.138 operational

**Testing Status**: 35/35 tests passing
**Performance**: API <100ms p95, Memory <500MB, Database <5ms queries
**Monitoring**: Prometheus metrics, correlation IDs, circuit breakers implemented

### Next Phase: Lists & Discovery (Week 6)

**Upcoming Features**:
- 📋 **List Management**: Trakt, IMDb, TMDb integration for watchlists
- 🔍 **Discovery Engine**: Trending movies, recommendations, popular lists
- 📊 **Analytics Dashboard**: Usage metrics, library statistics, performance monitoring
- 🎯 **Smart Recommendations**: ML-based movie suggestions based on library

---

## 💻 Development Commands

### Core Build and Test Commands
```bash
# For unified-radarr workspace (active development)
cd /home/thetu/radarr-mvp/unified-radarr

# Build entire workspace
cargo build --workspace

# Build with optimizations
cargo build --workspace --release

# Run all tests (35/35 passing)
cargo test --workspace

# Run tests for specific crate
cargo test -p radarr-core
cargo test -p radarr-analysis
cargo test -p radarr-api

# Run single test by name
cargo test test_movie_creation

# Clean and rebuild
cargo clean && cargo build

# Development with auto-reload
cargo watch -x "run"
```

### Quality Engine and HDBits Analysis Tools
```bash
# HDBits analysis tools (from analysis crate)
cargo run --bin hdbits-analyzer -- --help
cargo run --bin hdbits-comprehensive-analyzer -- --output results.json
cargo run --bin hdbits-session-analyzer -- --session-cookie "COOKIE_STRING"
cargo run --bin hdbits-browse-analyzer -- --max-pages 5

# Code quality
cargo clippy --workspace --all-targets --all-features
cargo fmt --all
cargo audit

# Documentation
cargo doc --no-deps --open

# Build validation script
./scripts/build_and_test.sh
```

### Test Server Deployment
```bash
# Build for deployment
cd /home/thetu/radarr-mvp/unified-radarr
cargo build --release

# Deploy to production server (ready for deployment)
./scripts/deploy.sh

# Manual deployment to target server
scp target/release/unified-radarr root@192.168.0.138:/opt/radarr/
ssh root@192.168.0.138 'systemctl restart radarr'

# Verify deployment
ssh root@192.168.0.138 'systemctl status radarr'
curl http://192.168.0.138:7878/health
```

### Database Operations
```bash
# Database migrations (requires sqlx-cli)
cargo install sqlx-cli --features postgres

# Run all migrations
sqlx migrate run

# Create new migration
sqlx migrate add description_of_migration

# Check migration status  
sqlx migrate info

# Reset database (development only)
sqlx database drop && sqlx database create && sqlx migrate run
```

---

## 🏗️ Architecture & Technology Stack

### Dual-Structure Repository
This repository contains two main development paths:

1. **`unified-radarr/`** - Primary active development workspace using clean architecture
2. **Root project** - Legacy/experimental structure with extensive documentation

**Work in `unified-radarr/` for active development**. The root contains planning docs, legacy code, and architecture documentation.

### Unified-Radarr Workspace Structure
```
/home/thetu/radarr-mvp/unified-radarr/
├── crates/
│   ├── core/           # Pure domain logic (no external dependencies)
│   ├── analysis/       # HDBits scene group analysis system  
│   ├── api/           # HTTP API layer (Axum framework)
│   ├── infrastructure/ # External concerns (database, HTTP, filesystem)
│   ├── indexers/      # Torrent indexer integrations
│   ├── decision/      # Release selection and quality profiles
│   ├── downloaders/   # Download client integrations (qBittorrent, SABnzbd)
│   ├── import/        # Media import pipeline
│   └── notifications/ # Notification systems
├── web/               # React frontend application
├── systemd/           # Service files for server deployment
├── scripts/           # Build and deployment scripts
└── migrations/        # Database schema migrations
```

### Clean Architecture Principles

**Domain-First Design**: The `core` crate contains pure business logic with no external dependencies. All other crates depend on core, never the reverse.

**Repository Pattern**: Infrastructure crate implements repository traits defined in core for data abstraction.

**Quality Engine**: Production-ready quality assessment with correlation IDs, custom formats, and advanced scoring algorithms.

**Async-First**: Built on Tokio with async/await throughout. Uses `async-trait` for abstractions.

**Error Handling**: Custom `RadarrError` enum with `thiserror`. All fallible operations return `Result<T, RadarrError>`.

### Technology Stack

**Core Technologies**:
- **Language**: Rust 2021 Edition
- **Web Framework**: Axum 0.7 (high-performance async)
- **Async Runtime**: Tokio (production-grade)
- **Database**: PostgreSQL 16 with SQLx 0.8 (async, compile-time checked queries)
- **Frontend**: React 18 with TypeScript, Vite build system

**Key Dependencies**:
- **Parsing**: nom 7.1, regex 1.10 for release name parsing
- **HTTP Client**: reqwest 0.12 with cookies for indexer integration
- **Serialization**: serde with JSON/YAML support
- **Testing**: proptest for property-based testing, criterion for benchmarks
- **Monitoring**: Prometheus metrics, correlation IDs, circuit breakers

### Database Architecture (PostgreSQL-Only)

**Single Database Design**: Consolidated from dual EdgeDB+PostgreSQL to PostgreSQL-only for 40% performance improvement and simplified deployment.

**Advanced Features**:
- **JSONB Support**: Metadata storage with GIN indexing for graph-like queries
- **Full-Text Search**: Built-in PostgreSQL search with ranking
- **Connection Pooling**: SQLx async pool with health checks
- **Migration System**: Version-controlled schema evolution with sqlx migrate
- **15+ Tables**: Complete schema for movies, releases, downloads, quality profiles, custom formats

### Critical Integration Points

**HDBits Analysis**: Sophisticated scene group reputation analysis with multiple strategies (browse, session, comprehensive). Rate-limited and production-ready with session authentication.

**Quality Engine**: Advanced decision engine evaluates releases against configurable profiles with custom format support and correlation ID tracking.

**Circuit Breakers**: Enterprise-grade resilience with automatic failure detection and recovery for all external services.

**Server Ready**: Complete systemd service files and SSH-based deployment scripts for direct server deployment to 192.168.0.138.

---

## 🔌 API Documentation & Endpoints

### Authentication
All API requests require authentication via API key in the request header:
```http
X-Api-Key: your-api-key-here
```

### Base URLs
- **Development**: `http://localhost:7878`
- **Test Server**: `http://192.168.0.138:7878`
- **API Version**: `/api/v3/` (current)
- **Legacy**: `/api/v1/` (compatibility)

### Core API Endpoints (25+ endpoints)

#### Health and System
- **GET /health** - Basic health check
- **GET /health/detailed** - Detailed component health
- **GET /api/v1/system/status** - Legacy system status

#### Movie Management
- **GET /api/v3/movie** - List movies with pagination
- **GET /api/v3/movie/{id}** - Get specific movie by UUID
- **POST /api/v3/movie** - Add new movie
- **PUT /api/v3/movie/{id}** - Update existing movie
- **DELETE /api/v3/movie/{id}** - Remove movie

#### Search and Discovery
- **GET /api/v3/movie/lookup** - Search movies on TMDB
- **POST /api/v3/indexer/search** - Search releases across indexers

#### Download Management
- **POST /api/v3/download** - Start download
- **GET /api/v3/download** - List active downloads
- **GET /api/v3/download/{id}** - Get download status
- **DELETE /api/v3/download/{id}** - Cancel download

#### Quality Profiles
- **GET /api/v3/qualityprofile** - List quality profiles
- **GET /api/v3/qualityprofile/{id}** - Get specific profile
- **POST /api/v3/qualityprofile** - Create quality profile
- **PUT /api/v3/qualityprofile/{id}** - Update profile
- **DELETE /api/v3/qualityprofile/{id}** - Delete profile

#### Custom Formats
- **GET /api/v3/customformat** - List custom formats
- **POST /api/v3/customformat** - Create custom format
- **PUT /api/v3/customformat/{id}** - Update format
- **DELETE /api/v3/customformat/{id}** - Delete format
- **POST /api/v3/customformat/{id}/test** - Test format

#### Calendar and Feeds
- **GET /api/v3/calendar** - Upcoming releases
- **GET /feed/v3/calendar/radarr.ics** - iCal feed

#### Commands and Tasks
- **POST /api/v3/command** - Execute system commands
- **GET /api/v3/command/{id}** - Get command status

### Response Formats
**Success Response**:
```json
{
  "id": "123e4567-e89b-12d3-a456-426614174000",
  "title": "The Matrix",
  "year": 1999,
  "status": "downloaded",
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Error Response**:
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Validation failed",
    "request_id": "req_123456789"
  }
}
```

### Rate Limiting
- **Default Limits**: 100 requests per minute per IP
- **Burst Requests**: 20 burst requests allowed
- **Headers**: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`

---

## 🎛️ Configuration & Environment

### Core Environment Variables

#### Server Configuration
```bash
RADARR_HOST=0.0.0.0              # Server bind address
RADARR_PORT=7878                 # Server port
RADARR_MAX_CONNECTIONS=1000      # Maximum concurrent connections
RADARR_REQUEST_TIMEOUT=30        # Request timeout in seconds
RADARR_BASE_URL=/                # Base URL for reverse proxy
```

#### Database Configuration
```bash
DATABASE_URL=postgresql://radarr:password@localhost:5432/radarr_prod
DATABASE_MAX_CONNECTIONS=20      # Connection pool size
DATABASE_CONNECT_TIMEOUT=30      # Connection timeout in seconds
DATABASE_LOG_QUERIES=false       # Log SQL queries (development only)
```

#### External Services
```bash
# HDBits Integration (no Prowlarr needed)
HDBITS_USERNAME=your_username
HDBITS_PASSWORD=your_password

# qBittorrent Integration
QBITTORRENT_BASE_URL=http://192.168.0.138:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=adminpass
QBITTORRENT_CATEGORY=radarr
QBITTORRENT_DOWNLOAD_PATH=/downloads

# TMDB Integration
TMDB_API_KEY=your_tmdb_api_key_here
TMDB_LANGUAGE=en-US
TMDB_REGION=US
```

#### Logging and Security
```bash
# Logging Configuration
RUST_LOG=info                    # Log level (error, warn, info, debug, trace)
LOG_JSON_FORMAT=true            # Use JSON log format (production)
LOG_FILE=/var/log/radarr/radarr.log  # Log file path

# Security Configuration
API_KEY=your_secure_api_key_here
CORS_ORIGINS=*                   # Allowed CORS origins
RATE_LIMIT_REQUESTS=100         # Requests per minute per IP
```

#### Import Configuration
```bash
# File Organization
IMPORT_ENABLED=true             # Enable automatic import
IMPORT_DOWNLOAD_PATH=/downloads # Download directory to monitor
IMPORT_MOVIE_PATH=/movies       # Movie library directory
IMPORT_USE_HARDLINKS=true       # Use hardlinks instead of copy
IMPORT_MOVIE_NAMING="{Movie Title} ({Release Year}) - {Quality}[{MediaInfo}]"
IMPORT_FOLDER_FORMAT="{Movie Title} ({Release Year})"
```

### Configuration Examples

#### Development Environment
```bash
# .env.development
RUST_LOG=debug
RADARR_PORT=7878
DATABASE_URL=postgresql://radarr:radarr@localhost:5432/radarr_dev
DATABASE_LOG_QUERIES=true
HDBITS_USERNAME=dev_user
QBITTORRENT_BASE_URL=http://localhost:8080
IMPORT_DOWNLOAD_PATH=/tmp/downloads
IMPORT_MOVIE_PATH=/tmp/movies
```

#### Production Environment (Test Server)
```bash
# .env.production
RUST_LOG=info
LOG_JSON_FORMAT=true
RADARR_HOST=0.0.0.0
RADARR_PORT=7878
DATABASE_URL=postgresql://radarr:secure_password@localhost:5432/radarr_prod
HDBITS_USERNAME=production_user
QBITTORRENT_BASE_URL=http://192.168.0.138:8080
IMPORT_DOWNLOAD_PATH=/opt/downloads
IMPORT_MOVIE_PATH=/opt/movies
API_KEY=your_secure_api_key_here
```

---

## 🚀 Deployment & Production

### Test Server Deployment (Operational)

**Target Environment**: `root@192.168.0.138`
- **OS**: Linux (Ubuntu/Debian)
- **Services**: PostgreSQL 16+, systemd, qBittorrent
- **Network**: Local network access on port 7878
- **Status**: ✅ OPERATIONAL

#### Automated Deployment
```bash
# From the unified-radarr directory
cd /home/thetu/radarr-mvp/unified-radarr
./scripts/deploy.sh

# Manual deployment
cargo build --release
scp target/release/unified-radarr root@192.168.0.138:/opt/radarr/
ssh root@192.168.0.138 'systemctl restart radarr'
```

#### Service Management
```bash
# Check service status
ssh root@192.168.0.138 'systemctl status radarr'

# View logs
ssh root@192.168.0.138 'journalctl -u radarr -f'

# Restart service
ssh root@192.168.0.138 'systemctl restart radarr'

# Verify deployment
curl http://192.168.0.138:7878/health
```

#### Systemd Service Configuration
```ini
[Unit]
Description=Radarr MVP - Movie Automation
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=radarr
Group=radarr
WorkingDirectory=/opt/radarr
EnvironmentFile=/opt/radarr/.env
ExecStart=/opt/radarr/unified-radarr
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Performance Optimization

**Current Performance Metrics**:
- **API Response Time**: <100ms p95
- **Memory Usage**: <500MB total system
- **Database Queries**: <5ms for complex operations
- **HDBits Integration**: <2 seconds per search
- **Startup Time**: ~250ms

**Release Build Optimizations**:
- `opt-level = 3` for maximum optimization
- `lto = true` for link-time optimization
- `codegen-units = 1` for better optimization
- `panic = "abort"` for smaller binaries

### Monitoring & Observability

**Health Check Endpoints**:
- **Basic**: `GET /health` - Service status and uptime
- **Detailed**: `GET /health/detailed` - Component health with metrics
- **Services**: `GET /health/services/{service}` - Individual service status

**Prometheus Metrics**:
- HTTP request duration and error rates
- Database connection pool utilization
- External service response times (TMDB, HDBits, qBittorrent)
- Memory and CPU usage patterns
- Download/import statistics

**Circuit Breakers**:
- Automatic failure detection for external services
- Graceful degradation during outages
- Configurable failure thresholds and recovery timeouts
- Real-time status reporting via health endpoints

**Correlation IDs**:
- Request tracking across components
- End-to-end tracing for debugging
- Performance analysis and bottleneck identification

### Security Features

**Authentication**:
- API key authentication for all endpoints
- Configurable rate limiting (100 req/min default)
- CORS support with configurable origins
- Session timeout management

**Data Protection**:
- Environment variable configuration
- Secure password handling for external services
- Database connection security
- Log sanitization for sensitive data

---

## 🎯 Quality Engine Implementation

### Advanced Quality Assessment

**Custom Formats**: Advanced pattern matching for release quality assessment
- **Remux Detection**: `remux|bdremux|bd25|bd50` patterns
- **HDR Content**: `HDR|HDR10|DV|Dolby.Vision` detection
- **Audio Formats**: `Atmos|DTS-HD|TrueHD` identification
- **Scene Groups**: Reputation-based scoring with HDBits integration

**Quality Profiles**: Configurable upgrade preferences
- **Cutoff Quality**: Minimum acceptable quality threshold
- **Allowed Qualities**: HDTV-720p, Bluray-1080p, etc.
- **Custom Format Scores**: Weighted scoring system (-100 to +100)
- **Size Constraints**: Min/max/preferred file sizes per quality

**Decision Engine**: Intelligent release selection
- **Multi-factor Scoring**: Quality + custom formats + size + seeders
- **Upgrade Logic**: Automatic quality improvements when better releases available
- **Language Preferences**: Multi-language support with priority ordering
- **Correlation IDs**: Track decision reasoning across pipeline

### HDBits Integration & Scene Group Analysis

**Production-Ready Features**:
- **Session Authentication**: Automatic login and cookie management
- **Rate Limiting**: Conservative delays to prevent IP bans
- **Multiple Strategies**: Browse, session, comprehensive analysis
- **Reputation Scoring**: Evidence-based scene group quality assessment
- **CLI Tools**: 4 standalone analyzers for manual scene group research

**Analysis Tools**:
```bash
# Comprehensive scene group analysis
cargo run --bin hdbits-comprehensive-analyzer -- --output results.json

# Session-based analysis with authentication
cargo run --bin hdbits-session-analyzer -- --session-cookie "COOKIE_STRING"

# Browse-based multi-page analysis
cargo run --bin hdbits-browse-analyzer -- --max-pages 5

# Basic analysis with dry run
cargo run --bin hdbits-analyzer -- --dry-run
```

---

## 🧪 Testing & Quality Assurance

### Test Suite Status: 35/35 Passing ✅

**Unit Tests**: Component-level functionality validation
- Core domain logic and business rules
- Repository patterns and data access
- Quality engine algorithms
- Custom format pattern matching

**Integration Tests**: Cross-component functionality
- Database operations with real PostgreSQL
- External service integration (mocked for CI)
- End-to-end API workflows
- Import pipeline with file system operations

**Property-Based Tests**: Edge case validation
- Release name parsing with random inputs
- Quality scoring algorithm validation
- UUID generation and validation
- Date/time handling across timezones

### Testing Commands
```bash
# Run all tests with output
cargo test --workspace -- --nocapture

# Run specific test module
cargo test movies::

# Run integration tests (requires test database)
TEST_DATABASE_URL=postgresql://radarr:test@localhost:5432/radarr_test cargo test integration

# Property-based tests
cargo test --lib proptest

# Benchmarks
cargo bench

# Code coverage
cargo tarpaulin --out html
```

### Code Quality Standards
```bash
# Comprehensive quality check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo audit

# Documentation coverage
cargo doc --no-deps --document-private-items

# Dependency analysis
cargo tree --duplicates
```

---

## 🚧 Development Workflow & Common Tasks

### Development Environment Setup
```bash
# Quick start (5-minute setup)
cd /home/thetu/radarr-mvp/unified-radarr
cp .env.example .env          # Configure environment
sudo systemctl start postgresql # Start PostgreSQL
sqlx migrate run             # Run migrations
cargo run                    # Start server
```

### Frontend Development
```bash
# Start frontend development server
cd /home/thetu/radarr-mvp/unified-radarr/web
npm install
npm run dev  # Available at http://localhost:5173
```

### Adding New Features

**1. Domain Model Addition**:
1. Define model in `crates/core/src/models/`
2. Add repository trait in `crates/core/src/domain/repositories.rs`
3. Implement repository in `crates/infrastructure/src/`
4. Add API endpoints in `crates/api/src/handlers/`
5. Create database migration in `migrations/`
6. Add comprehensive tests

**2. External Service Integration**:
1. Create client in appropriate crate (`crates/indexers/`, `crates/downloaders/`)
2. Implement circuit breaker pattern
3. Add health check integration
4. Configure environment variables
5. Add integration tests
6. Update deployment configuration

**3. Quality Engine Enhancement**:
1. Define custom format specification in `crates/decision/src/formats/`
2. Add pattern matching logic
3. Update scoring algorithms
4. Add test cases with real release examples
5. Document pattern reasoning

### Error Handling Patterns

All crates use the centralized `RadarrError` enum from core:
- `MovieNotFound` - Domain entity not found
- `InvalidQualityProfile` - Configuration validation errors
- `IndexerError` - External service failures (with circuit breaker state)
- `ValidationError` - Domain validation failures
- `ExternalServiceError` - Generic external service errors with context
- `DatabaseError` - SQLx error wrapping with query context

Use `Result<T, RadarrError>` as return type alias (defined as `RadarrResult<T>` in core).

### Performance Benchmarking

**Modern Testing Tools**:
- **k6**: Comprehensive load testing with scenarios
- **vegeta**: High-throughput HTTP testing
- **System Monitoring**: Real-time metrics during tests
- **Prometheus Integration**: Performance metrics collection

**Benchmarking Commands**:
```bash
# Performance test suite
./scripts/perf/benchmark.sh

# k6 load testing
k6 run scripts/perf/k6-load-test.js

# vegeta high-throughput testing
./scripts/perf/vegeta-test.sh

# System monitoring during tests
./scripts/perf/monitor.sh
```

---

## 📊 Current Sprint Status & Next Steps

### Week 6: Lists & Discovery Phase

**Current Sprint Achievements**:
- ✅ Quality Engine hardening completed with correlation IDs
- ✅ HDBits integration hardened with circuit breakers
- ✅ 35/35 tests passing with comprehensive coverage
- ✅ Prometheus metrics operational
- ✅ Test server deployment fully operational
- ✅ Performance benchmarking infrastructure modernized

**Week 6 Priorities**:
1. **List Management System**
   - Trakt watchlist integration
   - IMDb list synchronization
   - TMDb trending/popular lists
   - Custom user lists with CRUD operations

2. **Discovery Engine**
   - Trending movies dashboard
   - Recommendation algorithms based on library
   - Similar movie suggestions
   - Genre-based discovery

3. **Analytics Dashboard**
   - Library statistics and growth metrics
   - Download/import success rates
   - Quality distribution analysis
   - User engagement metrics

4. **Smart Recommendations**
   - ML-based movie suggestions
   - Collaborative filtering
   - Content-based recommendations
   - Integration with external rating services

### Success Metrics for Week 6
- [ ] Trakt watchlist synchronization operational
- [ ] TMDb trending/popular list integration working
- [ ] Basic recommendation engine providing suggestions
- [ ] Analytics dashboard showing library metrics
- [ ] User can discover and add movies from various list sources

### Future Roadmap (Beyond Week 6)

**Month 2: Advanced Features**
- Real-time notifications system
- Advanced scheduling and automation
- Multi-user support with permissions
- Mobile app development

**Month 3: Scale & Polish**
- Horizontal scaling preparation
- Advanced caching strategies
- UI/UX improvements
- Documentation and user guides

---

**This documentation serves as the single source of truth for all Radarr MVP development, deployment, and operational activities.**