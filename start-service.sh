#!/bin/bash
# Start Radarr MVP service with required environment variables

echo "Starting Radarr MVP service..."

# Kill any existing process
pkill -f radarr-mvp

# Export required environment variables
export TMDB_API_KEY="YOUR_TMDB_API_KEY_HERE"
export RADARR_API_KEY="test-api-key"

# Start the service
echo "Starting with environment:"
echo "  TMDB_API_KEY set: ${TMDB_API_KEY:+yes}"
echo "  RADARR_API_KEY set: ${RADARR_API_KEY:+yes}"
echo "  Service will start on http://localhost:7878"

cargo run --bin radarr-mvp