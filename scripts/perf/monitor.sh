#!/bin/bash

# Real-time Performance Monitoring Script for Radarr MVP
# Monitors system resources, application metrics, and database performance during load testing

set -e

# Configuration
INTERVAL="${INTERVAL:-5}"
DURATION="${DURATION:-300}"
OUTPUT_DIR="${OUTPUT_DIR:-scripts/perf/results/monitoring}"
RADARR_PID=""
POSTGRES_USER="${POSTGRES_USER:-radarr}"
POSTGRES_DB="${POSTGRES_DB:-radarr}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}📊 Radarr MVP Performance Monitor${NC}"
echo -e "${BLUE}=================================${NC}"
echo -e "${YELLOW}⏱️  Monitoring interval: ${INTERVAL}s${NC}"
echo -e "${YELLOW}⏰ Duration: ${DURATION}s${NC}"
echo -e "${YELLOW}📁 Output directory: $OUTPUT_DIR${NC}"
echo

# Find Radarr process
RADARR_PID=$(pgrep radarr-mvp || echo "")
if [[ -z "$RADARR_PID" ]]; then
    echo -e "${RED}❌ Radarr process not found${NC}"
    echo -e "${YELLOW}💡 Start the application: cargo run${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Found Radarr process (PID: $RADARR_PID)${NC}"

# Initialize log files
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
SYSTEM_LOG="$OUTPUT_DIR/system_metrics_${TIMESTAMP}.csv"
RADARR_LOG="$OUTPUT_DIR/radarr_metrics_${TIMESTAMP}.csv"
POSTGRES_LOG="$OUTPUT_DIR/postgres_metrics_${TIMESTAMP}.csv"
NETWORK_LOG="$OUTPUT_DIR/network_metrics_${TIMESTAMP}.csv"
SUMMARY_LOG="$OUTPUT_DIR/monitoring_summary_${TIMESTAMP}.log"

# CSV Headers
echo "timestamp,cpu_percent,memory_used_mb,memory_percent,swap_used_mb,load_avg_1m,load_avg_5m,load_avg_15m" > "$SYSTEM_LOG"
echo "timestamp,pid,cpu_percent,memory_mb,memory_percent,vsz_mb,rss_mb,threads,open_files" > "$RADARR_LOG"
echo "timestamp,total_connections,active_connections,idle_connections,total_queries,active_queries,slow_queries,db_size_mb,cache_hit_ratio" > "$POSTGRES_LOG"
echo "timestamp,rx_bytes,tx_bytes,rx_packets,tx_packets,rx_errors,tx_errors,connections_established" > "$NETWORK_LOG"

# Function to get system metrics
get_system_metrics() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # CPU usage
    local cpu_percent=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1 | tr -d ' ')
    
    # Memory usage
    local memory_info=$(free -m | grep '^Mem:')
    local memory_used=$(echo $memory_info | awk '{print $3}')
    local memory_total=$(echo $memory_info | awk '{print $2}')
    local memory_percent=$(awk "BEGIN {printf \"%.1f\", ($memory_used/$memory_total)*100}")
    
    # Swap usage
    local swap_info=$(free -m | grep '^Swap:')
    local swap_used=$(echo $swap_info | awk '{print $3}')
    
    # Load averages
    local load_avg=$(uptime | awk -F'load average:' '{print $2}' | tr -d ' ')
    local load_1m=$(echo $load_avg | cut -d',' -f1)
    local load_5m=$(echo $load_avg | cut -d',' -f2)
    local load_15m=$(echo $load_avg | cut -d',' -f3)
    
    echo "$timestamp,$cpu_percent,$memory_used,$memory_percent,$swap_used,$load_1m,$load_5m,$load_15m" >> "$SYSTEM_LOG"
}

# Function to get Radarr-specific metrics
get_radarr_metrics() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    if [[ -n "$RADARR_PID" ]] && kill -0 "$RADARR_PID" 2>/dev/null; then
        # Process stats
        local ps_info=$(ps -p "$RADARR_PID" -o pid,pcpu,pmem,vsz,rss,nlwp --no-headers 2>/dev/null || echo "")
        
        if [[ -n "$ps_info" ]]; then
            local pid=$(echo $ps_info | awk '{print $1}')
            local cpu_percent=$(echo $ps_info | awk '{print $2}')
            local memory_percent=$(echo $ps_info | awk '{print $3}')
            local vsz_kb=$(echo $ps_info | awk '{print $4}')
            local rss_kb=$(echo $ps_info | awk '{print $5}')
            local threads=$(echo $ps_info | awk '{print $6}')
            
            # Convert to MB
            local vsz_mb=$(awk "BEGIN {printf \"%.1f\", $vsz_kb/1024}")
            local rss_mb=$(awk "BEGIN {printf \"%.1f\", $rss_kb/1024}")
            local memory_mb="$rss_mb"
            
            # Open file descriptors
            local open_files=$(ls /proc/$RADARR_PID/fd 2>/dev/null | wc -l || echo "0")
            
            echo "$timestamp,$pid,$cpu_percent,$memory_mb,$memory_percent,$vsz_mb,$rss_mb,$threads,$open_files" >> "$RADARR_LOG"
        else
            echo "$timestamp,DEAD,0,0,0,0,0,0,0" >> "$RADARR_LOG"
        fi
    else
        echo "$timestamp,NOT_FOUND,0,0,0,0,0,0,0" >> "$RADARR_LOG"
    fi
}

# Function to get PostgreSQL metrics
get_postgres_metrics() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Default values
    local total_connections="0"
    local active_connections="0"
    local idle_connections="0"
    local total_queries="0"
    local active_queries="0"
    local slow_queries="0"
    local db_size_mb="0"
    local cache_hit_ratio="0"
    
    # Try to connect to PostgreSQL
    if command -v psql >/dev/null 2>&1; then
        # Connection stats
        local conn_stats=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
            SELECT 
                count(*) as total,
                count(*) FILTER (WHERE state = 'active') as active,
                count(*) FILTER (WHERE state = 'idle') as idle
            FROM pg_stat_activity;
        " 2>/dev/null | xargs || echo "0 0 0")
        
        if [[ "$conn_stats" != "0 0 0" ]]; then
            total_connections=$(echo $conn_stats | awk '{print $1}')
            active_connections=$(echo $conn_stats | awk '{print $2}')
            idle_connections=$(echo $conn_stats | awk '{print $3}')
        fi
        
        # Query stats (if pg_stat_statements is available)
        local query_stats=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
            SELECT 
                sum(calls) as total_queries,
                count(*) FILTER (WHERE mean_time > 1000) as slow_queries
            FROM pg_stat_statements;
        " 2>/dev/null | xargs || echo "0 0")
        
        if [[ "$query_stats" != "0 0" ]]; then
            total_queries=$(echo $query_stats | awk '{print $1}')
            slow_queries=$(echo $query_stats | awk '{print $2}')
        fi
        
        # Database size
        db_size_mb=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
            SELECT ROUND(pg_database_size('$POSTGRES_DB')/1024/1024, 1);
        " 2>/dev/null | xargs || echo "0")
        
        # Cache hit ratio
        cache_hit_ratio=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
            SELECT ROUND(
                sum(blks_hit)*100.0 / NULLIF(sum(blks_hit + blks_read), 0), 2
            ) FROM pg_stat_database WHERE datname = '$POSTGRES_DB';
        " 2>/dev/null | xargs || echo "0")
        
        # Currently active queries
        active_queries=$(psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -t -c "
            SELECT count(*) FROM pg_stat_activity WHERE state = 'active' AND query NOT LIKE '%pg_stat_activity%';
        " 2>/dev/null | xargs || echo "0")
    fi
    
    echo "$timestamp,$total_connections,$active_connections,$idle_connections,$total_queries,$active_queries,$slow_queries,$db_size_mb,$cache_hit_ratio" >> "$POSTGRES_LOG"
}

# Function to get network metrics
get_network_metrics() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Network interface stats (sum all interfaces)
    local net_stats=$(awk '
        NR > 2 {
            rx_bytes += $2; tx_bytes += $10
            rx_packets += $3; tx_packets += $11  
            rx_errors += $4; tx_errors += $12
        }
        END {
            printf "%d %d %d %d %d %d", rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors
        }
    ' /proc/net/dev)
    
    # TCP connection stats
    local connections_established=$(netstat -an 2>/dev/null | grep -c "ESTABLISHED" || echo "0")
    
    echo "$timestamp,$net_stats,$connections_established" >> "$NETWORK_LOG"
}

# Function to display real-time stats
display_realtime_stats() {
    clear
    echo -e "${BLUE}📊 Radarr MVP Real-time Performance Monitor${NC}"
    echo -e "${BLUE}===========================================${NC}"
    echo -e "${YELLOW}⏰ $(date)${NC}"
    echo
    
    # Latest system stats
    if [[ -f "$SYSTEM_LOG" ]]; then
        local latest_system=$(tail -n1 "$SYSTEM_LOG")
        local cpu=$(echo "$latest_system" | cut -d',' -f2)
        local mem_mb=$(echo "$latest_system" | cut -d',' -f3)
        local mem_pct=$(echo "$latest_system" | cut -d',' -f4)
        local load_1m=$(echo "$latest_system" | cut -d',' -f6)
        
        echo -e "${GREEN}🖥️  System Resources:${NC}"
        echo -e "   CPU Usage: ${cpu}%"
        echo -e "   Memory: ${mem_mb} MB (${mem_pct}%)"
        echo -e "   Load Average (1m): ${load_1m}"
        echo
    fi
    
    # Latest Radarr stats
    if [[ -f "$RADARR_LOG" ]]; then
        local latest_radarr=$(tail -n1 "$RADARR_LOG")
        local radarr_cpu=$(echo "$latest_radarr" | cut -d',' -f3)
        local radarr_mem=$(echo "$latest_radarr" | cut -d',' -f4)
        local radarr_threads=$(echo "$latest_radarr" | cut -d',' -f8)
        local radarr_files=$(echo "$latest_radarr" | cut -d',' -f9)
        
        echo -e "${GREEN}🎯 Radarr Application:${NC}"
        echo -e "   PID: $RADARR_PID"
        echo -e "   CPU Usage: ${radarr_cpu}%"
        echo -e "   Memory: ${radarr_mem} MB"
        echo -e "   Threads: ${radarr_threads}"
        echo -e "   Open Files: ${radarr_files}"
        echo
    fi
    
    # Latest PostgreSQL stats
    if [[ -f "$POSTGRES_LOG" ]]; then
        local latest_postgres=$(tail -n1 "$POSTGRES_LOG")
        local pg_total_conn=$(echo "$latest_postgres" | cut -d',' -f2)
        local pg_active_conn=$(echo "$latest_postgres" | cut -d',' -f3)
        local pg_active_queries=$(echo "$latest_postgres" | cut -d',' -f6)
        local pg_db_size=$(echo "$latest_postgres" | cut -d',' -f8)
        local pg_cache_hit=$(echo "$latest_postgres" | cut -d',' -f9)
        
        echo -e "${GREEN}🗄️  PostgreSQL Database:${NC}"
        echo -e "   Total Connections: ${pg_total_conn}"
        echo -e "   Active Connections: ${pg_active_conn}"
        echo -e "   Active Queries: ${pg_active_queries}"
        echo -e "   Database Size: ${pg_db_size} MB"
        echo -e "   Cache Hit Ratio: ${pg_cache_hit}%"
        echo
    fi
    
    # Latest network stats
    if [[ -f "$NETWORK_LOG" ]]; then
        local latest_network=$(tail -n1 "$NETWORK_LOG")
        local net_rx_mb=$(echo "$latest_network" | cut -d',' -f2 | awk '{printf "%.2f", $1/1024/1024}')
        local net_tx_mb=$(echo "$latest_network" | cut -d',' -f3 | awk '{printf "%.2f", $1/1024/1024}')
        local net_established=$(echo "$latest_network" | cut -d',' -f8)
        
        echo -e "${GREEN}🌐 Network Activity:${NC}"
        echo -e "   RX: ${net_rx_mb} MB"
        echo -e "   TX: ${net_tx_mb} MB"
        echo -e "   Established Connections: ${net_established}"
        echo
    fi
    
    echo -e "${YELLOW}📁 Log Files:${NC}"
    echo -e "   System: $SYSTEM_LOG"
    echo -e "   Radarr: $RADARR_LOG"
    echo -e "   PostgreSQL: $POSTGRES_LOG"
    echo -e "   Network: $NETWORK_LOG"
    echo
    echo -e "${BLUE}Press Ctrl+C to stop monitoring${NC}"
}

# Function to generate summary report
generate_summary() {
    echo -e "${YELLOW}📋 Generating monitoring summary...${NC}"
    
    cat > "$SUMMARY_LOG" << EOF
# Radarr MVP Performance Monitoring Summary

**Monitoring Period:** $(head -n2 "$SYSTEM_LOG" | tail -n1 | cut -d',' -f1) to $(tail -n1 "$SYSTEM_LOG" | cut -d',' -f1)
**Duration:** ${DURATION}s
**Interval:** ${INTERVAL}s
**Total Samples:** $(( $(wc -l < "$SYSTEM_LOG") - 1 ))

## System Resource Summary

EOF
    
    # Analyze system metrics
    if [[ -f "$SYSTEM_LOG" ]] && [[ $(wc -l < "$SYSTEM_LOG") -gt 1 ]]; then
        awk -F',' 'NR>1 {
            cpu_sum += $2; cpu_max = ($2 > cpu_max) ? $2 : cpu_max
            mem_sum += $4; mem_max = ($4 > mem_max) ? $4 : mem_max
            load_sum += $6; load_max = ($6 > load_max) ? $6 : load_max
            count++
        } END {
            printf "Average CPU Usage: %.1f%% (Peak: %.1f%%)\n", cpu_sum/count, cpu_max
            printf "Average Memory Usage: %.1f%% (Peak: %.1f%%)\n", mem_sum/count, mem_max
            printf "Average Load (1m): %.2f (Peak: %.2f)\n", load_sum/count, load_max
        }' "$SYSTEM_LOG" >> "$SUMMARY_LOG"
    fi
    
    cat >> "$SUMMARY_LOG" << EOF

## Radarr Application Summary

EOF
    
    # Analyze Radarr metrics
    if [[ -f "$RADARR_LOG" ]] && [[ $(wc -l < "$RADARR_LOG") -gt 1 ]]; then
        awk -F',' 'NR>1 && $2!="DEAD" && $2!="NOT_FOUND" {
            cpu_sum += $3; cpu_max = ($3 > cpu_max) ? $3 : cpu_max
            mem_sum += $4; mem_max = ($4 > mem_max) ? $4 : mem_max
            threads_sum += $8; threads_max = ($8 > threads_max) ? $8 : threads_max
            files_sum += $9; files_max = ($9 > files_max) ? $9 : files_max
            count++
        } END {
            if (count > 0) {
                printf "Average CPU Usage: %.1f%% (Peak: %.1f%%)\n", cpu_sum/count, cpu_max
                printf "Average Memory Usage: %.1f MB (Peak: %.1f MB)\n", mem_sum/count, mem_max
                printf "Average Threads: %.0f (Peak: %.0f)\n", threads_sum/count, threads_max
                printf "Average Open Files: %.0f (Peak: %.0f)\n", files_sum/count, files_max
            }
        }' "$RADARR_LOG" >> "$SUMMARY_LOG"
    fi
    
    cat >> "$SUMMARY_LOG" << EOF

## PostgreSQL Summary

EOF
    
    # Analyze PostgreSQL metrics
    if [[ -f "$POSTGRES_LOG" ]] && [[ $(wc -l < "$POSTGRES_LOG") -gt 1 ]]; then
        awk -F',' 'NR>1 {
            conn_sum += $2; conn_max = ($2 > conn_max) ? $2 : conn_max
            active_sum += $3; active_max = ($3 > active_max) ? $3 : active_max
            queries_sum += $6; queries_max = ($6 > queries_max) ? $6 : queries_max
            cache_sum += $9; count++
        } END {
            if (count > 0) {
                printf "Average Total Connections: %.1f (Peak: %.0f)\n", conn_sum/count, conn_max
                printf "Average Active Connections: %.1f (Peak: %.0f)\n", active_sum/count, active_max  
                printf "Average Active Queries: %.1f (Peak: %.0f)\n", queries_sum/count, queries_max
                printf "Average Cache Hit Ratio: %.1f%%\n", cache_sum/count
            }
        }' "$POSTGRES_LOG" >> "$SUMMARY_LOG"
    fi
    
    echo -e "${GREEN}✅ Summary report generated: $SUMMARY_LOG${NC}"
}

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}🛑 Stopping monitoring...${NC}"
    generate_summary
    echo -e "${GREEN}✅ Monitoring data saved${NC}"
}

# Trap cleanup on exit
trap cleanup EXIT INT TERM

# Main monitoring loop
echo -e "${GREEN}🚀 Starting performance monitoring...${NC}"
echo -e "${YELLOW}⏰ Will monitor for ${DURATION} seconds${NC}"
echo

START_TIME=$(date +%s)
COUNTER=0

while true; do
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))
    
    # Check if duration exceeded
    if [[ $ELAPSED -ge $DURATION ]]; then
        echo -e "\n${YELLOW}⏰ Monitoring duration reached${NC}"
        break
    fi
    
    # Collect metrics
    get_system_metrics
    get_radarr_metrics
    get_postgres_metrics
    get_network_metrics
    
    # Display real-time stats every 5 samples
    if [[ $((COUNTER % 5)) -eq 0 ]]; then
        display_realtime_stats
    fi
    
    COUNTER=$((COUNTER + 1))
    sleep "$INTERVAL"
done