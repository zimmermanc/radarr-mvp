#!/bin/bash
# =============================================================================
# Radarr MVP Quick Start Script
# =============================================================================
# One-command setup and start for development

set -euo pipefail

# Color codes for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}"
cat << 'EOF'
    ____            __               __  ____
   / __ \____ _____/ /___ ______    /  |/  / |    / / __ \
  / /_/ / __ `/ __  / __ `/ ___/   / /|_/ /| |   / / /_/ /
 / _, _/ /_/ / /_/ / /_/ / /      / /  / / | |  / / ____/
/_/ |_|\__,_/\__,_/\__,_/_/      /_/  /_/  |_| /_/_/

                Quick Start Setup
EOF
echo -e "${NC}"

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Step 1: Check prerequisites
log_step "1/7 Checking prerequisites..."

if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

if ! docker compose version &> /dev/null; then
    echo "❌ Docker Compose is not available."
    exit 1
fi

log_info "✅ Docker and Docker Compose are available"

# Step 2: Setup environment file
log_step "2/7 Setting up environment configuration..."

cd "$SCRIPT_DIR"

if [[ ! -f ".env" ]]; then
    if [[ -f ".env.docker" ]]; then
        cp ".env.docker" ".env"
        log_info "Created .env from .env.docker template"
    elif [[ -f ".env.example" ]]; then
        cp ".env.example" ".env"
        log_info "Created .env from .env.example template"
    else
        log_warn "No environment template found, creating minimal .env"
        cat > .env << 'EOE'
# Minimal Radarr MVP Configuration
POSTGRES_PASSWORD=radarr_dev_password
RADARR_PORT=7878
DATABASE_URL=postgresql://radarr:radarr@postgres:5432/radarr
RUST_LOG=info
EOE
    fi
else
    log_info "Using existing .env file"
fi

# Step 3: Create required directories
log_step "3/7 Creating required directories..."

mkdir -p data/{movies,downloads}
mkdir -p dev-data/{postgres,redis}
mkdir -p config/{dev,prod}
mkdir -p backups/postgres

log_info "Directories created successfully"

# Step 4: Check for API keys
log_step "4/7 Checking API configuration..."

if ! grep -q "TMDB_API_KEY=your_tmdb" .env 2>/dev/null; then
    log_info "TMDB API key appears to be configured"
else
    log_warn "TMDB API key not configured (optional for basic functionality)"
    echo "  Get a free API key from: https://www.themoviedb.org/settings/api"
fi

if ! grep -q "PROWLARR_API_KEY=your_prowlarr" .env 2>/dev/null; then
    log_info "Prowlarr API key appears to be configured"
else
    log_warn "Prowlarr API key not configured (required for full functionality)"
    echo "  Configure Prowlarr first, then get API key from Settings > General"
fi

# Step 5: Pull base images
log_step "5/7 Pulling Docker images..."
docker compose pull postgres redis

# Step 6: Build application
log_step "6/7 Building Radarr MVP application..."
docker compose build radarr

# Step 7: Start services
log_step "7/7 Starting services..."

echo ""
log_info "Starting Radarr MVP in development mode..."
echo "This may take a moment for the first start..."

# Start in detached mode first
docker compose up -d postgres redis

# Wait for database
log_info "Waiting for database to be ready..."
sleep 10

# Start Radarr
docker compose up radarr &
COMPOSE_PID=$!

# Wait a bit then show status
sleep 15

echo ""
echo "==============================================="
log_info "🎉 Radarr MVP is starting up!"
echo "==============================================="
echo ""
echo "📱 Services:"
echo "   • Radarr MVP:  http://localhost:7878"
echo "   • Health:      http://localhost:7878/health"
echo "   • API Status:  http://localhost:7878/api/v1/system/status"
echo "   • Metrics:     http://localhost:9090"
echo ""
echo "🗄️  Database:"
echo "   • PostgreSQL:  localhost:5432 (user: radarr, db: radarr_dev)"
echo "   • Redis:       localhost:6379"
echo ""
echo "📁 Directories:"
echo "   • Movies:      $SCRIPT_DIR/data/movies"
echo "   • Downloads:   $SCRIPT_DIR/data/downloads"
echo "   • Config:      $SCRIPT_DIR/config"
echo ""
echo "🔧 Management:"
echo "   • View logs:   docker compose logs -f"
echo "   • Stop:        docker compose down"
echo "   • Restart:     docker compose restart radarr"
echo "   • Shell:       docker compose exec radarr bash"
echo ""
echo "📚 Documentation:"
echo "   • Docker Guide: DOCKER.md"
echo "   • Full Setup:   CLAUDE.md"
echo ""

# Optional: Start full stack
echo -n "Would you like to start the full stack with Prowlarr and qBittorrent? (y/N): "
read -r response
if [[ "$response" =~ ^[Yy]$ ]]; then
    log_info "Starting full stack..."
    docker compose --profile full-stack up -d prowlarr qbittorrent
    
    echo ""
    echo "🎯 Additional Services:"
    echo "   • Prowlarr:    http://localhost:9696"
    echo "   • qBittorrent: http://localhost:8080"
    echo ""
fi

# Health check
log_info "Performing health check in 30 seconds..."
sleep 30

if curl -sf http://localhost:7878/health &> /dev/null; then
    echo "✅ Health check passed - Radarr MVP is running!"
else
    echo "⚠️  Health check failed - check logs with: docker compose logs radarr"
fi

echo ""
log_info "Setup complete! Press Ctrl+C to view logs, or close terminal to keep running in background."

# Wait for docker compose (allows Ctrl+C to interrupt and show logs)
wait $COMPOSE_PID 2>/dev/null || {
    echo ""
    log_info "Showing application logs (Ctrl+C to exit):"
    docker compose logs -f radarr
}