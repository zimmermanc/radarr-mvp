#!/bin/bash
# Production Deployment Script for Radarr MVP
# Handles blue-green deployments with health checks and monitoring

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ENV_FILE="${PROJECT_ROOT}/.env.production"
COMPOSE_FILE="${PROJECT_ROOT}/docker-compose.production.yml"
HEALTH_CHECK_TIMEOUT=300
HEALTH_CHECK_INTERVAL=10

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Cleanup function
cleanup() {
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        log_error "Deployment failed with exit code $exit_code"
        log_info "Rolling back to previous version..."
        rollback
    fi
    exit $exit_code
}

trap cleanup EXIT

# Validation functions
validate_environment() {
    log_info "Validating deployment environment..."
    
    # Check if environment file exists
    if [ ! -f "$ENV_FILE" ]; then
        log_error "Environment file not found: $ENV_FILE"
        log_info "Please copy .env.production.example to .env.production and configure it"
        exit 1
    fi
    
    # Check required environment variables
    source "$ENV_FILE"
    
    local required_vars=(
        "POSTGRES_PASSWORD"
        "HDBITS_API_KEY"
        "TMDB_API_KEY"
        "REDIS_PASSWORD"
        "GRAFANA_PASSWORD"
        "JWT_SECRET"
        "SESSION_SECRET"
    )
    
    for var in "${required_vars[@]}"; do
        if [ -z "${!var:-}" ]; then
            log_error "Required environment variable $var is not set"
            exit 1
        fi
    done
    
    # Check Docker and Docker Compose
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null; then
        log_error "Docker Compose is not installed"
        exit 1
    fi
    
    log_success "Environment validation passed"
}

# Backup functions
backup_database() {
    log_info "Creating database backup..."
    
    local backup_file="backup_$(date +%Y%m%d_%H%M%S).sql"
    local backup_path="${PROJECT_ROOT}/backups/${backup_file}"
    
    mkdir -p "${PROJECT_ROOT}/backups"
    
    # Create database backup
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" exec -T postgres \
        pg_dump -U radarr radarr > "$backup_path"
    
    if [ -f "$backup_path" ]; then
        log_success "Database backup created: $backup_path"
        echo "$backup_path" > "${PROJECT_ROOT}/.last_backup"
    else
        log_error "Failed to create database backup"
        exit 1
    fi
}

# Health check functions
wait_for_health() {
    local service="$1"
    local url="$2"
    local timeout="$3"
    local interval="$4"
    
    log_info "Waiting for $service to become healthy..."
    
    local elapsed=0
    while [ $elapsed -lt $timeout ]; do
        if curl -f "$url" > /dev/null 2>&1; then
            log_success "$service is healthy"
            return 0
        fi
        
        sleep $interval
        elapsed=$((elapsed + interval))
        log_info "Health check attempt $((elapsed / interval))/$((timeout / interval))"
    done
    
    log_error "$service failed to become healthy within ${timeout}s"
    return 1
}

# Deployment functions
deploy_application() {
    log_info "Starting application deployment..."
    
    # Pull latest images
    log_info "Pulling latest images..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" pull
    
    # Build application image
    log_info "Building application image..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" build radarr-mvp
    
    # Start infrastructure services first
    log_info "Starting infrastructure services..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" up -d postgres redis
    
    # Wait for database to be ready
    wait_for_health "PostgreSQL" "http://localhost:5432" 60 5
    
    # Run database migrations
    log_info "Running database migrations..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" run --rm radarr-mvp \
        /app/radarr-rust migrate
    
    # Start application
    log_info "Starting application..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" up -d radarr-mvp
    
    # Wait for application to be healthy
    wait_for_health "Radarr MVP" "http://localhost:8080/health" $HEALTH_CHECK_TIMEOUT $HEALTH_CHECK_INTERVAL
    
    # Start monitoring services
    log_info "Starting monitoring services..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" up -d prometheus grafana loki promtail
    
    # Start reverse proxy
    log_info "Starting reverse proxy..."
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" up -d nginx
    
    log_success "Application deployment completed successfully"
}

rollback() {
    log_warning "Rolling back to previous version..."
    
    if [ -f "${PROJECT_ROOT}/.last_backup" ]; then
        local backup_file=$(cat "${PROJECT_ROOT}/.last_backup")
        log_info "Restoring database from backup: $backup_file"
        
        # Restore database
        docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" exec -T postgres \
            psql -U radarr -d radarr < "$backup_file"
    fi
    
    # Stop current services
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" down
    
    log_info "Rollback completed"
}

# Performance testing
run_smoke_tests() {
    log_info "Running smoke tests..."
    
    # Test health endpoint
    if ! curl -f "http://localhost:8080/health" > /dev/null 2>&1; then
        log_error "Health endpoint test failed"
        return 1
    fi
    
    # Test API endpoints
    if ! curl -f "http://localhost:8080/api/movies" > /dev/null 2>&1; then
        log_error "API endpoint test failed"
        return 1
    fi
    
    # Test metrics endpoint (internal)
    if ! curl -f "http://localhost:8080/metrics" > /dev/null 2>&1; then
        log_warning "Metrics endpoint test failed (non-critical)"
    fi
    
    log_success "Smoke tests passed"
}

# Monitoring setup
setup_monitoring() {
    log_info "Setting up monitoring and alerting..."
    
    # Wait for Grafana to be ready
    wait_for_health "Grafana" "http://localhost:3000" 60 5
    
    # Wait for Prometheus to be ready
    wait_for_health "Prometheus" "http://localhost:9090" 60 5
    
    log_success "Monitoring setup completed"
    log_info "Grafana: http://localhost:3000 (admin/\$GRAFANA_PASSWORD)"
    log_info "Prometheus: http://localhost:9090"
}

# Main deployment flow
main() {
    log_info "Starting Radarr MVP production deployment..."
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Validate environment
    validate_environment
    
    # Create backup
    if docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" ps postgres | grep -q "Up"; then
        backup_database
    else
        log_info "Skipping backup - database not running"
    fi
    
    # Deploy application
    deploy_application
    
    # Run smoke tests
    run_smoke_tests
    
    # Setup monitoring
    setup_monitoring
    
    log_success "Deployment completed successfully!"
    log_info "Application: https://localhost"
    log_info "Health: http://localhost:8080/health"
    log_info "Monitoring: http://localhost:3000"
    
    # Display service status
    log_info "Service status:"
    docker-compose -f "$COMPOSE_FILE" --env-file="$ENV_FILE" ps
}

# Run main function
main "$@"