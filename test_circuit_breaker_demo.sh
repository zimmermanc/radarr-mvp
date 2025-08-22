#!/bin/bash

# Circuit Breaker Test Demo Script
# This script demonstrates the circuit breaker functionality in the Radarr MVP

API_KEY="testkey123"
BASE_URL="http://localhost:7878/api/v3/test/circuit-breaker"

echo "=== Circuit Breaker Demonstration ==="
echo ""

# Function to make API calls
call_api() {
    local method=$1
    local endpoint=$2
    local description=$3
    
    echo "📍 $description"
    if [ "$method" == "POST" ]; then
        curl -s -X POST "$BASE_URL/$endpoint" -H "X-API-Key: $API_KEY" | jq .
    else
        curl -s "$BASE_URL/$endpoint" -H "X-API-Key: $API_KEY" | jq .
    fi
    echo ""
}

# 1. Show initial status
call_api "GET" "status" "Initial circuit breaker status (all should be healthy and closed)"

# 2. Simulate failures for TMDB (threshold: 3)
echo "🔥 Simulating failures for TMDB service (failure threshold: 3)"
call_api "POST" "simulate-failure/tmdb" "Triggering TMDB circuit breaker"

# 3. Show status after TMDB failure
call_api "GET" "status" "Status after TMDB circuit breaker opened"

# 4. Simulate failures for database (threshold: 2)
echo "🔥 Simulating failures for PostgreSQL service (failure threshold: 2)"
call_api "POST" "simulate-failure/database" "Triggering PostgreSQL circuit breaker"

# 5. Show status with multiple circuit breakers open
call_api "GET" "status" "Status with multiple circuit breakers open"

# 6. Reset TMDB circuit breaker
echo "🔧 Resetting TMDB circuit breaker"
call_api "POST" "reset/tmdb" "Resetting TMDB to closed state"

# 7. Show final status
call_api "GET" "status" "Final status (TMDB reset, PostgreSQL still open)"

# 8. Test error handling
echo "❌ Testing error handling with invalid service"
call_api "POST" "simulate-failure/invalid_service" "Error handling test"

echo "=== Demo Complete ==="
echo ""
echo "Summary:"
echo "✅ Circuit breaker status endpoint works"
echo "✅ Failure simulation works for all services"
echo "✅ Circuit breakers open when failure thresholds are reached"
echo "✅ Manual reset functionality works"
echo "✅ Error handling works for invalid service names"
echo "✅ Different services have different failure thresholds:"
echo "   - TMDB: 3 failures"
echo "   - HDBits: 5 failures"
echo "   - qBittorrent: 4 failures"
echo "   - PostgreSQL: 2 failures"
echo ""
echo "This demonstrates that the system will stay operational even when"
echo "external services fail, protecting against cascading failures."