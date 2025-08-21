#!/bin/bash

# Deploy Modern Observability Stack for Radarr MVP
# This script deploys OpenTelemetry, Prometheus, Grafana, Loki, and Tempo

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NAMESPACE="monitoring"
RADARR_NAMESPACE="radarr-system"
MONITORING_DIR="k8s/monitoring"
TIMEOUT="300s"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
check_command() {
    if ! command -v "$1" &> /dev/null; then
        print_error "$1 is required but not installed."
        exit 1
    fi
}

# Function to wait for deployment
wait_for_deployment() {
    local namespace=$1
    local deployment=$2
    local timeout=${3:-300s}
    
    print_status "Waiting for deployment $deployment in namespace $namespace to be ready..."
    if kubectl wait --for=condition=available --timeout="$timeout" deployment/"$deployment" -n "$namespace" 2>/dev/null; then
        print_success "Deployment $deployment is ready"
    else
        print_warning "Deployment $deployment did not become ready within $timeout"
    fi
}

# Function to wait for pod
wait_for_pod() {
    local namespace=$1
    local selector=$2
    local timeout=${3:-300s}
    
    print_status "Waiting for pods with selector $selector in namespace $namespace..."
    if kubectl wait --for=condition=ready --timeout="$timeout" pod -l "$selector" -n "$namespace" 2>/dev/null; then
        print_success "Pods with selector $selector are ready"
    else
        print_warning "Pods with selector $selector did not become ready within $timeout"
    fi
}

# Function to create namespace if it doesn't exist
create_namespace() {
    local namespace=$1
    if kubectl get namespace "$namespace" &> /dev/null; then
        print_status "Namespace $namespace already exists"
    else
        print_status "Creating namespace $namespace..."
        kubectl create namespace "$namespace"
        kubectl label namespace "$namespace" name="$namespace" --overwrite
        print_success "Namespace $namespace created"
    fi
}

# Function to check if we're in the right directory
check_directory() {
    if [[ ! -d "$MONITORING_DIR" ]]; then
        print_error "Monitoring directory $MONITORING_DIR not found. Please run this script from the project root."
        exit 1
    fi
}

# Function to check cluster connectivity
check_cluster() {
    print_status "Checking Kubernetes cluster connectivity..."
    if ! kubectl cluster-info &> /dev/null; then
        print_error "Cannot connect to Kubernetes cluster. Please check your kubeconfig."
        exit 1
    fi
    print_success "Kubernetes cluster is accessible"
}

# Function to check available storage classes
check_storage() {
    print_status "Checking available storage classes..."
    if kubectl get storageclass &> /dev/null; then
        local storage_classes
        storage_classes=$(kubectl get storageclass -o name | head -3)
        print_status "Available storage classes:"
        echo "$storage_classes"
    else
        print_warning "No storage classes found. Some components may not start properly."
    fi
}

# Function to deploy component
deploy_component() {
    local component=$1
    local file=$2
    
    print_status "Deploying $component..."
    if kubectl apply -f "$MONITORING_DIR/$file"; then
        print_success "$component manifests applied"
    else
        print_error "Failed to deploy $component"
        return 1
    fi
}

# Function to verify component health
verify_component() {
    local component=$1
    local namespace=$2
    local selector=$3
    
    print_status "Verifying $component health..."
    
    # Check if pods are running
    local pod_count
    pod_count=$(kubectl get pods -n "$namespace" -l "$selector" --no-headers 2>/dev/null | wc -l)
    
    if [[ $pod_count -gt 0 ]]; then
        local ready_pods
        ready_pods=$(kubectl get pods -n "$namespace" -l "$selector" --no-headers 2>/dev/null | grep -c "Running" || true)
        print_status "$component: $ready_pods/$pod_count pods running"
        
        if [[ $ready_pods -eq $pod_count ]]; then
            print_success "$component is healthy"
            return 0
        else
            print_warning "$component has some pods not ready"
            return 1
        fi
    else
        print_warning "$component: No pods found"
        return 1
    fi
}

# Function to get access URLs
show_access_urls() {
    print_status "Getting access URLs..."
    
    # Check if ingress is configured
    if kubectl get ingress -n "$NAMESPACE" &> /dev/null; then
        local ingress_host
        ingress_host=$(kubectl get ingress -n "$NAMESPACE" -o jsonpath='{.items[0].spec.rules[0].host}' 2>/dev/null || echo "radarr.local")
        
        echo ""
        print_success "Access URLs (assuming ingress host: $ingress_host):"
        echo -e "  📊 Grafana:      https://$ingress_host/grafana"
        echo -e "  🔍 Prometheus:   https://$ingress_host/prometheus" 
        echo -e "  🚨 Alertmanager: https://$ingress_host/alertmanager"
        echo -e "  🔀 Jaeger:       https://$ingress_host/jaeger"
        echo -e "  📈 Radarr Metrics: https://$ingress_host/metrics"
        echo ""
        echo -e "  Default Grafana credentials:"
        echo -e "  Username: admin"
        echo -e "  Password: radarr-admin-2024"
    else
        echo ""
        print_success "Port-forward commands for local access:"
        echo -e "  kubectl port-forward -n $NAMESPACE svc/grafana 3000:3000"
        echo -e "  kubectl port-forward -n $NAMESPACE svc/prometheus 9090:9090"
        echo -e "  kubectl port-forward -n $NAMESPACE svc/alertmanager 9093:9093"
        echo -e "  kubectl port-forward -n $NAMESPACE svc/jaeger-query 16686:16686"
    fi
}

# Function to show useful commands
show_useful_commands() {
    echo ""
    print_status "Useful monitoring commands:"
    echo ""
    echo "# Check all monitoring pods:"
    echo "kubectl get pods -n $NAMESPACE"
    echo ""
    echo "# View OpenTelemetry Collector logs:"
    echo "kubectl logs -n $NAMESPACE deployment/otel-collector -f"
    echo ""
    echo "# Check Prometheus targets:"
    echo "kubectl port-forward -n $NAMESPACE svc/prometheus 9090:9090"
    echo "# Then visit: http://localhost:9090/targets"
    echo ""
    echo "# View Radarr application traces:"
    echo "kubectl port-forward -n $NAMESPACE svc/jaeger-query 16686:16686"
    echo "# Then visit: http://localhost:16686"
    echo ""
    echo "# Check alert rules:"
    echo "kubectl port-forward -n $NAMESPACE svc/prometheus 9090:9090"
    echo "# Then visit: http://localhost:9090/alerts"
    echo ""
    echo "# Scale components:"
    echo "kubectl scale deployment -n $NAMESPACE prometheus --replicas=2"
    echo "kubectl scale deployment -n $NAMESPACE otel-collector --replicas=3"
}

# Main deployment function
main() {
    echo ""
    echo "🚀 Deploying Modern Observability Stack for Radarr MVP"
    echo "========================================================"
    echo ""
    
    # Pre-flight checks
    print_status "Running pre-flight checks..."
    check_command "kubectl"
    check_command "grep"
    check_command "wc"
    check_directory
    check_cluster
    check_storage
    
    # Create namespaces
    create_namespace "$NAMESPACE"
    create_namespace "$RADARR_NAMESPACE"
    
    # Deploy components in order
    echo ""
    print_status "Deploying observability components..."
    
    # 1. OpenTelemetry Collector (core data pipeline)
    deploy_component "OpenTelemetry Collector Config" "otel-collector-config.yaml"
    deploy_component "OpenTelemetry Collector" "otel-collector.yaml"
    wait_for_deployment "$NAMESPACE" "otel-collector" "$TIMEOUT"
    
    # 2. Prometheus (metrics storage)
    deploy_component "Prometheus Stack" "prometheus-stack.yaml"
    wait_for_pod "$NAMESPACE" "app=prometheus" "$TIMEOUT"
    
    # 3. Grafana (visualization)
    deploy_component "Grafana Dashboards" "grafana-dashboards.yaml"
    deploy_component "Grafana" "grafana-deployment.yaml"
    wait_for_deployment "$NAMESPACE" "grafana" "$TIMEOUT"
    
    # 4. Loki Stack (logs)
    deploy_component "Loki Stack" "loki-stack.yaml"
    wait_for_deployment "$NAMESPACE" "loki" "$TIMEOUT"
    wait_for_pod "$NAMESPACE" "app=promtail" "60s"
    
    # 5. Tempo (tracing)
    deploy_component "Tempo" "tempo.yaml"
    wait_for_deployment "$NAMESPACE" "tempo" "$TIMEOUT"
    wait_for_deployment "$NAMESPACE" "jaeger-query" "$TIMEOUT"
    
    # 6. Alertmanager (alerting)
    deploy_component "Alertmanager" "alertmanager.yaml"
    wait_for_deployment "$NAMESPACE" "alertmanager" "$TIMEOUT"
    
    # Verification
    echo ""
    print_status "Verifying deployment health..."
    sleep 10  # Give pods time to stabilize
    
    verify_component "OpenTelemetry Collector" "$NAMESPACE" "app=otel-collector"
    verify_component "Prometheus" "$NAMESPACE" "app=prometheus"
    verify_component "Grafana" "$NAMESPACE" "app=grafana"
    verify_component "Loki" "$NAMESPACE" "app=loki"
    verify_component "Tempo" "$NAMESPACE" "app=tempo"
    verify_component "Alertmanager" "$NAMESPACE" "app=alertmanager"
    
    # Show access information
    echo ""
    show_access_urls
    show_useful_commands
    
    echo ""
    print_success "🎉 Observability stack deployment completed!"
    echo ""
    print_status "Next steps:"
    echo "1. Configure your Radarr application to send telemetry to the OTLP endpoint"
    echo "2. Import additional Grafana dashboards as needed"
    echo "3. Configure Slack/PagerDuty webhooks in Alertmanager"
    echo "4. Set up SSL certificates for production ingress"
    echo ""
    
    # Final health check
    print_status "Final health check in 30 seconds..."
    sleep 30
    
    local all_healthy=true
    for component in "otel-collector" "prometheus" "grafana" "loki" "tempo" "alertmanager"; do
        if ! kubectl get deployment "$component" -n "$NAMESPACE" &> /dev/null; then
            print_warning "$component deployment not found"
            all_healthy=false
        elif ! kubectl get deployment "$component" -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}' | grep -q '[1-9]'; then
            print_warning "$component deployment not ready"
            all_healthy=false
        fi
    done
    
    if $all_healthy; then
        print_success "All components are healthy! 🎊"
    else
        print_warning "Some components may need attention. Check logs with:"
        echo "kubectl logs -n $NAMESPACE deployment/<component-name>"
    fi
}

# Cleanup function
cleanup() {
    if [[ "${1:-}" == "--cleanup" ]]; then
        print_status "Cleaning up observability stack..."
        kubectl delete namespace "$NAMESPACE" --ignore-not-found=true
        print_success "Cleanup completed"
        exit 0
    fi
}

# Script usage
usage() {
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  --cleanup    Remove the entire observability stack"
    echo "  --help       Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                    # Deploy the observability stack"
    echo "  $0 --cleanup          # Remove the observability stack"
    echo ""
}

# Handle arguments
case "${1:-}" in
    --help)
        usage
        exit 0
        ;;
    --cleanup)
        cleanup --cleanup
        ;;
    "")
        main
        ;;
    *)
        print_error "Unknown option: $1"
        usage
        exit 1
        ;;
esac