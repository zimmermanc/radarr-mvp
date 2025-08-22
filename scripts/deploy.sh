#!/bin/bash

# Radarr MVP Deployment Script
# Deploys to test server root@192.168.0.138

set -e  # Exit on any error

# Configuration
SERVER="root@192.168.0.138"
REMOTE_DIR="/opt/radarr"
SERVICE_NAME="radarr"
BINARY_NAME="unified-radarr"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Radarr MVP Deployment Script ===${NC}"
echo "Target: $SERVER"
echo "Service: $SERVICE_NAME"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from unified-radarr directory${NC}"
    exit 1
fi

# Build release binary
echo -e "${YELLOW}Building release binary...${NC}"
cargo build --release

if [ ! -f "target/release/$BINARY_NAME" ]; then
    echo -e "${RED}Error: Binary not found at target/release/$BINARY_NAME${NC}"
    exit 1
fi

# Check server connectivity
echo -e "${YELLOW}Checking server connectivity...${NC}"
if ! ssh -o ConnectTimeout=5 "$SERVER" "echo 'Server accessible'" >/dev/null 2>&1; then
    echo -e "${RED}Error: Cannot connect to $SERVER${NC}"
    echo "Please check:"
    echo "  - Server is running"
    echo "  - SSH key is configured"
    echo "  - Network connectivity"
    exit 1
fi

# Create remote directory if it doesn't exist
echo -e "${YELLOW}Preparing remote directory...${NC}"
ssh "$SERVER" "mkdir -p $REMOTE_DIR"

# Stop service if running
echo -e "${YELLOW}Stopping service...${NC}"
ssh "$SERVER" "systemctl stop $SERVICE_NAME" || echo "Service was not running"

# Copy binary
echo -e "${YELLOW}Copying binary...${NC}"
scp "target/release/$BINARY_NAME" "$SERVER:$REMOTE_DIR/"

# Set permissions
echo -e "${YELLOW}Setting permissions...${NC}"
ssh "$SERVER" "chmod +x $REMOTE_DIR/$BINARY_NAME"

# Copy production environment file if it exists
if [ -f ".env.production" ]; then
    echo -e "${YELLOW}Copying production environment...${NC}"
    scp ".env.production" "$SERVER:$REMOTE_DIR/.env"
elif [ -f ".env" ]; then
    echo -e "${YELLOW}Copying development environment (update for production!)...${NC}"
    scp ".env" "$SERVER:$REMOTE_DIR/.env"
else
    echo -e "${YELLOW}No environment file found, copying example...${NC}"
    scp ".env.example" "$SERVER:$REMOTE_DIR/.env"
    echo -e "${RED}WARNING: Update $REMOTE_DIR/.env with production settings!${NC}"
fi

# Install systemd service if it doesn't exist
echo -e "${YELLOW}Installing/updating systemd service...${NC}"
ssh "$SERVER" "cat > /etc/systemd/system/$SERVICE_NAME.service" << 'EOF'
[Unit]
Description=Radarr MVP - Movie Management System
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=/opt/radarr
ExecStart=/opt/radarr/unified-radarr
EnvironmentFile=/opt/radarr/.env
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=radarr

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and start service
echo -e "${YELLOW}Starting service...${NC}"
ssh "$SERVER" "systemctl daemon-reload"
ssh "$SERVER" "systemctl enable $SERVICE_NAME"
ssh "$SERVER" "systemctl start $SERVICE_NAME"

# Wait a moment for service to start
sleep 3

# Check service status
echo -e "${YELLOW}Checking service status...${NC}"
if ssh "$SERVER" "systemctl is-active $SERVICE_NAME" | grep -q "active"; then
    echo -e "${GREEN}✓ Service is running${NC}"
else
    echo -e "${RED}✗ Service failed to start${NC}"
    echo "Checking logs..."
    ssh "$SERVER" "journalctl -u $SERVICE_NAME --no-pager -n 20"
    exit 1
fi

# Health check
echo -e "${YELLOW}Performing health check...${NC}"
sleep 2
if ssh "$SERVER" "curl -s http://localhost:7878/health" >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Health check passed${NC}"
else
    echo -e "${YELLOW}! Health check failed (service might still be starting)${NC}"
    echo "Check manually: curl http://192.168.0.138:7878/health"
fi

echo ""
echo -e "${GREEN}=== Deployment Complete ===${NC}"
echo "Service: systemctl status $SERVICE_NAME"
echo "Logs: journalctl -u $SERVICE_NAME -f"
echo "URL: http://192.168.0.138:7878"
echo "Health: http://192.168.0.138:7878/health"

# Optional: Show recent logs
echo ""
echo -e "${YELLOW}Recent logs (last 10 lines):${NC}"
ssh "$SERVER" "journalctl -u $SERVICE_NAME --no-pager -n 10"