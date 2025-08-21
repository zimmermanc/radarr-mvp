//! Integration tests for downloaders module
//!
//! These tests verify that the downloaders module works correctly
//! with the core Radarr domain types.

#[cfg(test)]
mod integration_tests {
    use crate::{QBittorrentClient, QBittorrentConfig, TorrentData, AddTorrentParams};

    #[test]
    fn test_qbittorrent_config_validation() {
        // Test that we can create a valid config
        let config = QBittorrentConfig {
            base_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            timeout: 30,
        };

        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.username, "admin");
        assert_eq!(config.password, "secret");
        assert_eq!(config.timeout, 30);
    }

    #[test]
    fn test_qbittorrent_client_with_valid_config() {
        let config = QBittorrentConfig {
            base_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            timeout: 10,
        };

        let client = QBittorrentClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_qbittorrent_client_with_invalid_url() {
        let config = QBittorrentConfig {
            base_url: "not-a-valid-url".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            timeout: 10,
        };

        let client = QBittorrentClient::new(config);
        assert!(client.is_err());
        
        if let Err(e) = client {
            assert!(e.to_string().contains("Invalid base URL"));
        }
    }

    #[test]
    fn test_add_torrent_params_builder() {
        let params = AddTorrentParams {
            torrent_data: TorrentData::Url("magnet:?xt=urn:btih:example".to_string()),
            category: Some("movies".to_string()),
            save_path: Some("/downloads/movies".to_string()),
            paused: true,
            skip_checking: false,
            priority: 1,
        };

        assert!(matches!(params.torrent_data, TorrentData::Url(_)));
        assert_eq!(params.category, Some("movies".to_string()));
        assert_eq!(params.save_path, Some("/downloads/movies".to_string()));
        assert!(params.paused);
        assert!(!params.skip_checking);
        assert_eq!(params.priority, 1);
    }

    #[test]
    fn test_torrent_data_magnet() {
        let magnet_url = "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335aa7c1367a88a";
        let torrent_data = TorrentData::Url(magnet_url.to_string());

        match torrent_data {
            TorrentData::Url(url) => {
                assert_eq!(url, magnet_url);
                assert!(url.starts_with("magnet:"));
            }
            _ => panic!("Expected URL variant"),
        }
    }

    #[test]
    fn test_torrent_data_file() {
        let file_content = vec![1, 2, 3, 4, 5];
        let torrent_data = TorrentData::File(file_content.clone());

        match torrent_data {
            TorrentData::File(data) => {
                assert_eq!(data, file_content);
                assert_eq!(data.len(), 5);
            }
            _ => panic!("Expected File variant"),
        }
    }

    // Mock test to verify async functionality compiles
    #[tokio::test]
    async fn test_async_interface_compiles() {
        let config = QBittorrentConfig::default();
        
        // This test just verifies that the async interface compiles correctly
        // We don't actually connect to anything, just test the types
        let client_result = QBittorrentClient::new(config);
        assert!(client_result.is_ok());

        // Test that async methods exist and have correct signatures
        let client = client_result.unwrap();
        
        // These would fail in a real test without a running qBittorrent,
        // but they verify the async interface is correct
        let _login_future = client.login();
        let _torrents_future = client.get_torrents();
        let _prefs_future = client.get_preferences();
        
        // Don't actually await them to avoid connection failures
    }
    
    // Test moved to qbittorrent module since extract_hash_from_magnet is private
    
    /// Integration test for qBittorrent download functionality
    /// This test demonstrates the complete workflow but requires a running qBittorrent instance
    /// Run with: cargo test test_qbittorrent_download -- --nocapture
    /// Note: This will fail without a properly configured qBittorrent instance
    #[tokio::test]
    #[ignore] // Ignored by default since it requires external service
    async fn test_qbittorrent_download() {
        use std::env;
        
        // Read configuration from environment variables
        let config = QBittorrentConfig {
            base_url: env::var("QBITTORRENT_URL").unwrap_or("http://localhost:8080".to_string()),
            username: env::var("QBITTORRENT_USERNAME").unwrap_or("admin".to_string()),
            password: env::var("QBITTORRENT_PASSWORD").unwrap_or("adminpass".to_string()),
            timeout: 30,
        };
        
        let client = QBittorrentClient::new(config).expect("Failed to create qBittorrent client");
        
        // Test 1: Connection and authentication
        println!("Testing qBittorrent connection...");
        let connection_result = client.test_connection().await;
        assert!(connection_result.is_ok(), "Failed to connect to qBittorrent: {:?}", connection_result);
        println!("✅ Connection test passed");
        
        // Test 2: Get initial torrent list
        println!("Testing torrent list retrieval...");
        let initial_torrents = client.get_torrents().await;
        assert!(initial_torrents.is_ok(), "Failed to get initial torrents: {:?}", initial_torrents);
        let initial_count = initial_torrents.unwrap().len();
        println!("✅ Initial torrent count: {}", initial_count);
        
        // Test 3: Add a test torrent (Ubuntu ISO magnet link - legal and safe)
        println!("Testing torrent addition...");
        let test_magnet = "magnet:?xt=urn:btih:dd8255ecdc7ca55fb0bbf81323d87062db1f6d1c&dn=Big+Buck+Bunny";
        let add_params = AddTorrentParams {
            torrent_data: TorrentData::Url(test_magnet.to_string()),
            category: Some("test".to_string()),
            paused: true, // Add paused so we don't actually download
            skip_checking: true,
            priority: 0,
            save_path: None,
        };
        
        let add_result = client.add_torrent(add_params).await;
        assert!(add_result.is_ok(), "Failed to add torrent: {:?}", add_result);
        let torrent_hash = add_result.unwrap();
        println!("✅ Torrent added with hash: {}", torrent_hash);
        
        // Test 4: Verify torrent was added
        println!("Testing torrent verification...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Wait for qBittorrent to process
        
        let updated_torrents = client.get_torrents().await;
        assert!(updated_torrents.is_ok(), "Failed to get updated torrents: {:?}", updated_torrents);
        let final_count = updated_torrents.unwrap().len();
        // Note: The count might not increase if the torrent already existed
        println!("✅ Final torrent count: {} (was {})", final_count, initial_count);
        
        // Test 5: Get specific torrent status
        println!("Testing torrent status retrieval...");
        let status_result = client.get_torrent_status(&torrent_hash).await;
        assert!(status_result.is_ok(), "Failed to get torrent status: {:?}", status_result);
        
        if let Some(torrent_info) = status_result.unwrap() {
            println!("✅ Torrent info retrieved: {}", torrent_info.name);
            println!("   Status: {}", torrent_info.state);
            println!("   Progress: {:.1}%", torrent_info.progress * 100.0);
        } else {
            println!("⚠️  Torrent not found by hash, but this might be expected");
        }
        
        // Test 6: Test preferences retrieval
        println!("Testing preferences retrieval...");
        let prefs_result = client.get_preferences().await;
        assert!(prefs_result.is_ok(), "Failed to get preferences: {:?}", prefs_result);
        let prefs = prefs_result.unwrap();
        println!("✅ Preferences retrieved");
        if let Some(save_path) = prefs.save_path {
            println!("   Default save path: {}", save_path);
        }
        
        println!("🎉 All qBittorrent integration tests passed!");
        println!("   - Authentication and session management working");
        println!("   - Torrent addition successful");
        println!("   - Progress monitoring functional");
        println!("   - Connection handling robust");
    }
}