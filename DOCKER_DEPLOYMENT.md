# Docker & Kubernetes Deployment Guide

This guide covers deployment of the Radarr MVP using Docker and Kubernetes.

## 🐳 Docker Deployment

### Prerequisites

- Docker Engine 20.10+
- Docker Compose 2.0+
- 4GB+ RAM available
- 20GB+ disk space

### Quick Start (Development)

1. **Clone and setup environment:**
```bash
cd unified-radarr
cp .env.docker .env
# Edit .env with your configuration
```

2. **Start services:**
```bash
# Start core services (PostgreSQL + Redis + Radarr)
docker-compose up -d

# Start with optional services (Prowlarr + qBittorrent)
docker-compose --profile optional up -d
```

3. **Verify deployment:**
```bash
# Check service status
docker-compose ps

# View logs
docker-compose logs radarr

# Access application
curl http://localhost:7878/health
```

### Production Deployment

1. **Environment setup:**
```bash
# Copy production environment template
cp .env.docker .env.production

# Configure production values
nano .env.production
```

2. **Deploy with production compose:**
```bash
# Build and deploy
docker-compose -f docker-compose.prod.yml up -d

# Check status
docker-compose -f docker-compose.prod.yml ps
```

### Build Custom Image

```bash
# Build image locally
./scripts/build-docker.sh

# Build with custom tag
./scripts/build-docker.sh --tag v1.0.0

# Build and push to registry
./scripts/build-docker.sh --registry your-registry.com --push
```

## ☸️ Kubernetes Deployment

### Prerequisites

- Kubernetes 1.24+
- kubectl configured
- kustomize 4.0+
- StorageClass `fast-ssd` available
- LoadBalancer or Ingress controller

### Quick Start

1. **Deploy to development:**
```bash
# Dry run first
./scripts/deploy-k8s.sh --environment dev --dry-run

# Deploy
./scripts/deploy-k8s.sh --environment dev
```

2. **Access the application:**
```bash
# Port forward for local access
kubectl port-forward -n radarr-mvp service/radarr-mvp-service 7878:7878

# Visit http://localhost:7878
```

### Production Deployment

1. **Update production secrets:**
```bash
# Edit production secrets
kubectl edit secret radarr-mvp-secrets -n radarr-mvp
```

2. **Deploy to production:**
```bash
./scripts/deploy-k8s.sh --environment prod
```

### Monitoring

Check deployment status:
```bash
# Pod status
kubectl get pods -n radarr-mvp

# Service status
kubectl get services -n radarr-mvp

# Recent events
kubectl get events -n radarr-mvp --sort-by='.lastTimestamp'

# Application logs
kubectl logs -n radarr-mvp deployment/radarr-mvp
```

## 🔧 Configuration

### Environment Variables

#### Core Configuration
| Variable | Description | Default |
|----------|-------------|---------|
| `RADARR_HOST` | Server bind address | `0.0.0.0` |
| `RADARR_PORT` | Server port | `7878` |
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `REDIS_URL` | Redis connection string | Optional |

#### External Services
| Variable | Description | Required |
|----------|-------------|----------|
| `PROWLARR_BASE_URL` | Prowlarr API URL | Yes |
| `PROWLARR_API_KEY` | Prowlarr API key | Yes |
| `QBITTORRENT_BASE_URL` | qBittorrent URL | Yes |
| `QBITTORRENT_USERNAME` | qBittorrent username | Yes |
| `QBITTORRENT_PASSWORD` | qBittorrent password | Yes |
| `TMDB_API_KEY` | TMDB API key | Yes |

#### Security
| Variable | Description | Required |
|----------|-------------|----------|
| `RADARR_API_KEY` | API authentication key | Yes |
| `JWT_SECRET` | JWT signing secret | Yes |

### Volume Mounts

#### Docker Compose
- `radarr_data:/var/lib/radarr` - Application data
- `radarr_logs:/var/log/radarr` - Application logs
- `${MOVIE_LIBRARY_PATH}:/media/movies` - Movie library
- `${DOWNLOAD_PATH}:/media/downloads` - Download directory

#### Kubernetes
- `radarr-data-pvc` - Application data (10Gi)
- `radarr-logs-pvc` - Application logs (5Gi)

## 🔍 Troubleshooting

### Common Issues

#### Container Won't Start
```bash
# Check logs
docker-compose logs radarr

# Common causes:
# - Invalid DATABASE_URL
# - Missing API keys
# - Port conflicts
```

#### Database Connection Failed
```bash
# Check PostgreSQL status
docker-compose logs postgres

# Verify database URL format:
# postgresql://username:password@host:port/database
```

#### High Memory Usage
```bash
# Check resource usage
docker stats

# Reduce memory limits in docker-compose.yml
# or increase available system memory
```

#### Kubernetes Pod CrashLoopBackOff
```bash
# Check pod events
kubectl describe pod -n radarr-mvp -l app.kubernetes.io/name=radarr-mvp

# Check logs
kubectl logs -n radarr-mvp -l app.kubernetes.io/name=radarr-mvp

# Common causes:
# - Invalid secrets
# - Resource limits too low
# - Health check failures
```

### Health Checks

#### Application Health
```bash
# Docker
curl http://localhost:7878/health

# Kubernetes
kubectl get pods -n radarr-mvp -w
```

#### Service Dependencies
```bash
# PostgreSQL
docker-compose exec postgres pg_isready -U radarr

# Redis
docker-compose exec redis redis-cli ping
```

## 🔒 Security Considerations

### Production Checklist

- [ ] Change all default passwords
- [ ] Use strong API keys (32+ characters)
- [ ] Configure TLS/SSL termination
- [ ] Set up proper backup procedures
- [ ] Configure monitoring and alerts
- [ ] Limit resource consumption
- [ ] Use non-root containers
- [ ] Enable audit logging

### Network Security

#### Docker
- Services isolated in custom network
- Only expose necessary ports
- Use secrets for sensitive data

#### Kubernetes
- NetworkPolicies restrict traffic
- RBAC controls access
- Secrets encrypted at rest
- Non-root security context

## 📊 Performance Tuning

### Docker Resources

```yaml
# Adjust in docker-compose.yml
services:
  radarr:
    deploy:
      resources:
        limits:
          memory: 1G
          cpus: '1.0'
        reservations:
          memory: 256M
          cpus: '0.25'
```

### Kubernetes Resources

```yaml
# Adjust in deployment.yaml
resources:
  requests:
    cpu: 100m
    memory: 256Mi
  limits:
    cpu: 500m
    memory: 512Mi
```

### Database Optimization

```bash
# PostgreSQL tuning in docker-compose.prod.yml
command: >
  postgres
  -c max_connections=200
  -c shared_buffers=256MB
  -c effective_cache_size=1GB
```

## 🚀 Scaling

### Horizontal Scaling

#### Kubernetes
```bash
# Scale deployment
kubectl scale deployment radarr-mvp --replicas=3 -n radarr-mvp

# Auto-scaling (HPA already configured)
# Scales based on CPU/memory usage
```

### Vertical Scaling

#### Increase Resources
```bash
# Docker Compose
# Edit docker-compose.yml and restart

# Kubernetes
kubectl patch deployment radarr-mvp -n radarr-mvp -p '{"spec":{"template":{"spec":{"containers":[{"name":"radarr-mvp","resources":{"limits":{"memory":"1Gi"}}}]}}}}'
```

## 🔄 Updates and Maintenance

### Update Application

#### Docker
```bash
# Pull latest image
docker-compose pull

# Restart services
docker-compose up -d
```

#### Kubernetes
```bash
# Update image tag in kustomization
# Then apply changes
./scripts/deploy-k8s.sh --environment prod
```

### Backup and Restore

#### Database Backup
```bash
# Create backup
docker-compose exec postgres pg_dump -U radarr radarr > backup.sql

# Restore
docker-compose exec -T postgres psql -U radarr radarr < backup.sql
```

#### Application Data
```bash
# Backup volumes
docker run --rm -v radarr_data:/data -v $(pwd):/backup busybox tar czf /backup/radarr-data.tar.gz -C /data .

# Restore
docker run --rm -v radarr_data:/data -v $(pwd):/backup busybox tar xzf /backup/radarr-data.tar.gz -C /data
```