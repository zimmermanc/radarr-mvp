#!/bin/bash

# HDBits Scene Group Analysis Automation Script
# Runs comprehensive analysis and archives results
# Should be run weekly via cron

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ANALYSIS_DIR="/opt/radarr/analysis"
ARCHIVE_DIR="$ANALYSIS_DIR/archive"
LOG_DIR="/var/log/radarr"
LOG_FILE="$LOG_DIR/hdbits_analysis_$(date +%Y%m%d_%H%M%S).log"

# Create directories if they don't exist
mkdir -p "$ANALYSIS_DIR"
mkdir -p "$ARCHIVE_DIR"
mkdir -p "$LOG_DIR"

# Function to log messages
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Function to send notification (can be customized)
notify() {
    local status=$1
    local message=$2
    
    # Log the notification
    log "NOTIFICATION: [$status] $message"
    
    # TODO: Add webhook/email notification here if needed
    # Example: curl -X POST "https://discord.webhook.url" -d "{\"content\": \"$message\"}"
}

# Check if session cookie is configured
if [ -z "$HDBITS_SESSION_COOKIE" ]; then
    log "ERROR: HDBITS_SESSION_COOKIE environment variable not set"
    log "Please export HDBITS_SESSION_COOKIE with a valid session cookie"
    notify "ERROR" "HDBits analysis failed: Missing session cookie"
    exit 1
fi

# Start analysis
log "Starting HDBits scene group analysis"
log "Output directory: $ANALYSIS_DIR"

# Change to project directory
cd "$PROJECT_ROOT"

# Build the analyzer if needed
log "Building HDBits comprehensive analyzer..."
if cargo build -p radarr-analysis --bin hdbits-comprehensive-analyzer --release >> "$LOG_FILE" 2>&1; then
    log "Build successful"
else
    log "ERROR: Build failed"
    notify "ERROR" "HDBits analyzer build failed"
    exit 1
fi

# Run the comprehensive analyzer
log "Running comprehensive scene group analysis..."
OUTPUT_FILE="$ANALYSIS_DIR/hdbits_analysis_$(date +%Y%m%d).json"
CSV_FILE="$ANALYSIS_DIR/scene_groups_$(date +%Y%m%d).csv"

if RUST_LOG=info cargo run -p radarr-analysis --bin hdbits-comprehensive-analyzer --release -- \
    --session-cookie "$HDBITS_SESSION_COOKIE" \
    --output "$OUTPUT_FILE" \
    --csv-output "$CSV_FILE" \
    --max-pages 100 \
    --rate-limit-seconds 3 >> "$LOG_FILE" 2>&1; then
    
    log "Analysis completed successfully"
    log "JSON output: $OUTPUT_FILE"
    log "CSV output: $CSV_FILE"
    
    # Archive old results (keep last 4 weeks)
    log "Archiving old results..."
    find "$ANALYSIS_DIR" -name "hdbits_analysis_*.json" -mtime +28 -exec mv {} "$ARCHIVE_DIR/" \;
    find "$ANALYSIS_DIR" -name "scene_groups_*.csv" -mtime +28 -exec mv {} "$ARCHIVE_DIR/" \;
    
    # Generate summary statistics
    if [ -f "$OUTPUT_FILE" ]; then
        TOTAL_GROUPS=$(jq '.scene_groups | length' "$OUTPUT_FILE")
        TOTAL_RELEASES=$(jq '.total_releases' "$OUTPUT_FILE")
        log "Analysis Summary:"
        log "  - Total scene groups analyzed: $TOTAL_GROUPS"
        log "  - Total releases processed: $TOTAL_RELEASES"
        
        # Copy latest analysis to standard location for application use
        cp "$OUTPUT_FILE" "$ANALYSIS_DIR/latest_analysis.json"
        cp "$CSV_FILE" "$ANALYSIS_DIR/latest_scene_groups.csv"
        
        notify "SUCCESS" "HDBits analysis complete: $TOTAL_GROUPS groups, $TOTAL_RELEASES releases"
    fi
else
    log "ERROR: Analysis failed"
    notify "ERROR" "HDBits scene group analysis failed - check logs at $LOG_FILE"
    exit 1
fi

# Cleanup old logs (keep 30 days)
log "Cleaning up old logs..."
find "$LOG_DIR" -name "hdbits_analysis_*.log" -mtime +30 -delete

# Update quality scoring database (if integration exists)
if [ -x "$SCRIPT_DIR/update_quality_scores.sh" ]; then
    log "Updating quality scoring database..."
    "$SCRIPT_DIR/update_quality_scores.sh" "$OUTPUT_FILE"
fi

log "HDBits analysis automation completed successfully"
exit 0