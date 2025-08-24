#!/bin/bash

# Radarr MVP Production Database Setup Script
# Sets up PostgreSQL database for production deployment
# Author: Radarr MVP Team
# Date: 2025-08-24

set -e
set -o pipefail

# Configuration
DB_NAME="radarr"
DB_USER="radarr"
DB_HOST="localhost"
DB_PORT="5432"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Functions
log() { echo -e "${2}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"; }
error() { log "ERROR: $1" "$RED"; exit 1; }
success() { log "✓ $1" "$GREEN"; }
warning() { log "⚠ $1" "$YELLOW"; }
info() { log "ℹ $1" "$BLUE"; }

echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Radarr MVP Production Database Setup Script       ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo

# Parse arguments
REMOTE_HOST=""
RESET_DB=false
GENERATE_PASSWORD=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --host)
            REMOTE_HOST="$2"
            shift 2
            ;;
        --reset)
            RESET_DB=true
            shift
            ;;
        --generate-password)
            GENERATE_PASSWORD=true
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --host HOST           Remote host to setup database on"
            echo "  --reset               Reset existing database (DESTRUCTIVE)"
            echo "  --generate-password   Generate a secure random password"
            echo "  --help                Show this help message"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
done

# Generate secure password if requested
if [ "$GENERATE_PASSWORD" = true ]; then
    DB_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
    info "Generated secure password: $DB_PASSWORD"
    echo -e "${YELLOW}⚠ Save this password securely! You'll need it for the .env file${NC}"
else
    # Prompt for password
    echo -n "Enter database password for user '$DB_USER': "
    read -s DB_PASSWORD
    echo
    
    if [ -z "$DB_PASSWORD" ]; then
        error "Password cannot be empty"
    fi
fi

# Function to execute SQL
exec_sql() {
    local sql="$1"
    local db="${2:-postgres}"
    
    if [ -n "$REMOTE_HOST" ]; then
        ssh "$REMOTE_HOST" "sudo -u postgres psql -d $db -c \"$sql\""
    else
        sudo -u postgres psql -d "$db" -c "$sql"
    fi
}

# Function to check if database exists
db_exists() {
    local result
    if [ -n "$REMOTE_HOST" ]; then
        result=$(ssh "$REMOTE_HOST" "sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -w $DB_NAME | wc -l")
    else
        result=$(sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -w "$DB_NAME" | wc -l)
    fi
    [ "$result" -eq 1 ]
}

# Check PostgreSQL installation
info "Checking PostgreSQL installation..."
if [ -n "$REMOTE_HOST" ]; then
    if ! ssh "$REMOTE_HOST" "command -v psql >/dev/null 2>&1"; then
        error "PostgreSQL is not installed on $REMOTE_HOST"
    fi
    success "PostgreSQL found on $REMOTE_HOST"
else
    if ! command -v psql >/dev/null 2>&1; then
        error "PostgreSQL is not installed locally"
    fi
    success "PostgreSQL found locally"
fi

# Check if database exists
if db_exists; then
    if [ "$RESET_DB" = true ]; then
        warning "Database '$DB_NAME' exists and will be RESET"
        echo -e "${RED}⚠ WARNING: This will DELETE all data in the database!${NC}"
        echo -n "Type 'RESET' to confirm: "
        read confirmation
        
        if [ "$confirmation" != "RESET" ]; then
            warning "Reset cancelled"
            exit 0
        fi
        
        info "Dropping existing database..."
        exec_sql "DROP DATABASE IF EXISTS $DB_NAME;"
        exec_sql "DROP USER IF EXISTS $DB_USER;"
        success "Existing database and user dropped"
    else
        warning "Database '$DB_NAME' already exists"
        echo "Use --reset flag to reset the database"
        exit 0
    fi
fi

# Create user
info "Creating database user '$DB_USER'..."
exec_sql "CREATE USER $DB_USER WITH PASSWORD '$DB_PASSWORD';"
success "User created"

# Create database
info "Creating database '$DB_NAME'..."
exec_sql "CREATE DATABASE $DB_NAME OWNER $DB_USER;"
success "Database created"

# Grant privileges
info "Granting privileges..."
exec_sql "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;"
exec_sql "ALTER DATABASE $DB_NAME SET timezone TO 'UTC';"
success "Privileges granted"

# Create extensions
info "Creating database extensions..."
exec_sql "CREATE EXTENSION IF NOT EXISTS pg_stat_statements;" "$DB_NAME"
exec_sql "CREATE EXTENSION IF NOT EXISTS pgcrypto;" "$DB_NAME"
exec_sql "CREATE EXTENSION IF NOT EXISTS uuid-ossp;" "$DB_NAME"
success "Extensions created"

# Optimize PostgreSQL configuration
info "Applying production optimizations..."

if [ -n "$REMOTE_HOST" ]; then
    ssh "$REMOTE_HOST" << 'EOF'
        # Create configuration snippet for Radarr
        sudo tee /etc/postgresql/*/main/conf.d/radarr.conf > /dev/null << 'CONFIG'
# Radarr MVP PostgreSQL Optimizations
# Generated by setup script

# Connection Settings
max_connections = 200
superuser_reserved_connections = 3

# Memory Settings (adjust based on available RAM)
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
work_mem = 4MB

# Checkpoint Settings
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1

# Logging
log_min_duration_statement = 1000  # Log slow queries (>1s)
log_checkpoints = on
log_connections = on
log_disconnections = on
log_lock_waits = on
log_temp_files = 0

# Performance
effective_io_concurrency = 200
max_parallel_workers_per_gather = 2
max_parallel_workers = 8
max_parallel_maintenance_workers = 2

# Enable query performance monitoring
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.max = 10000
pg_stat_statements.track = all
CONFIG

        # Reload PostgreSQL
        sudo systemctl reload postgresql
EOF
else
    # Local configuration
    sudo tee /etc/postgresql/*/main/conf.d/radarr.conf > /dev/null << 'CONFIG'
# Radarr MVP PostgreSQL Optimizations
# Generated by setup script

# Connection Settings
max_connections = 200
superuser_reserved_connections = 3

# Memory Settings (adjust based on available RAM)
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
work_mem = 4MB

# Checkpoint Settings
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1

# Logging
log_min_duration_statement = 1000  # Log slow queries (>1s)
log_checkpoints = on
log_connections = on
log_disconnections = on
log_lock_waits = on
log_temp_files = 0

# Performance
effective_io_concurrency = 200
max_parallel_workers_per_gather = 2
max_parallel_workers = 8
max_parallel_maintenance_workers = 2

# Enable query performance monitoring
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.max = 10000
pg_stat_statements.track = all
CONFIG

    sudo systemctl reload postgresql
fi

success "PostgreSQL optimizations applied"

# Create database schema preparation script
info "Creating schema preparation script..."

SCHEMA_SCRIPT="/tmp/radarr_schema.sql"
cat > "$SCHEMA_SCRIPT" << 'SCHEMA'
-- Radarr MVP Database Schema Preparation
-- This script prepares the database for migrations

-- Create schema if needed
CREATE SCHEMA IF NOT EXISTS public;

-- Set search path
ALTER DATABASE radarr SET search_path TO public;

-- Create initial tables will be handled by migrations
-- This is just to prepare the database

-- Create indexes for common queries (will be refined by migrations)
-- These are placeholder comments for documentation

-- Movies table (created by migration)
-- CREATE TABLE IF NOT EXISTS movies ...

-- Quality profiles table (created by migration)
-- CREATE TABLE IF NOT EXISTS quality_profiles ...

-- Indexers table (created by migration)
-- CREATE TABLE IF NOT EXISTS indexers ...

-- Download queue table (created by migration)
-- CREATE TABLE IF NOT EXISTS download_queue ...

-- Grant permissions
GRANT ALL ON SCHEMA public TO radarr;
GRANT ALL ON ALL TABLES IN SCHEMA public TO radarr;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO radarr;
GRANT ALL ON ALL FUNCTIONS IN SCHEMA public TO radarr;

-- Set default privileges
ALTER DEFAULT PRIVILEGES IN SCHEMA public 
    GRANT ALL ON TABLES TO radarr;
ALTER DEFAULT PRIVILEGES IN SCHEMA public 
    GRANT ALL ON SEQUENCES TO radarr;
ALTER DEFAULT PRIVILEGES IN SCHEMA public 
    GRANT ALL ON FUNCTIONS TO radarr;
SCHEMA

# Apply schema preparation
if [ -n "$REMOTE_HOST" ]; then
    scp "$SCHEMA_SCRIPT" "$REMOTE_HOST:/tmp/"
    ssh "$REMOTE_HOST" "sudo -u postgres psql -d $DB_NAME -f /tmp/radarr_schema.sql"
    ssh "$REMOTE_HOST" "rm /tmp/radarr_schema.sql"
else
    sudo -u postgres psql -d "$DB_NAME" -f "$SCHEMA_SCRIPT"
fi
rm "$SCHEMA_SCRIPT"

success "Schema preparation completed"

# Test connection
info "Testing database connection..."

CONNECTION_STRING="postgresql://$DB_USER:[PASSWORD]@$DB_HOST:$DB_PORT/$DB_NAME"

if [ -n "$REMOTE_HOST" ]; then
    # For remote, we'll create a test script
    ssh "$REMOTE_HOST" << EOF
        export PGPASSWORD='$DB_PASSWORD'
        if psql -h localhost -U $DB_USER -d $DB_NAME -c 'SELECT version();' >/dev/null 2>&1; then
            echo "Connection test successful"
        else
            echo "Connection test failed"
            exit 1
        fi
EOF
else
    export PGPASSWORD="$DB_PASSWORD"
    if psql -h localhost -U "$DB_USER" -d "$DB_NAME" -c 'SELECT version();' >/dev/null 2>&1; then
        success "Connection test successful"
    else
        error "Connection test failed"
    fi
fi

# Create backup script
info "Creating backup script..."

BACKUP_SCRIPT="/usr/local/bin/radarr-backup.sh"
if [ -n "$REMOTE_HOST" ]; then
    ssh "$REMOTE_HOST" "sudo tee $BACKUP_SCRIPT" > /dev/null << 'BACKUP'
#!/bin/bash
# Radarr database backup script

BACKUP_DIR="/opt/radarr/backups"
DB_NAME="radarr"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/radarr_$TIMESTAMP.sql"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Perform backup
sudo -u postgres pg_dump "$DB_NAME" > "$BACKUP_FILE"

# Compress
gzip "$BACKUP_FILE"

# Keep only last 7 backups
ls -t "$BACKUP_DIR"/*.gz 2>/dev/null | tail -n +8 | xargs rm -f 2>/dev/null || true

echo "Backup completed: ${BACKUP_FILE}.gz"
BACKUP

    ssh "$REMOTE_HOST" "sudo chmod +x $BACKUP_SCRIPT"
else
    sudo tee "$BACKUP_SCRIPT" > /dev/null << 'BACKUP'
#!/bin/bash
# Radarr database backup script

BACKUP_DIR="/opt/radarr/backups"
DB_NAME="radarr"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/radarr_$TIMESTAMP.sql"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Perform backup
sudo -u postgres pg_dump "$DB_NAME" > "$BACKUP_FILE"

# Compress
gzip "$BACKUP_FILE"

# Keep only last 7 backups
ls -t "$BACKUP_DIR"/*.gz 2>/dev/null | tail -n +8 | xargs rm -f 2>/dev/null || true

echo "Backup completed: ${BACKUP_FILE}.gz"
BACKUP

    sudo chmod +x "$BACKUP_SCRIPT"
fi

success "Backup script created at $BACKUP_SCRIPT"

# Create restore script
info "Creating restore script..."

RESTORE_SCRIPT="/usr/local/bin/radarr-restore.sh"
if [ -n "$REMOTE_HOST" ]; then
    ssh "$REMOTE_HOST" "sudo tee $RESTORE_SCRIPT" > /dev/null << 'RESTORE'
#!/bin/bash
# Radarr database restore script

if [ $# -eq 0 ]; then
    echo "Usage: $0 <backup_file.sql.gz>"
    exit 1
fi

BACKUP_FILE="$1"
DB_NAME="radarr"

if [ ! -f "$BACKUP_FILE" ]; then
    echo "Error: Backup file not found: $BACKUP_FILE"
    exit 1
fi

echo "Restoring from: $BACKUP_FILE"
echo "WARNING: This will replace all data in the database!"
echo -n "Type 'RESTORE' to confirm: "
read confirmation

if [ "$confirmation" != "RESTORE" ]; then
    echo "Restore cancelled"
    exit 0
fi

# Stop the service
systemctl stop radarr 2>/dev/null || true

# Restore
gunzip -c "$BACKUP_FILE" | sudo -u postgres psql "$DB_NAME"

# Start the service
systemctl start radarr 2>/dev/null || true

echo "Restore completed"
RESTORE

    ssh "$REMOTE_HOST" "sudo chmod +x $RESTORE_SCRIPT"
else
    sudo tee "$RESTORE_SCRIPT" > /dev/null << 'RESTORE'
#!/bin/bash
# Radarr database restore script

if [ $# -eq 0 ]; then
    echo "Usage: $0 <backup_file.sql.gz>"
    exit 1
fi

BACKUP_FILE="$1"
DB_NAME="radarr"

if [ ! -f "$BACKUP_FILE" ]; then
    echo "Error: Backup file not found: $BACKUP_FILE"
    exit 1
fi

echo "Restoring from: $BACKUP_FILE"
echo "WARNING: This will replace all data in the database!"
echo -n "Type 'RESTORE' to confirm: "
read confirmation

if [ "$confirmation" != "RESTORE" ]; then
    echo "Restore cancelled"
    exit 0
fi

# Stop the service
systemctl stop radarr 2>/dev/null || true

# Restore
gunzip -c "$BACKUP_FILE" | sudo -u postgres psql "$DB_NAME"

# Start the service
systemctl start radarr 2>/dev/null || true

echo "Restore completed"
RESTORE

    sudo chmod +x "$RESTORE_SCRIPT"
fi

success "Restore script created at $RESTORE_SCRIPT"

# Summary
echo
echo -e "${GREEN}╔════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║          Database Setup Complete                      ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════╝${NC}"
echo
success "PostgreSQL database configured successfully!"
echo
info "Database Details:"
echo "  • Database Name: $DB_NAME"
echo "  • Database User: $DB_USER"
echo "  • Database Host: $DB_HOST"
echo "  • Database Port: $DB_PORT"
echo
info "Connection String:"
echo "  postgresql://$DB_USER:[YOUR_PASSWORD]@$DB_HOST:$DB_PORT/$DB_NAME"
echo
warning "Important: Save the database password securely!"
echo
info "Backup Management:"
echo "  • Backup:  $BACKUP_SCRIPT"
echo "  • Restore: $RESTORE_SCRIPT <backup_file.sql.gz>"
echo
info "Next Steps:"
echo "  1. Update DATABASE_URL in your .env file with the connection string"
echo "  2. Run database migrations: sqlx migrate run"
echo "  3. Set up automated backups (cron job recommended)"
echo "  4. Monitor database performance with pg_stat_statements"
echo
success "Database is ready for production use!"