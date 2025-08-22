//! qBittorrent download client adapter

use async_trait::async_trait;
use radarr_core::{Result, DownloadClientService, ClientDownloadStatus};
use radarr_downloaders::{QBittorrentClient, QBittorrentConfig, TorrentData, AddTorrentParams};

/// qBittorrent download client adapter
pub struct QBittorrentDownloadClient {
    client: QBittorrentClient,
}

impl QBittorrentDownloadClient {
    /// Create a new qBittorrent download client
    pub fn new(config: QBittorrentConfig) -> Result<Self> {
        let client = QBittorrentClient::new(config)?;
        Ok(Self { client })
    }
}

#[async_trait]
impl DownloadClientService for QBittorrentDownloadClient {
    async fn add_download(
        &self,
        download_url: &str,
        category: Option<String>,
        save_path: Option<String>,
    ) -> Result<String> {
        let torrent_data = if download_url.starts_with("magnet:") {
            TorrentData::Url(download_url.to_string())
        } else {
            // For now, assume it's a magnet URL. In the future, we could support
            // torrent file downloads by fetching the URL and converting to bytes
            TorrentData::Url(download_url.to_string())
        };
        
        let params = AddTorrentParams {
            torrent_data,
            category,
            save_path,
            paused: false,
            skip_checking: false,
            priority: 0,
        };
        
        self.client.add_torrent(params).await
    }
    
    async fn get_download_status(&self, client_id: &str) -> Result<Option<ClientDownloadStatus>> {
        match self.client.get_torrent_status(client_id).await? {
            Some(torrent_info) => {
                let status = ClientDownloadStatus {
                    client_id: torrent_info.hash,
                    name: torrent_info.name,
                    status: torrent_info.state,
                    progress: torrent_info.progress,
                    download_speed: Some(torrent_info.dlspeed),
                    upload_speed: Some(torrent_info.upspeed),
                    downloaded_bytes: Some(torrent_info.completed as i64),
                    upload_bytes: None, // qBittorrent doesn't provide total uploaded in TorrentInfo
                    eta_seconds: Some(torrent_info.eta),
                    seeders: None, // Not available in TorrentInfo
                    leechers: None, // Not available in TorrentInfo
                    save_path: Some(torrent_info.save_path),
                };
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }
    
    async fn remove_download(&self, client_id: &str, delete_files: bool) -> Result<()> {
        self.client.delete_torrent(client_id, delete_files).await
    }
    
    async fn pause_download(&self, client_id: &str) -> Result<()> {
        self.client.pause_torrent(client_id).await
    }
    
    async fn resume_download(&self, client_id: &str) -> Result<()> {
        self.client.resume_torrent(client_id).await
    }
    
    async fn get_all_downloads(&self) -> Result<Vec<ClientDownloadStatus>> {
        let torrents = self.client.get_torrents().await?;
        let mut downloads = Vec::new();
        
        for torrent_info in torrents {
            let status = ClientDownloadStatus {
                client_id: torrent_info.hash,
                name: torrent_info.name,
                status: torrent_info.state,
                progress: torrent_info.progress,
                download_speed: Some(torrent_info.dlspeed),
                upload_speed: Some(torrent_info.upspeed),
                downloaded_bytes: Some(torrent_info.completed as i64),
                upload_bytes: None,
                eta_seconds: Some(torrent_info.eta),
                seeders: None,
                leechers: None,
                save_path: Some(torrent_info.save_path),
            };
            downloads.push(status);
        }
        
        Ok(downloads)
    }
}