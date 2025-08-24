#!/bin/bash

# Radarr MVP Production Deployment Script
# Deploys to production server root@192.168.0.131
# Author: Radarr MVP Team
# Date: 2025-08-24

set -e  # Exit on any error
set -o pipefail  # Exit on pipe failure

# Configuration
PRODUCTION_SERVER="root@192.168.0.131"
REMOTE_DIR="/opt/radarr"
SERVICE_NAME="radarr"
BINARY_NAME="radarr-mvp"
DB_NAME="radarr"
DB_USER="radarr"
BACKUP_DIR="/opt/radarr/backups"
LOG_DIR="/var/log/radarr"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Timestamp function
timestamp() {
    date +"%Y-%m-%d %H:%M:%S"
}

# Logging function
log() {
    echo -e "${2}[$(timestamp)] $1${NC}"
}

# Error handler
error_exit() {
    log "ERROR: $1" "$RED"
    exit 1
}

# Success message
success() {
    log "✓ $1" "$GREEN"
}

# Warning message
warning() {
    log "⚠ $1" "$YELLOW"
}

# Info message
info() {
    log "ℹ $1" "$BLUE"
}

# Header
echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║        Radarr MVP Production Deployment Script        ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo
info "Target Server: $PRODUCTION_SERVER"
info "Service Name: $SERVICE_NAME"
info "Deployment Time: $(timestamp)"
echo

# Parse command line arguments
SKIP_BUILD=false
SKIP_BACKUP=false
FORCE_DEPLOY=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --skip-backup)
            SKIP_BACKUP=true
            shift
            ;;
        --force)
            FORCE_DEPLOY=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            set -x
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --skip-build    Skip building the binary (use existing)"
            echo "  --skip-backup   Skip database backup"
            echo "  --force         Force deployment without confirmation"
            echo "  --verbose       Enable verbose output"
            echo "  --help          Show this help message"
            exit 0
            ;;
        *)
            error_exit "Unknown option: $1"
            ;;
    esac
done

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    error_exit "Must be run from unified-radarr directory"
fi

# Confirmation prompt (unless --force is used)
if [ "$FORCE_DEPLOY" = false ]; then
    echo -e "${YELLOW}⚠ WARNING: This will deploy to PRODUCTION${NC}"
    echo -e "${YELLOW}Server: $PRODUCTION_SERVER${NC}"
    echo -e "${YELLOW}Are you sure you want to continue? (yes/no)${NC}"
    read -r confirmation
    if [ "$confirmation" != "yes" ]; then
        warning "Deployment cancelled by user"
        exit 0
    fi
fi

# Check server connectivity
info "Checking server connectivity..."
if ! ssh -o ConnectTimeout=5 -o BatchMode=yes "$PRODUCTION_SERVER" "echo 'Server accessible'" >/dev/null 2>&1; then
    error_exit "Cannot connect to $PRODUCTION_SERVER. Please check SSH configuration."
fi
success "Server connection established"

# Build release binary (unless skipped)
if [ "$SKIP_BUILD" = false ]; then
    info "Building release binary..."
    
    # Clean previous builds
    cargo clean --release 2>/dev/null || true
    
    # Build with optimizations
    RUSTFLAGS="-C target-cpu=native" cargo build --release --workspace
    
    if [ ! -f "target/release/$BINARY_NAME" ]; then
        error_exit "Binary not found at target/release/$BINARY_NAME"
    fi
    
    # Strip debug symbols for smaller binary
    strip "target/release/$BINARY_NAME"
    
    success "Release binary built successfully"
    
    # Show binary size
    BINARY_SIZE=$(du -h "target/release/$BINARY_NAME" | cut -f1)
    info "Binary size: $BINARY_SIZE"
else
    warning "Skipping build step (--skip-build flag)"
    if [ ! -f "target/release/$BINARY_NAME" ]; then
        error_exit "Binary not found and build was skipped"
    fi
fi

# Create backup (unless skipped)
if [ "$SKIP_BACKUP" = false ]; then
    info "Creating database backup on production server..."
    
    ssh "$PRODUCTION_SERVER" << 'BACKUP_SCRIPT'
        set -e
        
        # Create backup directory if it doesn't exist
        mkdir -p /opt/radarr/backups
        
        # Check if PostgreSQL is running
        if systemctl is-active --quiet postgresql; then
            # Create backup with timestamp
            BACKUP_FILE="/opt/radarr/backups/radarr_$(date +%Y%m%d_%H%M%S).sql"
            
            # Perform backup
            if sudo -u postgres pg_dump radarr > "$BACKUP_FILE" 2>/dev/null; then
                # Compress backup
                gzip "$BACKUP_FILE"
                echo "Backup created: ${BACKUP_FILE}.gz"
                
                # Keep only last 7 backups
                ls -t /opt/radarr/backups/*.gz 2>/dev/null | tail -n +8 | xargs rm -f 2>/dev/null || true
            else
                echo "Warning: Database backup failed (database might not exist yet)"
            fi
        else
            echo "Warning: PostgreSQL is not running, skipping backup"
        fi
BACKUP_SCRIPT
    
    success "Backup process completed"
else
    warning "Skipping backup step (--skip-backup flag)"
fi

# Stop service if running
info "Stopping service on production server..."
ssh "$PRODUCTION_SERVER" "systemctl stop $SERVICE_NAME 2>/dev/null || echo 'Service was not running'"
success "Service stopped"

# Create remote directories
info "Preparing remote directories..."
ssh "$PRODUCTION_SERVER" << EOF
    set -e
    mkdir -p $REMOTE_DIR
    mkdir -p $BACKUP_DIR
    mkdir -p $LOG_DIR
    mkdir -p $REMOTE_DIR/migrations
    mkdir -p $REMOTE_DIR/config
    mkdir -p $REMOTE_DIR/data
EOF
success "Remote directories prepared"

# Copy binary
info "Copying binary to production server..."
scp -q "target/release/$BINARY_NAME" "$PRODUCTION_SERVER:$REMOTE_DIR/"
ssh "$PRODUCTION_SERVER" "chmod +x $REMOTE_DIR/$BINARY_NAME"
success "Binary deployed"

# Copy migrations
info "Copying database migrations..."
scp -q -r migrations/* "$PRODUCTION_SERVER:$REMOTE_DIR/migrations/" 2>/dev/null || warning "No migrations to copy"

# Copy configuration files
info "Copying configuration files..."

# Create production environment file if it doesn't exist
if [ ! -f "config/production.env" ]; then
    warning "Production environment file not found, creating template..."
    cat > config/production.env << 'ENV_TEMPLATE'
# Radarr MVP Production Configuration
# Generated on deployment

# Server Configuration
HOST=0.0.0.0
PORT=7878
RUST_LOG=info
ENVIRONMENT=production

# Database Configuration
DATABASE_URL=postgresql://radarr:CHANGE_ME@localhost/radarr
DATABASE_MAX_CONNECTIONS=100
DATABASE_MIN_CONNECTIONS=10

# Redis Configuration (optional)
REDIS_URL=redis://localhost:6379

# Security
JWT_SECRET=CHANGE_ME_TO_RANDOM_STRING
API_KEY=CHANGE_ME_TO_RANDOM_STRING

# External Services
TMDB_API_KEY=YOUR_TMDB_API_KEY
HDBITS_USERNAME=YOUR_HDBITS_USERNAME
HDBITS_PASSKEY=YOUR_HDBITS_PASSKEY
TRAKT_CLIENT_ID=YOUR_TRAKT_CLIENT_ID
TRAKT_CLIENT_SECRET=YOUR_TRAKT_CLIENT_SECRET
WATCHMODE_API_KEY=YOUR_WATCHMODE_API_KEY

# Feature Flags
ENABLE_STREAMING=true
ENABLE_LISTS=true
ENABLE_MONITORING=true

# Monitoring
METRICS_PORT=9090
HEALTH_CHECK_INTERVAL=30

# Performance
WORKER_THREADS=4
ASYNC_RUNTIME_THREADS=8
CONNECTION_POOL_SIZE=20
ENV_TEMPLATE
    warning "Please update config/production.env with actual values"
fi

# Copy environment file
scp -q "config/production.env" "$PRODUCTION_SERVER:$REMOTE_DIR/.env"
success "Configuration files deployed"

# Install systemd service
info "Installing systemd service..."
ssh "$PRODUCTION_SERVER" << 'SERVICE_SCRIPT'
cat > /etc/systemd/system/radarr.service << 'EOF'
[Unit]
Description=Radarr MVP - Advanced Movie Management System
Documentation=https://github.com/radarr-mvp/docs
After=network-online.target postgresql.service
Wants=network-online.target postgresql.service
StartLimitIntervalSec=600
StartLimitBurst=5

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=/opt/radarr
Environment="RUST_BACKTRACE=1"
EnvironmentFile=/opt/radarr/.env

# Security
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/radarr /var/log/radarr

# Resource Limits
LimitNOFILE=65536
LimitNPROC=4096
TasksMax=4096
MemoryMax=2G
CPUQuota=200%

# Execution
ExecStartPre=/usr/bin/test -f /opt/radarr/.env
ExecStart=/opt/radarr/radarr-mvp
ExecReload=/bin/kill -USR1 $MAINPID
Restart=always
RestartSec=10
TimeoutStartSec=300
TimeoutStopSec=30

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=radarr

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
SERVICE_SCRIPT
success "Systemd service installed"

# Setup database (if PostgreSQL is installed)
info "Setting up database..."
ssh "$PRODUCTION_SERVER" << 'DB_SCRIPT'
    set -e
    
    # Check if PostgreSQL is installed
    if command -v psql >/dev/null 2>&1; then
        # Check if database exists
        if sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -qw radarr; then
            echo "Database 'radarr' already exists"
        else
            echo "Creating database and user..."
            sudo -u postgres psql << EOF
                CREATE USER radarr WITH PASSWORD 'CHANGE_ME_TO_SECURE_PASSWORD';
                CREATE DATABASE radarr OWNER radarr;
                GRANT ALL PRIVILEGES ON DATABASE radarr TO radarr;
EOF
            echo "Database created successfully"
            echo "WARNING: Please update the database password in /opt/radarr/.env"
        fi
        
        # Run migrations
        if [ -d "/opt/radarr/migrations" ]; then
            echo "Running database migrations..."
            cd /opt/radarr
            # Note: You'll need to install sqlx-cli or use another migration tool
            # For now, we'll assume migrations are handled by the application
            echo "Migrations will be run by the application on startup"
        fi
    else
        echo "WARNING: PostgreSQL is not installed. Please install it and set up the database manually."
    fi
DB_SCRIPT
success "Database setup completed"

# Configure firewall (if ufw is installed)
info "Configuring firewall..."
ssh "$PRODUCTION_SERVER" << 'FIREWALL_SCRIPT'
    if command -v ufw >/dev/null 2>&1; then
        ufw allow 7878/tcp comment 'Radarr Web Interface' 2>/dev/null || true
        ufw allow 9090/tcp comment 'Radarr Metrics' 2>/dev/null || true
        echo "Firewall rules added"
    else
        echo "Firewall not configured (ufw not found)"
    fi
FIREWALL_SCRIPT

# Setup log rotation
info "Setting up log rotation..."
ssh "$PRODUCTION_SERVER" << 'LOGROTATE_SCRIPT'
cat > /etc/logrotate.d/radarr << 'EOF'
/var/log/radarr/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 root root
    sharedscripts
    postrotate
        systemctl reload radarr 2>/dev/null || true
    endscript
}
EOF
LOGROTATE_SCRIPT
success "Log rotation configured"

# Start service
info "Starting service..."
ssh "$PRODUCTION_SERVER" << EOF
    systemctl enable $SERVICE_NAME
    systemctl start $SERVICE_NAME
EOF

# Wait for service to start
sleep 5

# Check service status
info "Checking service status..."
if ssh "$PRODUCTION_SERVER" "systemctl is-active $SERVICE_NAME" | grep -q "active"; then
    success "Service is running"
else
    error_exit "Service failed to start. Check logs with: journalctl -u $SERVICE_NAME -n 50"
fi

# Perform health check
info "Performing health check..."
sleep 3

HEALTH_CHECK=$(ssh "$PRODUCTION_SERVER" "curl -s -o /dev/null -w '%{http_code}' http://localhost:7878/health" 2>/dev/null || echo "000")

if [ "$HEALTH_CHECK" = "200" ]; then
    success "Health check passed"
else
    warning "Health check returned status code: $HEALTH_CHECK"
    warning "Service might still be starting. Check manually: curl http://$PRODUCTION_SERVER:7878/health"
fi

# Show recent logs
info "Recent service logs:"
ssh "$PRODUCTION_SERVER" "journalctl -u $SERVICE_NAME --no-pager -n 20"

# Final summary
echo
echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║         Production Deployment Complete                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo
success "Deployment successful!"
echo
info "Service Management:"
echo "  • Status:  ssh $PRODUCTION_SERVER 'systemctl status $SERVICE_NAME'"
echo "  • Logs:    ssh $PRODUCTION_SERVER 'journalctl -u $SERVICE_NAME -f'"
echo "  • Restart: ssh $PRODUCTION_SERVER 'systemctl restart $SERVICE_NAME'"
echo "  • Stop:    ssh $PRODUCTION_SERVER 'systemctl stop $SERVICE_NAME'"
echo
info "Application URLs:"
echo "  • Web UI:     http://192.168.0.131:7878"
echo "  • Health:     http://192.168.0.131:7878/health"
echo "  • Metrics:    http://192.168.0.131:9090/metrics"
echo "  • API Docs:   http://192.168.0.131:7878/api/docs"
echo
info "Database:"
echo "  • Connect: ssh $PRODUCTION_SERVER 'sudo -u postgres psql -d radarr'"
echo "  • Backup:  ssh $PRODUCTION_SERVER 'sudo -u postgres pg_dump radarr > backup.sql'"
echo
warning "Post-Deployment Tasks:"
echo "  1. Update database password in /opt/radarr/.env"
echo "  2. Configure API keys for external services"
echo "  3. Set up SSL/TLS certificate (recommended)"
echo "  4. Configure monitoring and alerts"
echo "  5. Review and adjust resource limits if needed"
echo
info "For troubleshooting, check the deployment guide in README.md"