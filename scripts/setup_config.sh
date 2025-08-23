#!/bin/bash

# Setup script to create and configure services.env file

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$PROJECT_ROOT/config"
CONFIG_FILE="$CONFIG_DIR/services.env"
EXAMPLE_FILE="$CONFIG_DIR/services.env.example"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

print_color "$BLUE" "=========================================="
print_color "$BLUE" "Radarr MVP Configuration Setup"
print_color "$BLUE" "=========================================="
echo

# Check if config directory exists
if [ ! -d "$CONFIG_DIR" ]; then
    mkdir -p "$CONFIG_DIR"
    print_color "$GREEN" "✓ Created config directory"
fi

# Check if services.env already exists
if [ -f "$CONFIG_FILE" ]; then
    print_color "$YELLOW" "Configuration file already exists: $CONFIG_FILE"
    read -p "Do you want to update it? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_color "$YELLOW" "Setup cancelled. Your existing configuration was not modified."
        exit 0
    fi
    
    # Backup existing config
    BACKUP_FILE="$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
    cp "$CONFIG_FILE" "$BACKUP_FILE"
    print_color "$GREEN" "✓ Backed up existing config to: $BACKUP_FILE"
else
    # Copy example file
    if [ ! -f "$EXAMPLE_FILE" ]; then
        print_color "$RED" "✗ Example file not found: $EXAMPLE_FILE"
        exit 1
    fi
    
    cp "$EXAMPLE_FILE" "$CONFIG_FILE"
    print_color "$GREEN" "✓ Created configuration file from example"
fi

echo
print_color "$BLUE" "Let's configure your essential services:"
echo

# Function to update config value
update_config() {
    local key=$1
    local value=$2
    local file=$3
    
    # Escape special characters for sed
    value=$(echo "$value" | sed 's/[[\.*^$()+?{|]/\\&/g')
    
    # Update the value in the config file
    sed -i "s|^${key}=.*|${key}=${value}|" "$file"
}

# Function to prompt for value with default
prompt_value() {
    local prompt=$1
    local key=$2
    local default=$3
    local secret=$4
    
    current_value=$(grep "^${key}=" "$CONFIG_FILE" | cut -d'=' -f2-)
    
    if [ -n "$default" ]; then
        prompt_text="$prompt [$default]: "
    elif [ -n "$current_value" ] && [ "$current_value" != "your_"* ] && [ "$current_value" != "changeme" ]; then
        if [ "$secret" = "true" ]; then
            prompt_text="$prompt [configured]: "
        else
            prompt_text="$prompt [$current_value]: "
        fi
    else
        prompt_text="$prompt: "
    fi
    
    if [ "$secret" = "true" ]; then
        read -s -p "$prompt_text" value
        echo
    else
        read -p "$prompt_text" value
    fi
    
    if [ -z "$value" ]; then
        if [ -n "$default" ]; then
            value="$default"
        elif [ -n "$current_value" ]; then
            value="$current_value"
        fi
    fi
    
    if [ -n "$value" ]; then
        update_config "$key" "$value" "$CONFIG_FILE"
    fi
}

# Essential configurations
print_color "$YELLOW" "1. Database Configuration"
prompt_value "PostgreSQL URL" "DATABASE_URL" "postgresql://radarr:radarr@localhost:5432/radarr"

echo
print_color "$YELLOW" "2. TMDB Configuration (Required for movie metadata)"
print_color "$BLUE" "   Get your API key from: https://www.themoviedb.org/settings/api"
prompt_value "TMDB API Key" "TMDB_API_KEY" ""

echo
print_color "$YELLOW" "3. HDBits Configuration (For torrent indexing and analysis)"
prompt_value "HDBits Username" "HDBITS_USERNAME" ""
prompt_value "HDBits Passkey" "HDBITS_PASSKEY" "" "true"

echo
print_color "$YELLOW" "4. HDBits Session Cookie (For scene group analysis)"
print_color "$BLUE" "   Get this from your browser's DevTools after logging into HDBits"
print_color "$BLUE" "   Format: PHPSESSID=xxx; uid=xxx; pass=xxx"
prompt_value "HDBits Session Cookie" "HDBITS_SESSION_COOKIE" "" "true"

echo
print_color "$YELLOW" "5. Download Client (qBittorrent)"
prompt_value "qBittorrent Host" "QBITTORRENT_HOST" "http://localhost:8080"
prompt_value "qBittorrent Username" "QBITTORRENT_USERNAME" "admin"
prompt_value "qBittorrent Password" "QBITTORRENT_PASSWORD" "" "true"

echo
print_color "$YELLOW" "6. File Paths"
prompt_value "Movies Root Path" "MOVIES_ROOT_PATH" "/media/movies"
prompt_value "Downloads Path" "DOWNLOADS_PATH" "/downloads/complete"

echo
print_color "$YELLOW" "7. Optional: Discord Notifications"
print_color "$BLUE" "   Leave blank to skip"
prompt_value "Discord Webhook URL" "DISCORD_WEBHOOK_URL" ""

echo
print_color "$YELLOW" "8. Web UI Authentication"
prompt_value "Admin Username" "ADMIN_USERNAME" "admin"
prompt_value "Admin Password" "ADMIN_PASSWORD" "" "true"

# Generate secure keys if not set
echo
print_color "$YELLOW" "Generating secure keys..."

# API Key
current_api_key=$(grep "^API_KEY=" "$CONFIG_FILE" | cut -d'=' -f2-)
if [ -z "$current_api_key" ] || [ "$current_api_key" = "generate_secure_random_api_key_here" ]; then
    new_api_key=$(openssl rand -hex 32)
    update_config "API_KEY" "$new_api_key" "$CONFIG_FILE"
    print_color "$GREEN" "✓ Generated API key"
fi

# Session Secret
current_session_secret=$(grep "^SESSION_SECRET=" "$CONFIG_FILE" | cut -d'=' -f2-)
if [ -z "$current_session_secret" ] || [ "$current_session_secret" = "generate_random_session_secret_here" ]; then
    new_session_secret=$(openssl rand -hex 32)
    update_config "SESSION_SECRET" "$new_session_secret" "$CONFIG_FILE"
    print_color "$GREEN" "✓ Generated session secret"
fi

# Validate configuration
echo
print_color "$BLUE" "Validating configuration..."
source "$SCRIPT_DIR/load_config.sh"

if validate_config; then
    print_color "$GREEN" "✓ Configuration is valid"
else
    print_color "$YELLOW" "⚠ Some configuration issues were found (see above)"
fi

echo
print_config_summary

# Set proper permissions
chmod 600 "$CONFIG_FILE"
print_color "$GREEN" "✓ Set secure permissions on config file"

echo
print_color "$GREEN" "=========================================="
print_color "$GREEN" "Configuration Complete!"
print_color "$GREEN" "=========================================="
echo
print_color "$BLUE" "Your configuration is saved in:"
print_color "$BLUE" "  $CONFIG_FILE"
echo
print_color "$YELLOW" "Next steps:"
echo "1. Review and edit the config file for any additional settings"
echo "2. Test HDBits analysis: ./scripts/test_hdbits_analysis.sh"
echo "3. Run full analysis: ./scripts/run_hdbits_analysis_segmented.sh"
echo "4. Start the application: cargo run --release"
echo
print_color "$YELLOW" "Security reminder:"
echo "- Keep your config file secure (permissions already set to 600)"
echo "- Never commit services.env to version control"
echo "- Regularly rotate your API keys and passwords"
echo

# Add to .gitignore if not already there
if [ -f "$PROJECT_ROOT/.gitignore" ]; then
    if ! grep -q "config/services.env" "$PROJECT_ROOT/.gitignore"; then
        echo "config/services.env" >> "$PROJECT_ROOT/.gitignore"
        print_color "$GREEN" "✓ Added config/services.env to .gitignore"
    fi
fi

exit 0