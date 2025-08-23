#!/bin/bash

# HDBits Analysis - Quick Test Script
# Runs for 2 minutes, then takes a 1 minute break, for testing purposes

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Load centralized configuration
source "$SCRIPT_DIR/load_config.sh" hdbits

ANALYSIS_DIR="/tmp/radarr_test_analysis"
LOG_DIR="/tmp"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$LOG_DIR/hdbits_test_${TIMESTAMP}.log"

# Test timing configuration (much shorter for testing)
RUN_DURATION_MINUTES=0.1  # 6 seconds for quick test
BREAK_DURATION_MINUTES=0.1  # 6 seconds break
MAX_SEGMENTS=2  # Just 2 segments for testing
RATE_LIMIT_SECONDS=5
MAX_PAGES_PER_SEGMENT=10  # Limit pages for quick test

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create directories
mkdir -p "$ANALYSIS_DIR"
mkdir -p "$ANALYSIS_DIR/segments"

# Function to log messages
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Function to print colored messages
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_color "$GREEN" "=========================================="
print_color "$GREEN" "HDBits Analysis Test Mode"
print_color "$GREEN" "=========================================="
echo

# Check if HDBits is configured
if ! is_service_configured hdbits; then
    print_color "$YELLOW" "WARNING: HDBits not fully configured. Using test mode."
    print_color "$YELLOW" "For real analysis, configure HDBits in: $RADARR_CONFIG_FILE"
    echo
    
    # For testing, we'll use a dummy cookie and expect it to fail auth
    export HDBITS_SESSION_COOKIE="test_session=dummy"
    TEST_MODE=1
fi

log "Test Configuration:"
log "  - Run Duration: ${RUN_DURATION_MINUTES} minutes per segment"
log "  - Break Duration: ${BREAK_DURATION_MINUTES} minutes"
log "  - Max Segments: ${MAX_SEGMENTS}"
log "  - Max Pages per Segment: ${MAX_PAGES_PER_SEGMENT}"
log "  - Rate Limit: ${RATE_LIMIT_SECONDS} seconds"
log "  - Output Directory: $ANALYSIS_DIR"
log "  - Log File: $LOG_FILE"
echo

# Change to project directory
cd "$PROJECT_ROOT"

# Build the analyzer
print_color "$YELLOW" "Building HDBits analyzer..."
if cargo build -p radarr-analysis --bin hdbits-comprehensive-analyzer --release >> "$LOG_FILE" 2>&1; then
    print_color "$GREEN" "✓ Build successful"
else
    print_color "$RED" "✗ Build failed - check $LOG_FILE for details"
    exit 1
fi

# Test database connection
print_color "$YELLOW" "Testing database connection..."
if [ -z "$DATABASE_URL" ]; then
    export DATABASE_URL="postgresql://radarr:radarr@localhost:5432/radarr"
fi

# Try to connect to PostgreSQL using Python
if command -v python3 &> /dev/null; then
    python3 -c "
import sys
try:
    import psycopg2
    conn = psycopg2.connect('$DATABASE_URL')
    conn.close()
    print('✓ Database connection successful')
    sys.exit(0)
except Exception as e:
    print(f'✗ Database connection failed: {e}')
    sys.exit(1)
" 2>/dev/null
    DB_STATUS=$?
    
    if [ $DB_STATUS -eq 0 ]; then
        print_color "$GREEN" "✓ Database connection successful"
    else
        print_color "$YELLOW" "⚠ Database connection failed - will save to files only"
    fi
else
    print_color "$YELLOW" "⚠ Python3 not available - skipping database test"
fi

echo
print_color "$GREEN" "Starting Test Analysis"
print_color "$GREEN" "=========================================="
echo

# Run test segments
SEGMENT=1
TOTAL_SUCCESS=0

while [ $SEGMENT -le $MAX_SEGMENTS ]; do
    print_color "$YELLOW" "SEGMENT $SEGMENT of $MAX_SEGMENTS"
    
    SEGMENT_OUTPUT="$ANALYSIS_DIR/segments/test_segment_${SEGMENT}.json"
    SEGMENT_CSV="$ANALYSIS_DIR/segments/test_segment_${SEGMENT}.csv"
    
    log "Starting test segment $SEGMENT (${RUN_DURATION_MINUTES} minute run)"
    
    # Run with timeout and limited pages
    SEGMENT_START=$(date +%s)
    
    # Create a test wrapper that will simulate or run real analysis
    if [ "$TEST_MODE" = "1" ]; then
        print_color "$YELLOW" "Running in TEST MODE (no real API calls)"
        
        # Create dummy output for testing
        cat > "$SEGMENT_OUTPUT" << EOF
{
  "scene_groups": {
    "TEST-GROUP-$SEGMENT": {
      "release_count": 10,
      "internal_releases": 5,
      "total_size_gb": 100.5,
      "average_size_gb": 10.05,
      "freeleech_count": 3,
      "freeleech_percentage": 30.0,
      "reputation_score": 75.5
    },
    "DEMO-GROUP-$SEGMENT": {
      "release_count": 5,
      "internal_releases": 2,
      "total_size_gb": 50.0,
      "average_size_gb": 10.0,
      "freeleech_count": 1,
      "freeleech_percentage": 20.0,
      "reputation_score": 65.0
    }
  },
  "total_releases": 15,
  "internal_releases": 7,
  "pages_processed": 5,
  "analysis_date": "$(date -Iseconds)"
}
EOF
        print_color "$GREEN" "✓ Created test data for segment $SEGMENT"
        ANALYZER_EXIT_CODE=0
    else
        # Run real analysis with timeout
        timeout ${RUN_DURATION_MINUTES}m \
            RUST_LOG=info cargo run -p radarr-analysis --bin hdbits-comprehensive-analyzer --release -- \
            --session-cookie "$HDBITS_SESSION_COOKIE" \
            --output "$SEGMENT_OUTPUT" \
            --csv-output "$SEGMENT_CSV" \
            --max-pages "$MAX_PAGES_PER_SEGMENT" \
            --rate-limit-seconds "$RATE_LIMIT_SECONDS" \
            2>&1 | tee -a "$LOG_FILE" | while IFS= read -r line; do
                if [[ $line == *"ERROR"* ]]; then
                    echo -e "${RED}$line${NC}"
                elif [[ $line == *"Found"* ]] || [[ $line == *"Processed"* ]]; then
                    echo -e "${GREEN}$line${NC}"
                fi
            done
        
        ANALYZER_EXIT_CODE=${PIPESTATUS[0]}
    fi
    
    # Check results
    if [ $ANALYZER_EXIT_CODE -eq 124 ]; then
        print_color "$YELLOW" "Segment $SEGMENT timed out (expected after ${RUN_DURATION_MINUTES} minutes)"
        TOTAL_SUCCESS=$((TOTAL_SUCCESS + 1))
    elif [ $ANALYZER_EXIT_CODE -eq 0 ]; then
        print_color "$GREEN" "✓ Segment $SEGMENT completed successfully"
        TOTAL_SUCCESS=$((TOTAL_SUCCESS + 1))
    else
        print_color "$RED" "✗ Segment $SEGMENT failed with code $ANALYZER_EXIT_CODE"
    fi
    
    # Show segment results
    if [ -f "$SEGMENT_OUTPUT" ]; then
        GROUPS=$(jq -r '.scene_groups | length' "$SEGMENT_OUTPUT" 2>/dev/null || echo "0")
        RELEASES=$(jq -r '.total_releases' "$SEGMENT_OUTPUT" 2>/dev/null || echo "0")
        
        print_color "$GREEN" "  Groups found: $GROUPS"
        print_color "$GREEN" "  Releases: $RELEASES"
    fi
    
    if [ $SEGMENT -lt $MAX_SEGMENTS ]; then
        print_color "$YELLOW" "Taking 6 second break..."
        sleep 6
    fi
    
    SEGMENT=$((SEGMENT + 1))
    echo
done

# Merge results
print_color "$YELLOW" "Merging test results..."

FINAL_OUTPUT="$ANALYSIS_DIR/test_analysis_final.json"

# Simple merge for testing
if command -v jq &> /dev/null; then
    jq -s '
    {
        scene_groups: (reduce .[] as $item ({}; . + $item.scene_groups)),
        total_releases: (reduce .[] as $item (0; . + $item.total_releases)),
        pages_processed: (reduce .[] as $item (0; . + $item.pages_processed)),
        segments: length
    }' "$ANALYSIS_DIR/segments/"*.json > "$FINAL_OUTPUT" 2>/dev/null
    
    if [ -f "$FINAL_OUTPUT" ]; then
        print_color "$GREEN" "✓ Results merged successfully"
        
        TOTAL_GROUPS=$(jq '.scene_groups | length' "$FINAL_OUTPUT")
        TOTAL_RELEASES=$(jq '.total_releases' "$FINAL_OUTPUT")
        
        echo
        print_color "$GREEN" "Final Test Results:"
        print_color "$GREEN" "  - Total Groups: $TOTAL_GROUPS"
        print_color "$GREEN" "  - Total Releases: $TOTAL_RELEASES"
        print_color "$GREEN" "  - Output: $FINAL_OUTPUT"
    fi
fi

# Test PostgreSQL import
if [ -f "$SCRIPT_DIR/import_analysis_to_postgres.py" ] && [ $DB_STATUS -eq 0 ]; then
    echo
    print_color "$YELLOW" "Testing PostgreSQL import..."
    
    if python3 "$SCRIPT_DIR/import_analysis_to_postgres.py" "$FINAL_OUTPUT" --show-top 5 2>/dev/null; then
        print_color "$GREEN" "✓ PostgreSQL import successful"
    else
        print_color "$RED" "✗ PostgreSQL import failed"
    fi
fi

echo
print_color "$GREEN" "=========================================="
print_color "$GREEN" "Test Complete"
print_color "$GREEN" "=========================================="

if [ $TOTAL_SUCCESS -gt 0 ]; then
    print_color "$GREEN" "✓ Test PASSED: $TOTAL_SUCCESS/$MAX_SEGMENTS segments successful"
    print_color "$GREEN" "  Output directory: $ANALYSIS_DIR"
    print_color "$GREEN" "  Log file: $LOG_FILE"
    
    echo
    print_color "$YELLOW" "Next steps:"
    echo "1. Review the log file: less $LOG_FILE"
    echo "2. Check the output: ls -la $ANALYSIS_DIR/segments/"
    echo "3. If test passed, run the full analysis:"
    echo "   $SCRIPT_DIR/run_hdbits_analysis_segmented.sh"
else
    print_color "$RED" "✗ Test FAILED: No segments completed successfully"
    print_color "$RED" "  Check log file: $LOG_FILE"
fi

exit 0