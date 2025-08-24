# Radarr Downloaders

Download client integrations for the Radarr movie automation system. This crate provides unified interfaces for interacting with various BitTorrent and Usenet download clients, with primary focus on qBittorrent integration.

## Features

- **qBittorrent Integration**: Full-featured qBittorrent Web API client
- **Torrent Management**: Add, monitor, pause, resume, and remove torrents
- **Queue Monitoring**: Real-time download progress and status tracking
- **Category Management**: Automatic categorization and organization
- **Bandwidth Control**: Download/upload speed management
- **Authentication**: Session-based authentication with automatic renewal
- **Error Handling**: Comprehensive error handling with retry logic
- **Health Monitoring**: Download client health checking and status monitoring

## Key Dependencies

- **reqwest**: HTTP client for download client API communication
- **serde/serde_json**: JSON serialization for API requests and responses
- **url**: URL parsing and manipulation for API endpoints
- **thiserror**: Error handling and propagation
- **tracing**: Structured logging and debugging
- **md5**: Hash generation for torrent data and authentication

## qBittorrent Client

### Basic Setup

```rust
use radarr_downloaders::{QBittorrentClient, QBittorrentConfig};

// Create client with configuration
let config = QBittorrentConfig {
    base_url: "http://localhost:8080".to_string(),
    username: "admin".to_string(),
    password: "password".to_string(),
    timeout_seconds: 30,
    verify_ssl: true,
};

let client = QBittorrentClient::new(config);

// Login is handled automatically
let authenticated = client.login().await?;
```

### Adding Torrents

```rust
use radarr_downloaders::{AddTorrentParams, TorrentData};

// Add torrent from file data
let torrent_data = std::fs::read("/path/to/movie.torrent")?;
let params = AddTorrentParams {
    save_path: Some("/downloads/movies".to_string()),
    category: Some("movies".to_string()),
    tags: Some("radarr,movie".to_string()),
    paused: Some(false),
    priority: Some(1), // Normal priority
    skip_checking: Some(false),
    content_layout: Some("Subfolder".to_string()),
    auto_tmm: Some(false), // Disable automatic torrent management
};

let result = client.add_torrent_data(torrent_data, params).await?;
println!("Torrent added successfully");

// Add torrent from magnet link
let magnet_link = "magnet:?xt=urn:btih:...";
let result = client.add_torrent_url(magnet_link, params).await?;
```

### Monitoring Downloads

```rust
use radarr_downloaders::{TorrentInfo, TorrentState};

// Get all torrents
let torrents = client.get_torrents(None).await?;

for torrent in torrents {
    println!("Torrent: {}", torrent.name);
    println!("Progress: {:.1}%", torrent.progress * 100.0);
    println!("State: {:?}", torrent.state);
    println!("ETA: {:?}", torrent.eta);
    
    match torrent.state {
        TorrentState::Downloading => {
            println!("Speed: {} KB/s", torrent.dlspeed / 1024);
        }
        TorrentState::Completed => {
            println!("Download completed!");
        }
        TorrentState::Error => {
            println!("Error: {}", torrent.last_activity);
        }
        _ => {}
    }
}

// Get specific torrent by hash
let torrent_hash = "abc123...";
let torrent = client.get_torrent(torrent_hash).await?;
```

### Torrent Management

```rust
// Pause torrent
client.pause_torrent(&torrent_hash).await?;

// Resume torrent
client.resume_torrent(&torrent_hash).await?;

// Delete torrent (keep files)
client.delete_torrent(&torrent_hash, false).await?;

// Delete torrent and files
client.delete_torrent(&torrent_hash, true).await?;

// Set torrent priority (1-3, higher = more priority)
client.set_torrent_priority(&torrent_hash, 3).await?;

// Move torrent to different location
client.set_torrent_location(&torrent_hash, "/new/download/path").await?;
```

### Category and Tag Management

```rust
// Create category
client.create_category("movies", "/downloads/movies").await?;

// Set torrent category
client.set_torrent_category(&torrent_hash, "movies").await?;

// Add tags to torrent
client.add_torrent_tags(&torrent_hash, vec!["radarr", "2023"]).await?;

// Remove tags from torrent
client.remove_torrent_tags(&torrent_hash, vec!["old_tag"]).await?;
```

## Download Models

### Torrent Information

```rust
use radarr_downloaders::{TorrentInfo, TorrentState};
use chrono::{DateTime, Utc};

pub struct TorrentInfo {
    pub hash: String,                    // Torrent hash
    pub name: String,                    // Torrent name
    pub size: u64,                       // Total size in bytes
    pub progress: f64,                   // Progress (0.0-1.0)
    pub dlspeed: u64,                    // Download speed (bytes/sec)
    pub upspeed: u64,                    // Upload speed (bytes/sec)
    pub state: TorrentState,             // Current state
    pub eta: Option<u64>,                // ETA in seconds
    pub category: Option<String>,        // Category name
    pub tags: Vec<String>,               // Tags list
    pub save_path: String,               // Download path
    pub completed: u64,                  // Downloaded bytes
    pub downloaded: u64,                 // Total downloaded
    pub uploaded: u64,                   // Total uploaded
    pub ratio: f64,                      // Share ratio
    pub added_on: DateTime<Utc>,         // Date added
    pub completion_on: Option<DateTime<Utc>>, // Completion date
}

#[derive(Debug, Clone, PartialEq)]
pub enum TorrentState {
    Error,
    MissingFiles,
    Uploading,
    PausedUP,
    QueuedUP,
    StalledUP,
    CheckingUP,
    ForcedUP,
    Allocating,
    Downloading,
    MetaDL,
    PausedDL,
    QueuedDL,
    StalledDL,
    CheckingDL,
    ForcedDL,
    CheckingResumeData,
    Moving,
    Unknown,
}
```

### Add Torrent Parameters

```rust
use radarr_downloaders::AddTorrentParams;

pub struct AddTorrentParams {
    pub save_path: Option<String>,          // Download path
    pub cookie: Option<String>,             // HTTP cookie
    pub category: Option<String>,           // Category name
    pub tags: Option<String>,               // Comma-separated tags
    pub skip_checking: Option<bool>,        // Skip hash checking
    pub paused: Option<bool>,               // Add in paused state
    pub root_folder: Option<bool>,          // Create root folder
    pub rename: Option<String>,             // Rename torrent
    pub upload_limit: Option<i32>,          // Upload limit (KB/s)
    pub download_limit: Option<i32>,        // Download limit (KB/s)
    pub ratio_limit: Option<f64>,           // Ratio limit
    pub seeding_time_limit: Option<i32>,    // Seeding time limit (minutes)
    pub auto_tmm: Option<bool>,             // Automatic torrent management
    pub sequential_download: Option<bool>,   // Sequential download
    pub first_last_piece_prio: Option<bool>, // Prioritize first/last pieces
}
```

## Application Preferences

### Managing qBittorrent Settings

```rust
use radarr_downloaders::AppPreferences;

// Get current preferences
let prefs = client.get_preferences().await?;
println!("Download path: {:?}", prefs.save_path);
println!("Max connections: {:?}", prefs.max_connec);

// Update preferences
let mut new_prefs = AppPreferences::default();
new_prefs.save_path = Some("/downloads".to_string());
new_prefs.max_connec = Some(200);
new_prefs.max_uploads = Some(10);

client.set_preferences(new_prefs).await?;
```

## Error Handling

### Comprehensive Error Types

```rust
use radarr_downloaders::{DownloaderError, QBittorrentError};

pub enum DownloaderError {
    Authentication(String),         // Login/auth failures
    NotFound(String),              // Torrent not found
    InvalidRequest(String),        // Bad request parameters
    ServerError(String),           // qBittorrent server errors
    Network(reqwest::Error),       // Network/HTTP errors
    Serialization(String),         // JSON parsing errors
    Configuration(String),         // Invalid configuration
}

// Error handling in application code
match client.add_torrent_data(torrent_data, params).await {
    Ok(_) => println!("Torrent added successfully"),
    Err(DownloaderError::Authentication(_)) => {
        // Re-authenticate and retry
        client.login().await?;
        client.add_torrent_data(torrent_data, params).await?;
    }
    Err(DownloaderError::NotFound(msg)) => {
        println!("Torrent not found: {}", msg);
    }
    Err(e) => return Err(e),
}
```

## Integration Examples

### With Core Domain

```rust
use radarr_core::{Download, DownloadStatus, DownloadRepository};
use radarr_downloaders::{QBittorrentClient, TorrentState};

async fn sync_downloads(
    client: &QBittorrentClient,
    repo: &impl DownloadRepository,
) -> Result<()> {
    let torrents = client.get_torrents(None).await?;
    
    for torrent in torrents {
        let download_status = match torrent.state {
            TorrentState::Downloading => DownloadStatus::Downloading,
            TorrentState::Completed => DownloadStatus::Completed,
            TorrentState::Error => DownloadStatus::Failed,
            TorrentState::PausedDL => DownloadStatus::Paused,
            _ => DownloadStatus::Queued,
        };
        
        // Update download in database
        let download = Download {
            id: uuid::Uuid::parse_str(&torrent.hash)?,
            title: torrent.name,
            status: download_status,
            progress: torrent.progress,
            download_client: "qBittorrent".to_string(),
            download_id: torrent.hash,
            // ... other fields
        };
        
        repo.update_download(&download).await?;
    }
    
    Ok(())
}
```

### Download Service

```rust
use radarr_downloaders::QBittorrentClient;
use radarr_core::{Release, DownloadService};

pub struct DownloadServiceImpl {
    client: QBittorrentClient,
}

impl DownloadServiceImpl {
    pub async fn download_release(&self, release: &Release) -> Result<String> {
        let params = AddTorrentParams {
            save_path: Some("/downloads/movies".to_string()),
            category: Some("movies".to_string()),
            tags: Some(format!("radarr,{}", release.quality)),
            paused: Some(false),
            ..Default::default()
        };
        
        // Add torrent from URL or file
        match &release.download_url {
            url if url.starts_with("magnet:") => {
                self.client.add_torrent_url(url, params).await?;
            }
            url => {
                // Download torrent file and add
                let torrent_data = reqwest::get(url).await?.bytes().await?;
                self.client.add_torrent_data(torrent_data.to_vec(), params).await?;
            }
        }
        
        Ok(release.download_url.clone())
    }
}
```

## Testing

### Unit Tests

```bash
# Run downloader tests
cargo test -p radarr-downloaders

# Test specific modules
cargo test -p radarr-downloaders qbittorrent::tests
```

### Mock Client Testing

```rust
use radarr_downloaders::{QBittorrentClient, TorrentInfo, TorrentState};

#[tokio::test]
async fn test_torrent_monitoring() {
    let client = create_test_client();
    
    // Add test torrent
    let torrent_data = create_test_torrent_data();
    let params = AddTorrentParams::default();
    
    client.add_torrent_data(torrent_data, params).await.unwrap();
    
    // Wait for torrent to appear
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    let torrents = client.get_torrents(None).await.unwrap();
    assert!(!torrents.is_empty());
    
    let torrent = &torrents[0];
    assert!(!torrent.name.is_empty());
    assert!(matches!(torrent.state, TorrentState::Downloading | TorrentState::QueuedDL));
}
```

### Integration Testing

```rust
#[tokio::test]
#[ignore] // Run only with live qBittorrent instance
async fn test_qbittorrent_integration() {
    let config = QBittorrentConfig {
        base_url: "http://localhost:8080".to_string(),
        username: "admin".to_string(),
        password: "adminpass".to_string(),
        timeout_seconds: 30,
        verify_ssl: false,
    };
    
    let client = QBittorrentClient::new(config);
    
    // Test authentication
    let authenticated = client.login().await.unwrap();
    assert!(authenticated);
    
    // Test getting torrents
    let torrents = client.get_torrents(None).await.unwrap();
    println!("Found {} torrents", torrents.len());
    
    // Test getting preferences
    let prefs = client.get_preferences().await.unwrap();
    assert!(prefs.save_path.is_some());
}
```

## Configuration

### Environment Variables

```bash
# qBittorrent connection settings
QBITTORRENT_URL=http://localhost:8080
QBITTORRENT_USERNAME=admin
QBITTORRENT_PASSWORD=password
QBITTORRENT_TIMEOUT_SECONDS=30

# Download settings
DEFAULT_DOWNLOAD_PATH=/downloads/movies
DEFAULT_CATEGORY=movies
MAX_CONCURRENT_DOWNLOADS=5
```

### Configuration File

```rust
use radarr_downloaders::QBittorrentConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DownloaderConfig {
    pub qbittorrent: QBittorrentConfig,
    pub default_category: String,
    pub download_path: String,
    pub max_concurrent_downloads: u32,
}

// Load from file
let config: DownloaderConfig = serde_json::from_str(&config_json)?;
let client = QBittorrentClient::new(config.qbittorrent);
```

## Performance and Reliability

- **Connection Pooling**: Efficient HTTP connection reuse
- **Session Management**: Automatic login and session renewal
- **Request Batching**: Batch API calls where possible
- **Error Recovery**: Automatic retry with exponential backoff
- **Health Monitoring**: Regular health checks for download clients
- **Concurrent Operations**: Parallel torrent management operations
- **Rate Limiting**: Respect download client API limits