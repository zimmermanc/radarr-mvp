# Junior Developer Task Guide - Radarr MVP

**Start Date**: Week 1 (Foundation Stabilization)  
**Target**: Get the project building and tests passing  
**Time Estimate**: 1-2 weeks for a junior developer  

---

## 🤖 Claude Model & Agent Selection Guide

**Using the right model and agent for each task will make your work 3x faster and more effective.**

### 📊 Model Selection Strategy

| Model | Best For | Token Cost | Speed |
|-------|----------|------------|-------|
| **Haiku 3.5** | Simple fixes, formatting, basic tests, quick questions | Lowest | Fastest |
| **Sonnet 4** | Implementation, bug fixes, API work, day-to-day coding | Medium | Fast |
| **Opus 4.1** | Architecture decisions, complex algorithms, system design | Highest | Slower |

### 🎯 Agent Selection for This Guide

| Task Area | Primary Agent | Backup Agent | Model |
|-----------|---------------|--------------|-------|
| **Compilation Issues** | `rust-engineer` | `backend-developer` | Sonnet 4 |
| **Test Failures** | `test-writer-fixer` | `rust-engineer` | Sonnet 4 |
| **Database Setup** | `database-administrator` | `backend-developer` | Sonnet 4 |
| **qBittorrent Client** | `backend-developer` | `api-designer` | Sonnet 4 |
| **Complex Coordination** | `studio-coach` | - | Opus 4.1 |
| **Quick Fixes** | Any specialized agent | - | Haiku 3.5 |

### 🎮 How to Use Agents

**Command Format**: `/agent <agent-name> <task-description>`

**Examples**:
```
/agent rust-engineer Fix compilation errors in unified-radarr workspace
/agent test-writer-fixer Analyze and fix the 9 failing tests
/agent database-administrator Set up PostgreSQL test database for integration tests
/agent backend-developer Create qBittorrent client with authentication
```

### 💡 Pro Tips
- **Start with agents**: They have specialized context and expertise
- **Use studio-coach**: For multi-step tasks requiring coordination
- **Match model to complexity**: Don't use Opus for simple formatting
- **Agent commands preserve context**: Unlike direct tool usage

---

## Quick Start Setup

**Goal**: Verify your development environment is ready

### Environment Check

```bash
# Navigate to the main development directory
cd /home/thetu/radarr-mvp/unified-radarr

# Check Rust installation
rustc --version
# Should show: rustc 1.75+ 

# Check if PostgreSQL is running
pg_isready -h localhost -p 5432
# Should show: localhost:5432 - accepting connections

# Check environment file exists
ls -la .env
# If missing, copy from example: cp .env.example .env
```

### Success Criteria:
✅ Rust 1.75+ installed  
✅ PostgreSQL running on port 5432  
✅ `.env` file exists  

### Common Issues:
- **"rustc not found"** → Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **"pg_isready not found"** → Install PostgreSQL: `sudo apt install postgresql postgresql-contrib`
- **"connection refused"** → Start PostgreSQL: `sudo systemctl start postgresql`

---

## Task 1: Fix Compilation Errors

**Goal**: Get the codebase building without errors

> **🎯 Model/Agent Recommendation**  
> **Model**: Sonnet 4 (optimal for implementation and bug fixes)  
> **Primary Agent**: `rust-engineer` (specialized Rust expertise)  
> **Fallback**: `backend-developer` (general backend knowledge)  
> **Command**: `/agent rust-engineer Fix compilation errors in unified-radarr workspace - analyze build failures and implement fixes systematically`

### What You're Fixing:
The main codebase has compilation errors that prevent the code from building. We need to fix these systematically.

### Steps:

1. **Navigate to the active development directory**
```bash
cd /home/thetu/radarr-mvp/unified-radarr
```

2. **Try building to see current errors**
```bash
cargo build --workspace
```

3. **Use Rust's auto-fix tool**
```bash
# This fixes many common issues automatically
cargo fix --workspace --allow-dirty --allow-staged
```

4. **Check for remaining compilation errors**
```bash
cargo build --workspace
```

5. **Fix remaining errors one by one**
```bash
# Build specific crates to isolate issues
cargo build -p radarr-core
cargo build -p radarr-api
cargo build -p radarr-infrastructure
```

### Expected Output:
- First build will show multiple errors
- After `cargo fix`: Most errors should be resolved
- Final build should show: `Finished dev [unoptimized + debuginfo] target(s)`

### Success Criteria:
✅ `cargo build --workspace` completes without errors  
✅ All crates compile successfully  

### Common Issues and Solutions:

**Issue**: `use of moved value` errors
```bash
# Solution: Look for variables used after being moved
# Example fix: Clone values instead of moving them
let data_clone = data.clone();
```

**Issue**: `cannot find type X in this scope`
```bash
# Solution: Add missing imports at top of file
use crate::models::SomeType;
```

**Issue**: `trait bounds were not satisfied`
```bash
# Solution: Add required trait implementations
#[derive(Debug, Clone)]
```

### Files You'll Likely Need to Edit:
- `/home/thetu/radarr-mvp/unified-radarr/crates/core/src/lib.rs`
- `/home/thetu/radarr-mvp/unified-radarr/crates/api/src/lib.rs`
- Any files shown in compilation error messages

---

## Task 2: Fix Failing Tests

**Goal**: Get the test suite passing to ensure code quality

> **🎯 Model/Agent Recommendation**  
> **Model**: Sonnet 4 (excellent for test analysis and debugging)  
> **Primary Agent**: `test-writer-fixer` (specialized testing expertise)  
> **Fallback**: `rust-engineer` (Rust-specific test knowledge)  
> **Command**: `/agent test-writer-fixer Analyze the 9 failing tests in unified-radarr workspace and implement comprehensive fixes`

### What You're Fixing:
There are 9 failing tests that need to be identified and fixed.

### Steps:

1. **Run tests to see what's failing**
```bash
cd /home/thetu/radarr-mvp/unified-radarr
cargo test --workspace
```

2. **Run tests with detailed output**
```bash
cargo test --workspace -- --nocapture
```

3. **Fix tests one crate at a time**
```bash
# Test individual crates to isolate issues
cargo test -p radarr-core
cargo test -p radarr-api
cargo test -p radarr-infrastructure
```

4. **Run specific failing tests**
```bash
# If test_movie_creation is failing:
cargo test test_movie_creation -- --nocapture
```

### Expected Test Failures and Fixes:

**Common Failure**: Database connection tests
```bash
# Error: "connection refused"
# Fix: Start test database
docker-compose up -d postgres-test
```

**Common Failure**: TMDB API tests  
```bash
# Error: "API key not found"
# Fix: Add API key to .env file
echo "TMDB_API_KEY=your_key_here" >> .env
```

**Common Failure**: Release parser case sensitivity
```bash
# Error: "expected '2160p' but got '2160P'"
# Fix: Update parser to handle both cases
# Location: unified-radarr/crates/core/src/parser/quality_detection.rs
```

### Success Criteria:
✅ `cargo test --workspace` shows all tests passing  
✅ No test failures in output  
✅ All integration tests work  

### Debugging Tips:

1. **Read error messages carefully**
```bash
# Error messages tell you exactly what's wrong:
# "assertion failed: expected X but got Y"
```

2. **Use print debugging**
```rust
// Add this to see what values you're getting:
println!("DEBUG: value = {:?}", some_variable);
```

3. **Check test setup**
```bash
# Make sure test database is clean
cargo test --workspace -- --test-threads=1
```

---

## Task 3: Database Connection Issues  

**Goal**: Ensure all database operations work properly

> **🎯 Model/Agent Recommendation**  
> **Model**: Sonnet 4 (ideal for infrastructure and database setup)  
> **Primary Agent**: `database-administrator` (specialized database expertise)  
> **Fallback**: `backend-developer` (general backend/database knowledge)  
> **Command**: `/agent database-administrator Set up PostgreSQL test database, configure connections, and fix database integration test failures`

### What You're Fixing:
Integration tests are failing because the database isn't configured correctly.

### Steps:

1. **Check database is running**
```bash
pg_isready -h localhost -p 5432
# Should show: accepting connections
```

2. **Create test database**
```bash
# Connect to PostgreSQL
sudo -u postgres psql

# In PostgreSQL shell:
CREATE DATABASE radarr_test;
CREATE USER radarr_test WITH PASSWORD 'test123';
GRANT ALL PRIVILEGES ON DATABASE radarr_test TO radarr_test;
\q
```

3. **Update environment file**
```bash
# Add test database URL to .env
echo "TEST_DATABASE_URL=postgresql://radarr_test:test123@localhost:5432/radarr_test" >> .env
```

4. **Run database migrations**
```bash
# Install migration tool if needed
cargo install sqlx-cli --features postgres

# Run migrations on test database
sqlx migrate run --database-url postgresql://radarr_test:test123@localhost:5432/radarr_test
```

5. **Test database connection**
```bash
cargo test database_connection -- --nocapture
```

### Success Criteria:
✅ Test database created and accessible  
✅ Migrations run successfully  
✅ Database tests pass  

### Common Issues:

**Issue**: "role 'radarr_test' does not exist"
```bash
# Solution: Create the user
sudo -u postgres createuser -s radarr_test
```

**Issue**: "database 'radarr_test' does not exist"  
```bash
# Solution: Create the database
sudo -u postgres createdb radarr_test -O radarr_test
```

**Issue**: "connection refused"
```bash
# Solution: Start PostgreSQL service
sudo systemctl start postgresql
sudo systemctl enable postgresql
```

### Files to Check:
- `/home/thetu/radarr-mvp/unified-radarr/.env` (database URLs)
- `/home/thetu/radarr-mvp/unified-radarr/migrations/` (migration files)

---

## Task 4: Start qBittorrent Integration

**Goal**: Begin implementing the first download client integration

> **🎯 Model/Agent Recommendation**  
> **Model**: Sonnet 4 (perfect for API integration and client implementation)  
> **Primary Agent**: `backend-developer` (API and service integration expertise)  
> **Fallback**: `api-designer` (API design and HTTP client knowledge)  
> **Command**: `/agent backend-developer Implement qBittorrent client with authentication, torrent management, and monitoring capabilities`

### What You're Building:
A working connection to qBittorrent that can add torrents and monitor downloads.

### Steps:

1. **Install qBittorrent for testing**
```bash
# Install qBittorrent (headless version for servers)
sudo apt install qbittorrent-nox

# Start qBittorrent daemon (runs in background)
qbittorrent-nox --daemon
```

2. **Create qBittorrent client structure**
```bash
# Navigate to downloaders crate
cd /home/thetu/radarr-mvp/unified-radarr/crates/downloaders
```

3. **Create basic qBittorrent client file**
Create `/home/thetu/radarr-mvp/unified-radarr/crates/downloaders/src/qbittorrent.rs`:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct QBittorrentClient {
    base_url: String,
    client: Client,
    session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

impl QBittorrentClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
            session_id: None,
        }
    }

    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let login_url = format!("{}/api/v2/auth/login", self.base_url);
        
        let mut params = HashMap::new();
        params.insert("username", username);
        params.insert("password", password);

        let response = self.client
            .post(&login_url)
            .form(&params)
            .send()
            .await?;

        if response.status().is_success() {
            println!("Successfully logged in to qBittorrent");
            Ok(())
        } else {
            Err("Login failed".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_qbittorrent_connection() {
        let client = QBittorrentClient::new("http://localhost:8080".to_string());
        // This test will be expanded as we build more functionality
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
```

4. **Update the mod.rs file**
Edit `/home/thetu/radarr-mvp/unified-radarr/crates/downloaders/src/lib.rs`:

```rust
pub mod qbittorrent;

pub use qbittorrent::QBittorrentClient;
```

5. **Test the basic structure**
```bash
cargo test -p radarr-downloaders
```

6. **Configure qBittorrent for testing**
```bash
# qBittorrent default login: admin/adminpass
# Access web UI at: http://localhost:8080
# Change these defaults for security!
```

### Expected Outcomes:
- qBittorrent service running on port 8080
- Basic client struct compiles
- Simple connection test passes
- Foundation ready for adding torrent functionality

### Success Criteria:
✅ qBittorrent running and accessible  
✅ Client code compiles without errors  
✅ Basic connection test passes  
✅ Ready to add torrent download functionality  

### Common Issues:

**Issue**: "qBittorrent not starting"
```bash
# Solution: Check if port 8080 is already in use
sudo netstat -tlnp | grep 8080
# Kill process using port 8080 if needed
```

**Issue**: "Connection refused to localhost:8080"
```bash
# Solution: Wait for qBittorrent to fully start (takes 30 seconds)
# Or check if it's running: ps aux | grep qbittorrent
```

**Issue**: "Login failed"  
```bash
# Solution: Use default credentials admin/adminpass
# Or reset qBittorrent config: rm ~/.config/qBittorrent/qBittorrent.conf
```

### Next Steps (Week 2):
Once this basic structure works, you'll extend it to:
- Add torrents via API calls
- Monitor download progress  
- Handle download completion
- Integrate with the import pipeline

---

## General Development Tips

> **🎯 General Debugging Strategy**  
> **For Simple Issues**: Use `debugger` agent with Haiku 3.5  
> **For Complex Problems**: Use `studio-coach` with Opus 4.1 to coordinate multiple agents  
> **For Specific Domains**: Use specialized agents (rust-engineer, test-writer-fixer, etc.) with Sonnet 4  
> **Command Examples**:  
> - `/agent debugger Quick fix for this compilation error`  
> - `/agent studio-coach Coordinate fixing compilation, tests, and database setup together`

### Daily Workflow:
1. **Always start with a clean build**
```bash
cargo clean && cargo build --workspace
```

2. **Run tests frequently**
```bash
cargo test --workspace
```

3. **Use clippy for code quality**
```bash
cargo clippy --workspace -- -D warnings
```

4. **Format code before committing**
```bash
cargo fmt --all
```

### Debugging Strategies:

1. **Read error messages completely** - They tell you exactly what's wrong and where

2. **Use `println!` for debugging**
```rust
println!("DEBUG: variable value = {:?}", my_variable);
```

3. **Test one thing at a time** - Don't change multiple things simultaneously

4. **Use `cargo expand` to see macro expansions** if macros are confusing

### Getting Help:

1. **Use Claude agents first**: They have specialized expertise for your exact problem
   ```
   /agent rust-engineer Help me understand this error: [paste error message]
   /agent test-writer-fixer Why is this test failing and how do I fix it?
   ```

2. **Check Rust documentation**: https://doc.rust-lang.org/
3. **Search error messages** - Usually others have hit the same issue
4. **Use `cargo doc --open`** to see local documentation
5. **Ask specific questions** with error messages included

### 🎯 Agent Command Cheatsheet

| Situation | Agent Command |
|-----------|---------------|
| **Build fails** | `/agent rust-engineer Fix these compilation errors: [paste errors]` |
| **Tests fail** | `/agent test-writer-fixer Debug failing tests: [test names]` |
| **DB issues** | `/agent database-administrator Fix database connection problems` |
| **API problems** | `/agent backend-developer Debug API integration issues` |
| **Complex task** | `/agent studio-coach Coordinate fixing compilation, tests, and setup` |
| **Quick question** | `/agent debugger Quick help with [specific issue]` |

### File Organization:
```
unified-radarr/
├── crates/
│   ├── core/          # Business logic (you'll work here most)
│   ├── api/           # HTTP endpoints
│   ├── downloaders/   # Download clients (qBittorrent, etc.)
│   ├── infrastructure/# Database, external services
│   └── indexers/      # Torrent site integrations
```

### Key Files to Understand:
- `Cargo.toml` - Dependencies and project configuration
- `.env` - Environment variables and secrets
- `src/lib.rs` - Main library entry points
- `src/main.rs` - Application entry point
- `migrations/` - Database schema changes

---

## Week 2 Preview

Once you complete these tasks, Week 2 will focus on:

1. **Expanding qBittorrent Integration**
   - Add torrent functionality
   - Download monitoring
   - Queue management

2. **Starting Indexer Integration**  
   - Prowlarr API connection
   - Search request handling
   - Rate limiting implementation

3. **Import Pipeline Foundation**
   - File detection and analysis
   - Basic hardlink support
   - Rename functionality

Remember: **Take it one task at a time**, and don't hesitate to ask questions if you get stuck. The goal is to learn while making steady progress on the project.