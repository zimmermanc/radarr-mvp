//! Radarr downloaders module
//!
//! This crate provides integrations with various download clients
//! used for downloading torrents and managing the download queue.

pub mod qbittorrent;

#[cfg(test)]
mod tests;

// Re-export public types
pub use qbittorrent::{
    AddTorrentParams, AppPreferences, QBittorrentClient, QBittorrentConfig, TorrentData,
    TorrentInfo,
};
