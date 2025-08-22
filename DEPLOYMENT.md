# Deployment Guide - Local-First Development

This guide covers deploying Radarr MVP to a local test server using SSH-based deployment.

## Target Environment

**Test Server**: `root@192.168.0.138`
- **OS**: Linux (Ubuntu/Debian preferred)
- **Architecture**: x86_64
- **Services**: PostgreSQL 16+, systemd
- **Network**: Local network access on port 7878

## Quick Deployment

### 1. Automated Deployment Script

```bash
# From the unified-radarr directory
./scripts/deploy.sh
```

This script handles:
- Building the release binary
- Copying files to the server
- Installing/updating the systemd service
- Restarting the service
- Checking deployment status

### 2. Manual Deployment Steps

If you prefer manual control or the script doesn't exist yet:

```bash
# Build release binary
cargo build --release

# Copy binary to server
scp target/release/unified-radarr root@192.168.0.138:/opt/radarr/

# Copy configuration (if changed)
scp .env.production root@192.168.0.138:/opt/radarr/.env

# SSH into server and restart
ssh root@192.168.0.138 'systemctl restart radarr'
```

## Server Setup (One-time)

### 1. Prepare Server Directory Structure

```bash
ssh root@192.168.0.138

# Create application directory
mkdir -p /opt/radarr
mkdir -p /var/log/radarr
mkdir -p /etc/radarr

# Create radarr user (optional, for security)
useradd --system --shell /bin/false --home /opt/radarr radarr
chown -R radarr:radarr /opt/radarr /var/log/radarr
```

### 2. Install PostgreSQL

```bash
# On Ubuntu/Debian
apt update
apt install postgresql postgresql-contrib

# Start and enable PostgreSQL
systemctl start postgresql
systemctl enable postgresql

# Create database and user
sudo -u postgres psql
```

```sql
CREATE DATABASE radarr_prod;
CREATE USER radarr_user WITH PASSWORD 'secure_password_here';
GRANT ALL PRIVILEGES ON DATABASE radarr_prod TO radarr_user;
\q
```

### 3. Install Systemd Service

Create `/etc/systemd/system/radarr.service`:

```ini
[Unit]
Description=Radarr MVP - Movie Management System
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=radarr
Group=radarr
WorkingDirectory=/opt/radarr
ExecStart=/opt/radarr/unified-radarr
EnvironmentFile=/opt/radarr/.env
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=radarr

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/opt/radarr /var/log/radarr

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
systemctl daemon-reload
systemctl enable radarr
systemctl start radarr
```

## Environment Configuration

### Production Environment File

Create `/opt/radarr/.env` on the server:

```bash
# Radarr Production Configuration
RADARR_HOST=0.0.0.0
RADARR_PORT=7878
RADARR_API_KEY=your_secure_api_key_here

# Database (adjust credentials)
DATABASE_URL=postgresql://radarr_user:secure_password_here@localhost:5432/radarr_prod
DATABASE_MAX_CONNECTIONS=10

# External Services
PROWLARR_BASE_URL=http://192.168.0.138:9696
PROWLARR_API_KEY=your_prowlarr_api_key

QBITTORRENT_BASE_URL=http://192.168.0.138:8080
QBITTORRENT_USERNAME=your_username
QBITTORRENT_PASSWORD=your_password

# Logging
RUST_LOG=info
LOG_JSON_FORMAT=true

# Environment
ENVIRONMENT=production
```

### Security Considerations

- Change default passwords and API keys
- Use strong, unique credentials
- Consider firewall rules for port 7878
- Regular security updates on the server

## Database Migration

### Initial Setup

```bash
# On your development machine
cd unified-radarr

# Install sqlx-cli if not already installed
cargo install sqlx-cli --features postgres

# Run migrations against production database
DATABASE_URL=postgresql://radarr_user:secure_password_here@192.168.0.138:5432/radarr_prod sqlx migrate run
```

### Migration Updates

```bash
# After code changes with new migrations
DATABASE_URL=postgresql://radarr_user:secure_password_here@192.168.0.138:5432/radarr_prod sqlx migrate run
```

## Deployment Verification

### 1. Service Status

```bash
ssh root@192.168.0.138 'systemctl status radarr'
```

Expected output:
```
● radarr.service - Radarr MVP - Movie Management System
   Loaded: loaded (/etc/systemd/system/radarr.service; enabled; vendor preset: enabled)
   Active: active (running) since ...
```

### 2. Health Check

```bash
curl http://192.168.0.138:7878/health
```

Expected response:
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "database": "connected"
}
```

### 3. Log Monitoring

```bash
ssh root@192.168.0.138 'journalctl -u radarr -f'
```

## Troubleshooting

### Common Issues

1. **Service fails to start**
   ```bash
   journalctl -u radarr --no-pager
   ```

2. **Database connection issues**
   - Verify PostgreSQL is running: `systemctl status postgresql`
   - Check database credentials in `.env`
   - Test connection: `psql -h localhost -U radarr_user -d radarr_prod`

3. **Binary not found**
   - Ensure binary was copied: `ls -la /opt/radarr/unified-radarr`
   - Check file permissions: `chmod +x /opt/radarr/unified-radarr`

4. **Permission denied**
   - Check file ownership: `chown radarr:radarr /opt/radarr/unified-radarr`

### Log Locations

- **Application logs**: `journalctl -u radarr`
- **PostgreSQL logs**: `journalctl -u postgresql`
- **System logs**: `/var/log/syslog`

## Backup and Recovery

### Database Backup

```bash
# Create backup
ssh root@192.168.0.138 'pg_dump -U radarr_user radarr_prod > /opt/radarr/backup-$(date +%Y%m%d).sql'

# Restore backup
ssh root@192.168.0.138 'psql -U radarr_user radarr_prod < /opt/radarr/backup-20250821.sql'
```

### Configuration Backup

```bash
# Backup configuration
scp root@192.168.0.138:/opt/radarr/.env ./backup/.env.prod

# Restore configuration
scp ./backup/.env.prod root@192.168.0.138:/opt/radarr/.env
ssh root@192.168.0.138 'systemctl restart radarr'
```

## Monitoring and Maintenance

### Service Monitoring

```bash
# Check if service is running
ssh root@192.168.0.138 'systemctl is-active radarr'

# View recent logs
ssh root@192.168.0.138 'journalctl -u radarr --since "1 hour ago"'

# Monitor resource usage
ssh root@192.168.0.138 'top -p $(pgrep unified-radarr)'
```

### Update Deployment

```bash
# Build new version
cargo build --release

# Deploy update
./scripts/deploy.sh

# Verify deployment
curl http://192.168.0.138:7878/health
```

This local-first deployment approach provides simplicity and direct control while maintaining production-readiness for a test environment.