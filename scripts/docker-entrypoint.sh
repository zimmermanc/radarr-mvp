#!/bin/bash
# =============================================================================
# Docker Entrypoint Script for Radarr MVP
# =============================================================================
# This script handles initialization and startup logic for the container

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_debug() {
    if [[ "${DEBUG:-false}" == "true" ]]; then
        echo -e "${BLUE}[DEBUG]${NC} $1"
    fi
}

# =============================================================================
# ENVIRONMENT VALIDATION
# =============================================================================

validate_environment() {
    log_info "Validating environment variables..."
    
    # Required environment variables
    local required_vars=(
        "DATABASE_URL"
        "RADARR_PORT"
    )
    
    local missing_vars=()
    
    for var in "${required_vars[@]}"; do
        if [[ -z "${!var:-}" ]]; then
            missing_vars+=("$var")
        fi
    done
    
    if [[ ${#missing_vars[@]} -gt 0 ]]; then
        log_error "Missing required environment variables:"
        printf '%s\n' "${missing_vars[@]}"
        exit 1
    fi
    
    # Validate numeric values
    if ! [[ "$RADARR_PORT" =~ ^[0-9]+$ ]]; then
        log_error "RADARR_PORT must be a number, got: $RADARR_PORT"
        exit 1
    fi
    
    if [[ "$RADARR_PORT" -lt 1 || "$RADARR_PORT" -gt 65535 ]]; then
        log_error "RADARR_PORT must be between 1 and 65535, got: $RADARR_PORT"
        exit 1
    fi
    
    log_info "Environment validation passed"
}

# =============================================================================
# DIRECTORY SETUP
# =============================================================================

setup_directories() {
    log_info "Setting up application directories..."
    
    # Create required directories
    local dirs=(
        "/app/config"
        "/app/logs"
        "/app/temp"
        "/movies"
        "/downloads"
    )
    
    for dir in "${dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            log_debug "Creating directory: $dir"
            mkdir -p "$dir"
        fi
        
        # Ensure proper permissions
        if [[ -w "$dir" ]]; then
            log_debug "Directory $dir is writable"
        else
            log_warn "Directory $dir is not writable"
        fi
    done
    
    log_info "Directory setup completed"
}

# =============================================================================
# DATABASE CONNECTIVITY CHECK
# =============================================================================

wait_for_database() {
    log_info "Waiting for database connection..."
    
    local max_attempts=30
    local attempt=0
    local wait_time=2
    
    # Extract database connection details from DATABASE_URL
    # Format: postgresql://user:pass@host:port/db
    local db_host db_port db_name db_user
    
    if [[ "$DATABASE_URL" =~ postgresql://([^:]+):([^@]+)@([^:]+):([0-9]+)/(.+) ]]; then
        db_user="${BASH_REMATCH[1]}"
        db_host="${BASH_REMATCH[3]}"
        db_port="${BASH_REMATCH[4]}"
        db_name="${BASH_REMATCH[5]}"
    else
        log_error "Invalid DATABASE_URL format"
        exit 1
    fi
    
    log_debug "Database details - Host: $db_host, Port: $db_port, DB: $db_name, User: $db_user"
    
    while [[ $attempt -lt $max_attempts ]]; do
        log_debug "Database connection attempt $((attempt + 1))/$max_attempts"
        
        if pg_isready -h "$db_host" -p "$db_port" -U "$db_user" -d "$db_name" >/dev/null 2>&1; then
            log_info "Database connection established"
            return 0
        fi
        
        log_debug "Database not ready, waiting ${wait_time}s..."
        sleep $wait_time
        attempt=$((attempt + 1))
        
        # Exponential backoff (max 10s)
        if [[ $wait_time -lt 10 ]]; then
            wait_time=$((wait_time * 2))
            if [[ $wait_time -gt 10 ]]; then
                wait_time=10
            fi
        fi
    done
    
    log_error "Database connection failed after $max_attempts attempts"
    exit 1
}

# =============================================================================
# REDIS CONNECTIVITY CHECK (OPTIONAL)
# =============================================================================

wait_for_redis() {
    if [[ -z "${REDIS_URL:-}" ]]; then
        log_debug "REDIS_URL not set, skipping Redis check"
        return 0
    fi
    
    log_info "Waiting for Redis connection..."
    
    local max_attempts=10
    local attempt=0
    local wait_time=1
    
    # Extract Redis connection details from REDIS_URL
    # Format: redis://host:port or redis://host:port/db
    local redis_host="redis"
    local redis_port="6379"
    
    if [[ "$REDIS_URL" =~ redis://([^:]+):([0-9]+) ]]; then
        redis_host="${BASH_REMATCH[1]}"
        redis_port="${BASH_REMATCH[2]}"
    elif [[ "$REDIS_URL" =~ redis://([^/:]+) ]]; then
        redis_host="${BASH_REMATCH[1]}"
    fi
    
    log_debug "Redis details - Host: $redis_host, Port: $redis_port"
    
    while [[ $attempt -lt $max_attempts ]]; do
        log_debug "Redis connection attempt $((attempt + 1))/$max_attempts"
        
        if redis-cli -h "$redis_host" -p "$redis_port" ping >/dev/null 2>&1; then
            log_info "Redis connection established"
            return 0
        fi
        
        log_debug "Redis not ready, waiting ${wait_time}s..."
        sleep $wait_time
        attempt=$((attempt + 1))
        wait_time=$((wait_time + 1))
    done
    
    log_warn "Redis connection failed after $max_attempts attempts (continuing without Redis)"
}

# =============================================================================
# DATABASE MIGRATIONS
# =============================================================================

run_migrations() {
    if [[ "${RUN_MIGRATIONS_ON_STARTUP:-true}" != "true" ]]; then
        log_info "Database migrations disabled, skipping"
        return 0
    fi
    
    log_info "Running database migrations..."
    
    # Check if migrations directory exists
    if [[ ! -d "/app/migrations" ]]; then
        log_warn "No migrations directory found at /app/migrations"
        return 0
    fi
    
    # Run migrations with timeout
    local migration_timeout="${MIGRATION_TIMEOUT:-300}"
    
    if timeout "$migration_timeout" radarr-mvp --migrate 2>/dev/null; then
        log_info "Database migrations completed successfully"
    else
        local exit_code=$?
        if [[ $exit_code -eq 124 ]]; then
            log_error "Database migrations timed out after ${migration_timeout}s"
        else
            log_error "Database migrations failed with exit code $exit_code"
        fi
        exit 1
    fi
}

# =============================================================================
# HEALTH CHECK SETUP
# =============================================================================

setup_health_check() {
    log_info "Setting up health check endpoint..."
    
    # Create a simple health check script
    cat > /app/health-check.sh << 'EOF'
#!/bin/bash
# Health check script for Radarr MVP

set -euo pipefail

HEALTH_URL="http://localhost:${RADARR_PORT:-7878}/health"
TIMEOUT=10

if command -v curl >/dev/null 2>&1; then
    curl -sf --max-time "$TIMEOUT" "$HEALTH_URL" >/dev/null
elif command -v wget >/dev/null 2>&1; then
    wget -q --timeout="$TIMEOUT" -O /dev/null "$HEALTH_URL"
else
    echo "Neither curl nor wget available for health check"
    exit 1
fi
EOF
    
    chmod +x /app/health-check.sh
    log_debug "Health check script created"
}

# =============================================================================
# SIGNAL HANDLING
# =============================================================================

setup_signal_handlers() {
    log_debug "Setting up signal handlers..."
    
    # Function to handle shutdown signals
    shutdown_handler() {
        log_info "Received shutdown signal, gracefully shutting down..."
        
        # If radarr-mvp is running, send SIGTERM
        if [[ -n "${RADARR_PID:-}" ]]; then
            log_debug "Sending SIGTERM to radarr-mvp (PID: $RADARR_PID)"
            kill -TERM "$RADARR_PID" 2>/dev/null || true
            
            # Wait for graceful shutdown
            local wait_time=10
            while [[ $wait_time -gt 0 ]] && kill -0 "$RADARR_PID" 2>/dev/null; do
                log_debug "Waiting for graceful shutdown... (${wait_time}s remaining)"
                sleep 1
                wait_time=$((wait_time - 1))
            done
            
            # Force kill if still running
            if kill -0 "$RADARR_PID" 2>/dev/null; then
                log_warn "Force killing radarr-mvp process"
                kill -KILL "$RADARR_PID" 2>/dev/null || true
            fi
        fi
        
        log_info "Shutdown complete"
        exit 0
    }
    
    # Trap signals
    trap shutdown_handler SIGTERM SIGINT SIGQUIT
}

# =============================================================================
# APPLICATION STARTUP
# =============================================================================

start_application() {
    log_info "Starting Radarr MVP application..."
    
    # Build command line arguments
    local args=()
    
    # Add debug flag if enabled
    if [[ "${DEBUG:-false}" == "true" ]]; then
        args+=("--debug")
    fi
    
    # Add verbose flag if enabled
    if [[ "${VERBOSE:-false}" == "true" ]]; then
        args+=("--verbose")
    fi
    
    # Set environment variables for the application
    export RUST_LOG="${RUST_LOG:-info}"
    export RUST_BACKTRACE="${RUST_BACKTRACE:-0}"
    
    # Enable more verbose logging in development
    if [[ "${DEVELOPMENT_MODE:-false}" == "true" ]]; then
        export RUST_LOG="${RUST_LOG:-debug}"
        export RUST_BACKTRACE="1"
        log_info "Development mode enabled"
    fi
    
    log_info "Starting application with environment:"
    log_debug "  RUST_LOG: $RUST_LOG"
    log_debug "  RADARR_HOST: ${RADARR_HOST:-0.0.0.0}"
    log_debug "  RADARR_PORT: $RADARR_PORT"
    log_debug "  DATABASE_MAX_CONNECTIONS: ${DATABASE_MAX_CONNECTIONS:-10}"
    
    # Start the application in background to handle signals
    exec radarr-mvp "${args[@]}" &
    RADARR_PID=$!
    
    log_info "Radarr MVP started with PID: $RADARR_PID"
    
    # Wait for the process
    wait $RADARR_PID
    local exit_code=$?
    
    log_info "Radarr MVP exited with code: $exit_code"
    exit $exit_code
}

# =============================================================================
# MAIN EXECUTION
# =============================================================================

main() {
    log_info "Radarr MVP Docker Container Starting..."
    log_info "Version: ${VERSION:-unknown}"
    log_info "Build: ${BUILD_TARGET:-unknown}"
    log_info "Environment: ${ENVIRONMENT:-unknown}"
    
    # Validate environment
    validate_environment
    
    # Setup directories
    setup_directories
    
    # Setup signal handlers
    setup_signal_handlers
    
    # Setup health check
    setup_health_check
    
    # Wait for dependencies
    wait_for_database
    wait_for_redis
    
    # Run database migrations
    run_migrations
    
    # Start application
    start_application
}

# =============================================================================
# SCRIPT EXECUTION
# =============================================================================

# Handle special commands
case "${1:-}" in
    "health-check")
        exec /app/health-check.sh
        ;;
    "migrate")
        validate_environment
        wait_for_database
        run_migrations
        exit 0
        ;;
    "bash"|"shell")
        exec /bin/bash
        ;;
    "--help"|"-h")
        echo "Radarr MVP Docker Entrypoint"
        echo ""
        echo "Usage:"
        echo "  radarr-mvp                    Start the application (default)"
        echo "  radarr-mvp health-check       Run health check"
        echo "  radarr-mvp migrate             Run database migrations only"
        echo "  radarr-mvp bash|shell          Start interactive shell"
        echo "  radarr-mvp --help|-h           Show this help"
        echo ""
        echo "Environment Variables:"
        echo "  DATABASE_URL                   PostgreSQL connection string (required)"
        echo "  RADARR_PORT                   Server port (required)"
        echo "  RUN_MIGRATIONS_ON_STARTUP     Run migrations on startup (default: true)"
        echo "  DEBUG                         Enable debug logging (default: false)"
        echo "  DEVELOPMENT_MODE              Enable development mode (default: false)"
        echo ""
        exit 0
        ;;
    "")
        # Default behavior - start the application
        main
        ;;
    *)
        # Pass through to radarr-mvp binary
        exec radarr-mvp "$@"
        ;;
esac