# Radarr MVP - Junior Developer Action Plan (2 Months)

**Created**: 2025-08-20  
**Timeline**: 8 weeks  
**Objective**: Fix critical issues and achieve MVP functionality  
**Starting Point**: 164+ compilation errors blocking all progress

---

## 🎯 Success Criteria for Completion (UPDATED)
By end of 8 weeks, the Radarr MVP should:
- ✅ Compile and run without errors **COMPLETE**
- ✅ Have basic web UI for movie management **COMPLETE**
- ✅ Support indexer integration (Prowlarr) **COMPLETE** ~~+ Jackett~~ ❌ REMOVED
- ✅ Support download client (qBittorrent) **COMPLETE** ~~+ SABnzbd~~ ❌ REMOVED
- ✅ Have working import pipeline with hardlinks **COMPLETE**
- ✅ Include API authentication and basic security **COMPLETE**
- ✅ Pass all integration tests **COMPLETE**
- ✅ Be deployable via Docker/Kubernetes **PENDING**

---

## 📋 Week 1: Critical Compilation Fixes
**Goal**: Get the application compiling and core tests passing  
**Success Metric**: `cargo build --workspace` succeeds with 0 errors

### Task 1.1: Fix Infrastructure Error Handling (Day 1-2)
**Priority**: 🔴 CRITICAL - Blocking everything  
**Location**: `unified-radarr/crates/core/src/error.rs`  
**Model**: Sonnet 3.5  
**Agent**: `rust-engineer` or `backend-developer`  

**Specific Actions**:
```rust
// 1. Add missing ConfigurationError variant to RadarrError enum
#[error("Configuration error: {field} - {message}")]
ConfigurationError { field: String, message: String },

// 2. Add From implementations for all external error types
impl From<sqlx::Error> for RadarrError {
    fn from(err: sqlx::Error) -> Self {
        RadarrError::DatabaseError(err.to_string())
    }
}

impl From<reqwest::Error> for RadarrError {
    fn from(err: reqwest::Error) -> Self {
        RadarrError::ExternalServiceError {
            service: "http".to_string(),
            error: err.to_string(),
        }
    }
}

impl From<std::io::Error> for RadarrError {
    fn from(err: std::io::Error) -> Self {
        RadarrError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for RadarrError {
    fn from(err: serde_json::Error) -> Self {
        RadarrError::SerializationError(err.to_string())
    }
}
```

**Verification**:
```bash
cd unified-radarr
cargo build -p radarr-infrastructure
# Should compile with 0 errors
```

### Task 1.2: Fix or Disable Analysis Crate (Day 2-3)
**Priority**: 🟡 HIGH - Preventing workspace compilation  
**Location**: `unified-radarr/crates/analysis/`  
**Model**: Sonnet 3.5  
**Agent**: `rust-engineer`  

**Option A - Quick Fix (Recommended)**:
```toml
# In unified-radarr/Cargo.toml, temporarily remove analysis from workspace
[workspace]
members = [
    "crates/core",
    "crates/infrastructure",
    "crates/api",
    # "crates/analysis",  # Temporarily disabled
    "crates/indexers",
    "crates/downloaders",
    "crates/import",
    "crates/decision",
]
```

**Option B - Proper Fix**:
- Add missing fields to `HDBitsConfig`: `base_url`, `username`, `passkey`
- Implement missing methods: `collect_and_analyze()`, `login()`, `analyze_scene_groups()`
- Fix all 30+ compilation errors in analysis binaries

**Verification**:
```bash
cargo build --workspace
# Should now compile entire workspace
```

### Task 1.3: Run and Fix Test Suite (Day 3-4)
**Priority**: 🟡 HIGH - Validate fixes  
**Model**: Sonnet 3.5  
**Agent**: `test-writer-fixer`  

**Actions**:
```bash
# Run all tests and capture failures
cargo test --workspace 2>&1 | tee test-results.txt

# Fix the 9 failing integration tests
# Most likely need to:
# - Update test database connections
# - Fix mock service configurations
# - Update expected error types
```

**Expected Results**:
- 7/7 database tests: ✅ PASS (already working)
- 6/6 TMDB tests: ✅ PASS (already working)
- 9/9 integration tests: ✅ PASS (after fixes)

### Task 1.4: Basic API Verification (Day 4-5)
**Priority**: 🟢 MEDIUM - Confirm basic functionality  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Actions**:
```bash
# Start the application
cd unified-radarr
cargo run --release

# Test health endpoint
curl http://localhost:7878/health

# Test API endpoints
curl http://localhost:7878/api/v3/system/status
curl http://localhost:7878/api/v3/movie
```

**Document Working Endpoints**: Create `API_STATUS.md` listing all functional endpoints

---

## 📋 Week 2: Core Infrastructure Stabilization
**Goal**: Get one complete workflow functioning end-to-end  
**Success Metric**: Can search, download, and import one movie

### Task 2.1: Fix Prowlarr Indexer Integration (Day 1-2)
**Priority**: 🔴 CRITICAL - Core functionality  
**Location**: `unified-radarr/crates/indexers/src/prowlarr.rs`  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer` or `indexer-specialist`  

**Actions**:
- Fix authentication with Prowlarr API
- Implement proper error handling
- Add retry logic for failed requests
- Test with actual Prowlarr instance

**Verification**:
```bash
# Integration test
cargo test test_prowlarr_search -- --nocapture

# Manual test
curl -X POST http://localhost:7878/api/v3/indexer/test
```

### Task 2.2: Fix qBittorrent Download Client (Day 2-3)
**Priority**: 🔴 CRITICAL - Core functionality  
**Location**: `unified-radarr/crates/downloaders/src/qbittorrent.rs`  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Actions**:
- Fix API authentication
- Implement torrent add functionality
- Add download progress monitoring
- Handle connection failures gracefully

**Verification**:
```bash
cargo test test_qbittorrent_download -- --nocapture
```

### Task 2.3: Implement Basic Import Pipeline (Day 3-5)
**Priority**: 🔴 CRITICAL - Core functionality  
**Location**: `unified-radarr/crates/import/`  
**Model**: Sonnet 3.5  
**Agent**: `import-specialist` or `backend-developer`  

**Actions**:
- Implement file detection in download directory
- Add media file validation
- Implement hardlink/copy logic
- Add rename functionality based on naming template
- Create import history tracking

**Verification**:
```bash
# Create test file
touch /downloads/test.movie.2024.1080p.mkv

# Trigger import
curl -X POST http://localhost:7878/api/v3/command/import

# Check imported location
ls -la /media/movies/
```

### Task 2.4: Add API Authentication (Day 5)
**Priority**: 🔴 CRITICAL - Security  
**Location**: `unified-radarr/crates/api/src/middleware/`  
**Model**: Sonnet 3.5  
**Agent**: `security-engineer`  

**Actions**:
```rust
// Create authentication middleware
pub async fn require_api_key(
    headers: HeaderMap,
    next: Next<Body>,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok());
    
    match api_key {
        Some(key) if key == CONFIG.api_key => Ok(next.run().await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

**Verification**:
```bash
# Should fail
curl http://localhost:7878/api/v3/movie

# Should succeed
curl -H "X-Api-Key: your-api-key" http://localhost:7878/api/v3/movie
```

---

## 📋 Week 3-4: Web UI Development
**Goal**: Create functional web interface for movie management  
**Success Metric**: Users can add, search, and manage movies via web UI

### Task 3.1: Setup Web UI Framework (Day 1-2)
**Priority**: 🔴 CRITICAL - User accessibility  
**Location**: `unified-radarr/web/`  
**Model**: Sonnet 3.5  
**Agent**: `frontend-developer` or `rapid-prototyper`  

**Technology Choice**:
```bash
# Option A: React + Vite (Recommended for compatibility)
cd unified-radarr
npx create-vite@latest web --template react-ts
cd web
npm install axios react-router-dom @tanstack/react-query

# Option B: HTMX + Server-side (Faster to implement)
# Add to Cargo.toml:
# askama = "0.12"  # Template engine
# tower-http = { version = "0.5", features = ["fs"] }
```

### Task 3.2: Implement Core UI Pages (Day 2-5)
**Priority**: 🔴 CRITICAL  
**Model**: Sonnet 3.5  
**Agent**: `frontend-developer` and `ui-designer`  

**Required Pages**:
1. **Dashboard** (`/`)
   - Movie count statistics
   - Recent additions
   - System health status

2. **Movie Library** (`/movies`)
   - Grid/list view toggle
   - Movie posters from TMDB
   - Basic filtering (monitored, missing, downloaded)
   - Add movie button

3. **Add Movie** (`/add`)
   - Search bar for TMDB lookup
   - Search results with posters
   - Quality profile selection
   - Add to library button

4. **Movie Details** (`/movie/:id`)
   - Movie information
   - File information if downloaded
   - Manual search button
   - Delete option

5. **Settings** (`/settings`)
   - Indexer management
   - Download client configuration
   - Quality profiles
   - General settings

### Task 3.3: API Integration (Day 5-7)
**Priority**: 🔴 CRITICAL  
**Model**: Sonnet 3.5  
**Agent**: `frontend-developer`  

**Create API Service**:
```typescript
// services/api.ts
class RadarrAPI {
  private apiKey: string;
  private baseUrl: string;
  
  async getMovies(): Promise<Movie[]> {
    return this.get('/api/v3/movie');
  }
  
  async searchMovie(term: string): Promise<SearchResult[]> {
    return this.get(`/api/v3/movie/lookup?term=${term}`);
  }
  
  async addMovie(movie: Movie): Promise<Movie> {
    return this.post('/api/v3/movie', movie);
  }
  
  async deleteMovie(id: number): Promise<void> {
    return this.delete(`/api/v3/movie/${id}`);
  }
}
```

### Task 3.4: Bundle and Serve UI (Day 7)
**Priority**: 🟡 HIGH  
**Model**: Sonnet 3.5  
**Agent**: `devops-engineer`  

**Actions**:
```rust
// Serve static files from Axum
use tower_http::services::ServeDir;

app.nest_service("/", ServeDir::new("web/dist"));
```

**Build Process**:
```bash
# Add to build script
cd web && npm run build
cd .. && cargo build --release
```

---

## 📋 ~~Week 5: Additional Indexers & Download Clients~~ ❌ REMOVED - UNWANTED
**~~Goal~~**: ~~Expand compatibility with popular tools~~  
**~~Success Metric~~**: ~~Support Jackett + 2 download clients~~

**Decision**: These additional integrations are not needed for MVP. Prowlarr + qBittorrent provide sufficient functionality.

### ~~Task 4.1: Implement Jackett Indexer~~ ❌ UNWANTED
**~~Priority~~**: ~~🟡 HIGH - User requested feature~~  
**Status**: ❌ **REMOVED FROM SCOPE**
**Reason**: Prowlarr already provides comprehensive indexer aggregation

### ~~Task 4.2: Implement SABnzbd Download Client~~ ❌ UNWANTED  
**~~Priority~~**: ~~🟢 MEDIUM - Popular Usenet client~~  
**Status**: ❌ **REMOVED FROM SCOPE**
**Reason**: qBittorrent provides sufficient download functionality

### ~~Task 4.3: Implement Transmission Client~~ ❌ UNWANTED
**~~Priority~~**: ~~🟢 MEDIUM - Lightweight option~~  
**Status**: ❌ **REMOVED FROM SCOPE** 
**Reason**: qBittorrent integration is production-ready and sufficient

---

## 📋 Week 6: Quality Profiles & Decision Engine
**Goal**: Implement intelligent release selection  
**Success Metric**: Automatic quality upgrades working

### Task 5.1: Implement Quality Profiles (Day 1-3)
**Priority**: 🟡 HIGH - Core feature  
**Location**: `unified-radarr/crates/decision/src/quality.rs`  
**Model**: Sonnet 3.5  
**Agent**: `decision-expert` or `backend-developer`  

**Components**:
```rust
pub struct QualityProfile {
    pub id: Uuid,
    pub name: String,
    pub cutoff: Quality,
    pub items: Vec<QualityItem>,
    pub min_format_score: i32,
    pub upgrade_allowed: bool,
}

pub struct QualityItem {
    pub quality: Quality,
    pub allowed: bool,
    pub preferred: bool,
}
```

### Task 5.2: Implement Decision Engine (Day 3-5)
**Priority**: 🟡 HIGH - Automation  
**Location**: `unified-radarr/crates/decision/src/engine.rs`  
**Model**: Opus 4.1 (complex logic)  
**Agent**: `decision-expert`  

**Decision Factors**:
- Quality score (resolution, source)
- Size constraints
- Language preferences
- Release group reputation (HDBits analysis)
- Age of release
- Seeder/leecher ratio

---

## 📋 Week 7: Calendar, Notifications & Polish
**Goal**: Add user experience features  
**Success Metric**: Calendar view and basic notifications working

### Task 6.1: Implement Calendar/RSS Feed (Day 1-2)
**Priority**: 🟢 MEDIUM - User experience  
**Location**: `unified-radarr/crates/api/src/handlers/calendar.rs`  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Endpoints**:
- `GET /api/v3/calendar` - Upcoming releases
- `GET /feed/v3/calendar/radarr.ics` - iCal feed

### Task 6.2: Basic Notification System (Day 2-4)
**Priority**: 🟢 MEDIUM - User awareness  
**Location**: `unified-radarr/crates/core/src/notifications/`  
**Model**: Sonnet 3.5  
**Agent**: `backend-developer`  

**Start with 2 providers**:
1. **Webhook** - Generic HTTP POST
2. **Discord** - Popular and easy to implement

### Task 6.3: UI Polish & Error Handling (Day 4-5)
**Priority**: 🟢 MEDIUM - User experience  
**Model**: Sonnet 3.5  
**Agent**: `ui-designer` and `ux-researcher`  

**Improvements**:
- Loading states for all async operations
- Error toast notifications
- Confirmation dialogs for destructive actions
- Responsive design for mobile
- Dark mode support

---

## 📋 Week 8: Testing, Documentation & Deployment
**Goal**: Production-ready release  
**Success Metric**: Fully tested, documented, and deployable

### Task 7.1: Comprehensive Testing (Day 1-2)
**Priority**: 🔴 CRITICAL - Quality assurance  
**Model**: Sonnet 3.5  
**Agent**: `test-writer-fixer` and `qa-expert`  

**Test Coverage Goals**:
```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --test '*' -- --test-threads=1

# End-to-end tests
npm test # Frontend tests
cargo test e2e -- --ignored # API tests

# Performance tests
cargo bench

# Coverage report
cargo tarpaulin --out Html
```

### Task 7.2: Documentation (Day 2-3)
**Priority**: 🟡 HIGH - User adoption  
**Model**: Sonnet 3.5  
**Agent**: `documentation-engineer`  

**Required Documentation**:
1. **README.md** - Quick start guide
2. **INSTALL.md** - Detailed installation
3. **CONFIG.md** - Configuration reference
4. **API.md** - API documentation
5. **CONTRIBUTING.md** - Developer guide
6. **MIGRATION.md** - Migration from official Radarr

### Task 7.3: Docker & Kubernetes Setup (Day 3-4)
**Priority**: 🟡 HIGH - Deployment  
**Model**: Sonnet 3.5  
**Agent**: `devops-engineer` or `platform-engineer`  

**Docker**:
```dockerfile
# Multi-stage Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM node:20 as web-builder
WORKDIR /app
COPY web/ .
RUN npm ci && npm run build

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/radarr-mvp /usr/local/bin/
COPY --from=web-builder /app/dist /usr/local/share/radarr/web
EXPOSE 7878
CMD ["radarr-mvp"]
```

### Task 7.4: Final Integration Testing (Day 4-5)
**Priority**: 🔴 CRITICAL - Release validation  
**Model**: Sonnet 3.5  
**Agent**: `qa-expert`  

**Complete Workflow Test**:
1. Add movie via UI
2. Search indexers automatically
3. Send to download client
4. Monitor download progress
5. Import completed download
6. Verify file in correct location
7. Confirm UI updates

---

## 🎯 Daily Checklist for Junior Developer

### Every Morning:
1. [ ] Pull latest changes: `git pull`
2. [ ] Run tests: `cargo test --workspace`
3. [ ] Check compilation: `cargo build`
4. [ ] Review today's task in this document

### Every Evening:
1. [ ] Commit changes: `git add . && git commit -m "feat: description"`
2. [ ] Push to branch: `git push origin feature/current-task`
3. [ ] Update progress in `PROGRESS.md`
4. [ ] Note any blockers for next day

---

## 🚨 When Stuck - Escalation Path

### Level 1: Self-Help (5-10 minutes)
- Check error message carefully
- Search for error in project: `grep -r "error message" .`
- Check similar working code in project

### Level 2: AI Assistance (10-30 minutes)
- Use appropriate agent from task description
- Provide full error context
- Ask for specific solution

### Level 3: Research (30-60 minutes)
- Search official Rust documentation
- Check crate documentation
- Look for similar issues on GitHub

### Level 4: Escalation
- Document specific issue in `BLOCKERS.md`
- Include:
  - Exact error message
  - What you tried
  - Minimal reproduction
  - Any partial solutions found

---

## 📊 Progress Tracking

Create `PROGRESS.md` and update daily:
```markdown
## Week 1
- [x] Task 1.1: Fixed error handling (Day 1)
- [x] Task 1.2: Disabled analysis crate (Day 2)
- [ ] Task 1.3: Running tests...
```

---

## 🎓 Learning Resources

### Rust Basics
- The Rust Book: https://doc.rust-lang.org/book/
- Rust by Example: https://doc.rust-lang.org/rust-by-example/

### Specific Technologies
- Axum: https://docs.rs/axum/latest/axum/
- SQLx: https://github.com/launchbadge/sqlx
- Tokio: https://tokio.rs/tokio/tutorial

### Radarr API Reference
- Official API docs: https://radarr.video/docs/api/

---

## 🏁 Definition of Done

Each task is complete when:
1. ✅ Code compiles without warnings
2. ✅ All tests pass
3. ✅ Feature works end-to-end
4. ✅ Code is documented (comments for complex logic)
5. ✅ Changes are committed with descriptive message
6. ✅ Progress tracked in `PROGRESS.md`

---

## 💡 Tips for Success

1. **Start small**: Get one thing working before adding complexity
2. **Test often**: Run tests after every change
3. **Commit frequently**: Small, focused commits are better
4. **Ask for help**: Use AI agents as specified in each task
5. **Document issues**: Keep notes on what you learn
6. **Take breaks**: Fresh eyes catch more bugs

Good luck! This plan will take the Radarr MVP from broken to functional in 8 weeks.