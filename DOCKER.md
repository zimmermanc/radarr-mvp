# Docker Setup Guide for Radarr MVP

This guide covers the complete Docker setup for the Radarr MVP project, including development and production configurations.

## Quick Start

### 1. Prerequisites

- Docker Engine 20.10+ and Docker Compose V2
- At least 2GB of available RAM
- 10GB of free disk space for development

### 2. Environment Setup

```bash
# Clone and navigate to project
cd unified-radarr

# Copy environment template
cp .env.docker .env

# Edit environment variables (required)
nano .env
```

**Required Environment Variables:**
- `POSTGRES_PASSWORD` - Set a secure password
- `PROWLARR_API_KEY` - Get from Prowlarr settings
- `QBITTORRENT_PASSWORD` - Your qBittorrent password
- `TMDB_API_KEY` - Get from themoviedb.org (free)

### 3. Start Development Environment

```bash
# Option 1: Using the startup script (recommended)
./scripts/docker-compose-up.sh

# Option 2: Using Docker Compose directly
docker compose up
```

### 4. Access Services

- **Radarr MVP**: http://localhost:7878
- **Metrics**: http://localhost:9090
- **PostgreSQL**: localhost:5432
- **Redis**: localhost:6379

For full-stack with optional services:

```bash
./scripts/docker-compose-up.sh -p full-stack
```

- **Prowlarr**: http://localhost:9696
- **qBittorrent**: http://localhost:8080

## Docker Configuration Files

### Core Files

| File | Purpose |
|------|---------|
| `docker-compose.yml` | Main composition with core services |
| `docker-compose.override.yml` | Development overrides (auto-applied) |
| `docker-compose.prod.yml` | Production configuration |
| `Dockerfile` | Multi-stage application build |
| `.dockerignore` | Build optimization |
| `.env.docker` | Environment template |

### Scripts

| Script | Purpose |
|--------|---------|
| `scripts/docker-compose-up.sh` | Easy startup with environment selection |
| `scripts/docker-entrypoint.sh` | Container initialization and health checks |
| `scripts/init-db.sql` | Production database initialization |
| `scripts/dev-db-init.sql` | Development database with test data |
| `scripts/prod-db-init.sql` | Production database with monitoring |

## Environment Configurations

### Development Environment (Default)

**Features:**
- Hot reload with `cargo watch`
- Debug logging enabled
- Source code mounted for live editing
- Test data pre-loaded
- Relaxed security settings
- Direct port exposure for debugging

**Start Development:**

```bash
# Start with hot reload
./scripts/docker-compose-up.sh

# With full stack (Prowlarr + qBittorrent)
./scripts/docker-compose-up.sh -p full-stack

# Build from scratch
./scripts/docker-compose-up.sh --build

# With logs
./scripts/docker-compose-up.sh --logs
```

### Production Environment

**Features:**
- Optimized Rust binary
- Structured JSON logging
- Resource limits enforced
- Security hardening enabled
- Health checks configured
- No port exposure (internal only)

**Start Production:**

```bash
# Production mode
./scripts/docker-compose-up.sh -e production -d

# With build
./scripts/docker-compose-up.sh -e production --build -d

# Scale application
./scripts/docker-compose-up.sh -e production --scale radarr=2
```

### Staging Environment

Similar to production but with additional debugging capabilities:

```bash
./scripts/docker-compose-up.sh -e staging
```

## Service Architecture

### Core Services

#### Radarr MVP Application
- **Build**: Multi-stage Rust compilation
- **Base**: Debian bullseye-slim
- **User**: Non-root (UID 1001)
- **Health Check**: HTTP endpoint on `/health`
- **Volumes**: Configuration, logs, media directories
- **Security**: Read-only filesystem, no-new-privileges

#### PostgreSQL Database
- **Version**: PostgreSQL 16 Alpine
- **Features**: Extensions enabled (uuid-ossp, pg_trgm, btree_gin)
- **Volumes**: Persistent data storage
- **Health Check**: `pg_isready`
- **Configuration**: Optimized for OLTP workloads
- **Monitoring**: Query logging, performance stats

#### Redis Cache
- **Version**: Redis 7 Alpine
- **Configuration**: Persistence enabled, memory limits
- **Health Check**: PING command
- **Usage**: Caching, session storage, job queues

### Optional Services (Profile: full-stack)

#### Prowlarr
- **Purpose**: Indexer management
- **Image**: ghcr.io/hotio/prowlarr
- **Port**: 9696

#### qBittorrent
- **Purpose**: Download client
- **Image**: ghcr.io/hotio/qbittorrent
- **Ports**: 8080 (Web UI), 6881 (BitTorrent)

## Volume Management

### Development Volumes
```bash
# Local bind mounts for development
- ./src:/app/src:ro              # Source code (read-only)
- ./crates:/app/crates:ro        # Crate source code
- ./dev-data/movies:/movies      # Development movies
- ./dev-data/downloads:/downloads # Development downloads
```

### Production Volumes
```bash
# Named volumes for production
- postgres_data:/var/lib/postgresql/data
- radarr_config:/app/config
- radarr_logs:/app/logs
- /opt/radarr/movies:/movies     # Production movies
- /opt/radarr/downloads:/downloads # Production downloads
```

## Networking

### Development Network
- **Subnet**: 172.20.0.0/16
- **Name**: radarr-network
- **Mode**: Bridge with ICC enabled
- **Port Exposure**: All ports exposed for debugging

### Production Network
- **Subnet**: Configurable via DOCKER_SUBNET
- **Name**: radarr-prod
- **Mode**: Bridge with ICC disabled
- **Port Exposure**: Minimal (only necessary ports)

## Security Features

### Container Security
- Non-root user (UID 1001)
- Read-only filesystem where possible
- No-new-privileges security option
- Minimal base images (Alpine/Debian slim)
- Security scanning integrated

### Network Security
- Inter-container communication control
- Internal-only services in production
- TLS/SSL certificate support
- CORS restrictions configurable

### Data Security
- Secret management via environment variables
- Volume permission management
- Audit trail logging in production
- Database encryption at rest support

## Monitoring and Logging

### Health Checks
- Application: HTTP endpoint `/health`
- Database: PostgreSQL connection test
- Cache: Redis PING command
- Interval: 30s with configurable timeouts

### Logging
- **Development**: Human-readable format
- **Production**: Structured JSON logging
- **Rotation**: Configurable size and file limits
- **Aggregation**: Compatible with ELK/Fluentd

### Metrics
- Prometheus-compatible metrics on port 9090
- Application performance metrics
- Database connection pool metrics
- Resource usage monitoring

## Troubleshooting

### Common Issues

#### Database Connection Failed
```bash
# Check database status
docker compose ps postgres

# View database logs
docker compose logs postgres

# Test connection manually
docker compose exec postgres pg_isready -U radarr -d radarr
```

#### Build Failures
```bash
# Clean build
docker compose build --no-cache

# Check build logs
docker compose build radarr 2>&1 | tee build.log

# Free up space
docker system prune -a
```

#### Permission Issues
```bash
# Fix volume permissions
sudo chown -R 1001:1001 ./data/
sudo chown -R 1001:1001 ./dev-data/

# Check container user
docker compose exec radarr id
```

#### Performance Issues
```bash
# Check resource usage
docker stats

# View application logs
docker compose logs -f radarr

# Monitor database queries
docker compose exec postgres tail -f /var/log/postgresql/postgresql.log
```

### Debug Mode

Start with debug logging enabled:

```bash
# Enable debug mode
echo "DEBUG=true" >> .env
echo "RUST_LOG=debug" >> .env

# Restart with debug
./scripts/docker-compose-up.sh --build --logs
```

### Database Debugging

```bash
# Connect to database
docker compose exec postgres psql -U radarr -d radarr

# Run database migrations manually
docker compose exec radarr radarr-mvp migrate

# Reset development database
docker compose exec postgres psql -U radarr_dev -d radarr_dev -c "SELECT reset_test_data();"
```

## Production Deployment

### Pre-deployment Checklist

1. **Environment Variables**:
   - [ ] Strong passwords set for all services
   - [ ] API keys configured and valid
   - [ ] SSL certificates available if using HTTPS
   - [ ] Resource limits appropriate for hardware

2. **Security**:
   - [ ] Firewall configured
   - [ ] Non-root user configured
   - [ ] Secrets management in place
   - [ ] Backup strategy implemented

3. **Performance**:
   - [ ] Database connection limits configured
   - [ ] Memory limits set appropriately
   - [ ] Disk space monitoring enabled
   - [ ] Log rotation configured

### Production Deployment

```bash
# 1. Prepare environment
cp .env.docker .env.prod
# Edit .env.prod with production values

# 2. Create production directories
sudo mkdir -p /opt/radarr/{movies,downloads,config,logs,postgres,redis}
sudo chown -R 1001:1001 /opt/radarr/

# 3. Deploy
docker compose -f docker-compose.yml -f docker-compose.prod.yml --env-file .env.prod up -d

# 4. Verify deployment
docker compose -f docker-compose.yml -f docker-compose.prod.yml ps
curl -f http://localhost:7878/health
```

### Backup Strategy

```bash
# Database backup
docker compose exec postgres pg_dump -U radarr radarr > backup-$(date +%Y%m%d).sql

# Full backup with volumes
docker run --rm -v radarr_postgres_data:/data -v $(pwd):/backup ubuntu tar czf /backup/postgres-backup-$(date +%Y%m%d).tar.gz /data
```

### Updates and Maintenance

```bash
# Update images
docker compose pull

# Apply updates
docker compose up -d

# Clean up old images
docker image prune -a

# Update database (if needed)
docker compose exec radarr radarr-mvp migrate
```

## Integration with External Services

### Prowlarr Integration
1. Start Prowlarr: `./scripts/docker-compose-up.sh -p full-stack`
2. Configure indexers in Prowlarr UI (localhost:9696)
3. Add Radarr application in Prowlarr settings
4. Set API key in Radarr environment

### qBittorrent Integration
1. Start qBittorrent with full-stack profile
2. Login to qBittorrent UI (localhost:8080)
3. Configure download directories
4. Set authentication in Radarr environment

### External Monitoring
1. Prometheus metrics available on port 9090
2. Health check endpoint: `/health`
3. Detailed status: `/api/v1/system/status`

## Performance Optimization

### Resource Limits
```yaml
# Example production limits
services:
  radarr:
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 1G
        reservations:
          cpus: '0.5'
          memory: 512M
```

### Database Tuning
- Connection pooling configured
- Query optimization enabled
- Appropriate indexes created
- Regular VACUUM and ANALYZE

### Caching Strategy
- Redis for application caching
- Database query result caching
- Static asset caching
- Session management

## Contributing to Docker Configuration

### Development Workflow
1. Make changes to Docker configuration
2. Test with development environment
3. Test with production configuration
4. Update documentation
5. Submit pull request

### Testing Changes
```bash
# Test development
./scripts/docker-compose-up.sh --build

# Test production
./scripts/docker-compose-up.sh -e production --build -d

# Test scaling
./scripts/docker-compose-up.sh --scale radarr=2

# Test health checks
docker compose exec radarr /app/health-check.sh
```

This Docker setup provides a robust foundation for both development and production deployments of the Radarr MVP, with comprehensive monitoring, security, and scalability features built-in.