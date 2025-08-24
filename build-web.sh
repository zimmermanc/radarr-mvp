#!/bin/bash
set -e

echo "Building web UI assets..."

# Check if we're in CI or local development
if [ "$CI" = "true" ]; then
    echo "Running in CI environment"
fi

# Navigate to web directory
cd web

# Install Node.js dependencies
echo "Installing npm dependencies..."
if command -v npm >/dev/null 2>&1; then
    npm ci --prefer-offline --no-audit
else
    echo "npm not found, attempting to install Node.js..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
        sudo apt-get install -y nodejs
    else
        echo "Error: npm and curl not available. Please install Node.js 20+"
        exit 1
    fi
    npm ci --prefer-offline --no-audit
fi

# Build the React application
echo "Building React application..."
npm run build

# Verify build output exists
if [ ! -d "dist" ]; then
    echo "Error: Build failed - dist directory not found"
    exit 1
fi

echo "Web UI build completed successfully!"
echo "Output directory: $(pwd)/dist"
echo "Files created:"
ls -la dist/

# Return to original directory
cd ..