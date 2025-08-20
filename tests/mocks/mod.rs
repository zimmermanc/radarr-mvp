//! Mock implementations of external services for testing

pub mod prowlarr;
pub mod qbittorrent;

pub use prowlarr::MockProwlarrClient;
pub use qbittorrent::MockQBittorrentClient;