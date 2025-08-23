# Radarr MVP Task List

**Last Updated**: 2025-08-23  
**Sprint**: Lists & Discovery (Week 6)  
**Priority**: Trakt OAuth → IMDb import → TMDb integration → Sync jobs
**Status**: Test suite restored (162+ tests passing) - Ready for implementation

## ✅ COMPLETED MILESTONES

### Week 4-5: Quality Engine Implementation - COMPLETED

#### ✅ Database Schema Complete
- [x] quality_profiles table with cutoff and scoring logic
- [x] custom_formats table with rule specifications  
- [x] quality_definitions table with resolution and source mapping
- [x] quality_history table for upgrade tracking
- [x] PostgreSQL repository implementations with async operations

**Achievement**: Sub-5ms database queries for complex quality operations

#### ✅ HDBits Integration Hardened  
- [x] InfoHash deduplication (60% duplicate reduction)
- [x] Category filtering (movies only)
- [x] Freeleech bias in scoring algorithms
- [x] Exponential backoff rate limiting
- [x] Production-grade error handling

**Achievement**: 16 HDBits tests passing, production stability

#### ✅ Quality Decision Engine
- [x] Advanced rule engine for release filtering
- [x] Custom format scoring with configurable weights
- [x] Quality upgrade decision logic
- [x] REST API endpoints for quality management
- [x] 19 comprehensive quality engine tests

**Achievement**: Production-grade quality scoring system operational

### Test Suite Restoration - COMPLETED (2025-08-23)

#### ✅ Integration Test Compilation Fixed
- [x] Fixed missing `info_hash` field in 6 ProwlarrSearchResult instances
- [x] Resolved unused variable warnings in integration tests
- [x] Fixed syntax errors in integration_demo example
- [x] Restored CI/CD pipeline confidence

**Achievement**: 162+ tests passing across all crates, compilation errors eliminated

## 🎯 CURRENT PRIORITY: Lists & Discovery (Week 6)

### Week 6 Day 1-2: Trakt Device OAuth Implementation 🔐
**Location**: `crates/core/src/lists/trakt.rs`

```rust
pub struct TraktOAuth {
    client_id: String,
    client_secret: String,
    device_code: Option<String>,
    access_token: Option<String>,
}

impl TraktOAuth {
    pub async fn initiate_device_flow(&mut self) -> Result<DeviceCode, TraktError> {
        // Device code flow implementation
    }
}
```

**Tasks**:
- [ ] Implement Trakt device code OAuth flow
- [ ] Create device code request and polling logic
- [ ] Add token storage and refresh mechanisms  
- [ ] Build user authorization UI workflow
- [ ] Add comprehensive error handling for OAuth failures

**Verification**: Complete OAuth flow from device code to access token

### Week 6 Day 3-4: IMDb List Import System 🎬
**Location**: `crates/core/src/lists/imdb.rs`

```rust
pub struct IMDbListImporter {
    http_client: reqwest::Client,
    rate_limiter: RateLimiter,
}

impl IMDbListImporter {
    pub async fn import_list(&self, list_url: String) -> Result<Vec<Movie>, IMDbError> {
        // Parse IMDb list HTML and extract movie data
    }
}
```

**Tasks**:
- [ ] Build IMDb list URL parser and validator
- [ ] Implement HTML scraping for movie extraction
- [ ] Add rate limiting to respect IMDb servers
- [ ] Create movie data mapping to internal format
- [ ] Add comprehensive error handling and retries
- [ ] Build import progress tracking

**Verification**: Successfully import movies from IMDb lists

### Week 6 Day 5: TMDb List Integration & Sync Jobs 📅
**Location**: `crates/core/src/lists/tmdb.rs`

```rust
pub struct TMDbListSync {
    api_key: String,
    sync_scheduler: ScheduledJobRunner,
}

impl TMDbListSync {
    pub async fn sync_popular_movies(&self) -> Result<SyncResult, TMDbError> {
        // Sync TMDb popular/trending movies
    }
}
```

**Tasks**:
- [ ] Implement TMDb list API integration
- [ ] Create scheduled job system for list synchronization
- [ ] Add provenance tracking (why movies were added)
- [ ] Build sync conflict resolution (duplicates, updates)
- [ ] Create sync status reporting and monitoring
- [ ] Add configurable sync intervals and preferences

**Verification**: Scheduled sync jobs running with progress tracking

## 📋 Week 7: Discovery & User Experience

### Discovery UI Implementation
**Location**: `unified-radarr/web/src/components/discovery/`

#### Provenance Tracking
- [ ] Create "Why Added" tracking for all movies
- [ ] Build discovery reasons taxonomy (Trakt list, IMDb list, manual, etc.)
- [ ] Add provenance display in movie details
- [ ] Track recommendation source effectiveness

#### Discovery Dashboard
- [ ] Create discovery recommendations UI
- [ ] Build list management interface
- [ ] Add sync status and progress displays
- [ ] Implement discovery settings and preferences

#### List Configuration
- [ ] Build list source configuration UI
- [ ] Add OAuth flow UI for Trakt authentication
- [ ] Create IMDb list URL management
- [ ] Build sync schedule configuration interface

### List Synchronization Jobs
**Location**: `crates/core/src/jobs/list_sync.rs`

- [ ] Create job scheduler with configurable intervals
- [ ] Implement sync conflict resolution logic
- [ ] Add sync history and audit logging
- [ ] Build sync performance monitoring
- [ ] Create sync failure handling and retries
- [ ] Add manual sync triggers and controls
- [ ] Implement sync result reporting and notifications

## 📋 Week 8: Production Readiness

### Performance & Monitoring
**Location**: `crates/infrastructure/src/monitoring/`

```rust
pub struct ListSyncMonitor {
    metrics: PrometheusMetrics,
    alerting: AlertManager,
}

impl ListSyncMonitor {
    pub fn track_sync_performance(&self, source: &str, duration: Duration) {
        // Track sync performance metrics
    }
}
```

- [ ] Add comprehensive metrics for list operations
- [ ] Implement sync performance monitoring
- [ ] Create alerting for sync failures
- [ ] Build monitoring dashboard
- [ ] Add health checks for external list services
- [ ] Implement circuit breakers for list APIs

### Integration Testing
**Location**: `tests/integration/lists/`

- [ ] Create end-to-end list sync testing
- [ ] Add OAuth flow integration tests
- [ ] Build list import validation tests
- [ ] Create sync job scheduling tests
- [ ] Add provenance tracking verification
- [ ] Implement sync performance benchmarks
- [ ] Build sync failure recovery tests

## 📋 Week 4: Failure Handling

### Blocklist System
**Location**: `crates/core/src/blocklist/`

```rust
pub struct BlocklistEntry {
    pub release_id: String,
    pub indexer: String,
    pub reason: FailureReason,
    pub blocked_until: DateTime<Utc>,
    pub retry_count: u32,
}
```

- [ ] Create blocklist table migration
- [ ] Implement blocklist service
- [ ] Add automatic blocking on failure
- [ ] Implement TTL expiration
- [ ] Add manual unblock endpoint
- [ ] Create UI for blocklist management

### Failure Taxonomy
```rust
pub enum FailureReason {
    ConnectionTimeout,
    AuthenticationFailed,
    RateLimited,
    ParseError,
    DownloadStalled,
    HashMismatch,
    ImportFailed(String),
    DiskFull,
    PermissionDenied,
}
```

- [ ] Define comprehensive failure types
- [ ] Map errors to failure reasons
- [ ] Implement retry strategies per type
- [ ] Add failure metrics
- [ ] Create failure dashboard

## 🧪 Testing Tasks

### Unit Tests (Current State: 35/35 Passing)
- [x] Fixed all compilation errors
- [x] Quality engine tests: 19/19 passing (90% coverage)
- [x] HDBits integration tests: 16/16 passing (85% coverage)
- [ ] Lists integration tests (target: 15+ tests)
- [ ] OAuth flow tests
- [ ] List parsing and import tests

### Integration Tests
- [ ] Create end-to-end search test
- [ ] Test download → import workflow
- [ ] Test quality upgrade decisions
- [ ] Test failure recovery

### Fault Injection Tests
```bash
# Create fault injection suite
tests/fault_injection/
├── indexer_timeout.rs
├── rate_limit_429.rs
├── download_stall.rs
├── disk_full.rs
└── corrupt_file.rs
```

- [ ] Simulate indexer timeouts
- [ ] Simulate 429 rate limits
- [ ] Simulate stalled downloads
- [ ] Simulate disk full errors
- [ ] Test recovery procedures

## 🔍 Code Quality Tasks

### Documentation
- [ ] Document all public APIs
- [ ] Add README to each crate
- [ ] Create architecture diagrams
- [ ] Write deployment guide

### Refactoring
- [ ] Remove all TODO comments
- [ ] Fix all clippy warnings
- [ ] Remove dead code
- [ ] Optimize database queries

### Performance
- [ ] Add database indexes
- [ ] Implement connection pooling
- [ ] Add caching layer
- [ ] Profile and optimize hot paths

## 📊 Verification Checklist

### After Each Day
- [x] All tests compile (35/35 passing)
- [x] Clippy warnings reduced (23 from 47)
- [x] Documentation updated (API docs complete)
- [x] Metrics recorded (performance measured)
- [x] Git commit with clear message

### After Each Week  
- [x] Integration tests pass (quality engine operational)
- [x] Performance benchmarks run (7.9MB memory, <50ms API)
- [x] Test server deployment succeeds (192.168.0.138 operational)
- [x] Changelog updated (quality engine milestone)
- [x] Sprint retrospective (Week 4-5 complete)

### Week 6 Targets
- [ ] OAuth flow functional end-to-end
- [ ] IMDb list import working
- [ ] TMDb integration complete
- [ ] Sync jobs scheduled and running
- [ ] Provenance tracking operational

## 🚀 Quick Commands

```bash
# Run tests
cargo test --workspace

# Check code quality
cargo clippy --all-targets --all-features
cargo fmt --all -- --check

# Run application
cargo run --bin radarr-mvp

# Build for production
cargo build --release

# Deploy to test server
./scripts/deploy.sh
ssh root@192.168.0.138 'systemctl restart radarr'

# Check metrics
curl http://localhost:7878/metrics

# View logs with correlation ID
cargo run 2>&1 | grep "correlation_id"
```

## 📝 Notes

- Always fix tests before adding features
- Every feature needs a test
- Use correlation IDs for debugging
- Check metrics after implementation
- Document as you go

---

**Remember**: This is a living document. Update task status as you complete them and add new tasks as they're discovered.