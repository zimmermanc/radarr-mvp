#!/bin/bash

# HDBits Scene Group Analysis - Segmented Run Script
# Runs for 15 minutes, then takes 15 minute breaks
# Respects rate limiting and provides better resource management

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Load centralized configuration
source "$SCRIPT_DIR/load_config.sh" hdbits

# Use configured values or defaults
# Use /tmp for testing since /opt/radarr is not writable
ANALYSIS_DIR="${RADARR_ANALYSIS_DIR:-/tmp/radarr/analysis}"
LOG_DIR="${RADARR_LOGS:-/tmp/radarr/logs}"
ARCHIVE_DIR="$ANALYSIS_DIR/archive"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$LOG_DIR/hdbits_analysis_segmented_${TIMESTAMP}.log"

# Timing configuration from services.env or defaults
RUN_DURATION_MINUTES=${HDBITS_ANALYSIS_RUN_MINUTES:-15}
BREAK_DURATION_MINUTES=${HDBITS_ANALYSIS_BREAK_MINUTES:-15}
MAX_SEGMENTS=${HDBITS_ANALYSIS_MAX_SEGMENTS:-8}
RATE_LIMIT_SECONDS=${HDBITS_RATE_LIMIT_SECONDS:-5}

# Create directories
mkdir -p "$ANALYSIS_DIR"
mkdir -p "$ARCHIVE_DIR"
mkdir -p "$LOG_DIR"
mkdir -p "$ANALYSIS_DIR/segments"

# Function to log messages
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

# Function to send notification
notify() {
    local status=$1
    local message=$2
    
    log "NOTIFICATION: [$status] $message"
    
    # TODO: Add webhook/email notification here if needed
    # Example: curl -X POST "https://discord.webhook.url" -d "{\"content\": \"$message\"}"
}

# Function to calculate pages per segment
calculate_pages_per_segment() {
    # Assuming 5 seconds per page (rate limit), calculate how many pages fit in 15 minutes
    local seconds_per_run=$((RUN_DURATION_MINUTES * 60))
    local pages_per_segment=$((seconds_per_run / RATE_LIMIT_SECONDS))
    echo $pages_per_segment
}

# Validate HDBits configuration
if ! is_service_configured hdbits; then
    log "ERROR: HDBits is not configured properly"
    log "Please set HDBITS_SESSION_COOKIE in your configuration file:"
    log "  $RADARR_CONFIG_FILE"
    notify "ERROR" "HDBits analysis failed: Missing session cookie"
    exit 1
fi

# Start analysis
log "=========================================="
log "Starting Segmented HDBits Analysis"
log "=========================================="
log "Run Duration: ${RUN_DURATION_MINUTES} minutes"
log "Break Duration: ${BREAK_DURATION_MINUTES} minutes"
log "Max Segments: ${MAX_SEGMENTS}"
log "Rate Limit: ${RATE_LIMIT_SECONDS} seconds between requests"
log "Output Directory: $ANALYSIS_DIR"
log "=========================================="

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

# Calculate pages per segment
PAGES_PER_SEGMENT=$(calculate_pages_per_segment)
log "Pages per segment: $PAGES_PER_SEGMENT (based on ${RUN_DURATION_MINUTES} minute runtime)"

# Main segmented run loop
TOTAL_PAGES_PROCESSED=0
SEGMENT=1

while [ $SEGMENT -le $MAX_SEGMENTS ]; do
    log ""
    log "=========================================="
    log "SEGMENT $SEGMENT of $MAX_SEGMENTS"
    log "=========================================="
    
    # Calculate page range for this segment
    START_PAGE=$TOTAL_PAGES_PROCESSED
    END_PAGE=$((START_PAGE + PAGES_PER_SEGMENT))
    
    # Output files for this segment
    SEGMENT_OUTPUT="$ANALYSIS_DIR/segments/segment_${SEGMENT}_$(date +%Y%m%d_%H%M%S).json"
    SEGMENT_CSV="$ANALYSIS_DIR/segments/segment_${SEGMENT}_groups_$(date +%Y%m%d_%H%M%S).csv"
    
    log "Starting analysis segment $SEGMENT"
    log "Processing pages $START_PAGE to $END_PAGE"
    log "Segment will run for ${RUN_DURATION_MINUTES} minutes"
    
    # Create a timeout wrapper for the segment
    SEGMENT_START=$(date +%s)
    SEGMENT_MAX_DURATION=$((RUN_DURATION_MINUTES * 60))
    
    # Run the analyzer with timeout
    timeout ${SEGMENT_MAX_DURATION}s \
        RUST_LOG=info cargo run -p radarr-analysis --bin hdbits-comprehensive-analyzer --release -- \
        --session-cookie "$HDBITS_SESSION_COOKIE" \
        --output "$SEGMENT_OUTPUT" \
        --csv-output "$SEGMENT_CSV" \
        --max-pages "$PAGES_PER_SEGMENT" \
        --rate-limit-seconds "$RATE_LIMIT_SECONDS" \
        2>&1 | while IFS= read -r line; do
            echo "[SEGMENT $SEGMENT] $line" >> "$LOG_FILE"
            
            # Check if we should show progress
            if [[ $line == *"Processed"* ]] || [[ $line == *"Found"* ]]; then
                echo "[SEGMENT $SEGMENT] $line"
            fi
        done
    
    ANALYZER_EXIT_CODE=${PIPESTATUS[0]}
    
    # Check if the analyzer completed or timed out
    if [ $ANALYZER_EXIT_CODE -eq 124 ]; then
        log "Segment $SEGMENT completed (timeout reached after ${RUN_DURATION_MINUTES} minutes)"
    elif [ $ANALYZER_EXIT_CODE -eq 0 ]; then
        log "Segment $SEGMENT completed successfully"
    else
        log "WARNING: Segment $SEGMENT exited with code $ANALYZER_EXIT_CODE"
    fi
    
    # Update total pages processed
    if [ -f "$SEGMENT_OUTPUT" ]; then
        SEGMENT_PAGES=$(jq -r '.pages_processed // 0' "$SEGMENT_OUTPUT" 2>/dev/null || echo "0")
        TOTAL_PAGES_PROCESSED=$((TOTAL_PAGES_PROCESSED + SEGMENT_PAGES))
        
        SEGMENT_GROUPS=$(jq -r '.scene_groups | length // 0' "$SEGMENT_OUTPUT" 2>/dev/null || echo "0")
        SEGMENT_RELEASES=$(jq -r '.total_releases // 0' "$SEGMENT_OUTPUT" 2>/dev/null || echo "0")
        
        log "Segment $SEGMENT Results:"
        log "  - Pages processed: $SEGMENT_PAGES"
        log "  - Scene groups found: $SEGMENT_GROUPS"
        log "  - Releases analyzed: $SEGMENT_RELEASES"
        log "  - Total pages so far: $TOTAL_PAGES_PROCESSED"
    fi
    
    # Check if this is the last segment
    if [ $SEGMENT -eq $MAX_SEGMENTS ]; then
        log "Reached maximum segments ($MAX_SEGMENTS). Stopping analysis."
        break
    fi
    
    # Take a break between segments
    log ""
    log "Taking ${BREAK_DURATION_MINUTES} minute break before next segment..."
    log "Next segment will start at $(date -d "+${BREAK_DURATION_MINUTES} minutes" '+%Y-%m-%d %H:%M:%S')"
    
    # Show countdown during break
    for ((i=BREAK_DURATION_MINUTES; i>0; i--)); do
        if [ $((i % 5)) -eq 0 ] || [ $i -le 5 ]; then
            log "Break time remaining: $i minutes"
        fi
        sleep 60
    done
    
    SEGMENT=$((SEGMENT + 1))
done

log ""
log "=========================================="
log "Merging Segment Results"
log "=========================================="

# Merge all segment results into final output
FINAL_OUTPUT="$ANALYSIS_DIR/hdbits_analysis_$(date +%Y%m%d).json"
FINAL_CSV="$ANALYSIS_DIR/scene_groups_$(date +%Y%m%d).csv"

log "Creating merged analysis report..."

# Create a Python script to merge JSON results
cat > "$ANALYSIS_DIR/merge_segments.py" << 'EOF'
#!/usr/bin/env python3
import json
import glob
import sys
from datetime import datetime

def merge_segments(segment_dir, output_file):
    segments = glob.glob(f"{segment_dir}/segment_*.json")
    segments.sort()
    
    merged = {
        "scene_groups": {},
        "total_releases": 0,
        "internal_releases": 0,
        "pages_processed": 0,
        "analysis_date": datetime.now().isoformat(),
        "segments_processed": len(segments)
    }
    
    for segment_file in segments:
        try:
            with open(segment_file, 'r') as f:
                data = json.load(f)
                
            # Merge scene groups
            if "scene_groups" in data:
                for group, metrics in data["scene_groups"].items():
                    if group not in merged["scene_groups"]:
                        merged["scene_groups"][group] = metrics
                    else:
                        # Merge metrics
                        existing = merged["scene_groups"][group]
                        existing["release_count"] += metrics.get("release_count", 0)
                        existing["total_size_gb"] += metrics.get("total_size_gb", 0)
                        # Update reputation score (average)
                        if "reputation_score" in metrics:
                            existing["reputation_score"] = (
                                existing.get("reputation_score", 0) + metrics["reputation_score"]
                            ) / 2
            
            # Sum totals
            merged["total_releases"] += data.get("total_releases", 0)
            merged["internal_releases"] += data.get("internal_releases", 0)
            merged["pages_processed"] += data.get("pages_processed", 0)
            
        except Exception as e:
            print(f"Error processing {segment_file}: {e}", file=sys.stderr)
    
    # Write merged results
    with open(output_file, 'w') as f:
        json.dump(merged, f, indent=2)
    
    print(f"Merged {len(segments)} segments into {output_file}")
    print(f"Total scene groups: {len(merged['scene_groups'])}")
    print(f"Total releases: {merged['total_releases']}")
    print(f"Total pages: {merged['pages_processed']}")

if __name__ == "__main__":
    merge_segments("/tmp/radarr/analysis/segments", "/tmp/radarr/analysis/merged_analysis.json")
EOF

# Run the merge script
if command -v python3 &> /dev/null; then
    python3 "$ANALYSIS_DIR/merge_segments.py"
    
    if [ -f "$ANALYSIS_DIR/merged_analysis.json" ]; then
        mv "$ANALYSIS_DIR/merged_analysis.json" "$FINAL_OUTPUT"
        log "Merged analysis saved to: $FINAL_OUTPUT"
        
        # Copy to standard location
        cp "$FINAL_OUTPUT" "$ANALYSIS_DIR/latest_analysis.json"
    fi
else
    log "WARNING: Python3 not available for merging. Segment files remain separate."
    log "Segment files are in: $ANALYSIS_DIR/segments/"
fi

# Generate summary
log ""
log "=========================================="
log "Analysis Complete"
log "=========================================="
log "Total Segments Processed: $((SEGMENT - 1))"
log "Total Runtime: $(( (SEGMENT - 1) * (RUN_DURATION_MINUTES + BREAK_DURATION_MINUTES) )) minutes"
log "Total Pages Processed: $TOTAL_PAGES_PROCESSED"

if [ -f "$FINAL_OUTPUT" ]; then
    TOTAL_GROUPS=$(jq '.scene_groups | length' "$FINAL_OUTPUT" 2>/dev/null || echo "0")
    TOTAL_RELEASES=$(jq '.total_releases' "$FINAL_OUTPUT" 2>/dev/null || echo "0")
    
    log "Final Results:"
    log "  - Total Scene Groups: $TOTAL_GROUPS"
    log "  - Total Releases: $TOTAL_RELEASES"
    log "  - Output: $FINAL_OUTPUT"
    
    # Import to PostgreSQL for long-term storage
    log ""
    log "=========================================="
    log "Importing to PostgreSQL"
    log "=========================================="
    
    if [ -f "$SCRIPT_DIR/import_analysis_to_postgres.py" ]; then
        if command -v python3 &> /dev/null; then
            log "Importing analysis results to PostgreSQL..."
            
            # Set database URL if not already set
            if [ -z "$DATABASE_URL" ]; then
                export DATABASE_URL="postgresql://radarr:radarr@localhost:5432/radarr"
            fi
            
            if python3 "$SCRIPT_DIR/import_analysis_to_postgres.py" "$FINAL_OUTPUT" --show-top 20 >> "$LOG_FILE" 2>&1; then
                log "Successfully imported analysis to PostgreSQL"
                log "Top scene groups stored in database for quality scoring"
                
                # Show top groups in log
                python3 "$SCRIPT_DIR/import_analysis_to_postgres.py" "$FINAL_OUTPUT" --show-top 10 2>/dev/null | tail -12
            else
                log "WARNING: Failed to import to PostgreSQL - data saved in JSON/CSV only"
            fi
        else
            log "WARNING: Python3 not available - skipping PostgreSQL import"
        fi
    else
        log "WARNING: PostgreSQL import script not found - data saved in JSON/CSV only"
    fi
    
    notify "SUCCESS" "HDBits segmented analysis complete: $TOTAL_GROUPS groups, $TOTAL_RELEASES releases in $((SEGMENT - 1)) segments"
else
    log "Individual segment results in: $ANALYSIS_DIR/segments/"
    notify "SUCCESS" "HDBits segmented analysis complete: $((SEGMENT - 1)) segments processed"
fi

# Archive old segments (keep last 4 runs)
log "Archiving old segment files..."
find "$ANALYSIS_DIR/segments" -name "segment_*.json" -mtime +7 -exec mv {} "$ARCHIVE_DIR/" \; 2>/dev/null || true

# Cleanup old logs
find "$LOG_DIR" -name "hdbits_analysis_segmented_*.log" -mtime +30 -delete 2>/dev/null || true

log "Segmented analysis completed successfully"
exit 0