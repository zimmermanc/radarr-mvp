//! qBittorrent client implementation for download management
//!
//! This module provides a client for interacting with qBittorrent's Web API.
//! It handles authentication, torrent management, and progress monitoring.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use radarr_core::{
    circuit_breaker::{CircuitBreaker, CircuitBreakerConfig},
    RadarrError, Result,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use url::Url;

/// Configuration for qBittorrent client
#[derive(Debug, Clone)]
pub struct QBittorrentConfig {
    /// Base URL of the qBittorrent Web UI (e.g., "http://localhost:8080")
    pub base_url: String,
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
    /// Request timeout in seconds
    pub timeout: u64,
}

impl Default for QBittorrentConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: "".to_string(),
            timeout: 30,
        }
    }
}

/// Session state for tracking login status
#[derive(Debug, Default)]
struct SessionState {
    authenticated: bool,
    last_auth_time: Option<std::time::Instant>,
}

/// qBittorrent client for managing downloads
#[derive(Debug)]
pub struct QBittorrentClient {
    config: QBittorrentConfig,
    client: Client,
    base_url: Url,
    session_state: Arc<RwLock<SessionState>>,
    circuit_breaker: CircuitBreaker,
}

/// Torrent information from qBittorrent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentInfo {
    /// Torrent hash
    pub hash: String,
    /// Torrent name
    pub name: String,
    /// Current state (downloading, seeding, paused, etc.)
    pub state: String,
    /// Download progress (0.0 to 1.0)
    pub progress: f64,
    /// Download speed in bytes/second
    pub dlspeed: u64,
    /// Upload speed in bytes/second
    pub upspeed: u64,
    /// Total size in bytes
    pub size: u64,
    /// Completed size in bytes
    pub completed: u64,
    /// ETA in seconds (-1 if unknown)
    pub eta: i64,
    /// Priority (higher number = higher priority)
    pub priority: i32,
    /// Category
    pub category: String,
    /// Save path
    pub save_path: String,
}

/// Parameters for adding a torrent
#[derive(Debug, Clone)]
pub struct AddTorrentParams {
    /// Torrent data (either URL or base64-encoded torrent file)
    pub torrent_data: TorrentData,
    /// Category to assign
    pub category: Option<String>,
    /// Save path (overrides default)
    pub save_path: Option<String>,
    /// Start torrent paused
    pub paused: bool,
    /// Skip hash checking
    pub skip_checking: bool,
    /// Priority (-1 = decrease, 0 = normal, 1 = increase)
    pub priority: i32,
}

/// Torrent data for adding
#[derive(Debug, Clone)]
pub enum TorrentData {
    /// Magnet URL
    Url(String),
    /// Raw torrent file content (base64 encoded)
    File(Vec<u8>),
}

impl Default for AddTorrentParams {
    fn default() -> Self {
        Self {
            torrent_data: TorrentData::Url(String::new()),
            category: None,
            save_path: None,
            paused: false,
            skip_checking: false,
            priority: 0,
        }
    }
}

/// Application preferences from qBittorrent
#[derive(Debug, Deserialize)]
pub struct AppPreferences {
    /// Download directory
    pub save_path: Option<String>,
    /// Incomplete directory
    pub temp_path: Option<String>,
    /// Maximum number of connections
    pub max_connec: Option<i32>,
    /// Maximum number of uploads
    pub max_uploads: Option<i32>,
    /// Global download speed limit (bytes/sec, 0 = unlimited)
    pub dl_limit: Option<u64>,
    /// Global upload speed limit (bytes/sec, 0 = unlimited)
    pub up_limit: Option<u64>,
}

impl QBittorrentClient {
    /// Create a new qBittorrent client
    pub fn new(config: QBittorrentConfig) -> Result<Self> {
        let base_url =
            Url::parse(&config.base_url).map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Invalid base URL: {}", e),
            })?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .cookie_store(true)
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;

        // Configure circuit breaker for qBittorrent
        let circuit_breaker_config = CircuitBreakerConfig::new("qBittorrent")
            .with_failure_threshold(3)
            .with_timeout(Duration::from_secs(30))
            .with_request_timeout(Duration::from_secs(config.timeout))
            .with_success_threshold(1);

        Ok(Self {
            config,
            client,
            base_url,
            session_state: Arc::new(RwLock::new(SessionState::default())),
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        })
    }

    /// Create a new qBittorrent client with custom circuit breaker config
    pub fn new_with_circuit_breaker(
        config: QBittorrentConfig,
        circuit_breaker_config: CircuitBreakerConfig,
    ) -> Result<Self> {
        let base_url =
            Url::parse(&config.base_url).map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Invalid base URL: {}", e),
            })?;

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .cookie_store(true)
            .build()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self {
            config,
            client,
            base_url,
            session_state: Arc::new(RwLock::new(SessionState::default())),
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        })
    }

    /// Check if we need to re-authenticate (session older than 30 minutes)
    async fn needs_authentication(&self) -> bool {
        let state = self.session_state.read().await;
        if !state.authenticated {
            return true;
        }

        if let Some(last_auth) = state.last_auth_time {
            last_auth.elapsed() > Duration::from_secs(30 * 60) // 30 minutes
        } else {
            true
        }
    }

    /// Ensure we have a valid authenticated session
    async fn ensure_authenticated(&self) -> Result<()> {
        if self.needs_authentication().await {
            self.login().await?;
        }
        Ok(())
    }

    /// Login to qBittorrent and establish session
    pub async fn login(&self) -> Result<()> {
        let login_url = self.base_url.join("api/v2/auth/login").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct login URL: {}", e),
            }
        })?;

        let mut form = HashMap::new();
        form.insert("username", &self.config.username);
        form.insert("password", &self.config.password);

        debug!("Attempting login to qBittorrent at {}", login_url);

        let response = self
            .client
            .post(login_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Login request failed: {}", e),
            })?;

        if response.status().is_success() {
            let response_text =
                response
                    .text()
                    .await
                    .map_err(|e| RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Failed to read login response: {}", e),
                    })?;

            if response_text.contains("Fails") || response_text.contains("fail") {
                return Err(RadarrError::ExternalServiceError {
                    service: "qBittorrent".to_string(),
                    error: "Authentication failed - invalid credentials".to_string(),
                });
            }

            // Update session state
            {
                let mut state = self.session_state.write().await;
                state.authenticated = true;
                state.last_auth_time = Some(std::time::Instant::now());
            }

            info!("Successfully logged in to qBittorrent");
            Ok(())
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Login failed with status: {}", response.status()),
            })
        }
    }

    /// Reset authentication state for retry
    async fn reset_auth_state(&self) {
        let mut state = self.session_state.write().await;
        state.authenticated = false;
        state.last_auth_time = None;
    }

    /// Check if error indicates authentication failure
    fn is_auth_error(&self, error: &RadarrError) -> bool {
        let error_str = error.to_string().to_lowercase();
        error_str.contains("forbidden")
            || error_str.contains("unauthorized")
            || error_str.contains("403")
            || error_str.contains("login")
    }

    /// Extract torrent hash from magnet URL
    fn extract_hash_from_magnet(&self, magnet_url: &str) -> Option<String> {
        // Look for xt=urn:btih: in the magnet URL
        if let Some(start) = magnet_url.find("xt=urn:btih:") {
            let hash_start = start + "xt=urn:btih:".len();
            let hash_part = &magnet_url[hash_start..];

            // Find the end of the hash (next & or end of string)
            if let Some(end) = hash_part.find('&') {
                Some(hash_part[..end].to_uppercase())
            } else {
                Some(hash_part.to_uppercase())
            }
        } else {
            None
        }
    }

    /// Add a torrent to qBittorrent with retry logic
    pub async fn add_torrent(&self, params: AddTorrentParams) -> Result<String> {
        // Ensure we're authenticated before attempting
        self.ensure_authenticated().await?;

        // Try the operation, with one retry on auth failure
        match self.add_torrent_internal(&params).await {
            Ok(result) => Ok(result),
            Err(e) if self.is_auth_error(&e) => {
                warn!("Authentication error detected in add_torrent, retrying with fresh login");
                self.reset_auth_state().await;
                self.ensure_authenticated().await?;
                self.add_torrent_internal(&params).await
            }
            Err(e) => Err(e),
        }
    }

    /// Internal implementation of add_torrent
    async fn add_torrent_internal(&self, params: &AddTorrentParams) -> Result<String> {
        let add_url = self.base_url.join("api/v2/torrents/add").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct add torrent URL: {}", e),
            }
        })?;

        let mut form = reqwest::multipart::Form::new();

        // Add torrent data
        match &params.torrent_data {
            TorrentData::Url(url) => {
                form = form.text("urls", url.clone());
            }
            TorrentData::File(data) => {
                form = form.part(
                    "torrents",
                    reqwest::multipart::Part::bytes(data.clone())
                        .file_name("torrent.torrent")
                        .mime_str("application/x-bittorrent")
                        .map_err(|e| RadarrError::ExternalServiceError {
                            service: "qBittorrent".to_string(),
                            error: format!("Failed to set MIME type: {}", e),
                        })?,
                );
            }
        }

        // Add optional parameters
        if let Some(category) = &params.category {
            form = form.text("category", category.clone());
        }
        if let Some(save_path) = &params.save_path {
            form = form.text("savepath", save_path.clone());
        }
        if params.paused {
            form = form.text("paused", "true");
        }
        if params.skip_checking {
            form = form.text("skip_checking", "true");
        }

        debug!("Adding torrent to qBittorrent");

        let response = self
            .client
            .post(add_url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Add torrent request failed: {}", e),
            })?;

        if response.status().is_success() {
            let response_text =
                response
                    .text()
                    .await
                    .map_err(|e| RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Failed to read add torrent response: {}", e),
                    })?;

            if response_text.to_lowercase().contains("ok") || response_text.is_empty() {
                info!("Successfully added torrent to qBittorrent");

                // qBittorrent doesn't return the hash directly, so we need to extract it
                // For magnet links, we can extract the hash from the magnet URL
                match &params.torrent_data {
                    TorrentData::Url(url) if url.starts_with("magnet:") => {
                        if let Some(hash) = self.extract_hash_from_magnet(url) {
                            Ok(hash)
                        } else {
                            Ok(format!("magnet_{:x}", md5::compute(url.as_bytes())))
                        }
                    }
                    TorrentData::File(data) => {
                        // For torrent files, we'll use a hash of the file content
                        Ok(format!("file_{:x}", md5::compute(data)))
                    }
                    _ => Ok(format!(
                        "unknown_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    )),
                }
            } else {
                Err(RadarrError::ExternalServiceError {
                    service: "qBittorrent".to_string(),
                    error: format!("Failed to add torrent: {}", response_text),
                })
            }
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Add torrent failed with status: {}", response.status()),
            })
        }
    }

    /// Get information about all torrents with retry logic
    pub async fn get_torrents(&self) -> Result<Vec<TorrentInfo>> {
        // Ensure we're authenticated before attempting
        self.ensure_authenticated().await?;

        // Try the operation, with one retry on auth failure
        match self.get_torrents_internal().await {
            Ok(result) => Ok(result),
            Err(e) if self.is_auth_error(&e) => {
                warn!("Authentication error detected in get_torrents, retrying with fresh login");
                self.reset_auth_state().await;
                self.ensure_authenticated().await?;
                self.get_torrents_internal().await
            }
            Err(e) => Err(e),
        }
    }

    /// Internal implementation of get_torrents
    async fn get_torrents_internal(&self) -> Result<Vec<TorrentInfo>> {
        let torrents_url = self.base_url.join("api/v2/torrents/info").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct torrents URL: {}", e),
            }
        })?;

        debug!("Fetching torrent list from qBittorrent");

        let response = self.client.get(torrents_url).send().await.map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Get torrents request failed: {}", e),
            }
        })?;

        if response.status().is_success() {
            let torrents: Vec<TorrentInfo> =
                response
                    .json()
                    .await
                    .map_err(|e| RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Failed to parse torrents response: {}", e),
                    })?;

            debug!("Retrieved {} torrents from qBittorrent", torrents.len());
            Ok(torrents)
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Get torrents failed with status: {}", response.status()),
            })
        }
    }

    /// Get information about a specific torrent by hash
    pub async fn get_torrent_status(&self, hash: &str) -> Result<Option<TorrentInfo>> {
        let torrents = self.get_torrents().await?;
        Ok(torrents.into_iter().find(|t| t.hash == hash))
    }

    /// Delete a torrent from qBittorrent
    pub async fn delete_torrent(&self, hash: &str, delete_files: bool) -> Result<()> {
        let delete_url = self.base_url.join("api/v2/torrents/delete").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct delete URL: {}", e),
            }
        })?;

        let mut form = HashMap::new();
        form.insert("hashes", hash);
        form.insert("deleteFiles", if delete_files { "true" } else { "false" });

        debug!("Deleting torrent {} from qBittorrent", hash);

        let response = self
            .client
            .post(delete_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Delete torrent request failed: {}", e),
            })?;

        if response.status().is_success() {
            info!("Successfully deleted torrent {} from qBittorrent", hash);
            Ok(())
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Delete torrent failed with status: {}", response.status()),
            })
        }
    }

    /// Pause a torrent
    pub async fn pause_torrent(&self, hash: &str) -> Result<()> {
        let pause_url = self.base_url.join("api/v2/torrents/pause").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct pause URL: {}", e),
            }
        })?;

        let mut form = HashMap::new();
        form.insert("hashes", hash);

        let response = self
            .client
            .post(pause_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Pause torrent request failed: {}", e),
            })?;

        if response.status().is_success() {
            info!("Successfully paused torrent {}", hash);
            Ok(())
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Pause torrent failed with status: {}", response.status()),
            })
        }
    }

    /// Resume a torrent
    pub async fn resume_torrent(&self, hash: &str) -> Result<()> {
        let resume_url = self.base_url.join("api/v2/torrents/resume").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct resume URL: {}", e),
            }
        })?;

        let mut form = HashMap::new();
        form.insert("hashes", hash);

        let response = self
            .client
            .post(resume_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Resume torrent request failed: {}", e),
            })?;

        if response.status().is_success() {
            info!("Successfully resumed torrent {}", hash);
            Ok(())
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Resume torrent failed with status: {}", response.status()),
            })
        }
    }

    /// Get application preferences
    pub async fn get_preferences(&self) -> Result<AppPreferences> {
        let prefs_url = self.base_url.join("api/v2/app/preferences").map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Failed to construct preferences URL: {}", e),
            }
        })?;

        let response = self.client.get(prefs_url).send().await.map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Get preferences request failed: {}", e),
            }
        })?;

        if response.status().is_success() {
            let preferences: AppPreferences =
                response
                    .json()
                    .await
                    .map_err(|e| RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Failed to parse preferences response: {}", e),
                    })?;

            debug!("Retrieved qBittorrent preferences");
            Ok(preferences)
        } else {
            Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: format!("Get preferences failed with status: {}", response.status()),
            })
        }
    }

    /// Check if the client can connect to qBittorrent
    pub async fn test_connection(&self) -> Result<()> {
        debug!("Testing connection to qBittorrent");

        let base_url_clone = self.base_url.clone();
        let client_clone = self.client.clone();
        let username_clone = self.config.username.clone();
        let password_clone = self.config.password.clone();

        // Wrap the connection test in circuit breaker
        self.circuit_breaker
            .call(async move {
                // Try to login first
                let login_url = base_url_clone.join("api/v2/auth/login").map_err(|e| {
                    RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Failed to construct login URL: {}", e),
                    }
                })?;

                let mut form = HashMap::new();
                form.insert("username", &username_clone);
                form.insert("password", &password_clone);

                let response = client_clone
                    .post(login_url)
                    .form(&form)
                    .send()
                    .await
                    .map_err(|e| RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Login request failed: {}", e),
                    })?;

                if !response.status().is_success() {
                    return Err(RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Login failed with status: {}", response.status()),
                    });
                }

                let response_text =
                    response
                        .text()
                        .await
                        .map_err(|e| RadarrError::ExternalServiceError {
                            service: "qBittorrent".to_string(),
                            error: format!("Failed to read login response: {}", e),
                        })?;

                if response_text.contains("Fails") || response_text.contains("fail") {
                    return Err(RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: "Authentication failed - invalid credentials".to_string(),
                    });
                }

                // Test getting preferences
                let prefs_url = base_url_clone.join("api/v2/app/preferences").map_err(|e| {
                    RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Failed to construct preferences URL: {}", e),
                    }
                })?;

                let prefs_response = client_clone.get(prefs_url).send().await.map_err(|e| {
                    RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!("Get preferences request failed: {}", e),
                    }
                })?;

                if !prefs_response.status().is_success() {
                    return Err(RadarrError::ExternalServiceError {
                        service: "qBittorrent".to_string(),
                        error: format!(
                            "Get preferences failed with status: {}",
                            prefs_response.status()
                        ),
                    });
                }

                Ok(())
            })
            .await?;

        info!("qBittorrent connection test successful");
        Ok(())
    }

    /// Get circuit breaker metrics for monitoring
    pub async fn get_circuit_breaker_metrics(&self) -> radarr_core::CircuitBreakerMetrics {
        self.circuit_breaker.get_metrics().await
    }

    /// Check if qBittorrent service is healthy
    pub async fn is_healthy(&self) -> bool {
        self.circuit_breaker.is_healthy().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qbittorrent_config_default() {
        let config = QBittorrentConfig::default();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, "");
        assert_eq!(config.timeout, 30);
    }

    #[test]
    fn test_qbittorrent_client_creation() {
        let config = QBittorrentConfig::default();
        let client = QBittorrentClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_qbittorrent_client_invalid_url() {
        let config = QBittorrentConfig {
            base_url: "invalid-url".to_string(),
            ..Default::default()
        };
        let client = QBittorrentClient::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn test_add_torrent_params_default() {
        let params = AddTorrentParams::default();
        assert!(!params.paused);
        assert!(!params.skip_checking);
        assert_eq!(params.priority, 0);
        assert!(params.category.is_none());
        assert!(params.save_path.is_none());
    }

    #[test]
    fn test_torrent_data_variants() {
        let magnet = TorrentData::Url("magnet:?xt=urn:btih:example".to_string());
        let file_data = TorrentData::File(vec![1, 2, 3, 4]);

        match magnet {
            TorrentData::Url(url) => assert!(url.starts_with("magnet:")),
            _ => panic!("Expected URL variant"),
        }

        match file_data {
            TorrentData::File(data) => assert_eq!(data.len(), 4),
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_extract_hash_from_magnet() {
        let config = QBittorrentConfig::default();
        let client = QBittorrentClient::new(config).unwrap();

        // Test valid magnet URL
        let magnet = "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335aa7c1367a88a&dn=example";
        let hash = client.extract_hash_from_magnet(magnet);
        assert_eq!(
            hash,
            Some("C12FE1C06BBA254A9DC9F519B335AA7C1367A88A".to_string())
        );

        // Test magnet URL without additional parameters
        let magnet_simple = "magnet:?xt=urn:btih:abc123def456";
        let hash_simple = client.extract_hash_from_magnet(magnet_simple);
        assert_eq!(hash_simple, Some("ABC123DEF456".to_string()));

        // Test invalid magnet URL
        let invalid_magnet = "not-a-magnet-url";
        let no_hash = client.extract_hash_from_magnet(invalid_magnet);
        assert_eq!(no_hash, None);
    }

    // Integration tests would require a running qBittorrent instance
    // These are commented out but can be used for manual testing

    /*
    #[tokio::test]
    async fn test_qbittorrent_integration() {
        let config = QBittorrentConfig {
            base_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: "adminpass".to_string(),
            timeout: 10,
        };

        let client = QBittorrentClient::new(config).unwrap();

        // Test connection
        let result = client.test_connection().await;
        assert!(result.is_ok(), "Failed to connect: {:?}", result);

        // Test getting torrents
        let torrents = client.get_torrents().await;
        assert!(torrents.is_ok(), "Failed to get torrents: {:?}", torrents);
    }
    */
}
