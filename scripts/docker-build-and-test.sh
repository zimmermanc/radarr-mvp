#!/bin/bash
# Docker Build and Test Script for Radarr MVP
# Implements comprehensive testing, security scanning, and SBOM generation

set -euo pipefail

# Configuration
IMAGE_NAME="${RADARR_IMAGE:-radarr-mvp}"
IMAGE_TAG="${RADARR_TAG:-latest}"
FULL_IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"
BUILD_PLATFORM="${BUILD_PLATFORM:-linux/amd64,linux/arm64}"
SCAN_ENABLED="${SECURITY_SCAN:-true}"
SBOM_ENABLED="${SBOM_GENERATE:-true}"
PUSH_ENABLED="${PUSH_IMAGE:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed"
    fi
    
    # Check Docker Buildx
    if ! docker buildx version &> /dev/null; then
        error "Docker Buildx is not available"
    fi
    
    # Check Docker daemon
    if ! docker info &> /dev/null; then
        error "Docker daemon is not running"
    fi
    
    info "Docker version: $(docker --version)"
    info "Docker Buildx version: $(docker buildx version)"
}

# Setup buildx builder
setup_buildx() {
    log "Setting up Docker Buildx..."
    
    # Create multi-platform builder if it doesn't exist
    if ! docker buildx ls | grep -q "radarr-builder"; then
        docker buildx create --name radarr-builder --platform $BUILD_PLATFORM --use
        info "Created multi-platform builder 'radarr-builder'"
    else
        docker buildx use radarr-builder
        info "Using existing builder 'radarr-builder'"
    fi
    
    # Bootstrap the builder
    docker buildx inspect --bootstrap
}

# Build Docker image
build_image() {
    log "Building Docker image: $FULL_IMAGE"
    
    # Start build timing
    local start_time=$(date +%s)
    
    # Build with BuildKit features
    DOCKER_BUILDKIT=1 docker buildx build \
        --platform $BUILD_PLATFORM \
        --tag $FULL_IMAGE \
        --tag "${IMAGE_NAME}:build-$(date +%Y%m%d-%H%M%S)" \
        --build-arg BUILDKIT_INLINE_CACHE=1 \
        --build-arg CARGO_INCREMENTAL=0 \
        --build-arg RUST_BACKTRACE=1 \
        --metadata-file /tmp/build-metadata.json \
        --load \
        .
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    info "Build completed in ${duration}s"
    
    # Show image info
    docker images $IMAGE_NAME --format "table {{.Repository}}\\t{{.Tag}}\\t{{.Size}}\\t{{.CreatedAt}}"
}

# Test individual build stages
test_build_stages() {
    log "Testing build stages..."
    
    # Test chef stage
    info "Testing cargo-chef stage..."
    docker buildx build --target chef --tag "${IMAGE_NAME}:chef-test" . || warn "Chef stage test failed"
    
    # Test planner stage
    info "Testing planner stage..."
    docker buildx build --target planner --tag "${IMAGE_NAME}:planner-test" . || warn "Planner stage test failed"
    
    # Test rust-builder stage
    info "Testing rust-builder stage..."
    docker buildx build --target rust-builder --tag "${IMAGE_NAME}:rust-test" . || warn "Rust builder stage test failed"
    
    # Test web-builder stage
    info "Testing web-builder stage..."
    docker buildx build --target web-builder --tag "${IMAGE_NAME}:web-test" . || warn "Web builder stage test failed"
}

# Run container tests
test_container() {
    log "Testing container functionality..."
    
    # Test container startup
    info "Testing container startup..."
    local container_id=$(docker run -d --name radarr-test-container \
        -e DATABASE_URL="sqlite:///tmp/test.db" \
        -e RUST_LOG=debug \
        -p 17878:7878 \
        $FULL_IMAGE)
    
    # Wait for startup
    sleep 10
    
    # Test health endpoint
    info "Testing health endpoint..."
    local max_attempts=30
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -f http://localhost:17878/health &> /dev/null; then
            info "Health check passed"
            break
        fi
        
        attempt=$((attempt + 1))
        if [ $attempt -eq $max_attempts ]; then
            error "Health check failed after $max_attempts attempts"
        fi
        
        sleep 2
    done
    
    # Test graceful shutdown
    info "Testing graceful shutdown..."
    docker stop radarr-test-container
    
    # Cleanup
    docker rm radarr-test-container
    
    info "Container tests passed"
}

# Security scanning
security_scan() {
    if [ "$SCAN_ENABLED" != "true" ]; then
        warn "Security scanning disabled"
        return 0
    fi
    
    log "Running security scans..."
    
    # Check if Trivy is available
    if command -v trivy &> /dev/null; then
        info "Scanning with Trivy..."
        trivy image --severity HIGH,CRITICAL --exit-code 1 $FULL_IMAGE || warn "Trivy scan found vulnerabilities"
    else
        warn "Trivy not available, skipping vulnerability scan"
    fi
    
    # Check if Grype is available
    if command -v grype &> /dev/null; then
        info "Scanning with Grype..."
        grype $FULL_IMAGE || warn "Grype scan found vulnerabilities"
    else
        info "Grype not available, skipping additional vulnerability scan"
    fi
    
    # Docker Scout (if available)
    if docker scout version &> /dev/null; then
        info "Scanning with Docker Scout..."
        docker scout cves $FULL_IMAGE || warn "Docker Scout found vulnerabilities"
    else
        info "Docker Scout not available"
    fi
}

# Generate SBOM
generate_sbom() {
    if [ "$SBOM_ENABLED" != "true" ]; then
        warn "SBOM generation disabled"
        return 0
    fi
    
    log "Generating Software Bill of Materials (SBOM)..."
    
    # Create SBOM directory
    mkdir -p ./sbom
    
    # Generate SBOM with Syft (if available)
    if command -v syft &> /dev/null; then
        info "Generating SBOM with Syft..."
        syft $FULL_IMAGE -o spdx-json=./sbom/radarr-mvp-sbom.spdx.json
        syft $FULL_IMAGE -o cyclonedx-json=./sbom/radarr-mvp-sbom.cyclonedx.json
        info "SBOM files generated in ./sbom/"
    else
        warn "Syft not available, skipping SBOM generation"
    fi
}

# Performance analysis
analyze_performance() {
    log "Analyzing container performance..."
    
    # Image size analysis
    local image_size=$(docker images $FULL_IMAGE --format "{{.Size}}")
    info "Final image size: $image_size"
    
    # Layer analysis
    info "Image layers:"
    docker history $FULL_IMAGE --format "table {{.CreatedBy}}\\t{{.Size}}" | head -10
    
    # Memory usage test
    info "Testing memory usage..."
    local container_id=$(docker run -d --name radarr-perf-test \
        -e DATABASE_URL="sqlite:///tmp/test.db" \
        -m 512m \
        $FULL_IMAGE)
    
    sleep 5
    
    # Get memory stats
    docker stats radarr-perf-test --no-stream --format "table {{.Name}}\\t{{.MemUsage}}\\t{{.CPUPerc}}"
    
    # Cleanup
    docker stop radarr-perf-test
    docker rm radarr-perf-test
}

# Push image (optional)
push_image() {
    if [ "$PUSH_ENABLED" != "true" ]; then
        info "Image push disabled"
        return 0
    fi
    
    log "Pushing image to registry..."
    docker push $FULL_IMAGE
    info "Image pushed successfully"
}

# Cleanup function
cleanup() {
    log "Cleaning up test containers and images..."
    
    # Remove test images
    docker rmi "${IMAGE_NAME}:chef-test" "${IMAGE_NAME}:planner-test" \
               "${IMAGE_NAME}:rust-test" "${IMAGE_NAME}:web-test" 2>/dev/null || true
    
    # Remove dangling images
    docker image prune -f
    
    info "Cleanup completed"
}

# Main execution
main() {
    log "Starting Docker build and test process for Radarr MVP"
    
    check_prerequisites
    setup_buildx
    
    # Build stages
    build_image
    test_build_stages
    
    # Testing
    test_container
    analyze_performance
    
    # Security and compliance
    security_scan
    generate_sbom
    
    # Optional push
    push_image
    
    log "Docker build and test process completed successfully!"
    
    # Summary
    echo
    echo "=== BUILD SUMMARY ==="
    echo "Image: $FULL_IMAGE"
    echo "Platform: $BUILD_PLATFORM"
    echo "Size: $(docker images $FULL_IMAGE --format "{{.Size}}")"
    echo "Security Scan: $([ "$SCAN_ENABLED" = "true" ] && echo "Enabled" || echo "Disabled")"
    echo "SBOM Generated: $([ "$SBOM_ENABLED" = "true" ] && echo "Yes" || echo "No")"
    echo "===================="
}

# Trap for cleanup on exit
trap cleanup EXIT

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-scan)
            SCAN_ENABLED="false"
            shift
            ;;
        --no-sbom)
            SBOM_ENABLED="false"
            shift
            ;;
        --push)
            PUSH_ENABLED="true"
            shift
            ;;
        --tag)
            IMAGE_TAG="$2"
            FULL_IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"
            shift 2
            ;;
        --name)
            IMAGE_NAME="$2"
            FULL_IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"
            shift 2
            ;;
        --platform)
            BUILD_PLATFORM="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --no-scan      Disable security scanning"
            echo "  --no-sbom      Disable SBOM generation"
            echo "  --push         Push image to registry after build"
            echo "  --tag TAG      Set image tag (default: latest)"
            echo "  --name NAME    Set image name (default: radarr-mvp)"
            echo "  --platform PLATFORMS Set target platforms (default: linux/amd64,linux/arm64)"
            echo "  -h, --help     Show this help message"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Run main function
main