#!/bin/bash

# Vegeta Performance Testing Script for Radarr MVP
# Modern high-throughput HTTP load testing

set -e

# Configuration
BASE_URL="${BASE_URL:-http://localhost:7878}"
API_KEY="${API_KEY:-test-api-key}"
RESULTS_DIR="scripts/perf/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Radarr MVP Vegeta Performance Testing${NC}"
echo -e "${BLUE}=======================================${NC}"

# Create results directory
mkdir -p "$RESULTS_DIR"

# Health check
echo -e "${YELLOW}🏥 Performing health check...${NC}"
if ! curl -s "$BASE_URL/health" > /dev/null; then
    echo -e "${RED}❌ Health check failed. Is the application running on $BASE_URL?${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Health check passed${NC}"

# Function to run vegeta attack and generate reports
run_vegeta_test() {
    local test_name="$1"
    local rate="$2"
    local duration="$3"
    local description="$4"
    
    echo -e "${YELLOW}🧪 Running $test_name test ($description)${NC}"
    echo "   Rate: $rate req/s, Duration: $duration"
    
    local output_file="$RESULTS_DIR/vegeta_${test_name}_${TIMESTAMP}"
    
    # Run the attack
    vegeta attack \
        -targets=scripts/perf/vegeta-targets.txt \
        -rate="$rate" \
        -duration="$duration" \
        -timeout=30s \
        -workers=10 \
        -max-workers=20 \
        -output="$output_file.bin"
    
    # Generate text report
    vegeta report -type=text "$output_file.bin" > "$output_file.txt"
    
    # Generate JSON report
    vegeta report -type=json "$output_file.bin" > "$output_file.json"
    
    # Generate histogram
    vegeta report -type=hist[0,50ms,100ms,200ms,500ms,1s,2s] "$output_file.bin" > "$output_file.hist"
    
    # Generate plot (if available)
    if command -v vegeta &> /dev/null && vegeta plot --help 2>&1 | grep -q "plot"; then
        vegeta plot -title="$test_name Test Results" "$output_file.bin" > "$output_file.html"
    fi
    
    # Display summary
    echo -e "${GREEN}📊 $test_name Test Results:${NC}"
    grep -E "(Requests|Rate|Throughput|Success|Status Codes)" "$output_file.txt" | sed 's/^/   /'
    echo
}

# Function to analyze results
analyze_results() {
    local test_name="$1"
    local json_file="$RESULTS_DIR/vegeta_${test_name}_${TIMESTAMP}.json"
    
    if [[ -f "$json_file" ]]; then
        local latency_p50=$(jq -r '.latencies.p50 / 1000000' "$json_file")
        local latency_p95=$(jq -r '.latencies.p95 / 1000000' "$json_file")
        local latency_p99=$(jq -r '.latencies.p99 / 1000000' "$json_file")
        local success_rate=$(jq -r '.success * 100' "$json_file")
        local throughput=$(jq -r '.throughput' "$json_file")
        
        echo -e "${BLUE}📈 $test_name Analysis:${NC}"
        printf "   P50 Latency: %.2f ms\n" "$latency_p50"
        printf "   P95 Latency: %.2f ms\n" "$latency_p95"
        printf "   P99 Latency: %.2f ms\n" "$latency_p99"
        printf "   Success Rate: %.2f%%\n" "$success_rate"
        printf "   Throughput: %.2f req/s\n" "$throughput"
        
        # Check against targets
        echo -e "${BLUE}🎯 Target Compliance:${NC}"
        
        if (( $(echo "$latency_p95 < 50" | bc -l) )); then
            echo -e "   P95 < 50ms: ${GREEN}✅ PASS${NC} (${latency_p95}ms)"
        else
            echo -e "   P95 < 50ms: ${RED}❌ FAIL${NC} (${latency_p95}ms)"
        fi
        
        if (( $(echo "$latency_p99 < 100" | bc -l) )); then
            echo -e "   P99 < 100ms: ${GREEN}✅ PASS${NC} (${latency_p99}ms)"
        else
            echo -e "   P99 < 100ms: ${RED}❌ FAIL${NC} (${latency_p99}ms)"
        fi
        
        if (( $(echo "$success_rate > 99" | bc -l) )); then
            echo -e "   Success Rate > 99%: ${GREEN}✅ PASS${NC} (${success_rate}%)"
        else
            echo -e "   Success Rate > 99%: ${RED}❌ FAIL${NC} (${success_rate}%)"
        fi
        
        if (( $(echo "$throughput > 1000" | bc -l) )); then
            echo -e "   Throughput > 1000 req/s: ${GREEN}✅ PASS${NC} (${throughput} req/s)"
        else
            echo -e "   Throughput > 1000 req/s: ${RED}❌ FAIL${NC} (${throughput} req/s)"
        fi
        
        echo
    fi
}

# Test Suite
echo -e "${BLUE}🧪 Starting Vegeta Test Suite${NC}"
echo

# 1. Baseline Test - Low load to establish baseline
run_vegeta_test "baseline" "50" "60s" "Baseline performance measurement"
analyze_results "baseline"

# 2. Moderate Load Test - Normal operation
run_vegeta_test "moderate" "200" "120s" "Moderate load testing"
analyze_results "moderate"

# 3. High Throughput Test - Push throughput limits
run_vegeta_test "high_throughput" "500" "60s" "High throughput testing"
analyze_results "high_throughput"

# 4. Stress Test - Find breaking point
run_vegeta_test "stress" "1000" "30s" "Stress testing to find limits"
analyze_results "stress"

# 5. Burst Test - Sudden load spikes
echo -e "${YELLOW}🧪 Running burst test (Variable rate)${NC}"
echo "   Simulating traffic spikes"

vegeta attack \
    -targets=scripts/perf/vegeta-targets.txt \
    -rate="100/1s,500/5s,100/10s" \
    -timeout=30s \
    -workers=10 \
    -output="$RESULTS_DIR/vegeta_burst_${TIMESTAMP}.bin"

vegeta report -type=text "$RESULTS_DIR/vegeta_burst_${TIMESTAMP}.bin" > "$RESULTS_DIR/vegeta_burst_${TIMESTAMP}.txt"
vegeta report -type=json "$RESULTS_DIR/vegeta_burst_${TIMESTAMP}.bin" > "$RESULTS_DIR/vegeta_burst_${TIMESTAMP}.json"

echo -e "${GREEN}📊 Burst Test Results:${NC}"
grep -E "(Requests|Rate|Throughput|Success|Status Codes)" "$RESULTS_DIR/vegeta_burst_${TIMESTAMP}.txt" | sed 's/^/   /'
echo

analyze_results "burst"

# 6. Sustained Load Test - Endurance testing
run_vegeta_test "sustained" "100" "300s" "Sustained load endurance test"
analyze_results "sustained"

# Generate comprehensive report
echo -e "${BLUE}📋 Generating Comprehensive Report${NC}"

REPORT_FILE="$RESULTS_DIR/vegeta_comprehensive_report_${TIMESTAMP}.md"

cat > "$REPORT_FILE" << EOF
# Radarr MVP Vegeta Performance Test Report

**Test Date:** $(date)
**Base URL:** $BASE_URL
**Test Duration:** Various (see individual tests)

## Executive Summary

This report contains results from comprehensive Vegeta load testing of the Radarr MVP application.

## Test Results Summary

EOF

# Add results from all tests
for test_type in baseline moderate high_throughput stress burst sustained; do
    json_file="$RESULTS_DIR/vegeta_${test_type}_${TIMESTAMP}.json"
    if [[ -f "$json_file" ]]; then
        echo "### $test_type Test" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        
        # Extract key metrics
        local latency_p50=$(jq -r '.latencies.p50 / 1000000' "$json_file" 2>/dev/null || echo "N/A")
        local latency_p95=$(jq -r '.latencies.p95 / 1000000' "$json_file" 2>/dev/null || echo "N/A")
        local latency_p99=$(jq -r '.latencies.p99 / 1000000' "$json_file" 2>/dev/null || echo "N/A")
        local success_rate=$(jq -r '.success * 100' "$json_file" 2>/dev/null || echo "N/A")
        local throughput=$(jq -r '.throughput' "$json_file" 2>/dev/null || echo "N/A")
        
        cat >> "$REPORT_FILE" << EOF
- **P50 Latency:** ${latency_p50} ms
- **P95 Latency:** ${latency_p95} ms  
- **P99 Latency:** ${latency_p99} ms
- **Success Rate:** ${success_rate}%
- **Throughput:** ${throughput} req/s

EOF
    fi
done

cat >> "$REPORT_FILE" << EOF

## Performance Analysis

### Bottleneck Identification

Based on the test results, the following performance characteristics were observed:

1. **Response Time Patterns:** Analyze P95/P99 latencies across different load levels
2. **Throughput Scalability:** Maximum sustainable request rate before degradation
3. **Error Rate Correlation:** Relationship between load and error rates
4. **Resource Utilization:** Database connection pool and memory usage patterns

### Recommendations

1. **Immediate Optimizations:** Quick wins for performance improvement
2. **Infrastructure Scaling:** Horizontal/vertical scaling recommendations  
3. **Application Tuning:** Code-level optimizations
4. **Monitoring Setup:** Key metrics to track in production

## Test Files

All raw test results and detailed reports are available in:
- Text Reports: \`$RESULTS_DIR/vegeta_*_${TIMESTAMP}.txt\`
- JSON Data: \`$RESULTS_DIR/vegeta_*_${TIMESTAMP}.json\`
- Histograms: \`$RESULTS_DIR/vegeta_*_${TIMESTAMP}.hist\`

EOF

echo -e "${GREEN}✅ Comprehensive report generated: $REPORT_FILE${NC}"

# Display final summary
echo -e "${BLUE}🎯 Performance Testing Complete${NC}"
echo -e "${BLUE}================================${NC}"
echo -e "${GREEN}✅ All Vegeta tests completed successfully${NC}"
echo -e "${YELLOW}📁 Results saved to: $RESULTS_DIR/${NC}"
echo -e "${YELLOW}📋 Comprehensive report: $REPORT_FILE${NC}"

# List all generated files
echo -e "${BLUE}📁 Generated Files:${NC}"
find "$RESULTS_DIR" -name "*${TIMESTAMP}*" -type f | sort | sed 's/^/   /'

echo
echo -e "${BLUE}💡 Next Steps:${NC}"
echo -e "   1. Review the comprehensive report for detailed analysis"
echo -e "   2. Compare results against performance targets"
echo -e "   3. Identify bottlenecks and optimization opportunities"
echo -e "   4. Implement recommended improvements"
echo -e "   5. Re-run tests to validate improvements"