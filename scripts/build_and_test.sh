#!/bin/bash

# Build and Test HDBits Scene Group Analysis System
# Comprehensive build, test, and validation script

set -e

echo "🚀 HDBits Scene Group Analysis System - Build & Test"
echo "===================================================="

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Must be run from the project root directory"
    exit 1
fi

echo "📋 Step 1: Checking prerequisites..."

# Check for Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust/Cargo not found. Please install Rust: https://rustup.rs/"
    exit 1
fi

# Check for required system tools
for tool in git curl; do
    if ! command -v $tool &> /dev/null; then
        echo "❌ Error: $tool not found. Please install $tool"
        exit 1
    fi
done

echo "✅ Prerequisites check passed"

echo "🔧 Step 2: Building the project..."

# Clean and build
cargo clean
if ! cargo build --release; then
    echo "❌ Build failed"
    exit 1
fi

echo "✅ Build successful"

echo "🧪 Step 3: Running tests..."

# Run unit tests
if ! cargo test --lib; then
    echo "❌ Unit tests failed"
    exit 1
fi

# Run integration tests
if ! cargo test --test test_hdbits_analysis; then
    echo "❌ Integration tests failed"
    exit 1
fi

echo "✅ All tests passed"

echo "📊 Step 4: Validating CLI tool..."

# Test CLI help
if ! cargo run --bin hdbits-analyzer -- --help &> /dev/null; then
    echo "❌ CLI tool validation failed"
    exit 1
fi

# Test dry run
echo "Testing dry run functionality..."
if ! cargo run --bin hdbits-analyzer -- --dry-run; then
    echo "❌ Dry run test failed"
    exit 1
fi

echo "✅ CLI tool validation passed"

echo "📚 Step 5: Building documentation..."

# Generate documentation
if ! cargo doc --no-deps; then
    echo "❌ Documentation build failed"
    exit 1
fi

echo "✅ Documentation built successfully"

echo "🔍 Step 6: Running code quality checks..."

# Check formatting
if ! cargo fmt -- --check; then
    echo "⚠️  Code formatting issues detected. Run 'cargo fmt' to fix."
fi

# Check with Clippy (if available)
if command -v cargo-clippy &> /dev/null; then
    if ! cargo clippy -- -W clippy::all; then
        echo "⚠️  Clippy warnings detected"
    fi
else
    echo "ℹ️  Clippy not available, skipping lint checks"
fi

echo "📦 Step 7: Building examples..."

# Build integration example
if ! cargo build --example reputation_integration; then
    echo "❌ Example build failed"
    exit 1
fi

echo "✅ Examples built successfully"

echo "📋 Step 8: Generating build summary..."

# Get build info
BUILD_DATE=$(date -u +"%Y-%m-%d %H:%M:%S UTC")
RUST_VERSION=$(rustc --version)
CARGO_VERSION=$(cargo --version)
GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo "
🎉 Build Summary
================
Build Date: $BUILD_DATE
Git Hash: $GIT_HASH
Rust Version: $RUST_VERSION
Cargo Version: $CARGO_VERSION

📁 Generated Artifacts:
- Binary: target/release/hdbits-analyzer
- Example: target/debug/examples/reputation_integration
- Documentation: target/doc/radarr_rust/index.html

🚀 Usage:
- Run analyzer: ./target/release/hdbits-analyzer
- View docs: open target/doc/radarr_rust/index.html
- Run example: cargo run --example reputation_integration

⚡ Quick Start:
1. Run data collection: ./target/release/hdbits-analyzer -o ./analysis_results
2. Review results: ls -la analysis_results/
3. Integrate scores: cargo run --example reputation_integration

📖 Documentation: See HDBITS_ANALYSIS.md for detailed usage guide
"

echo "✅ Build and test complete!"

# Optional: Create a release archive
if [ "$1" = "--package" ]; then
    echo "📦 Creating release package..."
    
    PACKAGE_NAME="hdbits-analyzer-$(date +%Y%m%d)"
    mkdir -p "releases/$PACKAGE_NAME"
    
    # Copy essential files
    cp target/release/hdbits-analyzer "releases/$PACKAGE_NAME/"
    cp HDBITS_ANALYSIS.md "releases/$PACKAGE_NAME/"
    cp README.md "releases/$PACKAGE_NAME/" 2>/dev/null || echo "README.md not found, skipping"
    cp Cargo.toml "releases/$PACKAGE_NAME/"
    
    # Create archive
    cd releases
    tar -czf "$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
    cd ..
    
    echo "📦 Release package created: releases/$PACKAGE_NAME.tar.gz"
fi

echo "🎯 Next steps:"
echo "1. Review the documentation in HDBITS_ANALYSIS.md"
echo "2. Run your first analysis with: ./target/release/hdbits-analyzer"
echo "3. Integrate the results using the example code"
echo ""
echo "Happy analyzing! 🎉"