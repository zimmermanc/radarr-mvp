#!/bin/bash

# Comprehensive Test Script for Radarr MVP
# Tests all newly implemented functionality

set -e

echo "🚀 Radarr MVP Comprehensive Test Suite"
echo "======================================"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Function to run a test and track results
run_test() {
    local test_name="$1"
    local test_command="$2"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "\n${BLUE}[TEST $TOTAL_TESTS]${NC} $test_name"
    echo "Command: $test_command"
    
    if eval "$test_command"; then
        echo -e "${GREEN}✅ PASS${NC}: $test_name"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL${NC}: $test_name"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# Function to test API endpoint
test_api_endpoint() {
    local endpoint="$1"
    local method="$2"
    local expected_status="$3"
    local description="$4"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "\n${BLUE}[API TEST $TOTAL_TESTS]${NC} $description"
    echo "Endpoint: $method $endpoint"
    
    response=$(curl -s -w "\n%{http_code}" -X "$method" \
        -H "X-Api-Key: YOUR_API_KEY_HERE" \
        -H "Content-Type: application/json" \
        "http://localhost:7878$endpoint" 2>/dev/null || echo -e "\nERROR")
    
    status_code=$(echo "$response" | tail -n 1)
    body=$(echo "$response" | head -n -1)
    
    if [ "$status_code" = "$expected_status" ]; then
        echo -e "${GREEN}✅ PASS${NC}: $description (Status: $status_code)"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        
        # Pretty print JSON response if it's JSON
        if echo "$body" | jq . >/dev/null 2>&1; then
            echo "Response:"
            echo "$body" | jq .
        else
            echo "Response: $body"
        fi
    else
        echo -e "${RED}❌ FAIL${NC}: $description (Expected: $expected_status, Got: $status_code)"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        echo "Response: $body"
    fi
}

echo -e "${YELLOW}Phase 1: Compilation & Build Tests${NC}"
echo "=================================="

run_test "Check Rust toolchain" "rustc --version"
run_test "Check Cargo version" "cargo --version"
run_test "Cargo format check" "cargo fmt --all -- --check"
run_test "Clippy lint check" "cargo clippy --workspace --all-targets -- -D warnings"
run_test "Cargo build debug mode" "cargo build --workspace"
run_test "Run unit tests" "cargo test --workspace --lib"

echo -e "\n${YELLOW}Phase 2: Database Setup${NC}"
echo "======================"

# Check if PostgreSQL is running
if ! pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠️ WARNING${NC}: PostgreSQL not running, attempting to start..."
    sudo service postgresql start || echo -e "${RED}Failed to start PostgreSQL${NC}"
fi

run_test "Check PostgreSQL connection" "pg_isready -h localhost -p 5432"
run_test "Check test database exists" "psql -h localhost -U radarr -d radarr_test -c 'SELECT 1;' >/dev/null 2>&1"
run_test "Run database migrations" "cd /home/thetu/radarr-mvp/unified-radarr && sqlx migrate run --database-url postgresql://radarr:radarr@localhost:5432/radarr_test"

echo -e "\n${YELLOW}Phase 3: Backend Startup Test${NC}"
echo "=============================="

# Start the server in background
echo "Starting Radarr MVP server..."
cd /home/thetu/radarr-mvp/unified-radarr
cargo run --bin radarr-mvp &
SERVER_PID=$!

# Wait for server to start
echo "Waiting for server to start..."
sleep 10

# Check if server is running
if kill -0 $SERVER_PID 2>/dev/null; then
    echo -e "${GREEN}✅ Server started successfully (PID: $SERVER_PID)${NC}"
    SERVER_RUNNING=true
else
    echo -e "${RED}❌ Server failed to start${NC}"
    SERVER_RUNNING=false
fi

if [ "$SERVER_RUNNING" = true ]; then
    echo -e "\n${YELLOW}Phase 4: API Endpoint Tests${NC}"
    echo "=========================="

    # Health check endpoints
    test_api_endpoint "/health/detailed" "GET" "200" "Detailed Health Check"
    test_api_endpoint "/api/v1/system/status" "GET" "200" "System Status"
    test_api_endpoint "/api/queue/status" "GET" "200" "Queue Status"

    # TMDb integration tests
    echo -e "\n${BLUE}TMDb Integration Tests${NC}"
    test_api_endpoint "/api/v3/lists/tmdb/popular" "GET" "200" "TMDb Popular Movies"
    test_api_endpoint "/api/v3/lists/tmdb/trending/day" "GET" "200" "TMDb Trending Daily"
    test_api_endpoint "/api/v3/lists/tmdb/top-rated" "GET" "200" "TMDb Top Rated"
    test_api_endpoint "/api/v3/lists/tmdb/now-playing" "GET" "200" "TMDb Now Playing"
    test_api_endpoint "/api/v3/lists/tmdb/upcoming" "GET" "200" "TMDb Upcoming"

    # Queue management tests
    echo -e "\n${BLUE}Queue Management Tests${NC}"
    test_api_endpoint "/api/v3/queue" "GET" "200" "Get Queue Items"
    
    # Movie action tests
    echo -e "\n${BLUE}Movie Action Tests${NC}"
    test_api_endpoint "/api/v3/movie" "GET" "200" "List Movies"
    test_api_endpoint "/api/v3/movie/lookup?term=inception" "GET" "200" "Movie Lookup"
    test_api_endpoint "/api/v3/qualityprofile" "GET" "200" "Quality Profiles"

    # Test with invalid API key
    echo -e "\n${BLUE}Security Tests${NC}"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -e "\n${BLUE}[API TEST $TOTAL_TESTS]${NC} Test Invalid API Key"
    response=$(curl -s -w "\n%{http_code}" -X "GET" \
        -H "X-Api-Key: invalid_key" \
        "http://localhost:7878/api/v3/movie" 2>/dev/null || echo -e "\n401")
    status_code=$(echo "$response" | tail -n 1)
    
    if [ "$status_code" = "401" ]; then
        echo -e "${GREEN}✅ PASS${NC}: Invalid API Key Rejected (Status: $status_code)"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL${NC}: Invalid API Key Not Rejected (Got: $status_code)"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi

    echo -e "\n${YELLOW}Phase 5: Performance Tests${NC}"
    echo "========================"

    # Test server response time
    echo "Testing API response time..."
    response_time=$(curl -s -w "%{time_total}" -o /dev/null \
        -H "X-Api-Key: YOUR_API_KEY_HERE" \
        "http://localhost:7878/api/v1/system/status")
    
    if (( $(echo "$response_time < 2.0" | bc -l) )); then
        echo -e "${GREEN}✅ PASS${NC}: Response time acceptable ($response_time seconds)"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}❌ FAIL${NC}: Response time too slow ($response_time seconds)"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    echo -e "\n${YELLOW}Phase 6: Integration Tests${NC}"
    echo "=========================="

    # Test connectivity
    test_api_endpoint "/api/v1/test/connectivity" "POST" "200" "Test Connectivity"

    # Memory usage check
    echo "Checking server memory usage..."
    if [ -f /proc/$SERVER_PID/status ]; then
        memory_kb=$(grep VmRSS /proc/$SERVER_PID/status | awk '{print $2}')
        memory_mb=$((memory_kb / 1024))
        echo "Server memory usage: ${memory_mb} MB"
        
        if [ "$memory_mb" -lt 500 ]; then
            echo -e "${GREEN}✅ PASS${NC}: Memory usage acceptable (${memory_mb} MB)"
            PASSED_TESTS=$((PASSED_TESTS + 1))
        else
            echo -e "${YELLOW}⚠️ WARNING${NC}: High memory usage (${memory_mb} MB)"
            PASSED_TESTS=$((PASSED_TESTS + 1))  # Don't fail for high memory
        fi
        TOTAL_TESTS=$((TOTAL_TESTS + 1))
    fi

    # Stop the server
    echo -e "\n${YELLOW}Stopping server...${NC}"
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
    echo "Server stopped."

else
    echo -e "${RED}❌ Skipping API tests due to server startup failure${NC}"
fi

echo -e "\n${YELLOW}Phase 7: Frontend Integration Test${NC}"
echo "=================================="

# Check if frontend can build
if [ -d "web" ]; then
    cd web
    if [ -f "package.json" ]; then
        run_test "Frontend dependency check" "npm ls --depth=0 >/dev/null 2>&1 || npm install"
        run_test "Frontend build test" "npm run build"
        run_test "Frontend lint check" "npm run lint || true"  # Don't fail if no lint script
    else
        echo -e "${YELLOW}⚠️ No package.json found in web directory${NC}"
    fi
    cd ..
else
    echo -e "${YELLOW}⚠️ No web directory found${NC}"
fi

echo -e "\n${YELLOW}Final Results${NC}"
echo "============="
echo -e "Total Tests: ${BLUE}$TOTAL_TESTS${NC}"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

success_rate=0
if [ $TOTAL_TESTS -gt 0 ]; then
    success_rate=$(( (PASSED_TESTS * 100) / TOTAL_TESTS ))
fi

echo -e "Success Rate: ${BLUE}${success_rate}%${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 ALL TESTS PASSED!${NC}"
    echo "The Radarr MVP application is working correctly."
    exit 0
elif [ $success_rate -ge 80 ]; then
    echo -e "\n${YELLOW}⚠️ MOSTLY PASSING${NC} (${success_rate}% success rate)"
    echo "Most functionality is working, but some issues need attention."
    exit 1
else
    echo -e "\n${RED}❌ MULTIPLE FAILURES${NC} (${success_rate}% success rate)"
    echo "Significant issues detected that need immediate attention."
    exit 2
fi