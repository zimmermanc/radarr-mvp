# GitHub Repository Configuration

## Repository Structure

### rust-radarr-src (Public Repository)
- **Username**: zimmermanc
- **Repository Name**: rust-radarr-src
- **Visibility**: Public
- **Local Path**: `/home/thetu/radarr-mvp/unified-radarr`
- **Description**: Rust-based Radarr implementation with clean architecture
- **Topics**: rust, radarr, media-management, torrent, indexer, clean-architecture

## Initial Setup Commands

```bash
# Navigate to the unified-radarr directory
cd /home/thetu/radarr-mvp/unified-radarr

# Initialize git repository
git init

# Add all files
git add .

# Create initial commit
git commit -m "Initial commit: Rust Radarr implementation with clean architecture"

# Add remote origin
git remote add origin https://github.com/zimmermanc/rust-radarr-src.git

# Push to main branch
git branch -M main
git push -u origin main
```

## GitHub Repository Creation

### Using GitHub CLI (gh)

```bash
# Authenticate with GitHub CLI (if not already done)
gh auth login

# Create rust-radarr-src (public)
gh repo create zimmermanc/rust-radarr-src \
  --public \
  --description "Rust-based Radarr implementation with clean architecture" \
  --clone=false
```

### Using GitHub Web Interface

1. **rust-radarr-src (Public)**:
   - Go to https://github.com/new
   - Repository name: `rust-radarr-src`
   - Description: "Rust-based Radarr implementation with clean architecture"
   - Set to **Public**
   - Do NOT initialize with README (we'll push existing code)
   - Do NOT add .gitignore (we have our own)
   - Do NOT choose a license initially

## .gitignore Configuration

```gitignore
# Rust
target/
**/*.rs.bk
*.pdb
Cargo.lock

# Environment
.env
.env.local
.env.*.local

# IDE
.vscode/
.idea/
*.swp
*.swo
*~
.DS_Store

# Database
*.db
*.sqlite
*.sqlite3
data/

# Logs
*.log
logs/

# Test artifacts
test-results/
coverage/

# Build artifacts
dist/
build/

# Temporary files
tmp/
temp/

# Credentials and sensitive data
**/credentials/
**/secrets/
*.key
*.pem
*.cert

# Analysis outputs (keep structure but not data)
**/analysis_results/*.json
**/hdbits_analysis_results/*.json
scene_analysis_results.json
real_scene_analysis.json

# Session data
session_cookie.txt
cookies.txt
```

## Branch Protection Rules (Recommended)

1. **Main branch protection**:
   - Require pull request reviews before merging
   - Dismiss stale pull request approvals when new commits are pushed
   - Require status checks to pass before merging
   - Require branches to be up to date before merging
   - Include administrators in restrictions

## GitHub Actions (Optional)

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Build
      run: cargo build --verbose
    - name: Run tests
      run: cargo test --verbose
    - name: Run clippy
      run: cargo clippy -- -D warnings
    - name: Check formatting
      run: cargo fmt -- --check
```

## Repository Settings

- **Features**:
  - ✅ Issues
  - ✅ Projects
  - ✅ Wiki (optional)
  - ❌ Sponsorships (unless desired)
- **Merge button**:
  - ✅ Allow squash merging
  - ✅ Allow merge commits
  - ❌ Allow rebase merging
- **Topics**: rust, radarr, media-management, torrent, indexer, clean-architecture

## README Template

```markdown
# Rust Radarr Implementation

A modern, high-performance implementation of Radarr in Rust with clean architecture principles.

## Features

- Clean architecture design
- PostgreSQL database with async operations
- HDBits indexer integration
- TMDB metadata integration
- Scene group recognition
- Quality profile management
- Kubernetes-ready deployment

## Quick Start

\```bash
# Clone the repository
git clone https://github.com/zimmermanc/rust-radarr-src.git
cd rust-radarr-src

# Setup environment
cp .env.example .env

# Run migrations
sqlx migrate run

# Build and run
cargo build --release
cargo run
\```

## Architecture

Built using clean architecture principles with clear separation of concerns:
- Core domain logic with no external dependencies
- Repository pattern for data abstraction
- Async-first design with Tokio
- Type-safe database operations with SQLx

## License

[Your chosen license]
```

## Post-Setup Verification

After pushing to GitHub, verify:

1. **Repository Access**:
   - rust-radarr-src is publicly accessible

2. **Code Integrity**:
   - All files are properly tracked
   - No sensitive data in public repository
   - Proper .gitignore is working

3. **Branch Structure**:
   - Main branch is default
   - Protection rules are applied

4. **Documentation**:
   - README is visible and formatted correctly
   - License file is present (if applicable)

## Maintenance Commands

```bash
# Navigate to unified-radarr
cd /home/thetu/radarr-mvp/unified-radarr

# Check status
git status

# Add changes
git add .

# Commit with descriptive message
git commit -m "Your commit message"

# Push to GitHub
git push origin main
```

## Security Checklist

- [ ] No API keys or tokens in public repository
- [ ] No database credentials in code
- [ ] No HDBits session cookies or credentials
- [ ] No TMDB API keys hardcoded
- [ ] .env files are properly gitignored
- [ ] Sensitive analysis data is excluded

## Support and Issues

- **Issues**: Use https://github.com/zimmermanc/rust-radarr-src/issues
- **Discussions**: Use GitHub Discussions for questions and ideas

---

*Last Updated: 2025-08-20*
*Configuration Version: 2.0*