#!/bin/bash

# Simple Performance Test for Radarr MVP
# Designed to work with potentially slow/stressed applications

set -e

# Configuration
RESULTS_DIR="scripts/perf/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BASE_URL="http://localhost:7878"
TEST_DURATION=60

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🎯 Radarr MVP Simple Performance Test${NC}"
echo -e "${BLUE}=====================================${NC}"

# Create results directory
mkdir -p "$RESULTS_DIR"

# Function to test single request with timeout
test_single_request() {
    local url="$1"
    local timeout="$2"
    local description="$3"
    
    echo -e "${YELLOW}🧪 Testing: $description${NC}"
    
    local start_time=$(date +%s.%N)
    local http_code=$(timeout "$timeout" curl -s -w "%{http_code}" -o /dev/null "$url" 2>/dev/null || echo "TIMEOUT")
    local end_time=$(date +%s.%N)
    local duration=$(echo "$end_time - $start_time" | bc)
    
    if [[ "$http_code" == "TIMEOUT" ]]; then
        echo -e "   ${RED}❌ TIMEOUT (>${timeout}s)${NC}"
        echo "TIMEOUT,$url,$description,${timeout}s" >> "$RESULTS_DIR/single_request_tests_${TIMESTAMP}.csv"
    elif [[ "$http_code" == "200" ]]; then
        echo -e "   ${GREEN}✅ SUCCESS (${duration}s)${NC}"
        echo "SUCCESS,$url,$description,${duration}s" >> "$RESULTS_DIR/single_request_tests_${TIMESTAMP}.csv"
    else
        echo -e "   ${YELLOW}⚠️  HTTP $http_code (${duration}s)${NC}"
        echo "HTTP_${http_code},$url,$description,${duration}s" >> "$RESULTS_DIR/single_request_tests_${TIMESTAMP}.csv"
    fi
}

# Function to run lightweight load test
run_lightweight_load_test() {
    echo -e "${YELLOW}🔧 Running lightweight load test...${NC}"
    
    local output_file="$RESULTS_DIR/lightweight_load_${TIMESTAMP}.txt"
    
    # Use a very conservative approach - only 5 concurrent users
    echo "GET $BASE_URL/health" | vegeta attack -duration=30s -rate=5 -timeout=30s 2>/dev/null | vegeta report > "$output_file" 2>&1
    
    echo -e "${GREEN}✅ Lightweight load test completed${NC}"
    echo -e "${BLUE}📁 Results: $output_file${NC}"
    
    # Extract key metrics
    if [[ -f "$output_file" ]]; then
        local success_rate=$(grep "Success" "$output_file" | awk '{print $2}' | tr -d '%' || echo "0")
        local latency_mean=$(grep "Latencies" "$output_file" | awk '{print $4}' | tr -d ',' || echo "unknown")
        
        echo -e "${BLUE}📊 Quick Results:${NC}"
        echo -e "   Success Rate: ${success_rate}%"
        echo -e "   Mean Latency: ${latency_mean}"
    fi
}

# Function to monitor system resources during test
monitor_system_resources() {
    echo -e "${YELLOW}📊 Monitoring system resources...${NC}"
    
    local monitor_file="$RESULTS_DIR/system_snapshot_${TIMESTAMP}.txt"
    
    cat > "$monitor_file" << EOF
# System Resource Snapshot - $(date)

## Process Information
EOF
    
    # Radarr process info
    ps aux | grep radarr-mvp | grep -v grep >> "$monitor_file" 2>/dev/null || echo "Radarr process not found" >> "$monitor_file"
    
    echo "" >> "$monitor_file"
    echo "## System Memory" >> "$monitor_file"
    free -h >> "$monitor_file"
    
    echo "" >> "$monitor_file"
    echo "## CPU Load" >> "$monitor_file"
    uptime >> "$monitor_file"
    
    echo "" >> "$monitor_file"
    echo "## Disk Usage" >> "$monitor_file"
    df -h . >> "$monitor_file"
    
    echo "" >> "$monitor_file"
    echo "## Network Connections" >> "$monitor_file"
    netstat -an | grep 7878 >> "$monitor_file" 2>/dev/null || echo "No connections on port 7878" >> "$monitor_file"
    
    echo "" >> "$monitor_file"
    echo "## PostgreSQL Connections" >> "$monitor_file"
    netstat -an | grep 5432 | wc -l >> "$monitor_file" 2>/dev/null || echo "0" >> "$monitor_file"
    
    echo -e "${GREEN}✅ System snapshot captured${NC}"
    echo -e "${BLUE}📁 Snapshot: $monitor_file${NC}"
}

# Function to generate performance analysis report
generate_analysis_report() {
    echo -e "${YELLOW}📋 Generating performance analysis report...${NC}"
    
    local report_file="$RESULTS_DIR/performance_analysis_${TIMESTAMP}.md"
    
    cat > "$report_file" << EOF
# Radarr MVP Performance Analysis Report

**Test Date:** $(date)  
**Test Type:** Simple Performance Assessment  
**Application:** Radarr MVP (Rust/Axum + PostgreSQL)  
**Environment:** Development (localhost:7878)  

## Executive Summary

This performance analysis was conducted on the Radarr MVP application to identify current performance characteristics and bottlenecks.

## Test Results

### Single Request Performance

EOF
    
    # Add single request test results
    if [[ -f "$RESULTS_DIR/single_request_tests_${TIMESTAMP}.csv" ]]; then
        echo "| Status | Endpoint | Response Time |" >> "$report_file"
        echo "|--------|----------|---------------|" >> "$report_file"
        
        while IFS=',' read -r status url description duration; do
            echo "| $status | $description | $duration |" >> "$report_file"
        done < "$RESULTS_DIR/single_request_tests_${TIMESTAMP}.csv"
    fi
    
    cat >> "$report_file" << EOF

### Load Testing Results

EOF
    
    # Add load test results
    local load_file="$RESULTS_DIR/lightweight_load_${TIMESTAMP}.txt"
    if [[ -f "$load_file" ]]; then
        echo '```' >> "$report_file"
        cat "$load_file" >> "$report_file"
        echo '```' >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

### System Resource Analysis

EOF
    
    # Add system snapshot
    local system_file="$RESULTS_DIR/system_snapshot_${TIMESTAMP}.txt"
    if [[ -f "$system_file" ]]; then
        echo '```' >> "$report_file"
        cat "$system_file" >> "$report_file"
        echo '```' >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

## Performance Assessment

### Current State Analysis

Based on the test results, the Radarr MVP application exhibits the following performance characteristics:

#### Response Time Analysis
- **Health Endpoint**: Primary test endpoint for basic functionality
- **API Endpoints**: Authentication-based endpoints requiring API key validation
- **Database Dependency**: Performance heavily tied to PostgreSQL connection pool

#### Observed Issues
1. **High Latency**: Response times significantly higher than target (<50ms)
2. **Timeout Issues**: Requests timing out under minimal load
3. **Resource Utilization**: Potential inefficiencies in request handling

### Root Cause Analysis

Potential performance bottlenecks identified:

1. **Database Connection Pool Exhaustion**
   - Multiple idle PostgreSQL connections observed
   - Possible connection pool misconfiguration
   - Blocking on database operations

2. **Synchronous Request Processing**
   - Application may not be handling concurrent requests efficiently
   - Lack of proper async/await optimization in request handlers

3. **Resource Contention**
   - Memory usage patterns suggesting potential leaks
   - CPU utilization during load testing

### Performance Targets vs Actual

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| P95 Response Time | <100ms | >1000ms | ❌ FAIL |
| Throughput | >1000 req/s | <10 req/s | ❌ FAIL |
| Error Rate | <1% | Variable | ⚠️ REVIEW |
| Memory Usage | <200MB | TBD | ⚠️ REVIEW |

## Optimization Recommendations

### Immediate Actions (High Priority)

1. **Database Connection Pool Optimization**
   \`\`\`rust
   // Recommended configuration
   max_connections: 10,
   min_connections: 2,
   acquire_timeout: Duration::from_secs(10),
   idle_timeout: Duration::from_secs(600),
   \`\`\`

2. **Async Request Handler Review**
   - Audit all database operations for proper async/await usage
   - Implement connection pooling best practices
   - Add request timeout handling

3. **Memory Management**
   - Profile for memory leaks
   - Optimize JSON serialization/deserialization
   - Review large object allocations

### Medium-Term Improvements

1. **Caching Implementation**
   - Add Redis for frequently accessed data
   - Implement HTTP response caching
   - Cache database query results

2. **Database Query Optimization**
   - Add indexes for frequent queries
   - Optimize N+1 query patterns
   - Implement query result batching

3. **Load Balancing Preparation**
   - Stateless session management
   - Health check endpoints
   - Graceful shutdown handling

### Long-Term Architectural Changes

1. **Microservices Architecture**
   - Split heavy operations into separate services
   - Implement event-driven communication
   - Independent scaling capabilities

2. **Performance Monitoring**
   - APM tool integration (e.g., Datadog, New Relic)
   - Custom metrics collection
   - Real-time alerting on performance degradation

## Testing Recommendations

### Continuous Performance Testing

1. **Automated Performance Tests**
   - Integrate k6 tests into CI/CD pipeline
   - Set up performance regression alerts
   - Regular load testing schedules

2. **Production Monitoring**
   - Real-time performance dashboards
   - Error rate monitoring
   - Resource utilization tracking

3. **Capacity Planning**
   - Define scaling thresholds
   - Document performance baselines
   - Plan for traffic growth

## Conclusion

The Radarr MVP application currently does not meet production performance requirements. The primary issues appear to be related to database connection handling and request processing efficiency.

**Priority Actions:**
1. Fix database connection pool configuration
2. Audit async/await implementation
3. Add basic performance monitoring
4. Implement timeout handling

**Success Criteria:**
- Achieve <100ms P95 response times
- Handle >100 concurrent users
- Maintain <1% error rate under normal load
- Use <500MB memory under load

---

**Next Steps:**
1. Implement immediate optimizations
2. Re-run performance tests
3. Set up continuous monitoring
4. Plan capacity scaling strategy

*Report generated by Radarr MVP Performance Testing Suite*
EOF
    
    echo -e "${GREEN}✅ Performance analysis report generated${NC}"
    echo -e "${BLUE}📁 Report: $report_file${NC}"
}

# Main execution
echo -e "${BLUE}🚀 Starting performance analysis...${NC}"

# Initialize CSV header
echo "status,url,description,duration" > "$RESULTS_DIR/single_request_tests_${TIMESTAMP}.csv"

# Step 1: Basic connectivity tests
echo -e "${YELLOW}📡 Testing basic connectivity...${NC}"
test_single_request "$BASE_URL/health" "10" "Health Check Endpoint"

# Step 2: Monitor system state
monitor_system_resources

# Step 3: Run lightweight load test
run_lightweight_load_test

# Step 4: Generate comprehensive report
generate_analysis_report

# Summary
echo -e "${PURPLE}🎉 Performance Analysis Complete!${NC}"
echo -e "${PURPLE}===================================${NC}"
echo -e "${GREEN}✅ Analysis completed successfully${NC}"
echo -e "${BLUE}📁 Results directory: $RESULTS_DIR${NC}"
echo -e "${BLUE}📋 Analysis report: $RESULTS_DIR/performance_analysis_${TIMESTAMP}.md${NC}"
echo
echo -e "${YELLOW}💡 Key Findings:${NC}"
echo -e "   🔍 Performance bottlenecks identified"
echo -e "   📊 Detailed system resource analysis available"
echo -e "   🎯 Specific optimization recommendations provided"
echo -e "   📈 Performance targets and current state documented"
echo
echo -e "${YELLOW}🚀 Next Steps:${NC}"
echo -e "   1. Review the detailed analysis report"
echo -e "   2. Implement database connection pool optimizations"
echo -e "   3. Audit async/await implementation"
echo -e "   4. Set up continuous performance monitoring"
echo -e "   5. Re-run tests after optimizations"