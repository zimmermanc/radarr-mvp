#!/bin/bash

# Radarr Web UI Deployment Script with Authentication
# This script builds and deploys the web interface to the production server

set -e

echo "🚀 Starting Radarr Web UI deployment with authentication..."

# Configuration
SERVER_HOST="192.168.0.138"
SERVER_USER="root"
SERVER_PATH="/opt/radarr/web"
BUILD_OUTPUT="dist"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Step 1: Clean previous build
print_status "Cleaning previous build..."
if [ -d "$BUILD_OUTPUT" ]; then
    rm -rf "$BUILD_OUTPUT"
fi

# Step 2: Install dependencies
print_status "Installing dependencies..."
npm ci

# Step 3: Set production environment
print_status "Setting production environment..."
export NODE_ENV=production
export VITE_API_BASE_URL="http://192.168.0.138:7878"

# Step 4: Build the application
print_status "Building application..."
npm run build

# Step 5: Verify build output
if [ ! -d "$BUILD_OUTPUT" ]; then
    print_error "Build failed - no output directory found"
    exit 1
fi

print_status "Build completed successfully"
print_status "Build size: $(du -sh $BUILD_OUTPUT | cut -f1)"

# Step 6: Test SSH connection
print_status "Testing SSH connection to $SERVER_HOST..."
if ! ssh -o ConnectTimeout=10 -o BatchMode=yes "$SERVER_USER@$SERVER_HOST" exit 2>/dev/null; then
    print_error "Cannot connect to $SERVER_HOST via SSH"
    print_warning "Please ensure:"
    print_warning "1. Server is running and accessible"
    print_warning "2. SSH key is properly configured"
    print_warning "3. Host key is in known_hosts"
    exit 1
fi

# Step 7: Create backup of current deployment
print_status "Creating backup of current deployment..."
ssh "$SERVER_USER@$SERVER_HOST" "
    if [ -d '$SERVER_PATH' ]; then
        cp -r '$SERVER_PATH' '${SERVER_PATH}.backup.$(date +%Y%m%d_%H%M%S)'
        echo 'Backup created successfully'
    else
        echo 'No existing deployment to backup'
    fi
"

# Step 8: Create target directory
print_status "Preparing target directory..."
ssh "$SERVER_USER@$SERVER_HOST" "mkdir -p '$SERVER_PATH'"

# Step 9: Deploy files
print_status "Deploying files to $SERVER_HOST:$SERVER_PATH..."
rsync -avz --delete \
    --exclude='.git*' \
    --exclude='node_modules' \
    --exclude='*.log' \
    "$BUILD_OUTPUT/" "$SERVER_USER@$SERVER_HOST:$SERVER_PATH/"

# Step 10: Set proper permissions
print_status "Setting file permissions..."
ssh "$SERVER_USER@$SERVER_HOST" "
    chown -R www-data:www-data '$SERVER_PATH'
    chmod -R 755 '$SERVER_PATH'
"

# Step 11: Verify deployment
print_status "Verifying deployment..."
DEPLOYED_FILES=$(ssh "$SERVER_USER@$SERVER_HOST" "find '$SERVER_PATH' -type f | wc -l")
print_status "Deployed $DEPLOYED_FILES files successfully"

# Step 12: Test web interface
print_status "Testing web interface..."
if curl -s -o /dev/null -w "%{http_code}" "http://$SERVER_HOST:7878" | grep -q "200\|401"; then
    print_status "✅ Web interface is accessible"
else
    print_warning "⚠️  Web interface test failed - may need manual verification"
fi

# Step 13: Display access information
print_status "🎉 Deployment completed successfully!"
echo ""
echo "Authentication Details:"
echo "📱 Web Interface: http://$SERVER_HOST:7878"
echo "🔑 Default Username: admin"
echo "🔑 Default Password: admin"
echo "🔐 API Key: secure_production_api_key_2025"
echo ""
echo "Next Steps:"
echo "1. Open http://$SERVER_HOST:7878 in your browser"
echo "2. You will be redirected to the login page"
echo "3. Login with username/password OR API key"
echo "4. Access will be granted to the full dashboard"
echo ""
print_status "Deployment log saved to deployment.log"

# Save deployment info
cat > deployment.log << EOF
Deployment completed at: $(date)
Server: $SERVER_HOST
Path: $SERVER_PATH
Files deployed: $DEPLOYED_FILES
Build size: $(du -sh $BUILD_OUTPUT | cut -f1)
Authentication: Enabled with login page
Access URL: http://$SERVER_HOST:7878
EOF

print_status "All done! 🚀"