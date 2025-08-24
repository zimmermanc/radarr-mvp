//! Application configuration module
//!
//! This module defines the complete configuration structure for the Radarr application,
//! including database, external services, and component-specific settings.

pub mod retry_config;

use radarr_core::{RadarrError, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Simplified Prowlarr configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProwlarrConfig {
    pub base_url: String,
    pub api_key: String,
    pub timeout: u64,
    pub max_requests_per_minute: u32,
    pub user_agent: String,
    pub verify_ssl: bool,
}

impl Default for ProwlarrConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:9696".to_string(),
            api_key: String::new(),
            timeout: 30,
            max_requests_per_minute: 60,
            user_agent: "Radarr-Rust/1.0".to_string(),
            verify_ssl: true,
        }
    }
}

/// Simplified qBittorrent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QBittorrentConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub timeout: u64,
}

impl Default for QBittorrentConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: String::new(),
            timeout: 30,
        }
    }
}

/// Simplified import configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub dry_run: bool,
    pub min_confidence: f32,
    pub skip_samples: bool,
    pub continue_on_error: bool,
    pub max_parallel: usize,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            min_confidence: 0.3,
            skip_samples: true,
            continue_on_error: true,
            max_parallel: 4,
        }
    }
}

/// RSS service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssServiceConfig {
    pub update_interval_minutes: u64,
    pub max_items_per_feed: usize,
    pub timeout_seconds: u64,
    pub enabled: bool,
}

/// TMDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbConfig {
    pub api_key: String,
    pub timeout: u64,
    pub enabled: bool,
}

impl Default for RssServiceConfig {
    fn default() -> Self {
        Self {
            update_interval_minutes: 15,
            max_items_per_feed: 100,
            timeout_seconds: 30,
            enabled: true,
        }
    }
}

impl Default for TmdbConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            timeout: 30,
            enabled: false,
        }
    }
}

/// Complete application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Database configuration
    pub database: DatabaseConfig,
    /// Prowlarr indexer configuration
    pub prowlarr: ProwlarrConfig,
    /// qBittorrent downloader configuration
    pub qbittorrent: QBittorrentConfig,
    /// Import pipeline configuration
    pub import: ImportConfig,
    /// TMDB API configuration
    pub tmdb: TmdbConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host address
    pub host: String,
    /// Server port
    pub port: u16,
    /// API key for authentication
    pub api_key: String,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Request timeout in seconds
    pub request_timeout: u64,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connect_timeout: u64,
    /// Enable query logging
    pub log_queries: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,
    /// Enable JSON formatted logs
    pub json_format: bool,
    /// Log to file
    pub log_file: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            prowlarr: ProwlarrConfig::default(),
            qbittorrent: QBittorrentConfig::default(),
            import: ImportConfig::default(),
            tmdb: TmdbConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 7878,
            api_key: "changeme123".to_string(),
            max_connections: 1000,
            request_timeout: 30,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://radarr:radarr@localhost:5432/radarr".to_string(),
            max_connections: 10,
            connect_timeout: 30,
            log_queries: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            log_file: None,
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Server configuration
        if let Ok(host) = env::var("RADARR_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = env::var("RADARR_PORT") {
            config.server.port = port.parse().map_err(|e| RadarrError::ValidationError {
                field: "RADARR_PORT".to_string(),
                message: format!("Invalid port number: {}", e),
            })?;
        }
        if let Ok(api_key) = env::var("RADARR_API_KEY") {
            config.server.api_key = api_key;
        }
        if let Ok(max_conn) = env::var("RADARR_MAX_CONNECTIONS") {
            config.server.max_connections =
                max_conn.parse().map_err(|e| RadarrError::ValidationError {
                    field: "RADARR_MAX_CONNECTIONS".to_string(),
                    message: format!("Invalid max connections: {}", e),
                })?;
        }
        if let Ok(timeout) = env::var("RADARR_REQUEST_TIMEOUT") {
            config.server.request_timeout =
                timeout.parse().map_err(|e| RadarrError::ValidationError {
                    field: "RADARR_REQUEST_TIMEOUT".to_string(),
                    message: format!("Invalid timeout: {}", e),
                })?;
        }

        // Database configuration
        if let Ok(db_url) = env::var("DATABASE_URL") {
            config.database.url = db_url;
        }
        if let Ok(max_conn) = env::var("DATABASE_MAX_CONNECTIONS") {
            config.database.max_connections =
                max_conn.parse().map_err(|e| RadarrError::ValidationError {
                    field: "DATABASE_MAX_CONNECTIONS".to_string(),
                    message: format!("Invalid max connections: {}", e),
                })?;
        }
        if let Ok(timeout) = env::var("DATABASE_CONNECT_TIMEOUT") {
            config.database.connect_timeout =
                timeout.parse().map_err(|e| RadarrError::ValidationError {
                    field: "DATABASE_CONNECT_TIMEOUT".to_string(),
                    message: format!("Invalid timeout: {}", e),
                })?;
        }
        if let Ok(log_queries) = env::var("DATABASE_LOG_QUERIES") {
            config.database.log_queries = log_queries.parse().unwrap_or(false);
        }

        // Prowlarr configuration
        if let Ok(base_url) = env::var("PROWLARR_BASE_URL") {
            config.prowlarr.base_url = base_url;
        }
        if let Ok(api_key) = env::var("PROWLARR_API_KEY") {
            config.prowlarr.api_key = api_key;
        }
        if let Ok(timeout) = env::var("PROWLARR_TIMEOUT") {
            config.prowlarr.timeout =
                timeout.parse().map_err(|e| RadarrError::ValidationError {
                    field: "PROWLARR_TIMEOUT".to_string(),
                    message: format!("Invalid timeout: {}", e),
                })?;
        }
        if let Ok(rate_limit) = env::var("PROWLARR_RATE_LIMIT") {
            config.prowlarr.max_requests_per_minute =
                rate_limit
                    .parse()
                    .map_err(|e| RadarrError::ValidationError {
                        field: "PROWLARR_RATE_LIMIT".to_string(),
                        message: format!("Invalid rate limit: {}", e),
                    })?;
        }

        // qBittorrent configuration
        if let Ok(base_url) = env::var("QBITTORRENT_BASE_URL") {
            config.qbittorrent.base_url = base_url;
        }
        if let Ok(username) = env::var("QBITTORRENT_USERNAME") {
            config.qbittorrent.username = username;
        }
        if let Ok(password) = env::var("QBITTORRENT_PASSWORD") {
            config.qbittorrent.password = password;
        }
        if let Ok(timeout) = env::var("QBITTORRENT_TIMEOUT") {
            config.qbittorrent.timeout =
                timeout.parse().map_err(|e| RadarrError::ValidationError {
                    field: "QBITTORRENT_TIMEOUT".to_string(),
                    message: format!("Invalid timeout: {}", e),
                })?;
        }

        // TMDB configuration
        if let Ok(api_key) = env::var("TMDB_API_KEY") {
            config.tmdb.api_key = api_key;
            config.tmdb.enabled = true;
        }
        if let Ok(timeout) = env::var("TMDB_TIMEOUT") {
            config.tmdb.timeout = timeout.parse().map_err(|e| RadarrError::ValidationError {
                field: "TMDB_TIMEOUT".to_string(),
                message: format!("Invalid timeout: {}", e),
            })?;
        }
        if let Ok(enabled) = env::var("TMDB_ENABLED") {
            config.tmdb.enabled = enabled.parse().unwrap_or(false);
        }

        // Logging configuration
        if let Ok(level) = env::var("RUST_LOG") {
            config.logging.level = level;
        }
        if let Ok(json_format) = env::var("LOG_JSON_FORMAT") {
            config.logging.json_format = json_format.parse().unwrap_or(false);
        }
        if let Ok(log_file) = env::var("LOG_FILE") {
            config.logging.log_file = Some(log_file);
        }

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(RadarrError::ValidationError {
                field: "server.port".to_string(),
                message: "Port must be greater than 0".to_string(),
            });
        }

        if self.server.api_key.is_empty() {
            return Err(RadarrError::ValidationError {
                field: "server.api_key".to_string(),
                message: "API key cannot be empty".to_string(),
            });
        }

        if self.server.api_key.len() < 8 {
            return Err(RadarrError::ValidationError {
                field: "server.api_key".to_string(),
                message: "API key must be at least 8 characters long".to_string(),
            });
        }

        // Prevent default API key in production
        if self.server.api_key == "changeme123" {
            tracing::warn!("WARNING: Using default API key 'changeme123' - this should be changed for production!");
            // In production builds, this should be an error:
            #[cfg(not(debug_assertions))]
            return Err(RadarrError::ValidationError {
                field: "server.api_key".to_string(),
                message: "Default API key 'changeme123' is not allowed in production builds"
                    .to_string(),
            });
        }

        if self.server.max_connections == 0 {
            return Err(RadarrError::ValidationError {
                field: "server.max_connections".to_string(),
                message: "Max connections must be greater than 0".to_string(),
            });
        }

        // Validate database config
        if self.database.url.is_empty() {
            return Err(RadarrError::ValidationError {
                field: "database.url".to_string(),
                message: "Database URL cannot be empty".to_string(),
            });
        }

        if self.database.max_connections == 0 {
            return Err(RadarrError::ValidationError {
                field: "database.max_connections".to_string(),
                message: "Database max connections must be greater than 0".to_string(),
            });
        }

        // Validate Prowlarr config
        if self.prowlarr.base_url.is_empty() {
            return Err(RadarrError::ValidationError {
                field: "prowlarr.base_url".to_string(),
                message: "Prowlarr base URL cannot be empty".to_string(),
            });
        }

        // Note: API key validation is optional as it might be set later

        // Validate qBittorrent config
        if self.qbittorrent.base_url.is_empty() {
            return Err(RadarrError::ValidationError {
                field: "qbittorrent.base_url".to_string(),
                message: "qBittorrent base URL cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}
