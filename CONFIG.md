# Configuration Reference

Complete configuration guide for Radarr MVP including all environment variables, quality profiles, and external service integration.

## Table of Contents

1. [Environment Variables](#environment-variables)
2. [Quality Profiles](#quality-profiles)
3. [External Services](#external-services)
4. [Import Configuration](#import-configuration)
5. [Notification Settings](#notification-settings)
6. [Security Configuration](#security-configuration)
7. [Advanced Configuration](#advanced-configuration)

## Environment Variables

### Core Server Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `RADARR_HOST` | String | `0.0.0.0` | Server bind address |
| `RADARR_PORT` | Integer | `7878` | Server port |
| `RADARR_MAX_CONNECTIONS` | Integer | `1000` | Maximum concurrent connections |
| `RADARR_REQUEST_TIMEOUT` | Integer | `30` | Request timeout in seconds |
| `RADARR_WORKERS` | Integer | CPU cores | Number of worker threads |
| `RADARR_BASE_URL` | String | `/` | Base URL for reverse proxy |

**Example:**
```bash
RADARR_HOST=127.0.0.1
RADARR_PORT=7878
RADARR_MAX_CONNECTIONS=500
RADARR_REQUEST_TIMEOUT=60
```

### Database Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `DATABASE_URL` | String | **Required** | PostgreSQL connection string |
| `DATABASE_MAX_CONNECTIONS` | Integer | `10` | Connection pool size |
| `DATABASE_CONNECT_TIMEOUT` | Integer | `30` | Connection timeout in seconds |
| `DATABASE_IDLE_TIMEOUT` | Integer | `600` | Idle connection timeout |
| `DATABASE_LOG_QUERIES` | Boolean | `false` | Log SQL queries (development only) |
| `DATABASE_LOG_SLOW_QUERIES` | Integer | `1000` | Log queries slower than X ms |

**Connection String Format:**
```bash
# Basic format
DATABASE_URL=postgresql://username:password@host:port/database

# With SSL
DATABASE_URL=postgresql://username:password@host:port/database?sslmode=require

# With connection options
DATABASE_URL=postgresql://username:password@host:port/database?sslmode=prefer&connect_timeout=10&application_name=radarr-mvp
```

**Example:**
```bash
DATABASE_URL=postgresql://radarr:secure_password@localhost:5432/radarr_prod
DATABASE_MAX_CONNECTIONS=20
DATABASE_CONNECT_TIMEOUT=30
DATABASE_LOG_QUERIES=false
DATABASE_LOG_SLOW_QUERIES=500
```

### Prowlarr Integration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `PROWLARR_BASE_URL` | String | **Required** | Prowlarr instance URL |
| `PROWLARR_API_KEY` | String | **Required** | Prowlarr API key |
| `PROWLARR_TIMEOUT` | Integer | `30` | Request timeout in seconds |
| `PROWLARR_RATE_LIMIT` | Integer | `60` | Requests per minute |
| `PROWLARR_RETRY_COUNT` | Integer | `3` | Number of retry attempts |
| `PROWLARR_RETRY_DELAY` | Integer | `1000` | Retry delay in milliseconds |
| `PROWLARR_USER_AGENT` | String | `Radarr-MVP/1.0` | User agent for requests |

**Example:**
```bash
PROWLARR_BASE_URL=http://localhost:9696
PROWLARR_API_KEY=a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
PROWLARR_TIMEOUT=45
PROWLARR_RATE_LIMIT=30
PROWLARR_RETRY_COUNT=5
```

### qBittorrent Integration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `QBITTORRENT_BASE_URL` | String | **Required** | qBittorrent Web UI URL |
| `QBITTORRENT_USERNAME` | String | **Required** | qBittorrent username |
| `QBITTORRENT_PASSWORD` | String | **Required** | qBittorrent password |
| `QBITTORRENT_TIMEOUT` | Integer | `30` | Request timeout in seconds |
| `QBITTORRENT_CATEGORY` | String | `radarr` | Download category |
| `QBITTORRENT_DOWNLOAD_PATH` | String | `/downloads` | Download directory |
| `QBITTORRENT_SEQUENTIAL` | Boolean | `false` | Sequential download |
| `QBITTORRENT_FIRST_LAST_PIECE` | Boolean | `true` | Download first/last pieces first |

**Example:**
```bash
QBITTORRENT_BASE_URL=http://localhost:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=secure_password
QBITTORRENT_CATEGORY=movies
QBITTORRENT_DOWNLOAD_PATH=/data/downloads
QBITTORRENT_SEQUENTIAL=false
```

### TMDB Integration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `TMDB_API_KEY` | String | **Required** | TMDB API key |
| `TMDB_BASE_URL` | String | `https://api.themoviedb.org/3` | TMDB API base URL |
| `TMDB_IMAGE_BASE_URL` | String | `https://image.tmdb.org/t/p` | TMDB image base URL |
| `TMDB_TIMEOUT` | Integer | `30` | Request timeout |
| `TMDB_RATE_LIMIT` | Integer | `40` | Requests per 10 seconds |
| `TMDB_LANGUAGE` | String | `en-US` | Default language |
| `TMDB_REGION` | String | `US` | Default region |

**Example:**
```bash
TMDB_API_KEY=your_tmdb_api_key_here
TMDB_LANGUAGE=en-US
TMDB_REGION=US
TMDB_RATE_LIMIT=35
```

### Logging Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `RUST_LOG` | String | `info` | Log level (error, warn, info, debug, trace) |
| `LOG_JSON_FORMAT` | Boolean | `false` | Use JSON log format |
| `LOG_FILE` | String | None | Log file path (optional) |
| `LOG_ROTATION` | String | `daily` | Log rotation (daily, hourly, size) |
| `LOG_MAX_SIZE` | String | `100MB` | Maximum log file size |
| `LOG_MAX_FILES` | Integer | `10` | Maximum number of log files |

**Log Levels:**
- `error` - Only errors
- `warn` - Warnings and errors
- `info` - General information (recommended for production)
- `debug` - Detailed debugging (development)
- `trace` - Very verbose (development only)

**Example:**
```bash
# Development
RUST_LOG=debug
LOG_JSON_FORMAT=false

# Production
RUST_LOG=info
LOG_JSON_FORMAT=true
LOG_FILE=/var/log/radarr/radarr.log
LOG_ROTATION=daily
LOG_MAX_FILES=30
```

### Security Configuration

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `API_KEY` | String | Auto-generated | API authentication key |
| `API_KEY_HEADER` | String | `X-Api-Key` | API key header name |
| `CORS_ORIGINS` | String | `*` | Allowed CORS origins |
| `CORS_METHODS` | String | `GET,POST,PUT,DELETE` | Allowed HTTP methods |
| `RATE_LIMIT_REQUESTS` | Integer | `100` | Requests per minute per IP |
| `RATE_LIMIT_BURST` | Integer | `20` | Burst requests allowed |
| `SESSION_TIMEOUT` | Integer | `3600` | Session timeout in seconds |

**Example:**
```bash
API_KEY=your-secret-api-key-here
CORS_ORIGINS=http://localhost:3000,https://yourdomain.com
RATE_LIMIT_REQUESTS=200
RATE_LIMIT_BURST=50
```

## Quality Profiles

Quality profiles define which releases to download and upgrade preferences.

### Default Quality Profile

```json
{
  "name": "HD-1080p",
  "cutoff": "Bluray-1080p",
  "items": [
    {
      "quality": "HDTV-720p",
      "allowed": true,
      "preferred": false
    },
    {
      "quality": "HDTV-1080p",
      "allowed": true,
      "preferred": false
    },
    {
      "quality": "WEBRip-720p",
      "allowed": true,
      "preferred": false
    },
    {
      "quality": "WEBRip-1080p",
      "allowed": true,
      "preferred": true
    },
    {
      "quality": "Bluray-720p",
      "allowed": true,
      "preferred": false
    },
    {
      "quality": "Bluray-1080p",
      "allowed": true,
      "preferred": true
    }
  ],
  "min_format_score": 0,
  "upgrade_allowed": true,
  "language": "en"
}
```

### Quality Definitions

| Quality | Resolution | Source |
|---------|------------|--------|
| `SDTV` | 480p | Standard Definition TV |
| `DVD` | 480p | DVD Source |
| `HDTV-720p` | 720p | High Definition TV |
| `HDTV-1080p` | 1080p | Full HD TV |
| `WEBRip-720p` | 720p | Web Rip |
| `WEBRip-1080p` | 1080p | Web Rip |
| `Bluray-720p` | 720p | Blu-ray Source |
| `Bluray-1080p` | 1080p | Blu-ray Source |
| `WEBDL-720p` | 720p | Web Download |
| `WEBDL-1080p` | 1080p | Web Download |
| `Bluray-2160p` | 4K | 4K Blu-ray |
| `WEBDL-2160p` | 4K | 4K Web Download |

### Custom Formats

Custom formats allow fine-grained control over release selection:

```json
{
  "name": "x265",
  "specifications": [
    {
      "name": "x265",
      "implementation": "ReleaseTitleSpecification",
      "negate": false,
      "required": true,
      "fields": {
        "value": "(x265|h265|hevc)"
      }
    }
  ],
  "score": 10
}
```

## External Services

### Prowlarr Setup

1. **Install Prowlarr** following their documentation
2. **Configure Indexers** in Prowlarr web interface
3. **Generate API Key** in Settings → General
4. **Add to Radarr** environment configuration

**API Endpoints Used:**
- `GET /api/v1/indexer` - List indexers
- `GET /api/v1/search` - Search releases
- `GET /api/v1/indexer/{id}/schema` - Get indexer configuration

### qBittorrent Setup

1. **Enable Web UI** in qBittorrent preferences
2. **Set username/password** for authentication
3. **Configure download paths** for organization
4. **Create categories** for different content types

**Recommended qBittorrent Settings:**
```
# Connection
Port: 8080
Use UPnP/NAT-PMP: Yes

# Downloads
Default Save Path: /downloads
Keep incomplete torrents in: /downloads/incomplete
Copy .torrent files to: /downloads/torrents

# BitTorrent
Maximum active downloads: 5
Maximum active uploads: 5
Maximum active torrents: 10
```

### TMDB API Setup

1. **Create TMDB Account** at https://www.themoviedb.org/
2. **Request API Key** in account settings
3. **Add API Key** to environment configuration

**Rate Limits:**
- 40 requests per 10 seconds
- 1000 requests per day (free tier)

## Import Configuration

### File Organization

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `IMPORT_ENABLED` | Boolean | `true` | Enable automatic import |
| `IMPORT_INTERVAL` | Integer | `60` | Import check interval (seconds) |
| `IMPORT_DOWNLOAD_PATH` | String | `/downloads` | Download directory to monitor |
| `IMPORT_MOVIE_PATH` | String | `/movies` | Movie library directory |
| `IMPORT_USE_HARDLINKS` | Boolean | `true` | Use hardlinks instead of copy |
| `IMPORT_DELETE_AFTER` | Boolean | `false` | Delete source after import |
| `IMPORT_MIN_SIZE` | Integer | `100` | Minimum file size (MB) |
| `IMPORT_MAX_SIZE` | Integer | `50000` | Maximum file size (MB) |

### Naming Templates

**Movie Naming:**
```bash
# Default template
IMPORT_MOVIE_NAMING="{Movie Title} ({Release Year}) - {Quality}[{MediaInfo}]"

# Example output
"The Matrix (1999) - Bluray-1080p[x264]"

# Alternative templates
IMPORT_MOVIE_NAMING="{Movie Title} ({Release Year})"
IMPORT_MOVIE_NAMING="{Movie Title} ({Release Year}) [{Quality}]"
IMPORT_MOVIE_NAMING="{Movie Title} ({Release Year}) - {Resolution}"
```

**Folder Structure:**
```bash
# Default structure
IMPORT_FOLDER_FORMAT="{Movie Title} ({Release Year})"

# Example output
/movies/The Matrix (1999)/The Matrix (1999) - Bluray-1080p.mkv

# Alternative structures
IMPORT_FOLDER_FORMAT="{Movie Title}"
IMPORT_FOLDER_FORMAT="{Release Year}/{Movie Title}"
```

### File Processing

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `IMPORT_EXTRACT_ARCHIVES` | Boolean | `true` | Extract RAR/ZIP files |
| `IMPORT_DELETE_ARCHIVES` | Boolean | `true` | Delete archives after extraction |
| `IMPORT_SKIP_SAMPLES` | Boolean | `true` | Skip sample files |
| `IMPORT_ALLOWED_EXTENSIONS` | String | `mkv,mp4,avi,mov` | Allowed file extensions |
| `IMPORT_SUBTITLE_EXTENSIONS` | String | `srt,ass,ssa,sub` | Subtitle file extensions |
| `IMPORT_EXTRA_FILE_EXTENSIONS` | String | `nfo,jpg,png` | Extra file extensions |

**Example:**
```bash
IMPORT_DOWNLOAD_PATH=/data/downloads
IMPORT_MOVIE_PATH=/data/movies
IMPORT_USE_HARDLINKS=true
IMPORT_MOVIE_NAMING="{Movie Title} ({Release Year}) [{Quality}]"
IMPORT_FOLDER_FORMAT="{Movie Title} ({Release Year})"
IMPORT_ALLOWED_EXTENSIONS=mkv,mp4,avi,m4v
```

## Notification Settings

### Discord Notifications

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `DISCORD_ENABLED` | Boolean | `false` | Enable Discord notifications |
| `DISCORD_WEBHOOK_URL` | String | None | Discord webhook URL |
| `DISCORD_USERNAME` | String | `Radarr MVP` | Bot username |
| `DISCORD_AVATAR_URL` | String | None | Bot avatar URL |
| `DISCORD_NOTIFY_EVENTS` | String | `download,import,upgrade` | Events to notify |

**Example:**
```bash
DISCORD_ENABLED=true
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
DISCORD_USERNAME="Movie Bot"
DISCORD_NOTIFY_EVENTS=download,import,upgrade,error
```

### Webhook Notifications

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `WEBHOOK_ENABLED` | Boolean | `false` | Enable webhook notifications |
| `WEBHOOK_URL` | String | None | Webhook endpoint URL |
| `WEBHOOK_METHOD` | String | `POST` | HTTP method |
| `WEBHOOK_HEADERS` | String | None | Custom headers (JSON) |
| `WEBHOOK_TIMEOUT` | Integer | `30` | Request timeout |
| `WEBHOOK_RETRY_COUNT` | Integer | `3` | Retry attempts |

**Example:**
```bash
WEBHOOK_ENABLED=true
WEBHOOK_URL=https://your-api.com/webhooks/radarr
WEBHOOK_HEADERS='{"Authorization": "Bearer token", "Content-Type": "application/json"}'
WEBHOOK_TIMEOUT=10
```

## Security Configuration

### API Authentication

**API Key Generation:**
```bash
# Generate secure API key
openssl rand -hex 32

# Or use built-in generator
cargo run --bin generate-api-key
```

**API Key Usage:**
```bash
# Header-based authentication
curl -H "X-Api-Key: your-api-key" http://localhost:7878/api/v3/movie

# Query parameter (less secure)
curl "http://localhost:7878/api/v3/movie?apikey=your-api-key"
```

### HTTPS Configuration

**Environment Variables:**
```bash
# Enable TLS
RADARR_TLS_ENABLED=true
RADARR_TLS_CERT_PATH=/path/to/cert.pem
RADARR_TLS_KEY_PATH=/path/to/key.pem
RADARR_TLS_PORT=7879

# Redirect HTTP to HTTPS
RADARR_REDIRECT_HTTPS=true
```

**Generate Self-Signed Certificate (Development):**
```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

### Firewall Configuration

**Required Ports:**
- `7878` - Radarr web interface
- `5432` - PostgreSQL (if external)
- `9696` - Prowlarr (if external)
- `8080` - qBittorrent (if external)

**UFW Example:**
```bash
sudo ufw allow 7878/tcp comment "Radarr MVP"
sudo ufw allow from 192.168.1.0/24 to any port 5432 comment "PostgreSQL local"
```

## Advanced Configuration

### Performance Tuning

**High-Performance Settings:**
```bash
# Increase worker threads
RADARR_WORKERS=8

# Increase connection limits
RADARR_MAX_CONNECTIONS=2000
DATABASE_MAX_CONNECTIONS=50

# Optimize timeouts
RADARR_REQUEST_TIMEOUT=60
DATABASE_CONNECT_TIMEOUT=10
DATABASE_IDLE_TIMEOUT=300

# Enable query caching
DATABASE_ENABLE_CACHE=true
DATABASE_CACHE_SIZE=1000
```

### Circuit Breaker Configuration

**Prowlarr Circuit Breaker:**
```bash
PROWLARR_CIRCUIT_BREAKER_ENABLED=true
PROWLARR_CIRCUIT_BREAKER_FAILURE_THRESHOLD=5
PROWLARR_CIRCUIT_BREAKER_TIMEOUT=60
PROWLARR_CIRCUIT_BREAKER_RETRY_TIMEOUT=300
```

**qBittorrent Circuit Breaker:**
```bash
QBITTORRENT_CIRCUIT_BREAKER_ENABLED=true
QBITTORRENT_CIRCUIT_BREAKER_FAILURE_THRESHOLD=3
QBITTORRENT_CIRCUIT_BREAKER_TIMEOUT=30
```

### Monitoring Configuration

**Metrics Export:**
```bash
# Prometheus metrics
METRICS_ENABLED=true
METRICS_PORT=9090
METRICS_PATH=/metrics

# Health check endpoint
HEALTH_CHECK_ENABLED=true
HEALTH_CHECK_PATH=/health
HEALTH_CHECK_DETAILED_PATH=/health/detailed
```

**Application Metrics:**
- HTTP request duration
- Database query performance
- External service response times
- Memory and CPU usage
- Download/import statistics

### Feature Flags

**Experimental Features:**
```bash
# Enable beta features
FEATURE_AUTOMATIC_QUALITY_UPGRADE=true
FEATURE_ADVANCED_SEARCH_FILTERS=false
FEATURE_BATCH_OPERATIONS=true
FEATURE_CUSTOM_SCRIPTS=false
```

### Configuration Validation

**Startup Validation:**
```bash
# Validate configuration on startup
VALIDATE_CONFIG_ON_STARTUP=true

# Exit on validation errors
STRICT_CONFIG_VALIDATION=true

# Configuration test mode
CONFIG_TEST_MODE=false
```

**Manual Validation:**
```bash
# Test configuration
cargo run -- --test-config

# Validate specific components
cargo run -- --test-database
cargo run -- --test-prowlarr
cargo run -- --test-qbittorrent
```

## Configuration Examples

### Development Environment

```bash
# .env.development
RUST_LOG=debug
RADARR_PORT=7878
DATABASE_URL=postgresql://radarr:radarr@localhost:5432/radarr_dev
DATABASE_LOG_QUERIES=true
PROWLARR_BASE_URL=http://localhost:9696
PROWLARR_API_KEY=dev_api_key
QBITTORRENT_BASE_URL=http://localhost:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=adminpass
IMPORT_DOWNLOAD_PATH=/tmp/downloads
IMPORT_MOVIE_PATH=/tmp/movies
```

### Production Environment

```bash
# .env.production
RUST_LOG=info
LOG_JSON_FORMAT=true
LOG_FILE=/var/log/radarr/radarr.log
RADARR_HOST=0.0.0.0
RADARR_PORT=7878
RADARR_MAX_CONNECTIONS=1000
DATABASE_URL=postgresql://radarr:secure_password@postgres:5432/radarr_prod
DATABASE_MAX_CONNECTIONS=20
DATABASE_LOG_QUERIES=false
PROWLARR_BASE_URL=http://prowlarr:9696
PROWLARR_API_KEY=prod_api_key_here
QBITTORRENT_BASE_URL=http://qbittorrent:8080
QBITTORRENT_USERNAME=radarr_user
QBITTORRENT_PASSWORD=secure_password
IMPORT_DOWNLOAD_PATH=/data/downloads
IMPORT_MOVIE_PATH=/data/movies
DISCORD_ENABLED=true
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
API_KEY=your_secure_api_key_here
RATE_LIMIT_REQUESTS=100
```

### Docker Environment

```bash
# .env.docker
DATABASE_URL=postgresql://radarr:radarr@postgres:5432/radarr
PROWLARR_BASE_URL=http://prowlarr:9696
QBITTORRENT_BASE_URL=http://qbittorrent:8080
IMPORT_DOWNLOAD_PATH=/downloads
IMPORT_MOVIE_PATH=/movies
RUST_LOG=info
LOG_JSON_FORMAT=true
```

For more information, see the [Installation Guide](INSTALL.md) and [API Documentation](API.md).