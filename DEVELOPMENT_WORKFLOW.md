# Development Workflow - Radarr MVP

**Updated**: 2025-08-24  
**Tools**: cargo-release + cargo-dist + GitHub Actions  
**Strategy**: Semi-automated semantic versioning with manual release decisions

## 🚀 Quick Reference

### Daily Development
```bash
# Normal development workflow
git add .
git commit -m "feat: add new movie filtering feature"
git push origin main
```

### Creating Releases
```bash
# Patch release (bug fixes)
cargo release patch --execute

# Minor release (new features)
cargo release minor --execute

# Major release (breaking changes)
cargo release major --execute
```

### Emergency Fixes
```bash
# Quick patch for critical bugs
cargo release patch --execute
# This automatically: bumps version, creates tag, pushes, triggers GitHub release
```

## 🔄 Complete Development-to-Release Workflow

### 1. Development Phase

**Daily Development:**
```bash
# Work on features/fixes normally
git checkout main
git pull origin main

# Make your changes
vim src/main.rs
cargo test --workspace
cargo clippy --all-targets --all-features

# Commit with conventional commit format (recommended)
git add .
git commit -m "feat: add streaming provider filtering"
git push origin main
```

**Pre-Release Checks:**
```bash
# Before any release, ensure everything works
cargo test --workspace                    # All tests pass
cargo clippy --all-targets --all-features # No warnings
cargo fmt --all --check                   # Formatting correct
BUILD_WEB=1 cargo build --release         # Web UI builds correctly
./target/release/radarr-mvp --help        # Binary works
```

### 2. Release Decision

**Semantic Versioning Guide:**
- **Patch (1.0.1)**: Bug fixes, security updates, documentation
- **Minor (1.1.0)**: New features, API additions (backward compatible)
- **Major (2.0.0)**: Breaking changes, API removals, architecture changes

**Examples:**
```bash
# Bug fix
cargo release patch --execute

# New API endpoint
cargo release minor --execute  

# Breaking API changes
cargo release major --execute
```

### 3. Release Execution

**cargo-release handles automatically:**
1. ✅ Version bump in `Cargo.toml`
2. ✅ Update `CHANGELOG.md` with release date
3. ✅ Create git commit with release message
4. ✅ Create git tag (e.g., `v1.0.1`)
5. ✅ Push commit and tag to GitHub
6. ✅ **cargo-dist GitHub Action triggers automatically**

**What happens after push:**
1. 🤖 GitHub Actions detects new tag
2. 🔨 cargo-dist builds cross-platform binaries
3. 🌐 Web UI assets embedded in binaries
4. 📦 Creates installers (shell script, Homebrew)
5. 📝 Generates release notes
6. 🚀 Publishes GitHub Release with all assets

### 4. Release Validation

**After release:**
```bash
# Check release was created
gh release list

# Verify assets are available
curl -s https://api.github.com/repos/zimmermanc/radarr-mvp/releases/latest

# Test installer script
curl -fsSL https://github.com/zimmermanc/radarr-mvp/releases/latest/download/radarr-mvp-installer.sh

# Validate web UI works in release
# (Test on clean server or in Docker)
```

## 🛡️ Safety Mechanisms

### Pre-Release Validation
```bash
# Always run before release
cargo release patch --dry-run    # Preview changes without executing
dist plan                        # Validate cargo-dist configuration
cargo test --workspace           # Ensure tests pass
```

### Version Consistency Checks
- **cargo-release** ensures Cargo.toml and git tags stay synchronized
- **GitHub Actions** validates build before creating release
- **Pre-commit hooks** prevent invalid configurations

### Rollback Procedures
```bash
# If release fails or has issues
git tag -d v1.0.1                    # Delete local tag
git push origin :refs/tags/v1.0.1    # Delete remote tag
gh release delete v1.0.1             # Delete GitHub release

# Revert version in Cargo.toml if needed
git revert HEAD                       # Revert release commit
```

## 🎯 Workflow Best Practices

### Commit Message Conventions
```bash
# Use conventional commits for clear history
feat: add new streaming provider integration
fix: resolve database connection timeout
chore: update dependencies
docs: improve installation instructions
style: fix code formatting
test: add integration tests for queue management
```

### Release Timing
- **Patch releases**: As needed for bug fixes (can be immediate)
- **Minor releases**: Weekly/bi-weekly for feature releases
- **Major releases**: Monthly/quarterly for significant changes

### Testing Requirements
- ✅ All tests pass (`cargo test --workspace`)
- ✅ No clippy warnings (`cargo clippy --all-targets --all-features`)
- ✅ Proper formatting (`cargo fmt --all --check`)
- ✅ Web UI builds (`BUILD_WEB=1 cargo build --release`)
- ✅ Local manual testing of key features
- ✅ Database migrations work (`sqlx migrate run`)

## 🔧 Advanced Workflows

### Hotfix Releases
```bash
# For critical production issues
git checkout main
git pull origin main

# Make minimal fix
git add . && git commit -m "fix: critical security vulnerability"

# Test thoroughly
cargo test && cargo clippy

# Release immediately
cargo release patch --execute
```

### Feature Releases
```bash
# For major new features
git checkout main
git pull origin main

# Ensure feature is complete and tested
cargo test --workspace
BUILD_WEB=1 cargo build --release

# Document changes in CHANGELOG.md
vim CHANGELOG.md

# Release
cargo release minor --execute
```

### Pre-release Testing
```bash
# Create pre-release for testing
cargo release minor --no-push --tag-name="v{{version}}-beta"

# After validation, create final release
cargo release minor --execute
```

## 🚨 Troubleshooting

### Common Issues

**"Working directory is not clean":**
```bash
git status              # Check what's uncommitted
git add . && git commit # Commit or stash changes
```

**"Tag already exists":**
```bash
git tag -d v1.0.1                    # Delete local tag
git push origin :refs/tags/v1.0.1    # Delete remote tag
```

**"GitHub Actions failing":**
```bash
gh run list --limit 5    # Check recent runs
gh run view --log        # Check failure details
```

**"cargo-dist build failing":**
```bash
dist plan               # Test cargo-dist locally
BUILD_WEB=1 cargo build --release  # Test web UI build
```

### Recovery Procedures

**Failed Release Recovery:**
1. Check GitHub Actions logs for specific errors
2. Fix issues in code
3. Delete failed tag: `git tag -d v1.0.1 && git push origin :refs/tags/v1.0.1`
4. Delete GitHub release if created: `gh release delete v1.0.1`
5. Re-run release: `cargo release patch --execute`

## 📊 Monitoring and Validation

### Release Health Checks
```bash
# After each release, verify:
gh release list                                           # Release created
curl -s https://api.github.com/repos/zimmermanc/radarr-mvp/releases/latest | jq .tag_name  # Latest version
gh run list --workflow=release.yml --limit 1             # Build successful

# Test installation works
curl -fsSL https://github.com/zimmermanc/radarr-mvp/releases/latest/download/radarr-mvp-installer.sh | head -20
```

### Continuous Integration Status
```bash
# Check all CI workflows
gh run list --limit 5
gh workflow list

# Monitor specific workflows
gh run watch  # Watch current runs
```

## 🎯 Next Release Preparation

To prepare for your next release:

1. **Make your changes** and commit normally to main
2. **Run pre-release checks** (tests, clippy, formatting)
3. **Choose version bump** based on changes made
4. **Execute release**: `cargo release [patch|minor|major] --execute`
5. **Validate GitHub release** created successfully
6. **Test installation** on clean system

The workflow ensures **code never becomes separated** because:
- ✅ All releases are created from main branch
- ✅ Version numbers automatically synchronized
- ✅ cargo-release handles all git operations atomically
- ✅ Failed releases can be cleanly rolled back
- ✅ GitHub Actions validate builds before releasing

**No manual tag creation needed** - cargo-release handles everything!