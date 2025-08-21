#!/bin/bash
# Validation script for Radarr MVP deployment
# Tests Docker and Kubernetes deployments

set -euo pipefail

# Configuration
TIMEOUT=60
DOCKER_COMPOSE_FILE="${DOCKER_COMPOSE_FILE:-docker-compose.yml}"
NAMESPACE="${NAMESPACE:-radarr-mvp}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
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

# Wait for service to be ready
wait_for_service() {
    local url=$1
    local service_name=$2
    local timeout=${3:-$TIMEOUT}
    
    log_info "Waiting for $service_name to be ready at $url"
    
    for i in $(seq 1 $timeout); do
        if curl -sf "$url" >/dev/null 2>&1; then
            log_success "$service_name is ready"
            return 0
        fi
        
        if [[ $((i % 10)) -eq 0 ]]; then
            log_info "Still waiting for $service_name... (${i}s)"
        fi
        
        sleep 1
    done
    
    log_error "$service_name failed to become ready within ${timeout}s"
    return 1
}

# Validate Docker deployment
validate_docker() {
    log_info "Validating Docker deployment..."
    
    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        log_error "Docker is not running"
        return 1
    fi
    
    # Check if compose file exists
    if [[ ! -f "$DOCKER_COMPOSE_FILE" ]]; then
        log_error "Docker Compose file not found: $DOCKER_COMPOSE_FILE"
        return 1
    fi
    
    # Check services are running
    log_info "Checking Docker Compose services..."
    
    local services=$(docker-compose -f "$DOCKER_COMPOSE_FILE" ps --services)
    local failed_services=0
    
    for service in $services; do
        local status=$(docker-compose -f "$DOCKER_COMPOSE_FILE" ps -q "$service" | xargs docker inspect --format='{{.State.Status}}' 2>/dev/null || echo "not_found")
        
        if [[ "$status" == "running" ]]; then
            log_success "Service $service is running"
        else
            log_error "Service $service is not running (status: $status)"
            failed_services=$((failed_services + 1))
        fi
    done
    
    if [[ $failed_services -gt 0 ]]; then
        log_error "$failed_services service(s) are not running properly"
        return 1
    fi
    
    # Test application endpoints
    log_info "Testing application endpoints..."
    
    # Health check
    wait_for_service "http://localhost:7878/health" "Radarr MVP health endpoint" 30
    
    # API endpoint (should return 401 without API key)
    if curl -sf "http://localhost:7878/api/movies" >/dev/null 2>&1; then
        log_warning "API endpoint accessible without authentication"
    else
        log_success "API endpoint properly protected"
    fi
    
    log_success "Docker deployment validation completed"
}

# Validate Kubernetes deployment
validate_kubernetes() {
    log_info "Validating Kubernetes deployment..."
    
    # Check kubectl connectivity
    if ! kubectl cluster-info >/dev/null 2>&1; then
        log_error "Cannot connect to Kubernetes cluster"
        return 1
    fi
    
    # Check namespace exists
    if ! kubectl get namespace "$NAMESPACE" >/dev/null 2>&1; then
        log_error "Namespace $NAMESPACE does not exist"
        return 1
    fi
    
    # Check deployment status
    log_info "Checking deployment status..."
    
    local deployment_ready=$(kubectl get deployment radarr-mvp -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}' 2>/dev/null || echo "0")
    local deployment_replicas=$(kubectl get deployment radarr-mvp -n "$NAMESPACE" -o jsonpath='{.spec.replicas}' 2>/dev/null || echo "0")
    
    if [[ "$deployment_ready" == "$deployment_replicas" ]] && [[ "$deployment_ready" != "0" ]]; then
        log_success "Deployment radarr-mvp is ready ($deployment_ready/$deployment_replicas replicas)"
    else
        log_error "Deployment radarr-mvp is not ready ($deployment_ready/$deployment_replicas replicas)"
        
        # Show pod status for debugging
        log_info "Pod status:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=radarr-mvp
        
        return 1
    fi
    
    # Check service endpoints
    log_info "Checking service endpoints..."
    
    local service_ip=$(kubectl get service radarr-mvp-service -n "$NAMESPACE" -o jsonpath='{.spec.clusterIP}' 2>/dev/null || echo "")
    
    if [[ -n "$service_ip" ]]; then
        log_success "Service radarr-mvp-service has ClusterIP: $service_ip"
    else
        log_error "Service radarr-mvp-service not found or has no ClusterIP"
        return 1
    fi
    
    # Test health endpoint via port-forward
    log_info "Testing health endpoint via port-forward..."
    
    # Start port-forward in background
    kubectl port-forward -n "$NAMESPACE" service/radarr-mvp-service 8878:7878 >/dev/null 2>&1 &
    local pf_pid=$!
    
    # Wait a moment for port-forward to establish
    sleep 3
    
    # Test the endpoint
    if wait_for_service "http://localhost:8878/health" "Radarr MVP (via port-forward)" 30; then
        log_success "Health endpoint accessible via port-forward"
    else
        log_error "Health endpoint not accessible via port-forward"
    fi
    
    # Clean up port-forward
    kill $pf_pid 2>/dev/null || true
    
    log_success "Kubernetes deployment validation completed"
}

# Main execution
main() {
    local mode="${1:-all}"
    
    case "$mode" in
        docker)
            validate_docker
            ;;
        kubernetes|k8s)
            validate_kubernetes
            ;;
        all)
            log_info "Starting comprehensive deployment validation..."
            
            # Try Docker first
            if docker info >/dev/null 2>&1 && [[ -f "$DOCKER_COMPOSE_FILE" ]]; then
                validate_docker
            else
                log_warning "Skipping Docker validation (Docker not available or compose file missing)"
            fi
            
            echo
            
            # Try Kubernetes
            if kubectl cluster-info >/dev/null 2>&1; then
                validate_kubernetes
            else
                log_warning "Skipping Kubernetes validation (kubectl not available or not connected)"
            fi
            ;;
        *)
            echo "Usage: $0 [docker|kubernetes|k8s|all]"
            echo "  docker      - Validate Docker Compose deployment"
            echo "  kubernetes  - Validate Kubernetes deployment" 
            echo "  k8s         - Alias for kubernetes"
            echo "  all         - Validate both (default)"
            exit 1
            ;;
    esac
}

# Parse arguments and run
if [[ $# -eq 0 ]]; then
    main "all"
else
    main "$1"
fi