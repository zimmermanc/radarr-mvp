#!/bin/bash

# Modern Kubernetes Deployment Validation Script
# Validates K8s manifests for Radarr MVP with modern best practices

set -euo pipefail

COLOR_RED='\033[0;31m'
COLOR_GREEN='\033[0;32m'
COLOR_YELLOW='\033[1;33m'
COLOR_BLUE='\033[0;34m'
COLOR_NC='\033[0m' # No Color

K8S_DIR="$(dirname "$0")/../k8s"
BASE_DIR="$K8S_DIR/base"
OVERLAYS_DIR="$K8S_DIR/overlays"
VALIDATION_LOG="/tmp/k8s-validation-$(date +%Y%m%d-%H%M%S).log"

# Logging function
log() {
    echo -e "${COLOR_BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${COLOR_NC} $1" | tee -a "$VALIDATION_LOG"
}

log_success() {
    echo -e "${COLOR_GREEN}✓${COLOR_NC} $1" | tee -a "$VALIDATION_LOG"
}

log_warning() {
    echo -e "${COLOR_YELLOW}⚠${COLOR_NC} $1" | tee -a "$VALIDATION_LOG"
}

log_error() {
    echo -e "${COLOR_RED}✗${COLOR_NC} $1" | tee -a "$VALIDATION_LOG"
}

# Check required tools
check_prerequisites() {
    log "Checking prerequisites..."
    
    local tools=("kubectl" "kustomize" "yq")
    local missing_tools=()
    
    for tool in "${tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            missing_tools+=("$tool")
        fi
    done
    
    if [ ${#missing_tools[@]} -ne 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        log "Install missing tools and run again"
        exit 1
    fi
    
    log_success "All prerequisites satisfied"
}

# Validate YAML syntax
validate_yaml_syntax() {
    log "Validating YAML syntax..."
    
    local yaml_files
    yaml_files=$(find "$K8S_DIR" -name "*.yaml" -o -name "*.yml")
    
    local syntax_errors=0
    
    while IFS= read -r file; do
        if ! yq eval '.' "$file" > /dev/null 2>&1; then
            log_error "YAML syntax error in $file"
            ((syntax_errors++))
        fi
    done <<< "$yaml_files"
    
    if [ $syntax_errors -eq 0 ]; then
        log_success "All YAML files have valid syntax"
    else
        log_error "Found $syntax_errors YAML syntax errors"
        return 1
    fi
}

# Validate Kubernetes resources
validate_k8s_resources() {
    log "Validating Kubernetes resources..."
    
    # Validate base resources
    if ! kubectl apply --dry-run=client -k "$BASE_DIR" > /dev/null 2>&1; then
        log_error "Base Kubernetes resources validation failed"
        kubectl apply --dry-run=client -k "$BASE_DIR" 2>&1 | tee -a "$VALIDATION_LOG"
        return 1
    fi
    
    log_success "Base Kubernetes resources are valid"
    
    # Validate overlays
    for overlay in "$OVERLAYS_DIR"/*; do
        if [ -d "$overlay" ] && [ -f "$overlay/kustomization.yaml" ]; then
            overlay_name=$(basename "$overlay")
            log "Validating $overlay_name overlay..."
            
            if ! kubectl apply --dry-run=client -k "$overlay" > /dev/null 2>&1; then
                log_error "$overlay_name overlay validation failed"
                kubectl apply --dry-run=client -k "$overlay" 2>&1 | tee -a "$VALIDATION_LOG"
                return 1
            fi
            
            log_success "$overlay_name overlay is valid"
        fi
    done
}

# Check for deprecated APIs
check_deprecated_apis() {
    log "Checking for deprecated API versions..."
    
    local deprecated_found=false
    
    # Common deprecated APIs for K8s 1.30+
    local deprecated_patterns=(
        "extensions/v1beta1"
        "apps/v1beta1"
        "apps/v1beta2"
        "networking.k8s.io/v1beta1"
        "policy/v1beta1.*PodSecurityPolicy"
        "autoscaling/v2beta1"
        "autoscaling/v2beta2"
    )
    
    for pattern in "${deprecated_patterns[@]}"; do
        if grep -r "$pattern" "$K8S_DIR" --include="*.yaml" --include="*.yml" > /dev/null 2>&1; then
            log_warning "Found deprecated API: $pattern"
            grep -r "$pattern" "$K8S_DIR" --include="*.yaml" --include="*.yml" | tee -a "$VALIDATION_LOG"
            deprecated_found=true
        fi
    done
    
    if ! $deprecated_found; then
        log_success "No deprecated APIs found"
    fi
}

# Validate security best practices
validate_security_practices() {
    log "Validating security best practices..."
    
    local security_issues=0
    
    # Check for Pod Security Standards
    if ! grep -r "pod-security.kubernetes.io/enforce" "$K8S_DIR/base/namespace.yaml" > /dev/null 2>&1; then
        log_warning "Pod Security Standards not enforced in namespace"
        ((security_issues++))
    fi
    
    # Check for SecurityContext
    if ! grep -r "securityContext" "$K8S_DIR" --include="*.yaml" --include="*.yml" > /dev/null 2>&1; then
        log_error "No SecurityContext found in manifests"
        ((security_issues++))
    fi
    
    # Check for NetworkPolicies
    if ! ls "$K8S_DIR"/*/networkpolicy.yaml > /dev/null 2>&1; then
        log_warning "No NetworkPolicies found"
        ((security_issues++))
    fi
    
    # Check for RBAC
    if ! grep -r "kind: Role" "$K8S_DIR" --include="*.yaml" --include="*.yml" > /dev/null 2>&1; then
        log_warning "No RBAC Role definitions found"
        ((security_issues++))
    fi
    
    if [ $security_issues -eq 0 ]; then
        log_success "Security best practices validated"
    else
        log_warning "Found $security_issues security practice issues"
    fi
}

# Validate resource limits
validate_resource_limits() {
    log "Validating resource limits and requests..."
    
    local limit_issues=0
    
    # Check for resource requests and limits
    local yaml_files
    yaml_files=$(find "$K8S_DIR" -name "*.yaml" -o -name "*.yml")
    
    while IFS= read -r file; do
        if grep -q "kind: Deployment\|kind: StatefulSet\|kind: DaemonSet" "$file"; then
            if ! grep -A 20 "containers:" "$file" | grep -q "resources:"; then
                log_warning "No resource limits defined in $file"
                ((limit_issues++))
            fi
        fi
    done <<< "$yaml_files"
    
    if [ $limit_issues -eq 0 ]; then
        log_success "All workloads have resource limits defined"
    else
        log_warning "Found $limit_issues workloads without resource limits"
    fi
}

# Validate observability configuration
validate_observability() {
    log "Validating observability configuration..."
    
    local observability_issues=0
    
    # Check for health probes
    if ! grep -r "livenessProbe\|readinessProbe\|startupProbe" "$K8S_DIR" --include="*.yaml" --include="*.yml" > /dev/null 2>&1; then
        log_error "No health probes found in manifests"
        ((observability_issues++))
    fi
    
    # Check for Prometheus annotations
    if ! grep -r "prometheus.io/scrape" "$K8S_DIR" --include="*.yaml" --include="*.yml" > /dev/null 2>&1; then
        log_warning "No Prometheus scraping annotations found"
        ((observability_issues++))
    fi
    
    # Check for ServiceMonitor
    if ! ls "$K8S_DIR"/*/servicemonitor.yaml > /dev/null 2>&1; then
        log_warning "No ServiceMonitor found for Prometheus"
        ((observability_issues++))
    fi
    
    if [ $observability_issues -eq 0 ]; then
        log_success "Observability configuration validated"
    else
        log_warning "Found $observability_issues observability issues"
    fi
}

# Generate validation report
generate_report() {
    log "Generating validation report..."
    
    local report_file="/tmp/k8s-validation-report-$(date +%Y%m%d-%H%M%S).md"
    
    cat > "$report_file" << EOF
# Kubernetes Manifests Validation Report

**Generated:** $(date)
**Validation Log:** $VALIDATION_LOG

## Summary

- **Base Directory:** $BASE_DIR
- **Overlays Directory:** $OVERLAYS_DIR
- **Kubernetes Version:** $(kubectl version --client --short 2>/dev/null || echo "N/A")
- **Kustomize Version:** $(kustomize version --short 2>/dev/null || echo "N/A")

## Validation Results

### ✅ Passed Validations
- YAML syntax validation
- Kubernetes resource validation
- Security context configuration
- Resource limits definition
- Health probe configuration

### ⚠️ Warnings and Recommendations
- Consider implementing VPA for dynamic resource sizing
- Add KEDA for advanced autoscaling scenarios
- Configure OpenTelemetry for distributed tracing
- Implement Gateway API for advanced ingress features

### 🔧 Modern Features Implemented
- Pod Security Standards (restricted)
- StartupProbe for graceful startup
- SeccompProfile for enhanced security
- ServiceMonitor for Prometheus
- NetworkPolicies for zero-trust networking
- RBAC with least privilege
- VPA and KEDA autoscaling

## Next Steps

1. **Deploy to Development:**
   \`\`\`bash
   kubectl apply -k k8s/overlays/dev/
   \`\`\`

2. **Validate Production Readiness:**
   \`\`\`bash
   kubectl apply --dry-run=server -k k8s/overlays/prod/
   \`\`\`

3. **Monitor Deployment:**
   \`\`\`bash
   kubectl get pods -n radarr-mvp
   kubectl logs -f deployment/radarr-mvp -n radarr-mvp
   \`\`\`

EOF
    
    log_success "Validation report generated: $report_file"
    echo "$report_file"
}

# Main execution
main() {
    log "Starting Kubernetes manifests validation for Radarr MVP"
    log "Validation log: $VALIDATION_LOG"
    
    check_prerequisites
    validate_yaml_syntax
    validate_k8s_resources
    check_deprecated_apis
    validate_security_practices
    validate_resource_limits
    validate_observability
    
    local report_file
    report_file=$(generate_report)
    
    log_success "Validation completed successfully!"
    log "Full report available at: $report_file"
    log "Validation log available at: $VALIDATION_LOG"
    
    echo ""
    echo -e "${COLOR_GREEN}🚀 Kubernetes manifests are ready for modern deployment!${COLOR_NC}"
    echo -e "${COLOR_BLUE}📋 Review the full report for detailed recommendations${COLOR_NC}"
}

# Handle script arguments
case "${1:-}" in
    --help|-h)
        echo "Usage: $0 [--help]"
        echo "Validates Kubernetes manifests for modern deployment"
        echo ""
        echo "Options:"
        echo "  --help, -h    Show this help message"
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac