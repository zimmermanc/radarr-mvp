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
}