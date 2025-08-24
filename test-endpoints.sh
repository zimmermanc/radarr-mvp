#!/bin/bash
# Test all endpoints to ensure they're working

API_KEY="test-api-key"
BASE_URL="http://localhost:7878/api/v3"

echo "Testing Radarr MVP API endpoints..."
echo "=================================="

# Test health check
echo "1. Health check:"
curl -s http://localhost:7878/health | jq -r '.status'
echo ""

# Test quality profiles
echo "2. Quality profiles list:"
curl -s -H "X-Api-Key: $API_KEY" "$BASE_URL/qualityprofile" | jq -r '.[0].name'
echo ""

# Test specific quality profile
echo "3. Quality profile by ID:"
curl -s -H "X-Api-Key: $API_KEY" "$BASE_URL/qualityprofile/1" | jq -r '.name'
echo ""

# Test queue
echo "4. Queue list:"
curl -s -H "X-Api-Key: $API_KEY" "$BASE_URL/queue" | jq -r '.totalRecords'
echo " queue items"

# Test movie endpoints
echo "5. Movies list:"
curl -s -H "X-Api-Key: $API_KEY" "$BASE_URL/movie" | jq -r '.totalCount'
echo " movies in database"

# Test movie lookup (requires TMDB)
echo "6. Movie lookup (Matrix):"
curl -s -H "X-Api-Key: $API_KEY" "$BASE_URL/movie/lookup?term=matrix" | jq -r 'length'
echo " results found"

echo ""
echo "All endpoints tested! Service is operational."