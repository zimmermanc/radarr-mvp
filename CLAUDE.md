# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Testing
```bash
# Build entire workspace
cargo build --workspace

# Build with optimizations
cargo build --workspace --release

# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p radarr-core
cargo test -p radarr-analysis
cargo test -p radarr-api

# Run single test by name
cargo test test_movie_creation

# Run with verbose output
cargo test --workspace -- --nocapture

# Clean and rebuild
cargo clean && cargo build --workspace

# Build and test with validation script
./scripts/build_and_test.sh

# Create release package
./scripts/build_and_test.sh --package
```

### Analysis Tools
```bash
# HDBits analysis tools (from analysis crate)
cargo run --bin hdbits-analyzer -- --help
cargo run --bin hdbits-comprehensive-analyzer -- --output results.json
cargo run --bin hdbits-session-analyzer -- --session-cookie "COOKIE_STRING"
cargo run --bin hdbits-browse-analyzer -- --max-pages 5

# Dry run for testing
cargo run --bin hdbits-analyzer -- --dry-run

# Code quality
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features

# Documentation
cargo doc --no-deps
cargo doc --no-deps --open
```

### Server Deployment
```bash
# Build for deployment
cargo build --release

# Deploy to test server
./scripts/deploy.sh

# Manual deployment
scp target/release/unified-radarr root@192.168.0.138:/opt/radarr/
ssh root@192.168.0.138 'systemctl restart radarr'

# Check deployment status
ssh root@192.168.0.138 'systemctl status radarr'
ssh root@192.168.0.138 'journalctl -u radarr -f'
```

## Architecture Overview

### Clean Architecture Workspace Structure
This is a Rust workspace using clean architecture with domain-driven design:

```
unified-radarr/
├── crates/
│   ├── core/           # Pure domain logic (no external dependencies)
│   ├── analysis/       # HDBits scene group analysis system
│   ├── api/            # HTTP API layer (Axum framework)
│   ├── infrastructure/ # External concerns (database, HTTP, filesystem)
│   ├── indexers/       # Torrent indexer integrations
│   ├── decision/       # Release selection and quality profiles
│   ├── downloaders/    # Download client integrations
│   └── import/         # Media import pipeline
├── scripts/           # Build and deployment automation
└── systemd/           # Service files for server deployment
```

### Key Architectural Principles

**Domain-Driven Design**: The `core` crate contains pure business logic with zero external dependencies. All other crates depend on core through well-defined interfaces.

**Dependency Inversion**: Infrastructure implements repository traits defined in core, ensuring business logic remains independent of external concerns.

**Workspace Organization**: Shared dependencies managed at workspace level in root `Cargo.toml` with `workspace = true` inheritance in individual crates.

**Error Handling**: Centralized `RadarrError` enum in core using `thiserror` for structured error propagation across all crates.

### Crate Dependencies and Responsibilities

- **radarr-core**: Domain models (Movie, Release, Quality), repository traits, and business rules
- **radarr-analysis**: HDBits analyzers with 4 CLI binaries for scene group reputation scoring
- **radarr-api**: HTTP endpoints using Axum (depends on core)
- **radarr-infrastructure**: Database, file system, external service implementations (depends on core)
- **radarr-indexers**: Torrent site integrations (depends on core)
- **radarr-decision**: Quality profiles and release selection logic (depends on core)
- **radarr-downloaders**: qBittorrent/SABnzbd clients (depends on core)
- **radarr-import**: File import and library management (depends on core)

### Critical Integration Points

**HDBits Analysis System**: Production-ready analyzers with:
- Session cookie authentication support
- Rate limiting (conservative delays)
- Multiple analysis strategies (browse, session, comprehensive)
- Evidence-based reputation scoring
- CLI tools for standalone execution

**Server Deployment**: Complete deployment automation with:
- SSH-based deployment scripts
- Systemd service configuration
- Environment variable management
- Log rotation and monitoring
- Automatic restart on failure
- Health check integration

### Development Workflow

**Building**: Use `cargo build --workspace` to compile all crates. The release profile enables LTO and single codegen unit for maximum optimization.

**Testing**: Tests are organized per-crate. Run workspace-wide with `cargo test --workspace` or target specific crates with `-p` flag.

**Quality Checks**: Format with `cargo fmt --all`, lint with `cargo clippy --workspace --all-targets --all-features`.

**Analysis Tools**: The HDBits analyzers in `crates/analysis/src/bin/` are standalone CLI tools. Run with `cargo run --bin <analyzer-name>`.

**Documentation**: Generate with `cargo doc --no-deps`. Documentation follows Rust conventions with module-level docs and examples.

## Common Development Tasks

### Adding a New Domain Model
1. Define the model in `crates/core/src/models/`
2. Add repository trait in `crates/core/src/domain/repositories.rs`
3. Implement repository in `crates/infrastructure/src/`
4. Wire up in API layer if needed

### Running HDBits Analysis
```bash
# Basic analysis with dry run
cargo run --bin hdbits-analyzer -- --dry-run

# Full analysis with output
cargo run --bin hdbits-comprehensive-analyzer -- \
  --output ./analysis_results \
  --session-cookie "YOUR_SESSION_COOKIE"

# Browse-based analysis
cargo run --bin hdbits-browse-analyzer -- \
  --max-pages 5 \
  --output ./browse_results
```

### Working with Server Deployment
```bash
# Build and deploy
./scripts/deploy.sh

# Check service status
ssh root@192.168.0.138 'systemctl status radarr'

# View logs
ssh root@192.168.0.138 'journalctl -u radarr -f'

# Restart service
ssh root@192.168.0.138 'systemctl restart radarr'

# Access application (ensure port forwarding or direct access)
curl http://192.168.0.138:7878/health
```

## Performance Optimization

**Release Build**: The release profile in root `Cargo.toml` enables:
- `opt-level = 3` for maximum optimization
- `lto = true` for link-time optimization
- `codegen-units = 1` for better optimization at cost of compile time
- `panic = "abort"` for smaller binaries

**Development Build**: Optimized for fast compilation with `opt-level = 0` and debug symbols enabled.

## Error Handling Patterns

All crates use the centralized `RadarrError` enum from core:
- `MovieNotFound` - Domain entity not found
- `InvalidQualityProfile` - Configuration validation errors
- `IndexerError` - External service failures
- `ValidationError` - Domain validation failures
- `ExternalServiceError` - Generic external service errors

Use `Result<T, RadarrError>` as return type alias (defined as `RadarrResult<T>` in core).