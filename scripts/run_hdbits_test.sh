#!/bin/bash

# Simple HDBits analysis test runner
# Uses local directories for all output

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Load configuration
source "$SCRIPT_DIR/load_config.sh" hdbits

# Use local directories
ANALYSIS_DIR="$PROJECT_ROOT/hdbits_analysis"
LOG_FILE="$ANALYSIS_DIR/analysis_$(date +%Y%m%d_%H%M%S).log"

# Create directories
mkdir -p "$ANALYSIS_DIR"

# Simple logging
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

log "=========================================="
log "HDBits Analysis Test"
log "=========================================="
log "Output directory: $ANALYSIS_DIR"
log "Session cookie length: ${#HDBITS_SESSION_COOKIE}"
log ""

# Build the analyzer
log "Building analyzer..."
cd "$PROJECT_ROOT"
if cargo build -p radarr-analysis --bin hdbits-comprehensive-analyzer --release 2>&1 | tee -a "$LOG_FILE"; then
    log "Build successful"
else
    log "Build failed"
    exit 1
fi

# Run a quick test - just 2 pages to verify it works
log ""
log "Running analysis for 2 pages to test authentication..."
log ""

OUTPUT_FILE="$ANALYSIS_DIR/test_$(date +%Y%m%d_%H%M%S).json"

RUST_LOG=info cargo run -p radarr-analysis --bin hdbits-comprehensive-analyzer --release -- \
    --session-cookie "$HDBITS_SESSION_COOKIE" \
    --output "$OUTPUT_FILE" \
    --max-pages 2 \
    --delay 3 \
    --verbose \
    2>&1 | tee -a "$LOG_FILE"

EXIT_CODE=${PIPESTATUS[0]}

if [ $EXIT_CODE -eq 0 ]; then
    log ""
    log "=========================================="
    log "Test completed successfully!"
    log "=========================================="
    
    if [ -f "$OUTPUT_FILE" ]; then
        GROUPS=$(jq '.scene_groups | length' "$OUTPUT_FILE" 2>/dev/null || echo "0")
        RELEASES=$(jq '.total_releases' "$OUTPUT_FILE" 2>/dev/null || echo "0")
        
        log "Results:"
        log "  - Scene groups found: $GROUPS"
        log "  - Total releases: $RELEASES"
        log "  - Output file: $OUTPUT_FILE"
    fi
else
    log ""
    log "=========================================="
    log "Test failed with exit code: $EXIT_CODE"
    log "=========================================="
    log ""
    log "Common issues:"
    log "1. Session cookie expired - get a fresh one from browser"
    log "2. Rate limited - wait a few minutes"
    log "3. Network issues - check your connection"
fi

log ""
log "Full log saved to: $LOG_FILE"
exit $EXIT_CODE