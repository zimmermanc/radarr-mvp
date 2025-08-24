#!/bin/bash
set -e

cd /home/thetu/radarr-mvp/unified-radarr

echo "🚀 Radarr MVP Quick Compilation Test"
echo "===================================="

# Test compilation
echo "1. Testing Rust compilation..."
if cargo check --workspace --all-targets; then
    echo "✅ Compilation successful"
else
    echo "❌ Compilation failed"
    exit 1
fi

# Test cargo build
echo -e "\n2. Testing cargo build..."
if cargo build --workspace; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# Run unit tests
echo -e "\n3. Testing unit tests..."
if cargo test --workspace --lib; then
    echo "✅ Unit tests passed"
else
    echo "❌ Unit tests failed"
    exit 1
fi

# Test that main binary can be started briefly
echo -e "\n4. Testing main binary startup..."
timeout 10 cargo run --bin radarr-mvp &
SERVER_PID=$!

sleep 5

if kill -0 $SERVER_PID 2>/dev/null; then
    echo "✅ Server started successfully"
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
else
    echo "❌ Server failed to start"
    exit 1
fi

echo -e "\n🎉 All quick tests passed!"