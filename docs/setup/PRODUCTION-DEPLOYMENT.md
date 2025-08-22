# Radarr MVP Production Infrastructure Guide

## Overview

This guide provides comprehensive instructions for deploying the Radarr MVP automation system to production. The infrastructure is designed for enterprise-grade reliability, security, and scalability.

## Architecture

### Components

- **Application Layer**: Rust-based Radarr MVP with TDD automation
- **Container Orchestration**: Kubernetes with multi-node cluster
- **Database**: PostgreSQL 15 with high availability
- **Cache**: Redis with persistence
- **Load Balancer**: Nginx with SSL termination
- **Monitoring**: Prometheus + Grafana + Loki
- **Service Mesh**: Built-in Kubernetes networking with NetworkPolicies

### Infrastructure Patterns

- **Multi-tier Architecture**: Application, database, and monitoring tiers
- **High Availability**: 3+ replicas with pod anti-affinity
- **Auto-scaling**: HPA based on CPU, memory, and custom metrics
- **Security**: NetworkPolicies, RBAC, and container security
- **Observability**: Metrics, logs, and distributed tracing

## Prerequisites

### Required Tools

```bash
# Kubernetes tools
kubectl >= 1.28
kustomize >= 5.0
helm >= 3.12

# Cloud provider CLI
gcloud # for GCP
aws # for AWS
az # for Azure

# Container tools
docker >= 24.0
trivy # security scanning

# Infrastructure as Code
terraform >= 1.5
```

### Cloud Resources

- **Kubernetes Cluster**: 3+ nodes (e2-standard-4 or equivalent)
- **Storage**: SSD persistent volumes (200GB+ total)
- **Network**: VPC with private subnets
- **Load Balancer**: Cloud load balancer with static IP
- **DNS**: Domain with SSL certificates

## Quick Start

### 1. Clone and Setup

```bash
git clone <repository-url>
cd radarr-mvp

# Set environment variables
export DEPLOYMENT_ENV=production
export CLOUD_PROVIDER=gcp
export GCP_PROJECT_ID=your-project-id
export DOMAIN_NAME=radarr.yourdomain.com
```

### 2. Deploy Infrastructure

```bash
# Deploy cloud infrastructure
cd infrastructure/terraform
terraform init
terraform plan -var="project_id=${GCP_PROJECT_ID}"
terraform apply

# Get cluster credentials
gcloud container clusters get-credentials radarr-mvp-prod --region us-central1
```

### 3. Deploy Application

```bash
# Run production deployment
./scripts/deploy-production.sh --environment production --provider gcp

# Verify deployment
kubectl get pods -n radarr-mvp
kubectl get svc -n radarr-mvp
```

## Detailed Deployment

### Infrastructure Deployment

#### 1. Terraform Infrastructure

```bash
cd infrastructure/terraform

# Initialize Terraform
terraform init

# Review planned changes
terraform plan -var="project_id=${GCP_PROJECT_ID}" \
               -var="region=us-central1" \
               -var="environment=production"

# Apply infrastructure
terraform apply

# Note the outputs
terraform output
```

#### 2. Kubernetes Setup

```bash
# Configure kubectl
gcloud container clusters get-credentials radarr-mvp-prod --region us-central1

# Verify cluster access
kubectl cluster-info
kubectl get nodes
```

### Application Deployment

#### 1. Build and Push Images

```bash
# Build production image
docker build -f Dockerfile.production -t gcr.io/${GCP_PROJECT_ID}/radarr-mvp:$(git rev-parse --short HEAD) .

# Security scan
trivy image gcr.io/${GCP_PROJECT_ID}/radarr-mvp:$(git rev-parse --short HEAD)

# Push to registry
docker push gcr.io/${GCP_PROJECT_ID}/radarr-mvp:$(git rev-parse --short HEAD)
```

#### 2. Deploy Kubernetes Resources

```bash
# Update image tag in kustomization
cd k8s/overlays/prod
sed -i "s|newTag: .*|newTag: \"$(git rev-parse --short HEAD)\"|" kustomization.yaml

# Generate and apply manifests
kustomize build . | kubectl apply -f -

# Wait for rollout
kubectl rollout status deployment/radarr-mvp -n radarr-mvp
```

#### 3. Configure Secrets

```bash
# Create production secrets
kubectl create secret generic radarr-mvp-secrets -n radarr-mvp \
  --from-literal=POSTGRES_PASSWORD="$(openssl rand -base64 32)" \
  --from-literal=REDIS_PASSWORD="$(openssl rand -base64 32)" \
  --from-literal=HDBITS_API_KEY="your-hdbits-key" \
  --from-literal=TMDB_API_KEY="your-tmdb-key" \
  --from-literal=JWT_SECRET="$(openssl rand -base64 64)" \
  --from-literal=GRAFANA_PASSWORD="$(openssl rand -base64 32)"

# Verify secrets
kubectl get secrets -n radarr-mvp
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|----------|
| `DEPLOYMENT_ENV` | Environment name | `production` |
| `CLOUD_PROVIDER` | Cloud provider | `gcp` |
| `GCP_PROJECT_ID` | GCP project ID | |
| `DOMAIN_NAME` | Application domain | |
| `IMAGE_TAG` | Docker image tag | `git-commit-hash` |
| `REPLICA_COUNT` | Application replicas | `3` |

### Resource Limits

```yaml
# Application pods
resources:
  requests:
    cpu: 250m
    memory: 512Mi
  limits:
    cpu: 1000m
    memory: 1Gi

# Database pods
resources:
  requests:
    cpu: 250m
    memory: 512Mi
  limits:
    cpu: 1000m
    memory: 2Gi
```

### Scaling Configuration

```yaml
# Horizontal Pod Autoscaler
minReplicas: 3
maxReplicas: 10
targetCPUUtilizationPercentage: 70
targetMemoryUtilizationPercentage: 80
```

## Monitoring

### Accessing Dashboards

```bash
# Port forward to Grafana
kubectl port-forward svc/grafana-service 3000:3000 -n radarr-mvp

# Access Grafana at http://localhost:3000
# Username: admin
# Password: (from secret)

# Port forward to Prometheus
kubectl port-forward svc/prometheus-service 9090:9090 -n radarr-mvp
```

### Key Metrics

- **Application Health**: `up{job="radarr-mvp"}`
- **Request Rate**: `rate(http_requests_total[5m])`
- **Response Time**: `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))`
- **Error Rate**: `rate(http_requests_total{status=~"5.."}[5m])`
- **Download Queue**: `radarr_download_queue_size`

### Alerts

Key alerts are configured for:
- Application downtime
- High error rates
- Performance degradation
- Database issues
- Infrastructure problems

## Security

### Network Security

- **NetworkPolicies**: Restrict pod-to-pod communication
- **Ingress**: TLS termination with strong ciphers
- **Service Mesh**: mTLS between services (optional)

### Container Security

- **Non-root User**: All containers run as non-root
- **Read-only Filesystem**: Application containers use read-only root filesystem
- **Security Scanning**: Trivy scans for vulnerabilities
- **Pod Security Standards**: Restricted security context

### Access Control

- **RBAC**: Role-based access control for service accounts
- **Workload Identity**: Secure cloud resource access
- **Secrets Management**: Encrypted secrets with rotation

## Backup and Recovery

### Automated Backups

```bash
# PostgreSQL backups (daily at 2 AM)
kubectl get cronjob postgres-backup -n radarr-mvp

# Redis backups (daily at 3 AM)
kubectl get cronjob redis-backup -n radarr-mvp

# List backup files
kubectl exec -it postgres-backup-xxx -n radarr-mvp -- ls -la /backup
```

### Disaster Recovery

1. **Database Recovery**:
   ```bash
   # Restore PostgreSQL from backup
   kubectl exec -it postgres-0 -n radarr-mvp -- psql -U radarr -d radarr < backup_file.sql
   ```

2. **Application Recovery**:
   ```bash
   # Rollback deployment
   kubectl rollout undo deployment/radarr-mvp -n radarr-mvp
   ```

3. **Complete Cluster Recovery**:
   ```bash
   # Redeploy infrastructure
   terraform apply
   ./scripts/deploy-production.sh
   ```

## Performance Tuning

### Database Optimization

- **Connection Pooling**: PgBouncer for connection management
- **Indexing**: Optimized indexes for common queries
- **Memory Settings**: Tuned PostgreSQL memory parameters
- **Storage**: SSD storage with appropriate IOPS

### Application Optimization

- **Resource Limits**: Properly configured CPU and memory limits
- **JVM Tuning**: Optimized garbage collection (if applicable)
- **Connection Pools**: HTTP and database connection pooling
- **Caching**: Redis for application-level caching

### Infrastructure Optimization

- **Node Types**: Separate node pools for different workload types
- **Network**: Optimized networking with proper routing
- **Storage**: SSD storage classes for performance
- **Load Balancing**: Efficient load balancing algorithms

## Troubleshooting

### Common Issues

1. **Pod Startup Issues**:
   ```bash
   kubectl describe pod radarr-mvp-xxx -n radarr-mvp
   kubectl logs radarr-mvp-xxx -n radarr-mvp
   ```

2. **Database Connection Issues**:
   ```bash
   kubectl exec -it postgres-0 -n radarr-mvp -- pg_isready
   kubectl logs postgres-0 -n radarr-mvp
   ```

3. **Network Issues**:
   ```bash
   kubectl get networkpolicies -n radarr-mvp
   kubectl describe networkpolicy radarr-mvp-netpol -n radarr-mvp
   ```

4. **Performance Issues**:
   ```bash
   kubectl top pods -n radarr-mvp
   kubectl get hpa -n radarr-mvp
   ```

### Debugging Commands

```bash
# Check all resources
kubectl get all -n radarr-mvp

# Check events
kubectl get events -n radarr-mvp --sort-by='.lastTimestamp'

# Check resource usage
kubectl top nodes
kubectl top pods -n radarr-mvp

# Debug networking
kubectl exec -it radarr-mvp-xxx -n radarr-mvp -- netstat -tulpn
kubectl exec -it radarr-mvp-xxx -n radarr-mvp -- nslookup postgres-service
```

## Maintenance

### Regular Tasks

1. **Security Updates**: Monthly security patches
2. **Backup Verification**: Weekly backup restoration tests
3. **Performance Review**: Monthly performance analysis
4. **Cost Optimization**: Quarterly resource optimization

### Upgrade Procedures

1. **Application Updates**:
   ```bash
   # Build new image
   docker build -f Dockerfile.production -t gcr.io/${GCP_PROJECT_ID}/radarr-mvp:v1.1.0 .
   docker push gcr.io/${GCP_PROJECT_ID}/radarr-mvp:v1.1.0
   
   # Update deployment
   kubectl set image deployment/radarr-mvp radarr-mvp=gcr.io/${GCP_PROJECT_ID}/radarr-mvp:v1.1.0 -n radarr-mvp
   kubectl rollout status deployment/radarr-mvp -n radarr-mvp
   ```

2. **Infrastructure Updates**:
   ```bash
   # Update Terraform
   terraform plan
   terraform apply
   
   # Update Kubernetes
   gcloud container clusters upgrade radarr-mvp-prod --region us-central1
   ```

## Support

### Health Checks

- **Application**: `https://radarr.yourdomain.com/health`
- **Monitoring**: `https://monitoring.yourdomain.com/grafana`
- **Metrics**: `https://monitoring.yourdomain.com/prometheus`

### Contact Information

- **Operations Team**: ops@yourdomain.com
- **On-call**: +1-xxx-xxx-xxxx
- **Escalation**: escalation@yourdomain.com

---

**Note**: This production deployment is designed for enterprise use with proper security, monitoring, and reliability measures. Ensure all credentials are properly secured and regularly rotated.