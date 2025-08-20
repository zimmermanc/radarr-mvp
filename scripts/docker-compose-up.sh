#!/bin/bash
# =============================================================================
# Docker Compose Startup Script for Radarr MVP
# =============================================================================
# This script provides an easy way to start the Radarr MVP stack with different configurations

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DEFAULT_ENV_FILE="$PROJECT_ROOT/.env"

# Default values
ENVIRONMENT="development"
PROFILE=""
DETACH=false
BUILD=false
PULL=false
RECREATE=false
LOGS=false
SCALE=""

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

# Help function
show_help() {
    cat << EOF
Radarr MVP Docker Compose Startup Script

Usage: $0 [OPTIONS]

OPTIONS:
    -e, --env ENVIRONMENT       Environment (development, staging, production) [default: development]
    -p, --profile PROFILE       Docker Compose profile to use (full-stack, minimal)
    -d, --detach               Run in detached mode
    -b, --build                Build images before starting
    --pull                     Pull latest images before starting
    --recreate                 Recreate containers even if config hasn't changed
    --logs                     Show logs after startup (implies --detach)
    --scale SERVICE=NUM        Scale a service to NUM instances
    -h, --help                 Show this help message

ENVIRONMENTS:
    development                Development setup with hot reload and debug tools
    staging                    Staging environment with production-like settings
    production                 Production environment with optimizations and security

PROFILES:
    (none)                     Core services only (radarr, postgres, redis)
    full-stack                 All services including Prowlarr and qBittorrent
    minimal                    Just radarr and postgres (no redis)

EXAMPLES:
    # Start development environment
    $0

    # Start production environment in detached mode
    $0 -e production -d

    # Start with full stack (including Prowlarr and qBittorrent)
    $0 -p full-stack

    # Build and start with logs
    $0 --build --logs

    # Start staging with custom scaling
    $0 -e staging --scale radarr=2

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -e|--env)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -p|--profile)
                PROFILE="$2"
                shift 2
                ;;
            -d|--detach)
                DETACH=true
                shift
                ;;
            -b|--build)
                BUILD=true
                shift
                ;;
            --pull)
                PULL=true
                shift
                ;;
            --recreate)
                RECREATE=true
                shift
                ;;
            --logs)
                LOGS=true
                DETACH=true  # Logs imply detach
                shift
                ;;
            --scale)
                SCALE="$2"
                shift 2
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Validate environment
validate_environment() {
    case $ENVIRONMENT in
        development|staging|production)
            log_info "Environment: $ENVIRONMENT"
            ;;
        *)
            log_error "Invalid environment: $ENVIRONMENT"
            log_error "Valid options: development, staging, production"
            exit 1
            ;;
    esac
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if Docker is installed and running
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi
    
    # Check if Docker Compose is available
    if ! docker compose version &> /dev/null; then
        log_error "Docker Compose is not available"
        log_error "Please install Docker Compose or use 'docker-compose' instead of 'docker compose'"
        exit 1
    fi
    
    log_info "Prerequisites check passed"
}

# Setup environment file
setup_env_file() {
    local env_file="$PROJECT_ROOT/.env"
    local example_file="$PROJECT_ROOT/.env.example"
    local docker_env_file="$PROJECT_ROOT/.env.docker"
    
    if [[ ! -f "$env_file" ]]; then
        log_warn ".env file not found, creating from template..."
        
        if [[ -f "$docker_env_file" ]]; then
            log_info "Using .env.docker template"
            cp "$docker_env_file" "$env_file"
        elif [[ -f "$example_file" ]]; then
            log_info "Using .env.example template"
            cp "$example_file" "$env_file"
        else
            log_error "No environment template found (.env.example or .env.docker)"
            exit 1
        fi
        
        log_warn "Please review and customize $env_file before running again"
        exit 1
    fi
    
    log_info "Using environment file: $env_file"
}

# Create required directories
create_directories() {
    log_info "Creating required directories..."
    
    local dirs=(
        "$PROJECT_ROOT/data/movies"
        "$PROJECT_ROOT/data/downloads"
        "$PROJECT_ROOT/dev-data/postgres"
        "$PROJECT_ROOT/dev-data/redis"
        "$PROJECT_ROOT/config/dev"
        "$PROJECT_ROOT/config/prod"
        "$PROJECT_ROOT/backups/postgres"
    )
    
    for dir in "${dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            mkdir -p "$dir"
            log_info "Created directory: $dir"
        fi
    done
}

# Build Docker Compose command
build_compose_command() {
    local cmd="docker compose"
    
    # Base compose file
    cmd="$cmd -f docker-compose.yml"
    
    # Environment-specific compose file
    case $ENVIRONMENT in
        development)
            # docker-compose.override.yml is used automatically
            log_info "Using development configuration with override"
            ;;
        staging)
            if [[ -f "$PROJECT_ROOT/docker-compose.staging.yml" ]]; then
                cmd="$cmd -f docker-compose.staging.yml"
            fi
            ;;
        production)
            cmd="$cmd -f docker-compose.prod.yml"
            ;;
    esac
    
    # Add profile if specified
    if [[ -n "$PROFILE" ]]; then
        cmd="$cmd --profile $PROFILE"
        log_info "Using profile: $PROFILE"
    fi
    
    echo "$cmd"
}

# Start services
start_services() {
    log_info "Starting Radarr MVP stack..."
    
    local compose_cmd
    compose_cmd=$(build_compose_command)
    
    cd "$PROJECT_ROOT"
    
    # Pull images if requested
    if [[ "$PULL" == "true" ]]; then
        log_info "Pulling latest images..."
        $compose_cmd pull
    fi
    
    # Build command
    local up_cmd="$compose_cmd up"
    
    # Add build flag
    if [[ "$BUILD" == "true" ]]; then
        up_cmd="$up_cmd --build"
        log_info "Building images..."
    fi
    
    # Add recreate flag
    if [[ "$RECREATE" == "true" ]]; then
        up_cmd="$up_cmd --force-recreate"
        log_info "Force recreating containers..."
    fi
    
    # Add detach flag
    if [[ "$DETACH" == "true" ]]; then
        up_cmd="$up_cmd -d"
    fi
    
    # Add scale parameters
    if [[ -n "$SCALE" ]]; then
        up_cmd="$up_cmd --scale $SCALE"
        log_info "Scaling: $SCALE"
    fi
    
    log_info "Executing: $up_cmd"
    $up_cmd
    
    if [[ "$DETACH" == "true" ]]; then
        log_info "Services started in detached mode"
        
        # Show service status
        log_info "Service status:"
        $compose_cmd ps
        
        # Show logs if requested
        if [[ "$LOGS" == "true" ]]; then
            log_info "Showing logs (Ctrl+C to exit):"
            $compose_cmd logs -f
        fi
    else
        log_info "Services started in foreground mode (Ctrl+C to stop)"
    fi
}

# Health check
check_health() {
    if [[ "$DETACH" != "true" ]]; then
        return 0
    fi
    
    log_info "Checking service health..."
    
    local compose_cmd
    compose_cmd=$(build_compose_command)
    
    cd "$PROJECT_ROOT"
    
    # Wait a bit for services to start
    sleep 5
    
    # Check if services are running
    local services
    services=$($compose_cmd ps --services --filter "status=running")
    
    if [[ -z "$services" ]]; then
        log_warn "No services appear to be running"
        return 1
    fi
    
    log_info "Running services: $services"
    
    # Try to check Radarr health endpoint
    local max_attempts=10
    local attempt=0
    
    while [[ $attempt -lt $max_attempts ]]; do
        if curl -sf "http://localhost:${RADARR_PORT:-7878}/health" &> /dev/null; then
            log_info "Radarr health check passed"
            return 0
        fi
        
        attempt=$((attempt + 1))
        if [[ $attempt -lt $max_attempts ]]; then
            log_info "Waiting for Radarr to be ready... ($attempt/$max_attempts)"
            sleep 5
        fi
    done
    
    log_warn "Radarr health check failed, but services may still be starting"
    log_info "Check logs with: docker compose logs -f radarr"
}

# Main function
main() {
    log_info "Radarr MVP Docker Compose Startup Script"
    
    # Parse arguments
    parse_args "$@"
    
    # Validate environment
    validate_environment
    
    # Check prerequisites
    check_prerequisites
    
    # Setup environment file
    setup_env_file
    
    # Create directories
    create_directories
    
    # Start services
    start_services
    
    # Check health
    check_health
    
    log_info "Radarr MVP startup complete!"
    
    if [[ "$DETACH" == "true" ]]; then
        echo ""
        log_info "Useful commands:"
        echo "  View logs:      docker compose logs -f"
        echo "  Check status:   docker compose ps"
        echo "  Stop services:  docker compose down"
        echo "  Radarr Web UI:  http://localhost:${RADARR_PORT:-7878}"
        
        if [[ "$PROFILE" == "full-stack" ]]; then
            echo "  Prowlarr UI:    http://localhost:${PROWLARR_PORT:-9696}"
            echo "  qBittorrent UI: http://localhost:${QBITTORRENT_PORT:-8080}"
        fi
    fi
}

# Execute main function
main "$@"