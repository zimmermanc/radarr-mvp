#!/bin/bash

# Comprehensive Performance Benchmark Runner for Radarr MVP
# Coordinates k6, vegeta, and system monitoring for complete performance analysis

set -e

# Configuration
BASE_URL="${BASE_URL:-http://localhost:7878}"
API_KEY="${API_KEY:-test-api-key}"
RESULTS_DIR="scripts/perf/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
MONITOR_INTERVAL=5
POSTGRES_DB="${POSTGRES_DB:-radarr}"
POSTGRES_USER="${POSTGRES_USER:-radarr}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

echo -e "${PURPLE}🎯 Radarr MVP Comprehensive Performance Benchmark${NC}"
echo -e "${PURPLE}=================================================${NC}"
echo -e "${BLUE}📅 Test Date: $(date)${NC}"
echo -e "${BLUE}🌐 Base URL: $BASE_URL${NC}"
echo -e "${BLUE}📁 Results Dir: $RESULTS_DIR${NC}"
echo

# Create results directory structure
mkdir -p "$RESULTS_DIR"/{k6,vegeta,monitoring,reports}

# Function to check dependencies
check_dependencies() {
    echo -e "${YELLOW}🔍 Checking dependencies...${NC}"
    
    local missing_deps=()
    
    # Check for required tools
    command -v k6 >/dev/null 2>&1 || missing_deps+=("k6")
    command -v vegeta >/dev/null 2>&1 || missing_deps+=("vegeta")
    command -v curl >/dev/null 2>&1 || missing_deps+=("curl")
    command -v jq >/dev/null 2>&1 || missing_deps+=("jq")
    command -v bc >/dev/null 2>&1 || missing_deps+=("bc")
    command -v docker >/dev/null 2>&1 || missing_deps+=("docker")
    
    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        echo -e "${RED}❌ Missing dependencies: ${missing_deps[*]}${NC}"
        echo -e "${YELLOW}💡 Install missing tools and try again${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ All dependencies found${NC}"
}

# Function to verify application health
verify_application() {
    echo -e "${YELLOW}🏥 Verifying application health...${NC}"
    
    # Health check
    if ! curl -s "$BASE_URL/health" > /dev/null; then
        echo -e "${RED}❌ Health check failed. Application not responding on $BASE_URL${NC}"
        echo -e "${YELLOW}💡 Make sure the application is running: cargo run${NC}"
        exit 1
    fi
    
    # API check
    local api_response=$(curl -s -w "%{http_code}" -o /dev/null "$BASE_URL/api/v3/movie" -H "X-Api-Key: $API_KEY")
    if [[ "$api_response" != "200" ]]; then
        echo -e "${YELLOW}⚠️  API returned status $api_response (this may be expected)${NC}"
    fi
    
    echo -e "${GREEN}✅ Application health verified${NC}"
}

# Function to start system monitoring
start_monitoring() {
    echo -e "${YELLOW}📊 Starting system monitoring...${NC}"
    
    local monitor_file="$RESULTS_DIR/monitoring/system_metrics_${TIMESTAMP}.log"
    
    # System monitoring script
    cat > "$RESULTS_DIR/monitoring/monitor.sh" << 'EOF'
#!/bin/bash
INTERVAL=$1
OUTPUT_FILE=$2

echo "timestamp,cpu_percent,memory_mb,memory_percent,disk_io_read,disk_io_write,network_rx,network_tx,radarr_memory_mb,postgres_connections,postgres_active_queries" > "$OUTPUT_FILE"

while true; do
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # System metrics
    cpu_percent=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1)
    memory_info=$(free -m | grep '^Mem:')
    memory_used=$(echo $memory_info | awk '{print $3}')
    memory_total=$(echo $memory_info | awk '{print $2}')
    memory_percent=$(awk "BEGIN {printf \"%.1f\", ($memory_used/$memory_total)*100}")
    
    # Disk I/O (simplified)
    disk_io_read=$(cat /proc/diskstats | awk '{read_sectors += $6} END {print read_sectors}')
    disk_io_write=$(cat /proc/diskstats | awk '{write_sectors += $10} END {print write_sectors}')
    
    # Network (simplified)
    network_rx=$(cat /proc/net/dev | awk '{rx += $2} END {print rx}')
    network_tx=$(cat /proc/net/dev | awk '{tx += $10} END {print tx}')
    
    # Radarr process memory
    radarr_memory_mb=$(ps aux | grep radarr-mvp | grep -v grep | awk '{print $6}' | head -1)
    radarr_memory_mb=$((${radarr_memory_mb:-0} / 1024))
    
    # PostgreSQL metrics
    postgres_connections=$(psql -U radarr -d radarr -t -c "SELECT count(*) FROM pg_stat_activity;" 2>/dev/null | xargs || echo "0")
    postgres_active_queries=$(psql -U radarr -d radarr -t -c "SELECT count(*) FROM pg_stat_activity WHERE state = 'active';" 2>/dev/null | xargs || echo "0")
    
    echo "$timestamp,$cpu_percent,$memory_used,$memory_percent,$disk_io_read,$disk_io_write,$network_rx,$network_tx,$radarr_memory_mb,$postgres_connections,$postgres_active_queries" >> "$OUTPUT_FILE"
    
    sleep "$INTERVAL"
done
EOF
    
    chmod +x "$RESULTS_DIR/monitoring/monitor.sh"
    
    # Start background monitoring
    "$RESULTS_DIR/monitoring/monitor.sh" "$MONITOR_INTERVAL" "$monitor_file" &
    MONITOR_PID=$!
    
    echo -e "${GREEN}✅ System monitoring started (PID: $MONITOR_PID)${NC}"
    echo -e "${BLUE}📝 Monitoring data: $monitor_file${NC}"
}

# Function to stop monitoring
stop_monitoring() {
    if [[ -n "$MONITOR_PID" ]]; then
        echo -e "${YELLOW}⏹️  Stopping system monitoring...${NC}"
        kill "$MONITOR_PID" 2>/dev/null || true
        wait "$MONITOR_PID" 2>/dev/null || true
        echo -e "${GREEN}✅ Monitoring stopped${NC}"
    fi
}

# Function to run k6 tests
run_k6_tests() {
    echo -e "${YELLOW}🚀 Running k6 performance tests...${NC}"
    
    local k6_output="$RESULTS_DIR/k6/k6_results_${TIMESTAMP}"
    
    # Export environment variables for k6
    export BASE_URL API_KEY
    
    # Run k6 test with different scenarios
    echo -e "${BLUE}📊 Running comprehensive k6 test suite...${NC}"
    
    k6 run \
        --out csv="$k6_output.csv" \
        --out json="$k6_output.json" \
        --summary-trend-stats "min,avg,med,max,p(90),p(95),p(99)" \
        --summary-time-unit=ms \
        scripts/perf/k6-load-test.js
    
    echo -e "${GREEN}✅ k6 tests completed${NC}"
    echo -e "${BLUE}📁 Results saved to: $k6_output.*${NC}"
}

# Function to run vegeta tests  
run_vegeta_tests() {
    echo -e "${YELLOW}🧨 Running vegeta performance tests...${NC}"
    
    # Export environment variables for vegeta
    export BASE_URL API_KEY
    
    # Run the vegeta test suite
    ./scripts/perf/vegeta-test.sh
    
    echo -e "${GREEN}✅ Vegeta tests completed${NC}"
}

# Function to run additional performance tools
run_additional_tests() {
    echo -e "${YELLOW}🔧 Running additional performance tests...${NC}"
    
    # wrk2 test (if available)
    if command -v wrk2 >/dev/null 2>&1; then
        echo -e "${BLUE}📊 Running wrk2 constant throughput test...${NC}"
        
        wrk2 -t4 -c100 -d30s -R2000 \
            --script=scripts/perf/wrk2-script.lua \
            "$BASE_URL/health" \
            > "$RESULTS_DIR/monitoring/wrk2_results_${TIMESTAMP}.txt" 2>&1
        
        echo -e "${GREEN}✅ wrk2 test completed${NC}"
    else
        echo -e "${YELLOW}⚠️  wrk2 not found, skipping constant throughput test${NC}"
    fi
    
    # autocannon test (if available)
    if command -v autocannon >/dev/null 2>&1; then
        echo -e "${BLUE}📊 Running autocannon Node.js specific test...${NC}"
        
        autocannon -c 100 -d 30 -j \
            "$BASE_URL/health" \
            > "$RESULTS_DIR/monitoring/autocannon_results_${TIMESTAMP}.json"
        
        echo -e "${GREEN}✅ autocannon test completed${NC}"
    else
        echo -e "${YELLOW}⚠️  autocannon not found, skipping Node.js specific test${NC}"
    fi
}

# Function to analyze database performance
analyze_database_performance() {
    echo -e "${YELLOW}🗄️  Analyzing database performance...${NC}"
    
    local db_report="$RESULTS_DIR/monitoring/database_analysis_${TIMESTAMP}.txt"
    
    cat > "$db_report" << EOF
# Database Performance Analysis
Test Date: $(date)

## Connection Pool Status
EOF
    
    # Database connection analysis
    if command -v psql >/dev/null 2>&1; then
        echo "## Active Connections" >> "$db_report"
        psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
            SELECT count(*) as total_connections,
                   count(*) FILTER (WHERE state = 'active') as active_connections,
                   count(*) FILTER (WHERE state = 'idle') as idle_connections
            FROM pg_stat_activity;
        " >> "$db_report" 2>/dev/null || echo "Database query failed" >> "$db_report"
        
        echo "" >> "$db_report"
        echo "## Slow Queries (if any)" >> "$db_report"
        psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
            SELECT query, calls, total_time, mean_time 
            FROM pg_stat_statements 
            WHERE mean_time > 100 
            ORDER BY mean_time DESC 
            LIMIT 10;
        " >> "$db_report" 2>/dev/null || echo "pg_stat_statements not available" >> "$db_report"
        
        echo "" >> "$db_report"
        echo "## Database Size" >> "$db_report"
        psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "
            SELECT pg_size_pretty(pg_database_size('$POSTGRES_DB')) as database_size;
        " >> "$db_report" 2>/dev/null || echo "Database size query failed" >> "$db_report"
        
        echo -e "${GREEN}✅ Database analysis completed${NC}"
        echo -e "${BLUE}📁 Report saved to: $db_report${NC}"
    else
        echo -e "${YELLOW}⚠️  psql not available, skipping database analysis${NC}"
    fi
}

# Function to generate comprehensive report
generate_comprehensive_report() {
    echo -e "${YELLOW}📋 Generating comprehensive performance report...${NC}"
    
    local report_file="$RESULTS_DIR/reports/comprehensive_performance_report_${TIMESTAMP}.md"
    local html_report="$RESULTS_DIR/reports/comprehensive_performance_report_${TIMESTAMP}.html"
    
    cat > "$report_file" << EOF
# Radarr MVP Comprehensive Performance Report

**Test Date:** $(date)  
**Test Duration:** Multiple test suites  
**Base URL:** $BASE_URL  
**Environment:** $(uname -a)  

## Executive Summary

This report presents a comprehensive performance analysis of the Radarr MVP application using multiple modern testing tools:

- **k6**: Multi-scenario load testing with custom metrics
- **Vegeta**: High-throughput HTTP load testing  
- **System Monitoring**: Real-time resource utilization tracking
- **Database Analysis**: PostgreSQL performance metrics

## Test Results Overview

### k6 Load Testing Results

EOF
    
    # Extract k6 results if available
    local k6_json="$RESULTS_DIR/k6/k6_results_${TIMESTAMP}.json"
    if [[ -f "$k6_json" ]]; then
        echo "#### Key Metrics" >> "$report_file"
        echo '```' >> "$report_file"
        jq -r '.metrics | to_entries[] | select(.key | contains("http_req_duration")) | "\(.key): \(.value.values | to_entries[] | "\(.key)=\(.value)")"' "$k6_json" >> "$report_file" 2>/dev/null || echo "k6 results processing failed" >> "$report_file"
        echo '```' >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

### Vegeta High-Throughput Testing Results

EOF
    
    # Add vegeta results summary
    find "$RESULTS_DIR" -name "vegeta_*_${TIMESTAMP}.json" | while read -r vegeta_file; do
        test_name=$(basename "$vegeta_file" | cut -d'_' -f2)
        echo "#### $test_name Test" >> "$report_file"
        
        if [[ -f "$vegeta_file" ]]; then
            local latency_p95=$(jq -r '.latencies.p95 / 1000000' "$vegeta_file" 2>/dev/null || echo "N/A")
            local success_rate=$(jq -r '.success * 100' "$vegeta_file" 2>/dev/null || echo "N/A")
            local throughput=$(jq -r '.throughput' "$vegeta_file" 2>/dev/null || echo "N/A")
            
            echo "- **P95 Latency:** ${latency_p95} ms" >> "$report_file"
            echo "- **Success Rate:** ${success_rate}%" >> "$report_file"
            echo "- **Throughput:** ${throughput} req/s" >> "$report_file"
            echo "" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF

### System Resource Utilization

EOF
    
    # Analyze system monitoring data
    local monitor_file="$RESULTS_DIR/monitoring/system_metrics_${TIMESTAMP}.log"
    if [[ -f "$monitor_file" && $(wc -l < "$monitor_file") -gt 1 ]]; then
        echo "#### Resource Usage Summary" >> "$report_file"
        echo '```' >> "$report_file"
        
        # Calculate averages from monitoring data
        awk -F',' 'NR>1 {
            cpu_sum += $2; mem_sum += $4; radarr_mem_sum += $9; conn_sum += $10
            count++
        } END {
            if (count > 0) {
                printf "Average CPU Usage: %.1f%%\n", cpu_sum/count
                printf "Average Memory Usage: %.1f%%\n", mem_sum/count  
                printf "Average Radarr Memory: %.1f MB\n", radarr_mem_sum/count
                printf "Average DB Connections: %.1f\n", conn_sum/count
            }
        }' "$monitor_file" >> "$report_file"
        
        echo '```' >> "$report_file"
    fi
    
    cat >> "$report_file" << EOF

## Performance Analysis

### Target Compliance Assessment

#### Response Time Targets
- **P50 < 50ms**: $(check_target "p50" "50")
- **P95 < 100ms**: $(check_target "p95" "100")  
- **P99 < 200ms**: $(check_target "p99" "200")

#### Throughput Targets
- **>2000 req/s**: $(check_throughput_target "2000")
- **Error rate <0.1%**: $(check_error_rate_target "0.1")

#### Resource Targets
- **Memory <200MB**: $(check_memory_target "200")
- **CPU <40%**: $(check_cpu_target "40")
- **DB connections <8**: $(check_db_connections_target "8")

### Bottleneck Identification

Based on the comprehensive testing, the following performance characteristics were observed:

1. **Response Time Patterns**
   - Latency distribution across different load levels
   - Tail latency behavior under stress conditions
   - Impact of concurrent users on response times

2. **Throughput Scalability**  
   - Maximum sustainable request rate
   - Throughput degradation patterns
   - Connection pooling effectiveness

3. **Resource Utilization**
   - CPU usage correlation with load
   - Memory consumption patterns
   - Database connection pool efficiency

4. **Error Rate Analysis**
   - Error distribution across different endpoints
   - Load-dependent error patterns
   - Timeout and connection failures

### Optimization Recommendations

#### Immediate Actions (Quick Wins)
1. **Database Optimization**
   - Add missing indexes for frequent queries
   - Optimize slow queries identified in analysis
   - Tune connection pool size

2. **Caching Implementation**
   - Add Redis for API response caching
   - Implement application-level caching for metadata
   - Cache database query results

3. **API Performance**
   - Implement response compression
   - Add request/response size limits
   - Optimize JSON serialization

#### Medium-Term Improvements
1. **Architecture Scaling**
   - Horizontal scaling with load balancer
   - Database read replicas for query distribution
   - CDN for static assets

2. **Code Optimization**
   - Async/await optimization in hot paths
   - Database query batching
   - Connection pooling tuning

#### Long-Term Enhancements
1. **Infrastructure**
   - Microservices architecture for scalability
   - Event-driven architecture for async operations
   - Container orchestration optimization

2. **Monitoring & Observability**
   - APM tool integration
   - Custom metrics dashboards
   - Automated performance regression testing

## Test Artifacts

### Generated Files
EOF
    
    # List all generated files
    find "$RESULTS_DIR" -name "*${TIMESTAMP}*" -type f | sort | while read -r file; do
        echo "- \`$file\`" >> "$report_file"
    done
    
    cat >> "$report_file" << EOF

### Reproduction Instructions

To reproduce these tests:

1. **Setup Environment**
   \`\`\`bash
   # Start application
   cargo run
   
   # Verify health
   curl http://localhost:7878/health
   \`\`\`

2. **Run Individual Tests**
   \`\`\`bash
   # k6 load testing
   k6 run scripts/perf/k6-load-test.js
   
   # Vegeta throughput testing  
   ./scripts/perf/vegeta-test.sh
   
   # Comprehensive benchmark
   ./scripts/perf/benchmark.sh
   \`\`\`

3. **Monitor Resources**
   \`\`\`bash
   # System monitoring
   htop
   
   # Database monitoring
   psql -c "SELECT * FROM pg_stat_activity;"
   
   # Application logs
   tail -f radarr-output.log
   \`\`\`

## Conclusion

The performance testing revealed [summary of findings]. The application [meets/does not meet] the performance targets for production deployment.

**Next Steps:**
1. Implement recommended optimizations
2. Set up continuous performance monitoring
3. Establish performance regression testing in CI/CD
4. Plan capacity scaling strategy

---
*Generated by Radarr MVP Performance Testing Suite*
EOF
    
    # Generate HTML version
    if command -v pandoc >/dev/null 2>&1; then
        pandoc "$report_file" -o "$html_report" 2>/dev/null || echo "HTML generation failed"
    fi
    
    echo -e "${GREEN}✅ Comprehensive report generated${NC}"
    echo -e "${BLUE}📁 Markdown report: $report_file${NC}"
    [[ -f "$html_report" ]] && echo -e "${BLUE}📁 HTML report: $html_report${NC}"
}

# Helper functions for target checking
check_target() {
    echo "Analysis required"
}

check_throughput_target() {
    echo "Analysis required"  
}

check_error_rate_target() {
    echo "Analysis required"
}

check_memory_target() {
    echo "Analysis required"
}

check_cpu_target() {
    echo "Analysis required"
}

check_db_connections_target() {
    echo "Analysis required"
}

# Function to cleanup on exit
cleanup() {
    echo -e "${YELLOW}🧹 Cleaning up...${NC}"
    stop_monitoring
    echo -e "${GREEN}✅ Cleanup completed${NC}"
}

# Trap cleanup on exit
trap cleanup EXIT INT TERM

# Main execution
main() {
    echo -e "${BLUE}🚀 Starting comprehensive performance benchmark...${NC}"
    
    # Step 1: Pre-flight checks
    check_dependencies
    verify_application
    
    # Step 2: Start monitoring
    start_monitoring
    
    # Step 3: Run performance tests
    echo -e "${YELLOW}🧪 Running performance test suite...${NC}"
    
    # Give monitoring time to establish baseline
    sleep 10
    
    # Run k6 tests
    run_k6_tests
    
    # Brief pause between test suites
    sleep 5
    
    # Run vegeta tests
    run_vegeta_tests
    
    # Brief pause
    sleep 5
    
    # Run additional tests
    run_additional_tests
    
    # Step 4: Database analysis
    analyze_database_performance
    
    # Step 5: Generate reports
    generate_comprehensive_report
    
    # Summary
    echo -e "${PURPLE}🎉 Performance Benchmark Complete!${NC}"
    echo -e "${PURPLE}====================================${NC}"
    echo -e "${GREEN}✅ All tests completed successfully${NC}"
    echo -e "${BLUE}📁 Results directory: $RESULTS_DIR${NC}"
    echo -e "${BLUE}📋 Comprehensive report: $RESULTS_DIR/reports/comprehensive_performance_report_${TIMESTAMP}.md${NC}"
    echo
    echo -e "${YELLOW}💡 Next Steps:${NC}"
    echo -e "   1. Review the comprehensive performance report"
    echo -e "   2. Analyze bottlenecks and optimization opportunities"  
    echo -e "   3. Implement recommended performance improvements"
    echo -e "   4. Set up continuous performance monitoring"
    echo -e "   5. Integrate performance tests into CI/CD pipeline"
}

# Execute main function
main "$@"