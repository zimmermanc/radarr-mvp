#!/bin/bash

# Setup script for HDBits analysis cron job
# Installs the automation script and configures cron

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "HDBits Analysis Cron Setup"
echo "=========================="
echo

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then 
    echo "Please run as root or with sudo for system-wide installation"
    echo "Or continue for user-level installation (current user: $USER)"
    read -p "Continue with user-level installation? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
    INSTALL_DIR="$HOME/.local/opt/radarr"
    CRON_USER="$USER"
else
    INSTALL_DIR="/opt/radarr"
    CRON_USER="radarr"
fi

# Create installation directories
echo "Creating directories..."
mkdir -p "$INSTALL_DIR/scripts"
mkdir -p "$INSTALL_DIR/analysis/archive"
mkdir -p "/var/log/radarr" 2>/dev/null || mkdir -p "$HOME/.local/var/log/radarr"

# Copy analysis script
echo "Installing analysis script..."
cp "$SCRIPT_DIR/run_hdbits_analysis.sh" "$INSTALL_DIR/scripts/"
chmod +x "$INSTALL_DIR/scripts/run_hdbits_analysis.sh"

# Update paths in the script if needed
if [ "$INSTALL_DIR" != "/opt/radarr" ]; then
    sed -i "s|/opt/radarr|$INSTALL_DIR|g" "$INSTALL_DIR/scripts/run_hdbits_analysis.sh"
    sed -i "s|/var/log/radarr|$HOME/.local/var/log/radarr|g" "$INSTALL_DIR/scripts/run_hdbits_analysis.sh"
fi

# Setup environment file for session cookie
ENV_FILE="$INSTALL_DIR/scripts/hdbits_env.sh"
if [ ! -f "$ENV_FILE" ]; then
    echo "Creating environment configuration..."
    cat > "$ENV_FILE" << 'EOF'
#!/bin/bash
# HDBits Session Configuration
# Update this with your actual session cookie

# IMPORTANT: Get this from your browser's developer tools
# 1. Log into HDBits in your browser
# 2. Open Developer Tools (F12)
# 3. Go to Application/Storage -> Cookies
# 4. Copy all cookies as a single string (semicolon-separated)
export HDBITS_SESSION_COOKIE="PHPSESSID=your_session_id; uid=your_uid; pass=your_pass_hash"

# Optional: Configure notification webhook
# export NOTIFICATION_WEBHOOK="https://your.webhook.url"
EOF
    chmod 600 "$ENV_FILE"  # Restrict access to sensitive data
    echo
    echo "IMPORTANT: Edit $ENV_FILE with your HDBits session cookie"
    echo
fi

# Setup cron job
echo "Setting up cron job..."
CRON_CMD="source $INSTALL_DIR/scripts/hdbits_env.sh && $INSTALL_DIR/scripts/run_hdbits_analysis.sh"
CRON_SCHEDULE="0 2 * * 0"  # Weekly on Sunday at 2 AM

# Check if cron job already exists
if crontab -l 2>/dev/null | grep -q "run_hdbits_analysis.sh"; then
    echo "Cron job already exists. Updating..."
    (crontab -l 2>/dev/null | grep -v "run_hdbits_analysis.sh"; echo "$CRON_SCHEDULE $CRON_CMD") | crontab -
else
    echo "Adding new cron job..."
    (crontab -l 2>/dev/null; echo "$CRON_SCHEDULE $CRON_CMD") | crontab -
fi

# Create systemd timer as alternative (if systemd is available)
if command -v systemctl &> /dev/null && [ "$EUID" -eq 0 ]; then
    echo "Creating systemd timer (alternative to cron)..."
    
    cat > /etc/systemd/system/hdbits-analysis.service << EOF
[Unit]
Description=HDBits Scene Group Analysis
After=network.target

[Service]
Type=oneshot
User=$CRON_USER
EnvironmentFile=$INSTALL_DIR/scripts/hdbits_env.sh
ExecStart=$INSTALL_DIR/scripts/run_hdbits_analysis.sh
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    cat > /etc/systemd/system/hdbits-analysis.timer << EOF
[Unit]
Description=Weekly HDBits Scene Group Analysis
Requires=hdbits-analysis.service

[Timer]
OnCalendar=weekly
OnCalendar=Sun *-*-* 02:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

    systemctl daemon-reload
    systemctl enable hdbits-analysis.timer
    echo "Systemd timer created (not started - use 'systemctl start hdbits-analysis.timer' to enable)"
fi

# Test script (dry run)
echo
echo "Setup complete!"
echo
echo "Next steps:"
echo "1. Edit $ENV_FILE with your HDBits session cookie"
echo "2. Test the script manually:"
echo "   source $ENV_FILE"
echo "   $INSTALL_DIR/scripts/run_hdbits_analysis.sh"
echo "3. Check cron job with: crontab -l"
echo
echo "The analysis will run automatically every Sunday at 2:00 AM"
echo "Results will be stored in: $INSTALL_DIR/analysis/"
echo "Logs will be in: $([ "$EUID" -eq 0 ] && echo "/var/log/radarr" || echo "$HOME/.local/var/log/radarr")"
echo
echo "To run analysis immediately after configuration:"
echo "  source $ENV_FILE && $INSTALL_DIR/scripts/run_hdbits_analysis.sh"