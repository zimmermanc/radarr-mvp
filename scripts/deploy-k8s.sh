#!/bin/bash
# Kubernetes deployment script for Radarr MVP
# This script deploys the application to Kubernetes using Kustomize

set -euo pipefail

# Configuration
ENVIRONMENT="${ENVIRONMENT:-dev}"
NAMESPACE="${NAMESPACE:-radarr-mvp}"
KUSTOMIZE_PATH="${KUSTOMIZE_PATH:-k8s/overlays}"
DRY_RUN="${DRY_RUN:-false}"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-300s}"

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
while [[ $# -gt 0 ]]; do
    case $1 in
        --environment)
            ENVIRONMENT="$2"
            shift 2
            ;;
        --namespace)
            NAMESPACE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --wait-timeout)
            WAIT_TIMEOUT="$2"
            shift 2
            ;;
        --delete)
            DELETE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --environment ENV   Environment to deploy to (dev/staging/prod)"
            echo "  --namespace NS      Kubernetes namespace (default: radarr-mvp)"
            echo "  --dry-run          Perform a dry run without making changes"
            echo "  --wait-timeout TIME Timeout for waiting for deployment (default: 300s)"
            echo "  --delete           Delete the deployment instead of applying"
            echo "  -h, --help         Show this help message"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Pre-deployment checks
log_info "Starting Kubernetes deployment for Radarr MVP"
log_info "Environment: $ENVIRONMENT"
log_info "Namespace: $NAMESPACE"
log_info "Dry run: $DRY_RUN"

# Check if kubectl is available
if ! command -v kubectl &> /dev/null; then
    log_error "kubectl is not installed or not in PATH"
    exit 1
fi

# Check if kustomize is available
if ! command -v kustomize &> /dev/null; then
    log_error "kustomize is not installed or not in PATH"
    log_info "Install with: curl -s \"https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh\" | bash"
    exit 1
fi

# Check Kubernetes connectivity
log_info "Checking Kubernetes connectivity..."
if ! kubectl cluster-info &> /dev/null; then
    log_error "Cannot connect to Kubernetes cluster"
    exit 1
fi

# Check if kustomization exists
KUSTOMIZE_OVERLAY_PATH="$KUSTOMIZE_PATH/$ENVIRONMENT"
if [[ ! -f "$KUSTOMIZE_OVERLAY_PATH/kustomization.yaml" ]]; then
    log_error "Kustomization file not found: $KUSTOMIZE_OVERLAY_PATH/kustomization.yaml"
    exit 1
fi

# Validate kustomization
log_info "Validating Kustomization..."
if ! kustomize build "$KUSTOMIZE_OVERLAY_PATH" > /dev/null; then
    log_error "Kustomization validation failed"
    exit 1
fi

# Handle deletion if requested
if [[ "${DELETE:-false}" == true ]]; then
    log_warning "Deleting deployment..."
    
    if [[ "$DRY_RUN" == true ]]; then
        log_info "DRY RUN: Would delete resources in namespace $NAMESPACE"
        kubectl delete --dry-run=client -k "$KUSTOMIZE_OVERLAY_PATH"
    else
        kubectl delete -k "$KUSTOMIZE_OVERLAY_PATH" --timeout="$WAIT_TIMEOUT" || log_warning "Some resources may not have been deleted"
        log_success "Deployment deleted successfully"
    fi
    exit 0
fi

# Apply the manifests
log_info "Applying Kubernetes manifests..."

if [[ "$DRY_RUN" == true ]]; then
    log_info "DRY RUN: Validating manifests without applying"
    kubectl apply --dry-run=client -k "$KUSTOMIZE_OVERLAY_PATH"
    
    if [[ $? -eq 0 ]]; then
        log_success "Dry run completed successfully - manifests are valid"
    else
        log_error "Dry run failed - manifests contain errors"
        exit 1
    fi
else
    # Create namespace if it doesn't exist
    kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -
    
    # Apply the kustomization
    kubectl apply -k "$KUSTOMIZE_OVERLAY_PATH"
    
    if [[ $? -eq 0 ]]; then
        log_success "Manifests applied successfully"
    else
        log_error "Failed to apply manifests"
        exit 1
    fi
    
    # Wait for deployment to be ready
    log_info "Waiting for deployment to be ready..."
    kubectl wait --for=condition=available --timeout="$WAIT_TIMEOUT" deployment/radarr-mvp -n "$NAMESPACE"
    
    if [[ $? -eq 0 ]]; then
        log_success "Deployment is ready!"
    else
        log_error "Deployment did not become ready within timeout"
        
        # Show pod status for debugging
        log_info "Pod status:"
        kubectl get pods -n "$NAMESPACE" -l app.kubernetes.io/name=radarr-mvp
        
        log_info "Recent events:"
        kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' | tail -10
        
        exit 1
    fi
    
    # Show deployment status
    log_info "Deployment status:"
    kubectl get deployment,pods,services -n "$NAMESPACE" -l app.kubernetes.io/name=radarr-mvp
    
    # Get service URLs
    log_info "Service information:"
    kubectl get services -n "$NAMESPACE" -l app.kubernetes.io/name=radarr-mvp -o wide
    
    # Check if ingress is configured
    if kubectl get ingress -n "$NAMESPACE" -l app.kubernetes.io/name=radarr-mvp &> /dev/null; then
        log_info "Ingress information:"
        kubectl get ingress -n "$NAMESPACE" -l app.kubernetes.io/name=radarr-mvp
    fi
fi

log_success "Kubernetes deployment completed successfully!"

# Port forwarding instructions
if [[ "$DRY_RUN" != true ]]; then
    log_info "To access the application locally:"
    log_info "kubectl port-forward -n $NAMESPACE service/radarr-mvp-service 7878:7878"
    log_info "Then visit: http://localhost:7878"
fi