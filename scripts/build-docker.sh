#!/bin/bash
# Docker build script for Radarr MVP
# This script builds the Docker image with proper tagging and layer caching

set -euo pipefail

# Configuration
IMAGE_NAME="${IMAGE_NAME:-radarr-mvp}"
TAG="${TAG:-latest}"
PLATFORM="${PLATFORM:-linux/amd64}"
REGISTRY="${REGISTRY:-}"
BUILD_CONTEXT="${BUILD_CONTEXT:-.}"

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

# Parse command line arguments
PUSH=false
CACHE_FROM=""
BUILD_ARGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --push)
            PUSH=true
            shift
            ;;
        --cache-from)
            CACHE_FROM="--cache-from=$2"
            shift 2
            ;;
        --build-arg)
            BUILD_ARGS="$BUILD_ARGS --build-arg $2"
            shift 2
            ;;
        --registry)
            REGISTRY="$2"
            shift 2
            ;;
        --tag)
            TAG="$2"
            shift 2
            ;;
        --platform)
            PLATFORM="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --push              Push image to registry after build"
            echo "  --cache-from TAG    Use specified tag as cache source"
            echo "  --build-arg ARG     Pass build argument to Docker"
            echo "  --registry REGISTRY Registry to push to"
            echo "  --tag TAG           Tag for the image (default: latest)"
            echo "  --platform PLATFORM Platform to build for (default: linux/amd64)"
            echo "  -h, --help          Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Construct full image name
if [[ -n "$REGISTRY" ]]; then
    FULL_IMAGE_NAME="$REGISTRY/$IMAGE_NAME:$TAG"
else
    FULL_IMAGE_NAME="$IMAGE_NAME:$TAG"
fi

# Pre-build checks
log_info "Starting Docker build for Radarr MVP"
log_info "Image: $FULL_IMAGE_NAME"
log_info "Platform: $PLATFORM"
log_info "Build context: $BUILD_CONTEXT"

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    log_error "Docker is not installed or not in PATH"
    exit 1
fi

# Check if Dockerfile exists
if [[ ! -f "$BUILD_CONTEXT/Dockerfile" ]]; then
    log_error "Dockerfile not found in $BUILD_CONTEXT"
    exit 1
fi

# Build the image
log_info "Building Docker image..."

DOCKER_BUILDKIT=1 docker build \
    --platform "$PLATFORM" \
    --tag "$FULL_IMAGE_NAME" \
    $CACHE_FROM \
    $BUILD_ARGS \
    --label "org.opencontainers.image.created=$(date -u +'%Y-%m-%dT%H:%M:%SZ')" \
    --label "org.opencontainers.image.revision=$(git rev-parse HEAD 2>/dev/null || echo 'unknown')" \
    --label "org.opencontainers.image.version=$TAG" \
    --label "org.opencontainers.image.source=https://github.com/radarr/radarr-mvp" \
    --label "org.opencontainers.image.title=Radarr MVP" \
    --label "org.opencontainers.image.description=Movie automation and management system" \
    "$BUILD_CONTEXT"

if [[ $? -eq 0 ]]; then
    log_success "Successfully built image: $FULL_IMAGE_NAME"
else
    log_error "Failed to build Docker image"
    exit 1
fi

# Get image size
IMAGE_SIZE=$(docker images --format "table {{.Size}}" "$FULL_IMAGE_NAME" | tail -1)
log_info "Image size: $IMAGE_SIZE"

# Push to registry if requested
if [[ "$PUSH" == true ]]; then
    if [[ -z "$REGISTRY" ]]; then
        log_error "Cannot push: no registry specified"
        exit 1
    fi
    
    log_info "Pushing image to registry..."
    docker push "$FULL_IMAGE_NAME"
    
    if [[ $? -eq 0 ]]; then
        log_success "Successfully pushed image: $FULL_IMAGE_NAME"
    else
        log_error "Failed to push Docker image"
        exit 1
    fi
fi

# Security scan (if available)
if command -v docker &> /dev/null && docker --help | grep -q scan; then
    log_info "Running security scan..."
    docker scan "$FULL_IMAGE_NAME" || log_warning "Security scan failed or not available"
fi

log_success "Docker build completed successfully!"
log_info "To run the container locally:"
log_info "docker run -p 7878:7878 $FULL_IMAGE_NAME"