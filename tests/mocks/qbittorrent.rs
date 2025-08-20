//! Mock qBittorrent client for testing

use radarr_core::{Result, RadarrError};
use radarr_downloaders::qbittorrent::{AddTorrentParams, TorrentInfo, AppPreferences};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;

/// Mock qBittorrent client that simulates download operations
#[derive(Debug, Clone)]
pub struct MockQBittorrentClient {
    /// Whether to simulate failures
    pub should_fail: Arc<Mutex<bool>>,
    /// Mock torrents by hash
    pub torrents: Arc<Mutex<HashMap<String, TorrentInfo>>>,
    /// Request counter for testing
    pub request_count: Arc<Mutex<u32>>,
    /// Simulate download progress
    pub progress_simulation: Arc<Mutex<bool>>,
}

impl Default for MockQBittorrentClient {
    fn default() -> Self {
        Self {
            should_fail: Arc::new(Mutex::new(false)),
            torrents: Arc::new(Mutex::new(HashMap::new())),
            request_count: Arc::new(Mutex::new(0)),
            progress_simulation: Arc::new(Mutex::new(true)),
        }
    }
}

impl MockQBittorrentClient {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set whether the client should fail requests
    pub fn set_should_fail(&self, should_fail: bool) {
        *self.should_fail.lock().unwrap() = should_fail;
    }
    
    /// Get the number of requests made
    pub fn get_request_count(&self) -> u32 {
        *self.request_count.lock().unwrap()
    }
    
    /// Reset request count
    pub fn reset_request_count(&self) {
        *self.request_count.lock().unwrap() = 0;
    }
    
    /// Enable or disable progress simulation
    pub fn set_progress_simulation(&self, enabled: bool) {
        *self.progress_simulation.lock().unwrap() = enabled;
    }
    
    /// Simulate network delay
    async fn simulate_delay(&self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
    }
    
    /// Increment request counter
    fn increment_requests(&self) {
        *self.request_count.lock().unwrap() += 1;
    }
    
    /// Login to mock qBittorrent
    pub async fn login(&self) -> Result<()> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail login".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Add a torrent to mock qBittorrent
    pub async fn add_torrent(&self, params: AddTorrentParams) -> Result<String> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail adding torrent".to_string(),
            });
        }
        
        // Generate a mock hash
        let hash = format!("mock_hash_{}", Uuid::new_v4().simple());
        
        // Extract torrent name from URL or use default
        let torrent_name = match &params.torrent_data {
            radarr_downloaders::qbittorrent::TorrentData::Url(url) => {
                if url.contains("Fight Club") {
                    "Fight Club 1999 1080p BluRay x264-SPARKS"
                } else {
                    "Mock Torrent"
                }
            },
            radarr_downloaders::qbittorrent::TorrentData::File(_) => "Mock Torrent from File",
        }.to_string();
        
        // Create mock torrent info
        let torrent_info = TorrentInfo {
            hash: hash.clone(),
            name: torrent_name,
            state: "downloading".to_string(),
            progress: 0.0,
            dlspeed: 1_048_576, // 1 MB/s
            upspeed: 0,
            size: 1_500_000_000, // 1.5 GB
            completed: 0,
            eta: 1440, // 24 minutes
            priority: params.priority,
            category: params.category.unwrap_or_default(),
            save_path: params.save_path.unwrap_or_else(|| "/downloads".to_string()),
        };
        
        // Store the torrent
        self.torrents.lock().unwrap().insert(hash.clone(), torrent_info);
        
        Ok(hash)
    }
    
    /// Get information about all torrents
    pub async fn get_torrents(&self) -> Result<Vec<TorrentInfo>> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail getting torrents".to_string(),
            });
        }
        
        let mut torrents: Vec<TorrentInfo> = self.torrents.lock().unwrap().values().cloned().collect();
        
        // Simulate progress if enabled
        if *self.progress_simulation.lock().unwrap() {
            let mut torrents_map = self.torrents.lock().unwrap();
            for torrent in &mut torrents {
                if torrent.state == "downloading" && torrent.progress < 100.0 {
                    // Increase progress by 10% each call
                    torrent.progress = (torrent.progress + 10.0).min(100.0);
                    torrent.completed = ((torrent.progress / 100.0) * torrent.size as f64) as u64;
                    
                    if torrent.progress >= 100.0 {
                        torrent.state = "completed".to_string();
                        torrent.dlspeed = 0;
                        torrent.eta = 0;
                    }
                    
                    // Update stored torrent
                    torrents_map.insert(torrent.hash.clone(), torrent.clone());
                }
            }
        }
        
        Ok(torrents)
    }
    
    /// Get information about a specific torrent by hash
    pub async fn get_torrent_status(&self, hash: &str) -> Result<Option<TorrentInfo>> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail getting torrent status".to_string(),
            });
        }
        
        let mut torrents = self.torrents.lock().unwrap();
        
        if let Some(mut torrent) = torrents.get(hash).cloned() {
            // Simulate progress if enabled
            if *self.progress_simulation.lock().unwrap() && torrent.state == "downloading" && torrent.progress < 100.0 {
                torrent.progress = (torrent.progress + 10.0).min(100.0);
                torrent.completed = ((torrent.progress / 100.0) * torrent.size as f64) as u64;
                
                if torrent.progress >= 100.0 {
                    torrent.state = "completed".to_string();
                    torrent.dlspeed = 0;
                    torrent.eta = 0;
                }
                
                // Update stored torrent
                torrents.insert(hash.to_string(), torrent.clone());
            }
            
            Ok(Some(torrent))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a torrent from mock qBittorrent
    pub async fn delete_torrent(&self, hash: &str, _delete_files: bool) -> Result<()> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail deleting torrent".to_string(),
            });
        }
        
        self.torrents.lock().unwrap().remove(hash);
        Ok(())
    }
    
    /// Pause a torrent
    pub async fn pause_torrent(&self, hash: &str) -> Result<()> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail pausing torrent".to_string(),
            });
        }
        
        if let Some(torrent) = self.torrents.lock().unwrap().get_mut(hash) {
            torrent.state = "pausedDL".to_string();
            torrent.dlspeed = 0;
        }
        
        Ok(())
    }
    
    /// Resume a torrent
    pub async fn resume_torrent(&self, hash: &str) -> Result<()> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail resuming torrent".to_string(),
            });
        }
        
        if let Some(torrent) = self.torrents.lock().unwrap().get_mut(hash) {
            if torrent.progress < 100.0 {
                torrent.state = "downloading".to_string();
                torrent.dlspeed = 1_048_576; // 1 MB/s
            } else {
                torrent.state = "completed".to_string();
            }
        }
        
        Ok(())
    }
    
    /// Get application preferences
    pub async fn get_preferences(&self) -> Result<AppPreferences> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail getting preferences".to_string(),
            });
        }
        
        Ok(AppPreferences {
            save_path: Some("/downloads".to_string()),
            temp_path: Some("/downloads/incomplete".to_string()),
            max_connec: Some(200),
            max_uploads: Some(4),
            dl_limit: Some(0), // Unlimited
            up_limit: Some(1_048_576), // 1 MB/s upload limit
        })
    }
    
    /// Test connection to mock qBittorrent
    pub async fn test_connection(&self) -> Result<()> {
        self.simulate_delay().await;
        self.increment_requests();
        
        if *self.should_fail.lock().unwrap() {
            return Err(RadarrError::ExternalServiceError {
                service: "qBittorrent".to_string(),
                error: "Mock qBittorrent client configured to fail connection test".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Force complete a torrent (for testing)
    pub fn force_complete_torrent(&self, hash: &str) {
        if let Some(torrent) = self.torrents.lock().unwrap().get_mut(hash) {
            torrent.progress = 100.0;
            torrent.completed = torrent.size;
            torrent.state = "completed".to_string();
            torrent.dlspeed = 0;
            torrent.eta = 0;
        }
    }
    
    /// Get all torrent hashes (for testing)
    pub fn get_torrent_hashes(&self) -> Vec<String> {
        self.torrents.lock().unwrap().keys().cloned().collect()
    }
    
    /// Clear all torrents (for testing)
    pub fn clear_torrents(&self) {
        self.torrents.lock().unwrap().clear();
    }
}

/// Helper functions for creating test scenarios
pub mod download_helpers {
    use super::*;
    
    pub fn create_downloading_torrent(name: &str) -> TorrentInfo {
        TorrentInfo {
            hash: format!("hash_{}", name.replace(' ', "_")),
            name: name.to_string(),
            state: "downloading".to_string(),
            progress: 25.0,
            dlspeed: 2_097_152, // 2 MB/s
            upspeed: 0,
            size: 1_500_000_000, // 1.5 GB
            completed: 375_000_000, // 25% of 1.5 GB
            eta: 1080, // 18 minutes
            priority: 0,
            category: "movies".to_string(),
            save_path: "/downloads/movies".to_string(),
        }
    }
    
    pub fn create_completed_torrent(name: &str) -> TorrentInfo {
        TorrentInfo {
            hash: format!("hash_{}", name.replace(' ', "_")),
            name: name.to_string(),
            state: "completed".to_string(),
            progress: 100.0,
            dlspeed: 0,
            upspeed: 524_288, // 512 KB/s upload
            size: 1_500_000_000, // 1.5 GB
            completed: 1_500_000_000, // 100%
            eta: 0,
            priority: 0,
            category: "movies".to_string(),
            save_path: "/downloads/movies".to_string(),
        }
    }
    
    pub fn create_paused_torrent(name: &str) -> TorrentInfo {
        TorrentInfo {
            hash: format!("hash_{}", name.replace(' ', "_")),
            name: name.to_string(),
            state: "pausedDL".to_string(),
            progress: 50.0,
            dlspeed: 0,
            upspeed: 0,
            size: 1_500_000_000, // 1.5 GB
            completed: 750_000_000, // 50%
            eta: -1, // Unknown
            priority: 0,
            category: "movies".to_string(),
            save_path: "/downloads/movies".to_string(),
        }
    }
}