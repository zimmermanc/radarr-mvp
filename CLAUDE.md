# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Recent Completions (Week 6-7 - Infrastructure & Discovery)

### 🚧 Streaming Service Integration (2025-08-23 - In Progress)
- **TMDB Extensions**: Trending movies/TV (day/week), upcoming releases, watch providers
- **Trakt OAuth**: Device flow authentication with 24hr token refresh (March 2025 compliance)
- **Watchmode Integration**: CSV ID mapping (TMDB↔Watchmode), deep links, streaming availability
- **PostgreSQL Caching**: Aggressive caching strategy (24hr TTL) for API quota management
- **Trending Aggregation**: Multi-source scoring algorithm with de-duplication

### ✅ CI/CD Pipeline Implementation (2025-08-23)
- **GitHub Actions**: 6 comprehensive workflows deployed
- **Security Scanning**: SAST (Semgrep, CodeQL), SCA (cargo-audit, Snyk), secrets detection
- **Code Quality**: Codacy integration, coverage reporting, complexity analysis
- **Automation**: Dependabot updates, PR validation, multi-platform testing
- **Documentation**: Complete CI/CD guide with security best practices

### ✅ Test Suite Restoration (2025-08-23)
- Fixed compilation errors across all integration tests
- 162+ tests now passing (up from 35)
- Coverage reporting to Codecov and Codacy
- Test matrix across Linux, macOS, Windows

### ✅ Quality Engine (Week 4-5)
- Advanced release scoring with custom formats
- Quality profiles with upgrade logic
- 19 comprehensive quality tests
- Sub-5ms database query performance

## Development Commands

### Core Build and Test Commands
```bash
# For unified-radarr workspace (preferred active development)
cd unified-radarr

# Build entire workspace
cargo build --workspace

# Build with optimizations
cargo build --workspace --release

# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p radarr-core

# Run single test by name
cargo test test_movie_creation

# Clean and rebuild
cargo clean && cargo build

# Development with auto-reload
cargo watch -x "run"
```

### Analysis and Quality Tools
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

### CI/CD and Security Commands
```bash
# Setup GitHub secrets (interactive)
./scripts/setup-github-secrets.sh

# Trigger CI/CD workflows manually
gh workflow run ci.yml
gh workflow run security.yml
gh workflow run codacy.yml

# Security scanning locally
cargo audit --json
cargo deny check
gitleaks detect --source .

# Check for unused dependencies
cargo machete
cargo +nightly udeps

# Generate test coverage
cargo tarpaulin --workspace --all-features --out lcov

# View GitHub Actions status
gh run list
gh run view
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

### Streaming Service Commands
```bash
# Refresh Watchmode CSV mapping (manual trigger)
cargo run --bin watchmode-sync

# Test Trakt OAuth flow
cargo run --bin trakt-auth

# View cached trending data
psql -d radarr -c "SELECT cache_key, expires_at FROM streaming_cache WHERE cache_key LIKE 'trending%'"

# Clear streaming cache (force refresh)
psql -d radarr -c "DELETE FROM streaming_cache WHERE cache_key LIKE 'tmdb:%' OR cache_key LIKE 'trakt:%'"
```

### Server Deployment
```bash
# Build for deployment (from unified-radarr directory)
cd unified-radarr
cargo build --release

# Deploy to production server (ready for deployment)
./scripts/deploy.sh

# Manual deployment to target server (run from unified-radarr directory)
scp target/release/radarr-mvp root@192.168.0.138:/opt/radarr/
ssh root@192.168.0.138 'systemctl restart radarr'

# Verify deployment
ssh root@192.168.0.138 'systemctl status radarr'
curl http://192.168.0.138:7878/health
```

## Project Architecture

### Dual-Structure Repository
This repository contains two main development paths:

1. **`unified-radarr/`** - Primary active development workspace using clean architecture
2. **Root project** - Legacy/experimental structure with extensive documentation

**Work in `unified-radarr/` for active development**. The root contains planning docs, legacy code, and architecture documentation.

### Unified-Radarr Workspace Structure
```
unified-radarr/
├── .github/
│   ├── workflows/     # CI/CD pipelines (6 comprehensive workflows)
│   ├── dependabot.yml # Automated dependency updates
│   └── SECRETS_SETUP.md # Security configuration guide
├── crates/
│   ├── core/           # Pure domain logic (no external dependencies)
│   ├── analysis/       # HDBits scene group analysis system  
│   ├── api/           # HTTP API layer (Axum framework)
│   ├── infrastructure/ # External concerns (database, HTTP, filesystem)
│   ├── indexers/      # Torrent indexer integrations
│   ├── decision/      # Release selection and quality profiles
│   ├── downloaders/   # Download client integrations (qBittorrent, SABnzbd)
│   └── import/        # Media import pipeline
├── docs/              # Architecture, CI/CD guide, setup documentation
├── systemd/           # Service files for server deployment
├── scripts/           # Build, deployment, and CI/CD setup scripts
└── web/              # React frontend application
```

### Clean Architecture Principles

**Domain-First Design**: The `core` crate contains pure business logic with no external dependencies. All other crates depend on core, never the reverse.

**Repository Pattern**: Infrastructure crate implements repository traits defined in core for data abstraction.

**Analysis System**: Production-ready HDBits analyzers with session authentication, rate limiting, and evidence-based reputation scoring.

**Async-First**: Built on Tokio with async/await throughout. Uses `async-trait` for abstractions.

**Error Handling**: Custom `RadarrError` enum with `thiserror`. All fallible operations return `Result<T, RadarrError>`.

### Technology Stack

**Core**:
- **Language**: Rust 2021 Edition
- **Web Framework**: Axum 0.7 (high-performance async)
- **Async Runtime**: Tokio (production-grade)
- **Database**: PostgreSQL 16 with SQLx 0.8 (async, compile-time checked queries)

**Key Dependencies**:
- **Parsing**: nom 7.1, regex 1.10 for release name parsing
- **HTTP Client**: reqwest 0.12 with cookies for indexer integration
- **Serialization**: serde with JSON/YAML support
- **Testing**: proptest for property-based testing, criterion for benchmarks

### Database Architecture (PostgreSQL-Only)

**Single Database Design**: Consolidated from dual EdgeDB+PostgreSQL to PostgreSQL-only for 40% performance improvement and simplified deployment.

**Advanced Features**:
- **JSONB Support**: Metadata storage with GIN indexing for graph-like queries
- **Full-Text Search**: Built-in PostgreSQL search with ranking
- **Connection Pooling**: SQLx async pool with health checks
- **Migration System**: Version-controlled schema evolution with sqlx migrate

### Critical Integration Points

**HDBits Analysis**: Sophisticated scene group reputation analysis with multiple strategies (browse, session, comprehensive). Rate-limited and production-ready.

**Quality Profiles**: Decision engine evaluates releases against configurable profiles with custom format support.

**Streaming Service Integration**: TMDB + Trakt + Watchmode aggregation for trending content discovery with streaming availability and deep links. PostgreSQL-cached for aggressive rate limit management.

**Server Ready**: Complete systemd service files and deployment scripts for direct server deployment.

### Development Workflow Notes

**Workspace Dependencies**: Common dependencies defined at workspace level with `workspace = true` inheritance.

**Testing Strategy**: Unit tests per crate, integration tests cross-crate, property-based tests for parsers, and end-to-end automation.

**Build Profiles**: Release uses LTO and single codegen unit for optimization. Development prioritizes compile speed.

**Environment Setup**:
```bash
# Quick start (5-minute setup)
cp .env.example .env          # Configure environment
systemctl start postgresql   # Start PostgreSQL
sqlx migrate run             # Run migrations
cargo run                    # Start server
```

## Development Status and Known Issues

### Current State (~82% Complete)
**Working Components**:
- ✅ **HDBits Integration**: Scene group analysis, torrent search, rate limiting operational
- ✅ **qBittorrent Client**: Download management, progress tracking, torrent operations
- ✅ **Import Pipeline**: File analysis, hardlinking, renaming, library integration
- ✅ **Queue Processing**: Background job system with retry logic and event-driven workflows
- ✅ **Database Operations**: PostgreSQL with 15+ tables, full CRUD operations
- ✅ **RSS Monitoring**: Calendar tracking, release notifications, feed management
- ✅ **Web Interface**: React UI with real-time updates and progress tracking
- ✅ **API Layer**: 25+ endpoints with real data, authentication, rate limiting
- ✅ **CI/CD Pipeline**: GitHub Actions with security scanning, quality checks, automated updates
- ✅ **Test Suite**: 162+ tests passing across 8 crates with coverage reporting
- ✅ **Quality Engine**: Advanced release scoring with custom formats and profiles

**Production Ready Features**:
- ✅ **Search → Download → Import Pipeline**: Complete automation workflow
- ✅ **Event-Driven Architecture**: Component communication via tokio broadcast channels
- ✅ **External Service Integration**: TMDB API, HDBits scraper, qBittorrent client
- ✅ **Deployment Ready**: SSH-based deployment to root@192.168.0.138
- ✅ **Security Scanning**: SAST (Semgrep, CodeQL), SCA (cargo-audit, Snyk), secrets detection
- ✅ **Automated Dependencies**: Dependabot weekly updates with security patches
- ✅ **Code Quality**: Codacy integration, Clippy pedantic, coverage tracking
- ✅ **PR Validation**: Size checks, conventional commits, automated testing

### Performance Targets
- API Response: <100ms p95
- Database Queries: <5ms for complex operations  
- HDBits Integration: <2 seconds per search
- Memory Usage: <500MB total system

## Agent and Model Integration

### Model Selection Strategy
- **Opus 4.1**: Research, architecture design, complex algorithms
- **Sonnet 4**: Implementation, APIs, testing, day-to-day development
- **Haiku 3.5**: Simple tasks, formatting, configuration updates

### Specialized Agents
- **Development**: rust-specialist, backend-developer, database-architect
- **Quality**: test-engineer, code-reviewer, performance-engineer  
- **Infrastructure**: devops-engineer, security-auditor
- **Domain-Specific**: parser-expert, decision-expert, import-specialist, indexer-specialist

## Security and Configuration

**Database Security**: Connection pooling with health checks, SQLx compile-time query verification prevents SQL injection.

**API Security**: Rate limiting, input validation, API key authentication.

**Environment Configuration**: Use `.env` files for development, systemd environment files for production.

**API Keys Required**:
- `TMDB_API_KEY`: Existing TMDB API key (already configured)
- `TRAKT_CLIENT_ID` & `TRAKT_CLIENT_SECRET`: Trakt OAuth credentials
- `WATCHMODE_API_KEY`: Watchmode API key (free tier: 1000 req/month)

**HDBits Integration**: Production-ready scraper with scene group analysis, session authentication, and intelligent rate limiting. Operational with comprehensive error handling.

**Streaming Service Quotas**:
- **Watchmode**: 1000 requests/month (~33/day) - aggressively cached
- **TMDB**: Standard rate limits - cached 3-24 hours
- **Trakt**: OAuth with 24hr token expiry - auto-refresh required

## Testing and Quality Assurance

### Test Execution
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

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace --all-features --out lcov
```

### Code Quality
```bash
# Comprehensive quality check (same as CI)
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo audit
cargo deny check

# Check for unused dependencies
cargo machete
cargo +nightly udeps

# Security scanning
cargo audit --json
gitleaks detect --source .
```

## Key Files and Locations

**Primary Development**: Work in `unified-radarr/` directory
**Configuration**: `.env` for local development, `systemd/` for service deployment
**Migrations**: `unified-radarr/migrations/` for database schema
**Documentation**: `docs/` contains architecture decisions, CI/CD guide, and setup guides
**Analysis Tools**: `unified-radarr/crates/analysis/src/bin/` for HDBits analyzers
**CI/CD**: `.github/workflows/` for GitHub Actions pipelines
**Security**: `.github/SECRETS_SETUP.md` for secure token configuration