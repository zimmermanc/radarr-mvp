#!/bin/bash

# Load centralized configuration file
# This script sources the services.env file and makes all variables available

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration file locations (in order of preference)
CONFIG_LOCATIONS=(
    "$PROJECT_ROOT/config/services.env"
    "$PROJECT_ROOT/.env"
    "$HOME/.radarr/services.env"
    "/etc/radarr/services.env"
)

# Find and load configuration
CONFIG_LOADED=false

for config_file in "${CONFIG_LOCATIONS[@]}"; do
    if [ -f "$config_file" ]; then
        echo "Loading configuration from: $config_file"
        set -a  # Export all variables
        source "$config_file"
        set +a  # Stop exporting
        CONFIG_LOADED=true
        export RADARR_CONFIG_FILE="$config_file"
        break
    fi
done

if [ "$CONFIG_LOADED" = false ]; then
    echo "WARNING: No configuration file found!"
    echo "Please create one of the following:"
    for location in "${CONFIG_LOCATIONS[@]}"; do
        echo "  - $location"
    done
    echo ""
    echo "You can copy the example file:"
    echo "  cp $PROJECT_ROOT/config/services.env.example $PROJECT_ROOT/config/services.env"
    echo "  Then edit it with your credentials"
    exit 1
fi

# Validate required variables
validate_config() {
    local errors=0
    
    # Check critical variables
    if [ -z "$DATABASE_URL" ]; then
        echo "ERROR: DATABASE_URL is not set"
        errors=$((errors + 1))
    fi
    
    if [ -z "$TMDB_API_KEY" ] || [ "$TMDB_API_KEY" = "your_tmdb_api_key_here" ]; then
        echo "WARNING: TMDB_API_KEY is not configured"
    fi
    
    if [ -z "$HDBITS_SESSION_COOKIE" ] && [ "$1" = "hdbits" ]; then
        echo "ERROR: HDBITS_SESSION_COOKIE is required for HDBits operations"
        errors=$((errors + 1))
    fi
    
    if [ $errors -gt 0 ]; then
        echo "Configuration validation failed with $errors errors"
        return 1
    fi
    
    return 0
}

# Export common paths
export RADARR_ROOT="$PROJECT_ROOT"
export RADARR_SCRIPTS="$PROJECT_ROOT/scripts"
export RADARR_LOGS="${LOG_FILE%/*}"
export RADARR_ANALYSIS_DIR="${ANALYSIS_DIR:-/tmp/radarr/analysis}"

# Helper function to check if a service is configured
is_service_configured() {
    local service=$1
    case $service in
        tmdb)
            [ -n "$TMDB_API_KEY" ] && [ "$TMDB_API_KEY" != "your_tmdb_api_key_here" ]
            ;;
        hdbits)
            [ -n "$HDBITS_SESSION_COOKIE" ] || ([ -n "$HDBITS_USERNAME" ] && [ -n "$HDBITS_PASSKEY" ])
            ;;
        qbittorrent)
            [ -n "$QBITTORRENT_HOST" ] && [ -n "$QBITTORRENT_USERNAME" ]
            ;;
        discord)
            [ -n "$DISCORD_WEBHOOK_URL" ] && [ "$DISCORD_WEBHOOK_URL" != "https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN" ]
            ;;
        *)
            return 1
            ;;
    esac
}

# Print configuration summary (with sensitive data masked)
print_config_summary() {
    echo "Configuration Summary:"
    echo "====================="
    echo "Database: ${DATABASE_URL%%@*}@***"
    echo "Environment: ${ENVIRONMENT:-production}"
    echo "Debug Mode: ${DEBUG_MODE:-false}"
    echo ""
    echo "Services Status:"
    
    if is_service_configured tmdb; then
        echo "  ✓ TMDB configured"
    else
        echo "  ✗ TMDB not configured"
    fi
    
    if is_service_configured hdbits; then
        echo "  ✓ HDBits configured"
    else
        echo "  ✗ HDBits not configured"
    fi
    
    if is_service_configured qbittorrent; then
        echo "  ✓ qBittorrent configured"
    else
        echo "  ✗ qBittorrent not configured"
    fi
    
    if is_service_configured discord; then
        echo "  ✓ Discord notifications configured"
    else
        echo "  ✗ Discord notifications not configured"
    fi
    
    echo ""
}

# If script is run directly, validate and show summary
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    validate_config "$@"
    print_config_summary
fi