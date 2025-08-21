# Installation Guide

Comprehensive installation instructions for Radarr MVP across different environments.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Development Setup](#development-setup)
3. [Production Deployment](#production-deployment)
4. [Docker Installation](#docker-installation)
5. [Kubernetes Deployment](#kubernetes-deployment)
6. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| **CPU** | 2 cores | 4+ cores |
| **RAM** | 2GB | 4GB+ |
| **Storage** | 10GB | 50GB+ SSD |
| **OS** | Linux, macOS, Windows | Linux (Ubuntu 20.04+) |

### Required Software

#### Rust Development Environment
```bash
# Install Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should be 1.75+
cargo --version
```

#### Node.js and npm
```bash
# Option 1: Using Node Version Manager (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc
nvm install 18
nvm use 18

# Option 2: Direct installation
# Visit https://nodejs.org/ and download LTS version

# Verify installation
node --version  # Should be 18+
npm --version
```

#### PostgreSQL Database

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database and user
sudo -u postgres psql
CREATE DATABASE radarr_dev;
CREATE USER radarr WITH PASSWORD 'radarr';
GRANT ALL PRIVILEGES ON DATABASE radarr_dev TO radarr;
\q
```

**macOS (with Homebrew):**
```bash
brew install postgresql
brew services start postgresql

# Create database
psql postgres
CREATE DATABASE radarr_dev;
CREATE USER radarr WITH PASSWORD 'radarr';
GRANT ALL PRIVILEGES ON DATABASE radarr_dev TO radarr;
\q
```

**Windows:**
1. Download PostgreSQL from https://www.postgresql.org/download/windows/
2. Run installer and note the password for postgres user
3. Use pgAdmin or psql to create database and user

#### SQLx CLI (for database migrations)
```bash
cargo install sqlx-cli --features postgres
```

### External Services

#### Prowlarr (Required)
```bash
# Docker installation (recommended)
docker run -d \
  --name=prowlarr \
  -e PUID=1000 \
  -e PGID=1000 \
  -e TZ=Etc/UTC \
  -p 9696:9696 \
  -v /path/to/prowlarr/config:/config \
  --restart unless-stopped \
  lscr.io/linuxserver/prowlarr:latest

# Manual installation: https://prowlarr.com/
```

#### qBittorrent (Required)
```bash
# Docker installation
docker run -d \
  --name=qbittorrent \
  -e PUID=1000 \
  -e PGID=1000 \
  -e TZ=Etc/UTC \
  -e WEBUI_PORT=8080 \
  -p 8080:8080 \
  -p 6881:6881 \
  -p 6881:6881/udp \
  -v /path/to/qbittorrent/config:/config \
  -v /path/to/downloads:/downloads \
  --restart unless-stopped \
  lscr.io/linuxserver/qbittorrent:latest

# Manual installation: https://www.qbittorrent.org/
```

## Development Setup

### 1. Clone Repository
```bash
git clone <your-repository-url>
cd unified-radarr
```

### 2. Environment Configuration
```bash
# Copy example environment file
cp .env.example .env

# Edit configuration
nano .env  # or your preferred editor
```

**Required Environment Variables:**
```bash
# Database
DATABASE_URL=postgresql://radarr:radarr@localhost:5432/radarr_dev

# Prowlarr
PROWLARR_BASE_URL=http://localhost:9696
PROWLARR_API_KEY=your_prowlarr_api_key_here

# qBittorrent
QBITTORRENT_BASE_URL=http://localhost:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=adminpass
```

**Get Prowlarr API Key:**
1. Open Prowlarr web interface (http://localhost:9696)
2. Go to Settings → General
3. Copy the API Key
4. Add to `.env` file

### 3. Database Setup
```bash
# Run database migrations
sqlx migrate run

# Verify migration success
sqlx migrate info
```

### 4. Build and Test
```bash
# Build entire workspace
cargo build --workspace

# Run tests to verify setup
cargo test --workspace

# Build optimized release
cargo build --workspace --release
```

### 5. Frontend Setup
```bash
# Install dependencies
cd web
npm install

# Verify build
npm run build

# Return to root
cd ..
```

### 6. Start Development Environment

**Terminal 1 - Backend:**
```bash
# Start with hot reload
RUST_LOG=debug cargo run

# Or with file watching
cargo install cargo-watch
cargo watch -x "run"
```

**Terminal 2 - Frontend:**
```bash
cd web
npm run dev
```

**Access Points:**
- **Web UI**: http://localhost:5173
- **API**: http://localhost:7878
- **Health Check**: http://localhost:7878/health

## Production Deployment

### 1. System User Setup
```bash
# Create system user
sudo useradd --system --shell /bin/bash --home /opt/radarr radarr
sudo mkdir -p /opt/radarr
sudo chown radarr:radarr /opt/radarr
```

### 2. Application Installation
```bash
# Clone to production directory
sudo -u radarr git clone <repository-url> /opt/radarr/app
cd /opt/radarr/app

# Build optimized release
sudo -u radarr cargo build --release

# Build frontend
cd web
sudo -u radarr npm ci --production
sudo -u radarr npm run build
cd ..
```

### 3. Configuration
```bash
# Create production environment file
sudo -u radarr cp .env.example /opt/radarr/.env
sudo -u radarr nano /opt/radarr/.env
```

**Production Environment:**
```bash
# Server
RADARR_HOST=0.0.0.0
RADARR_PORT=7878

# Database (use production PostgreSQL)
DATABASE_URL=postgresql://radarr:secure_password@localhost:5432/radarr_prod
DATABASE_MAX_CONNECTIONS=20

# Logging
RUST_LOG=info
LOG_JSON_FORMAT=true
LOG_FILE=/var/log/radarr/radarr.log

# External services
PROWLARR_BASE_URL=http://localhost:9696
PROWLARR_API_KEY=production_api_key

QBITTORRENT_BASE_URL=http://localhost:8080
QBITTORRENT_USERNAME=radarr_user
QBITTORRENT_PASSWORD=secure_password
```

### 4. Systemd Service
```bash
# Create service file
sudo nano /etc/systemd/system/radarr.service
```

**Service Configuration:**
```ini
[Unit]
Description=Radarr MVP - Movie Automation
After=network.target postgresql.service
Requires=postgresql.service

[Service]
Type=simple
User=radarr
Group=radarr
WorkingDirectory=/opt/radarr/app
EnvironmentFile=/opt/radarr/.env
ExecStart=/opt/radarr/app/target/release/radarr-mvp
Restart=always
RestartSec=10

# Security settings
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/radarr /var/log/radarr /tmp

# Resource limits
LimitNOFILE=65536
LimitMEMLOCK=infinity

[Install]
WantedBy=multi-user.target
```

### 5. Start and Enable Service
```bash
# Create log directory
sudo mkdir -p /var/log/radarr
sudo chown radarr:radarr /var/log/radarr

# Run database migrations
sudo -u radarr bash -c 'cd /opt/radarr/app && sqlx migrate run'

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable radarr
sudo systemctl start radarr

# Check status
sudo systemctl status radarr
sudo journalctl -u radarr -f
```

### 6. Reverse Proxy (Nginx)
```bash
# Install nginx
sudo apt install nginx

# Create site configuration
sudo nano /etc/nginx/sites-available/radarr
```

**Nginx Configuration:**
```nginx
server {
    listen 80;
    server_name your-domain.com;
    
    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;
    
    # SSL configuration (get certificates with certbot)
    ssl_certificate /etc/letsencrypt/live/your-domain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/your-domain.com/privkey.pem;
    
    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    
    location / {
        proxy_pass http://localhost:7878;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/radarr /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl restart nginx
```

## Docker Installation

### 1. Using Docker Compose (Recommended)

**Create `docker-compose.yml`:**
```yaml
version: '3.8'

services:
  radarr-mvp:
    build: .
    ports:
      - "7878:7878"
    environment:
      - DATABASE_URL=postgresql://radarr:radarr@postgres:5432/radarr
      - PROWLARR_BASE_URL=http://prowlarr:9696
      - PROWLARR_API_KEY=${PROWLARR_API_KEY}
      - QBITTORRENT_BASE_URL=http://qbittorrent:8080
      - QBITTORRENT_USERNAME=admin
      - QBITTORRENT_PASSWORD=adminpass
      - RUST_LOG=info
    volumes:
      - ./data:/data
      - /path/to/movies:/movies
      - /path/to/downloads:/downloads
    depends_on:
      - postgres
    restart: unless-stopped

  postgres:
    image: postgres:16-alpine
    environment:
      - POSTGRES_DB=radarr
      - POSTGRES_USER=radarr
      - POSTGRES_PASSWORD=radarr
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  prowlarr:
    image: lscr.io/linuxserver/prowlarr:latest
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Etc/UTC
    volumes:
      - ./prowlarr:/config
    ports:
      - "9696:9696"
    restart: unless-stopped

  qbittorrent:
    image: lscr.io/linuxserver/qbittorrent:latest
    environment:
      - PUID=1000
      - PGID=1000
      - TZ=Etc/UTC
      - WEBUI_PORT=8080
    volumes:
      - ./qbittorrent:/config
      - /path/to/downloads:/downloads
    ports:
      - "8080:8080"
      - "6881:6881"
      - "6881:6881/udp"
    restart: unless-stopped

volumes:
  postgres_data:
```

### 2. Deploy with Docker Compose
```bash
# Create environment file
echo "PROWLARR_API_KEY=your_api_key_here" > .env

# Build and start
docker-compose up -d

# View logs
docker-compose logs -f radarr-mvp

# Run database migrations
docker-compose exec radarr-mvp sqlx migrate run
```

## Kubernetes Deployment

### 1. Prerequisites
```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

# Install kustomize
curl -s "https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh" | bash
sudo mv kustomize /usr/local/bin/
```

### 2. Configure Environment
```bash
# Create namespace
kubectl create namespace radarr

# Create secrets
kubectl create secret generic radarr-secrets -n radarr \
  --from-literal=database-url="postgresql://radarr:password@postgres:5432/radarr" \
  --from-literal=prowlarr-api-key="your_api_key" \
  --from-literal=qbittorrent-password="secure_password"
```

### 3. Deploy Application
```bash
# Deploy to development
kubectl apply -k k8s/overlays/dev/

# Deploy to production
kubectl apply -k k8s/overlays/prod/

# Check deployment status
kubectl get pods -n radarr
kubectl logs -f deployment/radarr-mvp -n radarr
```

### 4. Access Application
```bash
# Port forward for testing
kubectl port-forward svc/radarr-mvp 7878:7878 -n radarr

# Access via LoadBalancer or Ingress (production)
kubectl get svc -n radarr
```

## Troubleshooting

### Common Issues

#### 1. Compilation Errors
```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build

# Check for missing dependencies
cargo check
```

#### 2. Database Connection Issues
```bash
# Test PostgreSQL connection
psql "postgresql://radarr:radarr@localhost:5432/radarr_dev" -c "SELECT 1;"

# Check if database exists
sudo -u postgres psql -l | grep radarr

# Recreate database
sudo -u postgres dropdb radarr_dev
sudo -u postgres createdb radarr_dev
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE radarr_dev TO radarr;"
```

#### 3. Migration Failures
```bash
# Check migration status
sqlx migrate info

# Revert last migration
sqlx migrate revert

# Force migration version
sqlx migrate run --source migrations/
```

#### 4. External Service Connection
```bash
# Test Prowlarr connection
curl -H "X-Api-Key: your_api_key" http://localhost:9696/api/v1/indexer

# Test qBittorrent connection
curl -X POST http://admin:adminpass@localhost:8080/api/v2/auth/login
```

#### 5. Frontend Build Issues
```bash
# Clear npm cache
npm cache clean --force

# Remove node_modules and reinstall
rm -rf node_modules package-lock.json
npm install

# Check Node.js version
node --version  # Should be 18+
```

### Log Analysis

#### Backend Logs
```bash
# Development
RUST_LOG=debug cargo run

# Production service
sudo journalctl -u radarr -f

# Docker
docker-compose logs -f radarr-mvp

# Kubernetes
kubectl logs -f deployment/radarr-mvp -n radarr
```

#### Database Logs
```bash
# PostgreSQL logs (Ubuntu)
sudo tail -f /var/log/postgresql/postgresql-*.log

# Docker PostgreSQL
docker-compose logs postgres
```

### Performance Tuning

#### Database Optimization
```sql
-- Check connection count
SELECT count(*) FROM pg_stat_activity;

-- Check slow queries
SELECT query, mean_exec_time 
FROM pg_stat_statements 
ORDER BY mean_exec_time DESC 
LIMIT 10;
```

#### Memory Usage
```bash
# Check memory usage
ps aux | grep radarr-mvp

# Monitor with htop
htop -p $(pgrep radarr-mvp)
```

### Getting Help

1. **Check logs** for specific error messages
2. **Verify configuration** in `.env` file
3. **Test external services** independently
4. **Check GitHub issues** for similar problems
5. **Join community discussions** for support

### Health Checks

```bash
# Application health
curl http://localhost:7878/health

# Detailed health with component status
curl http://localhost:7878/health/detailed

# Database connectivity
curl http://localhost:7878/api/v1/system/status
```

For additional help, check the [Configuration Reference](CONFIG.md) and [API Documentation](API.md).