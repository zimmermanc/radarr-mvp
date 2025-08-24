#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
INSTALL_DIR="/opt/radarr"
SERVICE_USER="radarr"
CREATE_SERVICE=true
START_SERVICE=true
SETUP_DATABASE=false
INSTALL_POSTGRES=false

# Print colored output
print_info() {
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

# Help function
show_help() {
    cat << EOF
Radarr MVP Installer Script

Usage: $0 [OPTIONS]

Options:
    --help                  Show this help message
    --install-dir DIR       Installation directory (default: /opt/radarr)
    --user USER             Service user (default: radarr)
    --no-service            Don't create systemd service
    --no-start              Don't start service after installation
    --setup-database        Run database setup script
    --install-postgres      Install PostgreSQL (requires --setup-database)
    --uninstall             Uninstall Radarr MVP

Examples:
    $0                      # Basic installation
    $0 --setup-database     # Install with database setup
    $0 --install-postgres --setup-database  # Full installation
    $0 --uninstall          # Remove installation

EOF
}

# Parse command line arguments
UNINSTALL=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            show_help
            exit 0
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --user)
            SERVICE_USER="$2"
            shift 2
            ;;
        --no-service)
            CREATE_SERVICE=false
            shift
            ;;
        --no-start)
            START_SERVICE=false
            shift
            ;;
        --setup-database)
            SETUP_DATABASE=true
            shift
            ;;
        --install-postgres)
            INSTALL_POSTGRES=true
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Check if running as root
if [[ $EUID -ne 0 ]]; then
   print_error "This script must be run as root (use sudo)"
   exit 1
fi

# Uninstall function
uninstall() {
    print_info "Uninstalling Radarr MVP..."
    
    # Stop and disable service
    if systemctl is-active --quiet radarr 2>/dev/null; then
        systemctl stop radarr
        print_info "Stopped radarr service"
    fi
    
    if systemctl is-enabled --quiet radarr 2>/dev/null; then
        systemctl disable radarr
        print_info "Disabled radarr service"
    fi
    
    # Remove service file
    if [ -f /etc/systemd/system/radarr.service ]; then
        rm /etc/systemd/system/radarr.service
        systemctl daemon-reload
        print_info "Removed systemd service"
    fi
    
    # Remove installation directory
    if [ -d "$INSTALL_DIR" ]; then
        print_warning "Removing $INSTALL_DIR (this will delete all data!)"
        read -p "Are you sure? [y/N]: " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$INSTALL_DIR"
            print_success "Removed $INSTALL_DIR"
        else
            print_info "Keeping $INSTALL_DIR"
        fi
    fi
    
    # Remove user
    if id "$SERVICE_USER" &>/dev/null; then
        userdel "$SERVICE_USER" 2>/dev/null || true
        print_info "Removed user $SERVICE_USER"
    fi
    
    print_success "Uninstallation complete"
    exit 0
}

# Handle uninstall
if [ "$UNINSTALL" = true ]; then
    uninstall
fi

print_info "Radarr MVP Installer"
print_info "===================="

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VER=$VERSION_ID
elif type lsb_release >/dev/null 2>&1; then
    OS=$(lsb_release -si | tr '[:upper:]' '[:lower:]')
    VER=$(lsb_release -sr)
else
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
fi

print_info "Detected OS: $OS"

# Create service user
if ! id "$SERVICE_USER" &>/dev/null; then
    print_info "Creating user $SERVICE_USER..."
    useradd --system --no-create-home --shell /bin/false --home-dir "$INSTALL_DIR" "$SERVICE_USER"
    print_success "User $SERVICE_USER created"
else
    print_info "User $SERVICE_USER already exists"
fi

# Create installation directory
print_info "Creating installation directory: $INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR/data"
mkdir -p "$INSTALL_DIR/backups"
mkdir -p "$INSTALL_DIR/logs"

# Copy binary (assume it's in the current directory)
if [ -f "radarr-mvp" ]; then
    print_info "Installing Radarr MVP binary..."
    cp radarr-mvp "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/radarr-mvp"
    print_success "Binary installed"
else
    print_error "radarr-mvp binary not found in current directory"
    print_info "Please ensure you run this script from the directory containing the binary"
    exit 1
fi

# Copy migrations if they exist
if [ -d "migrations" ]; then
    print_info "Installing database migrations..."
    cp -r migrations "$INSTALL_DIR/"
    print_success "Migrations installed"
fi

# Copy configuration template
if [ -f ".env.example" ]; then
    cp .env.example "$INSTALL_DIR/.env"
elif [ -f "config/production.env.template" ]; then
    cp config/production.env.template "$INSTALL_DIR/.env"
else
    print_info "Creating default configuration..."
    cat > "$INSTALL_DIR/.env" << EOF
# Radarr MVP Configuration
HOST=0.0.0.0
PORT=7878
API_KEY=changeme123_please_change_this
DATABASE_URL=postgresql://radarr:CHANGE_ME_PASSWORD@localhost/radarr
DATABASE_MAX_CONNECTIONS=10
TMDB_API_KEY=your_tmdb_api_key_here
RUST_LOG=info
EOF
fi

# Set ownership
chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR"
chmod 600 "$INSTALL_DIR/.env"

print_success "Installation directory configured"

# Setup database if requested
if [ "$SETUP_DATABASE" = true ]; then
    print_info "Setting up database..."
    
    # Check if setup-database.sh exists
    if [ -f "setup-database.sh" ]; then
        # Build database setup command
        DB_SETUP_CMD="./setup-database.sh --generate-password"
        if [ "$INSTALL_POSTGRES" = true ]; then
            DB_SETUP_CMD="$DB_SETUP_CMD --install-postgres"
        fi
        
        # Run database setup
        cd "$INSTALL_DIR"
        cp "$(dirname "$0")/setup-database.sh" .
        chmod +x setup-database.sh
        
        print_info "Running: $DB_SETUP_CMD"
        $DB_SETUP_CMD
        
        # Move generated .env to installation directory
        if [ -f ".env" ]; then
            cp .env "$INSTALL_DIR/.env"
            chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/.env"
            chmod 600 "$INSTALL_DIR/.env"
        fi
        
        print_success "Database setup complete"
    else
        print_warning "setup-database.sh not found, skipping database setup"
    fi
fi

# Create systemd service
if [ "$CREATE_SERVICE" = true ]; then
    print_info "Creating systemd service..."
    
    cat > /etc/systemd/system/radarr.service << EOF
[Unit]
Description=Radarr MVP - Movie Collection Manager
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
ExecStart=$INSTALL_DIR/radarr-mvp
WorkingDirectory=$INSTALL_DIR
Restart=always
RestartSec=10

# Environment
EnvironmentFile=-$INSTALL_DIR/.env

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_DIR
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true

# Resource limits
MemoryMax=2G
CPUQuota=200%

# Logging
StandardOutput=append:$INSTALL_DIR/logs/radarr.log
StandardError=append:$INSTALL_DIR/logs/radarr-error.log

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable radarr
    print_success "Systemd service created and enabled"
fi

# Start service if requested
if [ "$START_SERVICE" = true ] && [ "$CREATE_SERVICE" = true ]; then
    print_info "Starting Radarr MVP service..."
    systemctl start radarr
    sleep 3
    
    if systemctl is-active --quiet radarr; then
        print_success "Service started successfully"
    else
        print_error "Service failed to start"
        print_info "Check logs with: journalctl -u radarr -f"
    fi
fi

# Create backup script
print_info "Creating backup script..."
cat > "$INSTALL_DIR/backup.sh" << 'EOF'
#!/bin/bash
BACKUP_DIR="/opt/radarr/backups"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/radarr_backup_$DATE.sql.gz"

# Extract database info from .env
DB_URL=$(grep DATABASE_URL /opt/radarr/.env | cut -d'=' -f2- | tr -d '"')
if [ -n "$DB_URL" ]; then
    echo "Creating database backup: $BACKUP_FILE"
    pg_dump "$DB_URL" | gzip > "$BACKUP_FILE"
    echo "Backup created: $BACKUP_FILE"
    
    # Keep only last 7 backups
    find "$BACKUP_DIR" -name "radarr_backup_*.sql.gz" -type f -mtime +7 -delete
else
    echo "Could not find DATABASE_URL in .env file"
    exit 1
fi
EOF

chmod +x "$INSTALL_DIR/backup.sh"
chown "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR/backup.sh"

# Setup log rotation
print_info "Setting up log rotation..."
cat > /etc/logrotate.d/radarr << EOF
$INSTALL_DIR/logs/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    su $SERVICE_USER $SERVICE_USER
    postrotate
        systemctl reload radarr 2>/dev/null || true
    endscript
}
EOF

print_success "Log rotation configured"

# Final information
print_info ""
print_success "Installation Complete!"
print_info "====================="
print_info "Installation Directory: $INSTALL_DIR"
print_info "Service User: $SERVICE_USER"
print_info "Configuration: $INSTALL_DIR/.env"
print_info ""

if [ "$CREATE_SERVICE" = true ]; then
    print_info "Service Commands:"
    print_info "  Start:   systemctl start radarr"
    print_info "  Stop:    systemctl stop radarr"
    print_info "  Restart: systemctl restart radarr"
    print_info "  Status:  systemctl status radarr"
    print_info "  Logs:    journalctl -u radarr -f"
    print_info ""
fi

print_info "Web Interface: http://localhost:7878"
print_info "API Documentation: http://localhost:7878/docs"
print_info ""
print_warning "Next Steps:"
print_info "1. Edit $INSTALL_DIR/.env to configure API keys"
print_info "2. Update API_KEY in the configuration"
print_info "3. Add your TMDB API key"
if [ "$SETUP_DATABASE" = false ]; then
    print_info "4. Setup database manually or run setup-database.sh"
fi

if systemctl is-active --quiet radarr 2>/dev/null; then
    print_success "Radarr MVP is running!"
else
    print_info "Start the service: systemctl start radarr"
fi