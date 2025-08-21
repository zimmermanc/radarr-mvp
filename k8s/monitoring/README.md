# Modern Observability Stack for Radarr MVP

This directory contains a comprehensive, production-ready observability stack built around OpenTelemetry, featuring:

## 🚀 Stack Components

### Core Telemetry
- **OpenTelemetry Collector** - Unified telemetry data collection and processing
- **Prometheus + Thanos** - Metrics storage with long-term retention
- **Grafana** - Unified visualization for metrics, logs, and traces
- **Loki + Promtail** - Log aggregation and processing
- **Tempo + Jaeger** - Distributed tracing storage and querying

### Key Features
- **Three Pillars of Observability**: Logs, Metrics, and Traces fully integrated
- **Distributed Tracing**: End-to-end request tracing with correlation IDs
- **SLI/SLO Monitoring**: Error budgets and burn rate alerting
- **Business Metrics**: Movie imports, downloads, and user activity
- **Infrastructure Monitoring**: Kubernetes cluster and application health
- **Advanced Alerting**: Multi-channel alerts with smart routing

## 📊 Pre-Built Dashboards

### Radarr Overview Dashboard
- **RED Metrics**: Rate, Errors, Duration for all API endpoints
- **Business KPIs**: Movies added, downloads completed, imports processed
- **Performance Metrics**: Response time percentiles, memory usage
- **Database Health**: Query performance, connection pool usage

### Infrastructure Dashboard
- **Kubernetes Metrics**: Pod health, resource utilization, events
- **Node Metrics**: CPU, memory, disk, network across cluster
- **Application Performance**: JVM metrics, garbage collection

### Observability Stack Dashboard
- **Stack Health**: All monitoring components status
- **Data Pipeline**: Ingestion rates, processing latency
- **Resource Usage**: Monitoring infrastructure consumption

## 🎯 Alert Rules

### Critical Alerts (PagerDuty + Slack)
- High error rate (>5% for 5 minutes)
- Database connection pool exhausted
- Memory usage >90%
- Radarr application down

### Warning Alerts (Slack only)
- Slow response time (P95 >100ms for 10 minutes)
- High memory usage (>500MB for 15 minutes)
- Low import rate (<0.1/hour for 2 hours)
- Download failure rate >20%

### Business Alerts
- No movie imports for 2+ hours
- High download failure rate
- Search performance degradation

## 🔧 Deployment Instructions

### Prerequisites
```bash
# Ensure monitoring namespace exists
kubectl create namespace monitoring --dry-run=client -o yaml | kubectl apply -f -

# Label namespace for network policies
kubectl label namespace monitoring name=monitoring

# Install kustomize if not available
curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
```

### Quick Deploy (All Components)
```bash
# Deploy entire observability stack
kubectl apply -k k8s/monitoring/

# Check deployment status
kubectl get pods -n monitoring
kubectl get svc -n monitoring
kubectl get ingress -n monitoring
```

### Individual Component Deployment
```bash
# Deploy only OpenTelemetry Collector
kubectl apply -f k8s/monitoring/otel-collector-config.yaml
kubectl apply -f k8s/monitoring/otel-collector.yaml

# Deploy only Prometheus
kubectl apply -f k8s/monitoring/prometheus-stack.yaml

# Deploy only Grafana
kubectl apply -f k8s/monitoring/grafana-deployment.yaml
kubectl apply -f k8s/monitoring/grafana-dashboards.yaml

# Deploy log stack
kubectl apply -f k8s/monitoring/loki-stack.yaml

# Deploy tracing
kubectl apply -f k8s/monitoring/tempo.yaml

# Deploy alerting
kubectl apply -f k8s/monitoring/alertmanager.yaml
```

### Verification Commands
```bash
# Check all monitoring pods are running
kubectl get pods -n monitoring

# Verify services are accessible
kubectl port-forward -n monitoring svc/grafana 3000:3000
kubectl port-forward -n monitoring svc/prometheus 9090:9090
kubectl port-forward -n monitoring svc/jaeger-query 16686:16686

# Check metrics ingestion
kubectl logs -n monitoring deployment/otel-collector -f

# Verify alert rules are loaded
kubectl port-forward -n monitoring svc/prometheus 9090:9090
# Visit http://localhost:9090/alerts
```

## 🌐 Access URLs (with Ingress)

Assuming your ingress is configured for `radarr.local`:

- **Grafana**: https://radarr.local/grafana
  - Username: `admin`
  - Password: `radarr-admin-2024`
- **Prometheus**: https://radarr.local/prometheus
- **Alertmanager**: https://radarr.local/alertmanager  
- **Jaeger UI**: https://radarr.local/jaeger
- **Radarr Metrics**: https://radarr.local/metrics

## 📈 Key Metrics Being Collected

### Application Metrics
```promql
# Request rate
rate(radarr_http_requests_total[5m])

# Error rate  
rate(radarr_http_requests_total{status=~"5.."}[5m]) / rate(radarr_http_requests_total[5m])

# Response time P95
histogram_quantile(0.95, rate(radarr_http_request_duration_seconds_bucket[5m]))

# Business metrics
rate(radarr_business_events_total{event_type="movie_added"}[1h])
rate(radarr_business_events_total{event_type="download_completed"}[1h])
```

### Infrastructure Metrics
```promql
# CPU usage
1 - rate(node_cpu_seconds_total{mode="idle"}[5m])

# Memory usage
1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)

# Database connections
radarr_system_metrics{metric_type="db_connections_active"} / radarr_system_metrics{metric_type="db_pool_size"}
```

## 📋 Trace Example

Example trace for movie search operation:
```
http_request (span_id: abc123)
├── business_operation: movie_search (span_id: def456)
│   ├── external_call: tmdb_api (span_id: ghi789)
│   └── external_call: prowlarr_search (span_id: jkl012)
├── database_operation: search_movies (span_id: mno345)
└── business_operation: log_search_result (span_id: pqr678)
```

## 🔧 Configuration Customization

### Environment Variables
```yaml
# OpenTelemetry Collector
ENVIRONMENT: production
CLUSTER_NAME: radarr-cluster
OTEL_EXPORTER_OTLP_ENDPOINT: http://otel-collector:4317

# Prometheus Remote Write (optional)
PROMETHEUS_REMOTE_WRITE_ENDPOINT: https://your-remote-write-endpoint
PROMETHEUS_REMOTE_WRITE_TOKEN: your-token

# Application telemetry
OTEL_TRACES_ENABLED: true
OTEL_METRICS_ENABLED: true
OTEL_LOGS_ENABLED: true
RUST_LOG: info
```

### Alert Configuration
Edit `k8s/monitoring/alertmanager.yaml` to configure:
- Slack webhook URLs
- PagerDuty integration keys
- Email recipients
- Alert routing rules

### Dashboard Customization
- Import additional dashboards via Grafana UI
- Edit existing dashboards in `k8s/monitoring/dashboards/`
- Add custom business metrics panels

## 🛠️ Troubleshooting

### Common Issues

**OpenTelemetry Collector not receiving data:**
```bash
# Check collector logs
kubectl logs -n monitoring deployment/otel-collector -f

# Verify OTLP endpoint is accessible from application
kubectl exec -it -n radarr-system deployment/radarr-api -- nslookup otel-collector.monitoring.svc.cluster.local
```

**Grafana dashboards not loading:**
```bash
# Check dashboard ConfigMaps
kubectl get configmaps -n monitoring | grep dashboard

# Verify dashboard provisioning
kubectl logs -n monitoring deployment/grafana -f | grep dashboard
```

**Prometheus not scraping targets:**
```bash
# Port-forward and check targets page
kubectl port-forward -n monitoring svc/prometheus 9090:9090
# Visit http://localhost:9090/targets
```

**No traces in Tempo:**
```bash
# Check if application is sending traces
kubectl logs -n radarr-system deployment/radarr-api | grep trace

# Verify Tempo is receiving data
kubectl logs -n monitoring deployment/tempo -f
```

### Performance Tuning

**High memory usage:**
- Adjust retention periods in configurations
- Increase resource limits for components
- Enable metric relabeling to reduce cardinality

**Slow query performance:**
- Add recording rules for complex queries
- Optimize dashboard queries
- Consider horizontal scaling

## 📊 Cost Optimization

### Storage Optimization
- **Prometheus**: 24h local retention + Thanos for long-term
- **Loki**: 7d retention for logs
- **Tempo**: 7d retention for traces
- **Metrics downsampling**: Automatic via Thanos

### Resource Requests/Limits
All components have production-ready resource configurations:
- Conservative requests for guaranteed resources
- Generous limits for burst capacity
- Horizontal Pod Autoscaling enabled where appropriate

## 🔒 Security Considerations

### Network Policies
- Ingress: Only from application pods and monitoring tools
- Egress: Only to required external services

### RBAC
- Minimal required permissions for each component
- Separate service accounts for each service

### Secrets Management
- TLS certificates for external endpoints
- API keys and tokens in Kubernetes secrets
- No hardcoded credentials in configurations

## 📚 Additional Resources

- [OpenTelemetry Documentation](https://opentelemetry.io/)
- [Prometheus Best Practices](https://prometheus.io/docs/practices/)
- [Grafana Dashboard Best Practices](https://grafana.com/docs/grafana/latest/best-practices/)
- [Kubernetes Monitoring Guide](https://kubernetes.io/docs/tasks/debug-application-cluster/resource-usage-monitoring/)

This observability stack provides enterprise-grade monitoring capabilities while maintaining simplicity and cost-effectiveness for the Radarr MVP.