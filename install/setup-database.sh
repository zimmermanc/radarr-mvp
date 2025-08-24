#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
DB_NAME="radarr"
DB_USER="radarr"
DB_HOST="localhost"
DB_PORT="5432"
GENERATE_PASSWORD=false
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
Radarr MVP Database Setup Script

Usage: $0 [OPTIONS]

Options:
    --help                  Show this help message
    --db-name NAME          Database name (default: radarr)
    --db-user USER          Database user (default: radarr)
    --db-host HOST          Database host (default: localhost)
    --db-port PORT          Database port (default: 5432)
    --generate-password     Generate a secure random password
    --install-postgres      Install PostgreSQL if not present (Ubuntu/Debian only)
    --password PASSWORD     Specify password directly (not recommended)

Examples:
    $0 --generate-password
    $0 --db-name myradarr --db-user myuser --generate-password
    $0 --install-postgres --generate-password

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            show_help
            exit 0
            ;;
        --db-name)
            DB_NAME="$2"
            shift 2
            ;;
        --db-user)
            DB_USER="$2"
            shift 2
            ;;
        --db-host)
            DB_HOST="$2"
            shift 2
            ;;
        --db-port)
            DB_PORT="$2"
            shift 2
            ;;
        --generate-password)
            GENERATE_PASSWORD=true
            shift
            ;;
        --install-postgres)
            INSTALL_POSTGRES=true
            shift
            ;;
        --password)
            DB_PASSWORD="$2"
            shift 2
            ;;
        *)
            print_error "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

print_info "Radarr MVP Database Setup"
print_info "========================="

# Generate password if requested
if [ "$GENERATE_PASSWORD" = true ]; then
    DB_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
    print_info "Generated secure password: $DB_PASSWORD"
fi

# Check if password is set
if [ -z "$DB_PASSWORD" ]; then
    print_error "No password specified. Use --password or --generate-password"
    exit 1
fi

# Install PostgreSQL if requested
if [ "$INSTALL_POSTGRES" = true ]; then
    print_info "Installing PostgreSQL..."
    
    # Detect OS
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y postgresql postgresql-contrib postgresql-client
        sudo systemctl start postgresql
        sudo systemctl enable postgresql
        print_success "PostgreSQL installed and started"
    elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y postgresql-server postgresql-contrib
        sudo postgresql-setup initdb
        sudo systemctl start postgresql
        sudo systemctl enable postgresql
        print_success "PostgreSQL installed and started"
    elif command -v brew >/dev/null 2>&1; then
        brew install postgresql
        brew services start postgresql
        print_success "PostgreSQL installed and started"
    else
        print_error "Cannot auto-install PostgreSQL on this system"
        print_info "Please install PostgreSQL manually and run this script again"
        exit 1
    fi
fi

# Check if PostgreSQL is running
if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" >/dev/null 2>&1; then
    print_error "PostgreSQL is not running or not accessible at $DB_HOST:$DB_PORT"
    print_info "Please start PostgreSQL and ensure it's accessible"
    exit 1
fi

print_success "PostgreSQL is running"

# Create database user
print_info "Creating database user '$DB_USER'..."
if sudo -u postgres psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='$DB_USER'" | grep -q 1; then
    print_warning "User '$DB_USER' already exists"
    print_info "Updating password..."
    sudo -u postgres psql -c "ALTER USER \"$DB_USER\" WITH PASSWORD '$DB_PASSWORD';"
else
    sudo -u postgres psql -c "CREATE USER \"$DB_USER\" WITH PASSWORD '$DB_PASSWORD';"
    print_success "User '$DB_USER' created"
fi

# Create database
print_info "Creating database '$DB_NAME'..."
if sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -qw "$DB_NAME"; then
    print_warning "Database '$DB_NAME' already exists"
else
    sudo -u postgres createdb -O "$DB_USER" "$DB_NAME"
    print_success "Database '$DB_NAME' created"
fi

# Grant permissions
print_info "Granting permissions..."
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE \"$DB_NAME\" TO \"$DB_USER\";"
sudo -u postgres psql -d "$DB_NAME" -c "GRANT ALL ON SCHEMA public TO \"$DB_USER\";"
sudo -u postgres psql -d "$DB_NAME" -c "GRANT ALL ON ALL TABLES IN SCHEMA public TO \"$DB_USER\";"
sudo -u postgres psql -d "$DB_NAME" -c "GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO \"$DB_USER\";"
sudo -u postgres psql -d "$DB_NAME" -c "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO \"$DB_USER\";"
sudo -u postgres psql -d "$DB_NAME" -c "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO \"$DB_USER\";"

print_success "Permissions granted"

# Test connection
print_info "Testing database connection..."
export PGPASSWORD="$DB_PASSWORD"
if psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "SELECT version();" >/dev/null 2>&1; then
    print_success "Database connection successful"
else
    print_error "Database connection failed"
    exit 1
fi

# Create .env template
print_info "Creating .env configuration..."
DATABASE_URL="postgresql://$DB_USER:$DB_PASSWORD@$DB_HOST:$DB_PORT/$DB_NAME"

cat > .env << EOF
# Radarr MVP Configuration
# Generated by setup-database.sh on $(date)

# Database Configuration
DATABASE_URL="$DATABASE_URL"
DATABASE_MAX_CONNECTIONS=10

# Server Configuration
HOST=0.0.0.0
PORT=7878
API_KEY=changeme123_please_change_this

# TMDB Configuration (required for movie lookup)
TMDB_API_KEY=your_tmdb_api_key_here

# Optional: HDBits Configuration
# HDBITS_USERNAME=your_username
# HDBITS_PASSKEY=your_passkey

# Optional: Streaming Services
# TRAKT_CLIENT_ID=your_trakt_client_id
# TRAKT_CLIENT_SECRET=your_trakt_client_secret
# WATCHMODE_API_KEY=your_watchmode_api_key

# Logging
RUST_LOG=info

EOF

print_success ".env file created"

print_info ""
print_info "Database Setup Complete!"
print_info "======================="
print_info "Database Name: $DB_NAME"
print_info "Database User: $DB_USER"
print_info "Database Host: $DB_HOST:$DB_PORT"
print_info "Database URL: $DATABASE_URL"
print_info ""
print_warning "Next Steps:"
print_info "1. Update the API_KEY in .env file"
print_info "2. Add your TMDB API key to .env file"
print_info "3. Run database migrations: sqlx migrate run"
print_info "4. Start Radarr MVP: ./radarr-mvp"
print_info ""
print_warning "Security Note:"
print_info "Store your database password securely:"
echo "$DB_PASSWORD" > .db_password
chmod 600 .db_password
print_info "Password saved to .db_password (restricted access)"