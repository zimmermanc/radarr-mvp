# Radarr MVP - Critical Path to Actual Completion

**Created**: 2025-08-21  
**Current Reality**: 15% complete (per running instance)  
**Target**: Minimum Viable Product (not production fantasy)  
**Timeline**: 8-12 weeks of focused development

---

## Reality Check

Your application is **not** production-ready. It's a proof of concept with:
- 7 of 50+ required API endpoints (14%)
- Broken integration tests that don't compile
- No indexer integration (core Radarr functionality)
- No calendar/RSS (makes Radarr useful)
- No queue management (can't track downloads)
- Emergency performance fixes applied yesterday

---

## Phase 1: Fix What's Broken (Week 1)

### Task 1.1: Fix Integration Tests
**Priority**: 🔴 CRITICAL - Can't ship broken code  
**Timeline**: 2-3 days  
**Model**: Sonnet 3.5  
**Agent**: `rust-engineer` or `backend-developer`  

**The Problem**:
```rust
// Your Movie struct doesn't match what tests expect
error[E0560]: struct `radarr_core::Movie` has no field named `title`
error[E0560]: struct `radarr_core::Movie` has no field named `year`
// ... 16 more field mismatches
```

**Actions**:
1. Audit the actual Movie struct in `crates/core/src/models/movie.rs`
2. Fix all 18 compilation errors in `tests/end_to_end_workflow.rs`
3. Ensure consistent field definitions across crates
4. Run `cargo test --workspace` until it actually completes

**Verification**:
```bash
cargo test --workspace 2>&1 | grep "test result"
# Should show ALL tests running, not compilation errors
```

### Task 1.2: Implement Calendar/RSS Endpoints
**Priority**: 🔴 CRITICAL - Core Radarr functionality  
**Timeline**: 3-4 days  
**Model**: Sonnet 3.5  
**Agent**: `backend-architect`  

**Missing Endpoints** (currently return 404):
- `GET /api/v3/calendar` - Upcoming releases
- `GET /api/v3/calendar/radarr.ics` - iCal feed
- `GET /feed/v3/calendar/radarr.ics` - RSS feed

**Implementation**:
```rust
// crates/api/src/routes/calendar.rs
pub fn calendar_routes() -> Router<AppState> {
    Router::new()
        .route("/calendar", get(get_calendar))
        .route("/calendar/radarr.ics", get(get_ical_feed))
}

async fn get_calendar(
    State(state): State<AppState>,
    Query(params): Query<CalendarParams>,
) -> Result<Json<Vec<CalendarEntry>>> {
    // Return movies with release dates in range
}
```

**Verification**:
```bash
curl http://localhost:7878/api/v3/calendar
# Should return movie release schedule, not 404
```

---

## Phase 2: Core Functionality (Weeks 2-3)

### Task 2.1: Indexer Management API
**Priority**: 🔴 CRITICAL - Can't search without indexers  
**Timeline**: 4-5 days  
**Model**: Opus 4.1  
**Agent**: `backend-architect`  

**Missing Endpoints**:
- `GET /api/v3/indexer` - List indexers
- `POST /api/v3/indexer` - Add indexer
- `PUT /api/v3/indexer/{id}` - Update indexer
- `DELETE /api/v3/indexer/{id}` - Remove indexer
- `POST /api/v3/indexer/test` - Test connection

**Database Schema**:
```sql
CREATE TABLE indexers (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    implementation VARCHAR(100) NOT NULL, -- 'Prowlarr', 'Jackett', etc
    settings JSONB NOT NULL,
    enabled BOOLEAN DEFAULT true,
    priority INTEGER DEFAULT 25,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Task 2.2: Download Queue Management
**Priority**: 🔴 CRITICAL - Can't track downloads  
**Timeline**: 3-4 days  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Missing Endpoints**:
- `GET /api/v3/queue` - Download queue
- `GET /api/v3/queue/{id}` - Queue item details
- `DELETE /api/v3/queue/{id}` - Remove from queue
- `POST /api/v3/queue/grab/{id}` - Retry download

**Implementation Requirements**:
- Track download progress
- Handle failed downloads
- Integrate with qBittorrent client
- Update movie status on completion

### Task 2.3: Command System
**Priority**: 🔴 CRITICAL - Can't trigger operations  
**Timeline**: 3-4 days  
**Model**: Sonnet 3.5  
**Agent**: `backend-architect`  

**Missing Endpoints**:
- `GET /api/v3/command` - List running commands
- `POST /api/v3/command` - Execute command
- `GET /api/v3/command/{id}` - Command status
- `DELETE /api/v3/command/{id}` - Cancel command

**Commands to Implement**:
- `RefreshMovie` - Update movie metadata
- `MoviesSearch` - Search all missing movies
- `DownloadedMoviesScan` - Scan download folder
- `RenameMovie` - Rename movie files
- `MissingMoviesSearch` - Search for missing movies

---

## Phase 3: Quality & History (Week 4)

### Task 3.1: Quality Profile API
**Priority**: 🟡 HIGH - Needed for release selection  
**Timeline**: 2-3 days  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Missing Endpoints**:
- `GET /api/v3/qualityprofile` - List profiles
- `POST /api/v3/qualityprofile` - Create profile
- `PUT /api/v3/qualityprofile/{id}` - Update profile
- `DELETE /api/v3/qualityprofile/{id}` - Delete profile

### Task 3.2: History Tracking
**Priority**: 🟡 HIGH - Needed for troubleshooting  
**Timeline**: 2-3 days  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Missing Endpoints**:
- `GET /api/v3/history` - Activity history
- `GET /api/v3/history/movie` - Movie history
- `GET /api/v3/history/failed` - Failed downloads

---

## Phase 4: System Management (Week 5)

### Task 4.1: System Status API
**Priority**: 🟡 HIGH - Needed for monitoring  
**Timeline**: 1-2 days  
**Model**: Haiku 3.5  
**Agent**: `backend-developer`  

**Missing Endpoints**:
- `GET /api/v3/system/status` - System information
- `GET /api/v3/system/health` - Health checks
- `GET /api/v3/system/updates` - Available updates
- `POST /api/v3/system/restart` - Restart application

### Task 4.2: Root Folder Management
**Priority**: 🟡 HIGH - Needed for library setup  
**Timeline**: 1-2 days  
**Model**: Haiku 3.5  
**Agent**: `backend-developer`  

**Missing Endpoints**:
- `GET /api/v3/rootfolder` - List root folders
- `POST /api/v3/rootfolder` - Add root folder
- `DELETE /api/v3/rootfolder/{id}` - Remove root folder

---

## Phase 5: Integration Testing (Week 6)

### Task 5.1: End-to-End Test Suite
**Priority**: 🔴 CRITICAL - Can't ship untested code  
**Timeline**: 3-4 days  
**Model**: Opus 4.1  
**Agent**: `test-engineer`  

**Requirements**:
1. Fix broken struct definitions
2. Test complete workflow:
   - Add movie → Search indexers → Download → Import → Rename
3. Test error scenarios
4. Test performance under load

### Task 5.2: API Compatibility Testing
**Priority**: 🟡 HIGH - Ensure Radarr compatibility  
**Timeline**: 2-3 days  
**Model**: Sonnet 3.5  
**Agent**: `test-engineer`  

**Test Against**:
- Official Radarr API documentation
- Third-party tools (Prowlarr, Overseerr)
- Mobile apps that use Radarr API

---

## Phase 6: Production Readiness (Weeks 7-8)

### Task 6.1: Performance Validation
**Priority**: 🟡 HIGH  
**Timeline**: 2-3 days  
**Model**: Sonnet 3.5  
**Agent**: `performance-engineer`  

**Metrics to Validate**:
- Actually achieve 100 req/s (not 0)
- Memory usage under 100MB
- Database connection pooling works
- No connection leaks

### Task 6.2: Docker & Deployment
**Priority**: 🟡 HIGH  
**Timeline**: 2-3 days  
**Model**: Sonnet 3.5  
**Agent**: `devops-engineer`  

**Deliverables**:
- Working Docker image
- Docker Compose configuration
- Kubernetes manifests that actually work
- Deployment documentation

---

## Success Metrics

### Minimum Viable Product (Real MVP)
- [ ] All integration tests pass
- [ ] 30+ API endpoints implemented (not 7)
- [ ] Calendar/RSS feeds working
- [ ] Indexer integration functional
- [ ] Queue management operational
- [ ] Can actually search, download, and import movies
- [ ] Running instance reports >80% completion (not 15%)

### Production Ready (Actual)
- [ ] 50+ API endpoints (Radarr v3 standard)
- [ ] All tests passing (including integration)
- [ ] Performance validated under load
- [ ] Docker deployment working
- [ ] Documentation complete
- [ ] Security audit passed

---

## Reality-Based Timeline

**Current State**: 15% complete (7 endpoints, broken tests)

**Phase Completion Estimates**:
- Phase 1 (Fix Broken): 1 week → 20% complete
- Phase 2 (Core Features): 2 weeks → 40% complete
- Phase 3 (Quality/History): 1 week → 50% complete
- Phase 4 (System Management): 1 week → 60% complete
- Phase 5 (Testing): 1 week → 70% complete
- Phase 6 (Production): 2 weeks → 85% complete

**Total**: 8 weeks to actual MVP (85% complete)

**Full Parity**: 12-16 weeks (100% Radarr v3 compatibility)

---

## Stop Lying, Start Shipping

Your current "100% complete" claim is delusional. The running instance knows the truth: 15% complete.

Focus on:
1. Making tests compile
2. Implementing missing endpoints
3. Adding core functionality
4. Testing what you build
5. Being honest about progress

The path to completion requires acknowledging reality, not claiming false victory.