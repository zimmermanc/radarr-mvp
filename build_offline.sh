#!/bin/bash

# Build script for offline compilation (without database access)
# Use when database migrations haven't been run yet

echo "Building with SQLX offline mode..."
export SQLX_OFFLINE=true
cargo build --workspace "$@"