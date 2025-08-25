# Radarr MVP - Rust Implementation

![CI Pipeline](https://github.com/zimmermanc/radarr-mvp/workflows/CI%20Pipeline/badge.svg)
![Security Scanning](https://github.com/zimmermanc/radarr-mvp/workflows/Security%20Scanning/badge.svg)
![Code Quality](https://github.com/zimmermanc/radarr-mvp/workflows/Code%20Quality/badge.svg)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/PROJECT_GRADE)](https://www.codacy.com/gh/zimmermanc/radarr-mvp/dashboard)
[![Codacy Coverage](https://app.codacy.com/project/badge/Coverage/PROJECT_COVERAGE)](https://www.codacy.com/gh/zimmermanc/radarr-mvp/dashboard)
[![codecov](https://codecov.io/gh/zimmermanc/radarr-mvp/branch/main/graph/badge.svg)](https://codecov.io/gh/zimmermanc/radarr-mvp)
[![Dependency Status](https://deps.rs/repo/github/zimmermanc/radarr-mvp/status.svg)](https://deps.rs/repo/github/zimmermanc/radarr-mvp)
![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)

A modern, high-performance movie collection manager built with Rust, featuring automated downloading, quality management, and comprehensive media organization.

## 🚀 Features

- **Automated Downloads**: Integration with torrent indexers and download clients
- **Quality Management**: Smart quality decision engine with custom formats
- **HDBits Integration**: Advanced scene group analysis and reputation scoring
- **Import Pipeline**: Automated file organization with hardlinking support
- **WebSocket Updates**: Real-time progress tracking and notifications
- **Circuit Breaker**: Resilient external service integration
- **PostgreSQL Backend**: Robust data persistence with migrations

## 📊 Project Status

- **Completion**: ~82% MVP Complete
- **Tests**: 162+ passing across 8 crates
- **Production**: Deployed at http://YOUR_SERVER_IP:7878/
- **CI/CD**: Comprehensive GitHub Actions pipeline with security scanning

## 🛠️ Technology Stack

- **Backend**: Rust 2021, Axum, Tokio, SQLx
- **Frontend**: React, TypeScript, Vite, TailwindCSS
- **Database**: PostgreSQL 16
- **Testing**: 162+ tests with Tarpaulin coverage
- **CI/CD**: GitHub Actions, Codacy, Dependabot

## 🏗️ Architecture

```
unified-radarr/
├── crates/
│   ├── core/          # Domain logic (no external deps)
│   ├── api/           # HTTP API (Axum)
│   ├── indexers/      # Torrent indexer integrations
│   ├── decision/      # Quality decision engine
│   ├── downloaders/   # Download client integrations
│   ├── import/        # Media import pipeline
│   ├── infrastructure/# Database, external services
│   ├── analysis/      # HDBits scene analysis tools
│   └── notifications/ # Notification providers
├── web/               # React frontend
├── migrations/        # SQL migrations
└── tests/            # Integration tests
```

## 🚦 CI/CD Pipeline

Our comprehensive CI/CD pipeline ensures code quality and security:

### Security Scanning
- **SAST**: Semgrep, CodeQL for static analysis
- **SCA**: cargo-audit, Snyk for dependency vulnerabilities
- **Secrets**: GitLeaks, TruffleHog for credential scanning
- **Container**: Trivy for Docker image scanning

### Code Quality
- **Linting**: Clippy with pedantic rules
- **Formatting**: rustfmt, Prettier
- **Coverage**: Tarpaulin with Codecov/Codacy integration
- **Complexity**: cargo-bloat, cargo-geiger

### Automation
- **Dependabot**: Weekly dependency updates
- **PR Validation**: Size checks, conventional commits
- **Multi-platform**: Linux, macOS, Windows testing
- **Performance**: Benchmark regression detection

## 🚀 Installation

### Option 1: Pre-built Binaries (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/zimmermanc/radarr-mvp/releases):

```bash
# Linux x86_64 - using curl
curl -L https://github.com/zimmermanc/radarr-mvp/releases/latest/download/radarr-mvp-x86_64-unknown-linux-gnu.tar.gz | tar xz

# Or using our installer script (automatically detects platform)
curl -fsSL https://github.com/zimmermanc/radarr-mvp/releases/latest/download/radarr-mvp-installer.sh | sh
```

**Supported Platforms:**
- **Linux**: x86_64, ARM64 (aarch64)
- **macOS**: Intel (x86_64), Apple Silicon (aarch64)  
- **Windows**: x86_64

### Option 2: Automated Installation

For a complete setup including database configuration:

```bash
# Download and run the installer
curl -fsSL https://github.com/zimmermanc/radarr-mvp/releases/latest/download/install.sh | sudo bash -s -- --setup-database --install-postgres

# Or step by step:
wget https://github.com/zimmermanc/radarr-mvp/releases/latest/download/radarr-mvp-x86_64-unknown-linux-gnu.tar.gz
tar xzf radarr-mvp-*.tar.gz
cd radarr-mvp-*/
sudo ./install/install.sh --setup-database --install-postgres
```

### Option 3: Package Managers

**Homebrew (macOS/Linux):**
```bash
brew install zimmermanc/tap/radarr-mvp
```

**cargo-binstall (Cross-platform):**
```bash
cargo binstall radarr-mvp
```

## 🛠️ Quick Setup

### Prerequisites
- **PostgreSQL 14+** (automatically installed with `--install-postgres`)
- **Linux/macOS/Windows** x86_64 or ARM64

### 1. Database Setup

If you used the automated installer with `--setup-database`, skip this step.

Otherwise, set up PostgreSQL manually:

```bash
# Run the database setup script (included in releases)
./install/setup-database.sh --generate-password

# Or manually:
sudo -u postgres createdb radarr
sudo -u postgres createuser radarr
sudo -u postgres psql -c "ALTER USER radarr WITH PASSWORD 'your_password';"
```

### 2. Configuration

Edit the configuration file:

```bash
# If installed system-wide
sudo nano /opt/radarr/.env

# Or in your download directory
nano .env
```

**Required Settings:**
```bash
DATABASE_URL="postgresql://radarr:your_password@localhost/radarr"
API_KEY="your_secure_api_key_here"
TMDB_API_KEY="your_tmdb_api_key"  # Get from https://www.themoviedb.org/
```

### 3. Start Radarr MVP

**System Service (recommended):**
```bash
sudo systemctl start radarr
sudo systemctl enable radarr  # Start on boot

# View logs
journalctl -u radarr -f
```

**Manual Start:**
```bash
./radarr-mvp
# Or if installed system-wide
/opt/radarr/radarr-mvp
```

### 4. Access Web Interface

Open your browser to: **http://localhost:7878**

- **API Documentation**: http://localhost:7878/docs
- **Health Check**: http://localhost:7878/health
- **Metrics**: http://localhost:7878/metrics

## ⚙️ Configuration

### Environment Variables

All configuration is done via the `.env` file or environment variables:

```bash
# Core Settings
HOST=0.0.0.0                    # Listen address
PORT=7878                       # Listen port
API_KEY=your_secure_key         # API authentication key
DATABASE_URL=postgresql://...    # PostgreSQL connection string

# Movie Database (Required)
TMDB_API_KEY=your_tmdb_key      # Get from themoviedb.org

# Optional: Indexer Configuration
HDBITS_USERNAME=your_username
HDBITS_PASSKEY=your_passkey

# Optional: Streaming Services
TRAKT_CLIENT_ID=your_trakt_id
TRAKT_CLIENT_SECRET=your_trakt_secret
WATCHMODE_API_KEY=your_watchmode_key

# Logging
RUST_LOG=info                   # Log level: error, warn, info, debug, trace
```

### Database Migrations

Migrations are automatically included in releases:

```bash
# Apply migrations (done automatically on first start)
sqlx migrate run

# Check migration status
sqlx migrate info
```

## 🛠️ Management Commands

### Service Management
```bash
# System service
sudo systemctl start radarr     # Start service
sudo systemctl stop radarr      # Stop service  
sudo systemctl restart radarr   # Restart service
sudo systemctl status radarr    # Check status
journalctl -u radarr -f         # View logs

# Manual process management
pkill -f radarr-mvp             # Stop manual process
```

### Database Operations
```bash
# Backup database
/opt/radarr/backup.sh           # Automated backup script

# Manual backup
pg_dump "$DATABASE_URL" | gzip > radarr_backup.sql.gz

# Restore backup  
zcat radarr_backup.sql.gz | psql "$DATABASE_URL"

# Reset database (⚠️ destructive)
sudo -u postgres dropdb radarr && sudo -u postgres createdb -O radarr radarr
sqlx migrate run
```

### Maintenance
```bash
# Update to latest version
curl -fsSL https://github.com/zimmermanc/radarr-mvp/releases/latest/download/install.sh | sudo bash

# Check version
./radarr-mvp --version
curl -s http://localhost:7878/health | jq .version

# View configuration
cat /opt/radarr/.env
```

## 🔍 Troubleshooting

### Common Issues

**Service won't start:**
```bash
# Check logs
journalctl -u radarr -n 50

# Check configuration
sudo -u radarr /opt/radarr/radarr-mvp --help

# Test database connection
psql "$DATABASE_URL" -c "SELECT version();"
```

**Port already in use:**
```bash
# Find what's using port 7878
sudo lsof -i :7878
sudo netstat -tulpn | grep 7878

# Change port in configuration
sudo nano /opt/radarr/.env
# Set PORT=8080 or another available port
```

**Permission denied:**
```bash
# Fix ownership
sudo chown -R radarr:radarr /opt/radarr

# Fix permissions
chmod 600 /opt/radarr/.env
chmod +x /opt/radarr/radarr-mvp
```

**Database connection failed:**
```bash
# Check PostgreSQL is running
sudo systemctl status postgresql

# Test connection manually
psql -h localhost -U radarr -d radarr

# Check database exists
sudo -u postgres psql -l | grep radarr
```

### Performance Tuning

**Memory Usage:**
```bash
# Adjust memory limits
sudo systemctl edit radarr
# Add: [Service]
#      MemoryMax=4G

# Monitor memory usage
htop -p $(pgrep radarr-mvp)
```

**Database Performance:**
```bash
# Add database indexes (if needed)
psql "$DATABASE_URL" -c "CREATE INDEX CONCURRENTLY idx_movies_tmdb_id ON movies(tmdb_id);"

# Analyze database
psql "$DATABASE_URL" -c "ANALYZE;"
```

## 🗑️ Uninstallation

### Complete Removal
```bash
# Stop and remove service
sudo systemctl stop radarr
sudo systemctl disable radarr
sudo rm /etc/systemd/system/radarr.service
sudo systemctl daemon-reload

# Remove installation (⚠️ removes all data)
sudo rm -rf /opt/radarr

# Remove user
sudo userdel radarr

# Remove database (⚠️ removes all data)  
sudo -u postgres dropdb radarr
sudo -u postgres dropuser radarr
```

### Keep Data (Remove only binary)
```bash
# Stop service
sudo systemctl stop radarr

# Backup data
sudo cp -r /opt/radarr/data /opt/radarr/data.backup
/opt/radarr/backup.sh

# Remove only binary and service
sudo rm /opt/radarr/radarr-mvp
sudo rm /etc/systemd/system/radarr.service
```

## 🔧 Development Setup

For developers who want to build from source:

### Prerequisites
- Rust 1.75+
- PostgreSQL 16
- Node.js 20+ (for web UI development)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/zimmermanc/radarr-mvp.git
cd radarr-mvp/unified-radarr

# Install dependencies
cargo build --workspace

# Setup database
./install/setup-database.sh --generate-password

# Run tests
cargo test --workspace

# Start the server
cargo run --bin radarr-mvp
```

### Frontend Development

```bash
cd web
npm install
npm run dev        # Development server on port 5173
npm run build      # Production build
```

## 🚀 Production Deployment

### Quick Production Deploy to 192.168.0.131

```bash
# One-command deployment (from unified-radarr directory)
./scripts/deploy-production.sh

# Or with options
./scripts/deploy-production.sh --skip-build  # Use existing binary
./scripts/deploy-production.sh --skip-backup # Skip database backup
./scripts/deploy-production.sh --force       # Skip confirmation prompt
```

### Prerequisites for Production

1. **Target Server Requirements**:
   - Ubuntu 20.04+ or Debian 11+
   - 4GB+ RAM (2GB minimum)
   - 20GB+ disk space
   - PostgreSQL 14+ installed
   - SSH access with root or sudo privileges

2. **Local Build Requirements**:
   - Rust 1.75+ with cargo
   - SSH key configured for root@192.168.0.131

### Step-by-Step Production Setup

#### 1. Prepare Configuration

```bash
# Copy and edit production environment
cp config/production.env.template config/production.env
nano config/production.env

# Update critical values:
# - DATABASE_URL with secure password
# - JWT_SECRET (generate: openssl rand -base64 64)
# - API_KEY (generate: openssl rand -base64 32)
# - External service API keys (TMDB, HDBits, etc.)
```

#### 2. Setup Production Database

```bash
# Run database setup on production server
./scripts/setup-production-db.sh --host root@192.168.0.131 --generate-password

# Or setup locally then run on server
ssh root@192.168.0.131
./setup-production-db.sh --generate-password
```

#### 3. Deploy Application

```bash
# Full deployment with all checks
./scripts/deploy-production.sh

# The script will:
# ✓ Build optimized release binary
# ✓ Create database backup (if exists)
# ✓ Stop existing service
# ✓ Deploy binary and configurations
# ✓ Setup systemd service
# ✓ Configure firewall rules
# ✓ Setup log rotation
# ✓ Start service and verify health
```

#### 4. Post-Deployment Configuration

```bash
# SSH to production server
ssh root@192.168.0.131

# Update database password in environment
nano /opt/radarr/.env

# Run database migrations
cd /opt/radarr
export DATABASE_URL="postgresql://radarr:CHANGE_ME_PASSWORD@localhost/radarr"
sqlx migrate run

# Verify service status
systemctl status radarr
journalctl -u radarr -f
```

### Production Management Commands

```bash
# Service Management
ssh root@192.168.0.131 'systemctl status radarr'     # Check status
ssh root@192.168.0.131 'systemctl restart radarr'    # Restart service
ssh root@192.168.0.131 'systemctl stop radarr'       # Stop service
ssh root@192.168.0.131 'journalctl -u radarr -f'     # View logs

# Database Management
ssh root@192.168.0.131 '/usr/local/bin/radarr-backup.sh'              # Create backup
ssh root@192.168.0.131 '/usr/local/bin/radarr-restore.sh backup.sql.gz' # Restore backup
ssh root@192.168.0.131 'sudo -u postgres psql -d radarr'              # Database console

# Application URLs
curl http://192.168.0.131:7878/health       # Health check
curl http://192.168.0.131:7878/metrics      # Prometheus metrics
browse http://192.168.0.131:7878            # Web interface
```

### Production Security Checklist

- [ ] Change default database password
- [ ] Generate unique JWT_SECRET and API_KEY
- [ ] Configure firewall rules (ports 7878, 9090)
- [ ] Set up SSL/TLS certificate (recommended: Let's Encrypt)
- [ ] Configure fail2ban for SSH protection
- [ ] Set up automated backups (cron job)
- [ ] Configure monitoring and alerts
- [ ] Review and adjust systemd security settings
- [ ] Restrict file permissions: `chmod 600 /opt/radarr/.env`

### Automated Backup Setup

```bash
# Add to crontab on production server
ssh root@192.168.0.131
crontab -e

# Add daily backup at 2 AM
0 2 * * * /usr/local/bin/radarr-backup.sh >> /var/log/radarr-backup.log 2>&1
```

### Monitoring Setup

```bash
# Prometheus configuration (add to prometheus.yml)
scrape_configs:
  - job_name: 'radarr'
    static_configs:
      - targets: ['192.168.0.131:9090']
    scrape_interval: 30s

# Grafana dashboard import
# Use dashboard ID: [TBD] from grafana.com
```

### Troubleshooting Production Issues

```bash
# Check service logs
ssh root@192.168.0.131 'journalctl -u radarr -n 100'

# Check database connectivity
ssh root@192.168.0.131 'sudo -u postgres pg_isready'

# Test application health
curl -v http://192.168.0.131:7878/health

# Check disk space
ssh root@192.168.0.131 'df -h'

# Check memory usage
ssh root@192.168.0.131 'free -h'

# View running processes
ssh root@192.168.0.131 'ps aux | grep radarr'

# Check network connections
ssh root@192.168.0.131 'ss -tulpn | grep 7878'
```

### Rolling Back Deployment

```bash
# Stop service
ssh root@192.168.0.131 'systemctl stop radarr'

# Restore database from backup
ssh root@192.168.0.131 '/usr/local/bin/radarr-restore.sh /opt/radarr/backups/latest.sql.gz'

# Deploy previous version
git checkout <previous-tag>
./scripts/deploy-production.sh
```

### Performance Tuning

1. **Database Optimization**:
   ```sql
   -- Add indexes for common queries
   CREATE INDEX idx_movies_tmdb_id ON movies(tmdb_id);
   CREATE INDEX idx_queue_status ON download_queue(status);
   
   -- Analyze tables
   ANALYZE movies;
   ANALYZE download_queue;
   ```

2. **System Resources**:
   ```bash
   # Edit systemd service limits
   ssh root@192.168.0.131
   nano /etc/systemd/system/radarr.service
   
   # Adjust:
   # MemoryMax=4G      # Increase if needed
   # CPUQuota=400%     # For 4 cores
   ```

3. **Application Tuning**:
   ```bash
   # Edit environment file
   nano /opt/radarr/.env
   
   # Adjust:
   # WORKER_THREADS=8
   # DATABASE_MAX_CONNECTIONS=200
   # CACHE_TTL=7200
   ```

## 🧪 Testing

```bash
# Run all tests
cargo test --workspace

# Run with coverage
cargo tarpaulin --workspace --all-features

# Run specific crate tests
cargo test -p radarr-core

# Run integration tests
cargo test --test integration
```

## 📖 Documentation

- [CI/CD Guide](docs/CI-CD-GUIDE.md) - Complete CI/CD pipeline documentation
- [API Documentation](docs/api/) - OpenAPI specification
- [Development Setup](docs/DEVELOPER_SETUP.md) - Local development guide
- [Architecture Decisions](docs/decisions/) - ADRs and design choices

## 🔒 Security

- Regular dependency audits via cargo-audit and Dependabot
- SAST scanning with Semgrep and CodeQL
- Secret scanning with GitLeaks and TruffleHog
- Security advisories tracked in GitHub Security tab

## 🤝 Contributing

We welcome contributions! Please ensure:

1. All tests pass: `cargo test --workspace`
2. Code is formatted: `cargo fmt --all`
3. No Clippy warnings: `cargo clippy --all`
4. PR follows conventional commits format
5. Description includes what and why

## 📊 Metrics

- **Test Coverage**: ~70% (increasing)
- **Code Quality**: Codacy Grade A
- **Dependencies**: All up-to-date via Dependabot
- **Security**: No known vulnerabilities

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Original Radarr project for inspiration
- Rust community for excellent libraries
- Contributors and testers

## 📞 Support

- [GitHub Issues](https://github.com/zimmermanc/radarr-mvp/issues)
- [Documentation](docs/)
- [CI/CD Status](https://github.com/zimmermanc/radarr-mvp/actions)

---

**Note**: This is an MVP implementation focusing on core functionality. See [TASKLIST.md](TASKLIST.md) for development progress and roadmap.