# Radarr MVP Production Deployment Guide

Comprehensive guide for deploying Radarr MVP TDD automation system to production with enterprise-grade CI/CD, monitoring, and security.

## 📋 Prerequisites

### System Requirements
- **CPU**: 2+ cores (4+ recommended)
- **RAM**: 4GB minimum (8GB recommended)
- **Storage**: 20GB+ SSD storage
- **OS**: Linux (Ubuntu 20.04+ or CentOS 8+)

### Required Services
- **Docker**: 20.10+
- **Docker Compose**: 2.0+
- **Git**: Latest version
- **Domain**: SSL certificate required for production

### External Dependencies
- **HDBits API Key**: Private tracker integration
- **TMDB API Key**: Movie metadata
- **PostgreSQL**: Database (included in compose)
- **Redis**: Caching layer (included in compose)

## 🚀 Quick Production Deployment

### 1. Clone and Configure

```bash
# Clone repository
git clone <repository-url>
cd radarr-mvp

# Copy environment configuration
cp .env.production.example .env.production

# Edit configuration with your values
nano .env.production
```

### 2. Required Environment Variables

```bash
# Critical Configuration (MUST be set)
POSTGRES_PASSWORD=your_secure_db_password
HDBITS_API_KEY=your_hdbits_api_key
TMDB_API_KEY=your_tmdb_api_key
REDIS_PASSWORD=your_redis_password
GRAFANA_PASSWORD=your_grafana_password
JWT_SECRET=$(openssl rand -hex 64)
SESSION_SECRET=$(openssl rand -hex 32)
```

### 3. Deploy Services

```bash
# Run deployment script
./scripts/deployment/deploy.sh

# Or manual deployment
docker-compose -f docker-compose.production.yml --env-file .env.production up -d
```

### 4. Verify Deployment

```bash
# Check service health
curl http://localhost:8080/health

# View service status
docker-compose -f docker-compose.production.yml ps

# Check logs
docker-compose -f docker-compose.production.yml logs -f radarr-mvp
```

## 🏗️ CI/CD Pipeline Architecture

### GitHub Actions Workflow

The production CI/CD pipeline includes:

1. **Multi-Stage Testing**
   - Unit tests with PostgreSQL
   - Integration tests with real APIs
   - Property-based testing
   - Test coverage reporting

2. **Security Scanning**
   - Dependency vulnerability scanning (cargo-audit)
   - Container security scanning (Trivy)
   - License compliance checking (cargo-deny)
   - SARIF integration with GitHub Security tab

3. **Performance Testing**
   - Automated benchmarks
   - Performance regression detection
   - Memory usage analysis
   - Criterion.rs integration

4. **Container Build & Deploy**
   - Multi-stage Docker builds
   - Layer caching optimization
   - Multi-architecture support (AMD64/ARM64)
   - Automated deployment to staging/production

### Pipeline Triggers

```yaml
# Automatic triggers
on:
  push:
    branches: [ main, master, develop ]
  pull_request:
    branches: [ main, master ]

# Manual deployment
workflow_dispatch:
  inputs:
    environment:
      description: 'Deployment environment'
      required: true
      default: 'staging'
```

### Required GitHub Secrets

```bash
# Docker Registry
DOCKER_USERNAME=your_docker_username
DOCKER_PASSWORD=your_docker_token

# API Keys
HDBITS_API_KEY=your_hdbits_key
TMDB_API_KEY=your_tmdb_key

# Coverage Reporting
CODECOV_TOKEN=your_codecov_token

# Production Deployment
PRODUCTION_SSH_KEY=your_deployment_ssh_key
PRODUCTION_HOST=your_production_server
```

## 🐳 Docker Production Configuration

### Multi-Stage Build Benefits

1. **Build Optimization**
   - Cached dependency layer
   - Minimal runtime image
   - Security hardening

2. **Performance Features**
   - Non-root user execution
   - Health check integration
   - Resource constraints
   - Signal handling

### Production Dockerfile Features

```dockerfile
# Security hardening
USER radarr
EXPOSE 8080
HEALTHCHECK --interval=30s CMD curl -f http://localhost:8080/health

# Resource optimization
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
VOLUME ["/app/data"]
```

## 📊 Monitoring & Observability

### Integrated Monitoring Stack

1. **Prometheus**: Metrics collection and alerting
2. **Grafana**: Visualization and dashboards
3. **Loki**: Log aggregation and analysis
4. **Promtail**: Log shipping and processing

### Key Metrics

- **Application Metrics**
  - Request latency (p50, p95, p99)
  - Error rates by endpoint
  - Active automation workflows
  - HDBits API response times

- **System Metrics**
  - CPU and memory usage
  - Disk I/O and space
  - Network throughput
  - Container health status

- **Business Metrics**
  - Movies processed per hour
  - Download success rate
  - Quality profile effectiveness
  - User satisfaction scores

### Access Monitoring Services

```bash
# Grafana Dashboard
http://localhost:3000
# Login: admin / $GRAFANA_PASSWORD

# Prometheus Metrics
http://localhost:9090

# Application Health
http://localhost:8080/health

# Application Metrics
http://localhost:8080/metrics
```

## 🔒 Security Implementation

### Security Scanning Integration

Automated security scanning includes:

1. **Dependency Scanning**
   - Known vulnerability detection
   - License compliance
   - Supply chain security

2. **Container Security**
   - Base image vulnerabilities
   - Configuration security
   - Runtime security policies

3. **Code Analysis**
   - Static analysis (Clippy)
   - Secret detection
   - Security lint rules

### Security Best Practices

```bash
# Run security scan
./scripts/security-scan.sh

# Check for secrets
./scripts/security-scan.sh --secrets-only

# Container security
docker scan radarr-mvp:latest
```

### Production Security Checklist

- [ ] SSL/TLS certificates configured
- [ ] Non-root container execution
- [ ] Secret management via environment variables
- [ ] Network security groups configured
- [ ] Regular security updates scheduled
- [ ] Backup encryption enabled
- [ ] Audit logging configured

## ⚡ Performance Optimization

### Automated Performance Testing

```bash
# Run performance analysis
./scripts/performance-analysis.sh

# Create performance baseline
./scripts/performance-analysis.sh --baseline-only

# Custom regression threshold
./scripts/performance-analysis.sh --threshold=15
```

### Performance Targets

- **Parser Performance**: <50μs per release
- **API Response Time**: <200ms (p95)
- **Database Queries**: <100ms average
- **Memory Usage**: <1GB under normal load
- **CPU Usage**: <50% average

### Optimization Features

1. **Database Optimization**
   - Connection pooling
   - Query optimization
   - Index management
   - Read replicas for scaling

2. **Caching Strategy**
   - Redis for session data
   - Application-level caching
   - CDN for static assets
   - Query result caching

3. **Resource Management**
   - Container resource limits
   - Auto-scaling policies
   - Load balancing
   - Circuit breaker patterns

## 🔄 Deployment Strategies

### Blue-Green Deployment

```bash
# Deploy to staging environment
docker-compose -f docker-compose.production.yml up -d --scale radarr-mvp=0

# Health check new version
curl http://localhost:8081/health

# Switch traffic
./scripts/deployment/switch-traffic.sh

# Rollback if needed
./scripts/deployment/rollback.sh
```

### Rolling Updates

```bash
# Rolling update with health checks
docker-compose -f docker-compose.production.yml up -d --scale radarr-mvp=2
docker-compose -f docker-compose.production.yml up -d --scale radarr-mvp=1
```

### Canary Deployments

```bash
# Route 10% traffic to new version
./scripts/deployment/canary.sh --percentage=10

# Monitor metrics for issues
./scripts/deployment/monitor-canary.sh

# Full deployment if successful
./scripts/deployment/promote-canary.sh
```

## 🚨 Troubleshooting

### Common Issues

1. **Database Connection Failed**
   ```bash
   # Check PostgreSQL status
   docker-compose -f docker-compose.production.yml logs postgres
   
   # Verify credentials
   grep POSTGRES .env.production
   ```

2. **API Key Authentication Failed**
   ```bash
   # Test HDBits API key
   curl -H "Authorization: Bearer $HDBITS_API_KEY" https://hdbits.org/api/torrents
   
   # Check environment variables
   docker-compose -f docker-compose.production.yml exec radarr-mvp env | grep API
   ```

3. **High Memory Usage**
   ```bash
   # Monitor memory usage
   docker stats radarr-mvp_radarr-mvp_1
   
   # Check for memory leaks
   docker-compose -f docker-compose.production.yml exec radarr-mvp top
   ```

4. **Performance Degradation**
   ```bash
   # Run performance analysis
   ./scripts/performance-analysis.sh
   
   # Check database performance
   docker-compose -f docker-compose.production.yml exec postgres pg_stat_activity
   ```

### Log Analysis

```bash
# Application logs
docker-compose -f docker-compose.production.yml logs -f radarr-mvp

# Database logs
docker-compose -f docker-compose.production.yml logs -f postgres

# Nginx access logs
docker-compose -f docker-compose.production.yml logs -f nginx

# All service logs
docker-compose -f docker-compose.production.yml logs -f
```

### Health Check Endpoints

```bash
# Application health
curl http://localhost:8080/health
# Response: {"status":"healthy","version":"1.0.0","uptime":"2h 15m"}

# Database health
curl http://localhost:8080/health/database
# Response: {"status":"healthy","connection_pool":"active"}

# External services health
curl http://localhost:8080/health/external
# Response: {"hdbits":"healthy","tmdb":"healthy"}
```

## 📈 Scaling Considerations

### Horizontal Scaling

1. **Load Balancer Configuration**
   - Multiple application instances
   - Session affinity handling
   - Health check integration

2. **Database Scaling**
   - Read replicas for queries
   - Connection pooling
   - Query optimization

3. **Caching Layer**
   - Redis clustering
   - Cache invalidation strategy
   - Session management

### Vertical Scaling

```yaml
# Resource allocation
services:
  radarr-mvp:
    deploy:
      resources:
        limits:
          memory: 2G
          cpus: '1.0'
        reservations:
          memory: 1G
          cpus: '0.5'
```

## 🔧 Maintenance

### Regular Maintenance Tasks

1. **Security Updates**
   ```bash
   # Update base images
   docker-compose -f docker-compose.production.yml pull
   
   # Rebuild with latest security patches
   docker-compose -f docker-compose.production.yml build --no-cache
   ```

2. **Database Maintenance**
   ```bash
   # Backup database
   ./scripts/deployment/backup-database.sh
   
   # Vacuum and analyze
   docker-compose -f docker-compose.production.yml exec postgres psql -U radarr -c "VACUUM ANALYZE;"
   ```

3. **Log Rotation**
   ```bash
   # Configure log rotation
   docker-compose -f docker-compose.production.yml exec nginx logrotate /etc/logrotate.conf
   ```

4. **Performance Monitoring**
   ```bash
   # Weekly performance analysis
   ./scripts/performance-analysis.sh --weekly-report
   
   # Security scan
   ./scripts/security-scan.sh --full-scan
   ```

### Backup Strategy

```bash
# Database backup
pg_dump -h localhost -U radarr -d radarr > backup_$(date +%Y%m%d).sql

# Configuration backup
tar -czf config_backup_$(date +%Y%m%d).tar.gz .env.production nginx/ monitoring/

# Application data backup
tar -czf data_backup_$(date +%Y%m%d).tar.gz volumes/radarr_data/
```

## 📞 Support

### Monitoring Alerts

- **Critical**: Service down, database unreachable
- **High**: High error rate, performance degradation
- **Medium**: Disk space low, memory usage high
- **Low**: Dependency updates available

### Contact Information

- **Technical Issues**: Create GitHub issue
- **Security Concerns**: security@yourdomain.com
- **Performance Issues**: Include performance report

### Documentation

- **API Documentation**: `/api/docs`
- **Architecture Decisions**: `/docs/architecture/`
- **TDD Foundation**: `/TDD_FOUNDATION.md`
- **Test Coverage**: `/TEST_SUITE_SUMMARY.md`

---

**Production Deployment Complete!** 🎉

Your Radarr MVP TDD automation system is now running with enterprise-grade CI/CD, comprehensive monitoring, and production-ready security.