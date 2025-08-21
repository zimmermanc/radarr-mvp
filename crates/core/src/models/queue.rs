//! Queue domain model for download management

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a queue item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueStatus {
    /// Item is in queue waiting to be processed
    Queued,
    /// Item is currently being downloaded
    Downloading,
    /// Download completed successfully
    Completed,
    /// Download failed
    Failed,
    /// Download was cancelled by user
    Cancelled,
    /// Item is paused
    Paused,
    /// Item is stalled (no progress)
    Stalled,
    /// Item is seeding after completion
    Seeding,
}

impl Default for QueueStatus {
    fn default() -> Self {
        QueueStatus::Queued
    }
}

/// Priority level for queue items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueuePriority {
    Low,
    Normal,
    High,
    VeryHigh,
}

impl Default for QueuePriority {
    fn default() -> Self {
        QueuePriority::Normal
    }
}

/// A download queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub movie_id: Uuid,
    pub release_id: Uuid,
    
    // Download information
    pub title: String,
    pub download_url: String,
    pub magnet_url: Option<String>,
    pub size_bytes: Option<i64>,
    
    // Status tracking
    pub status: QueueStatus,
    pub priority: QueuePriority,
    pub progress: f64, // 0.0 to 1.0
    
    // Download client information
    pub download_client_id: Option<String>, // ID from download client (e.g., torrent hash)
    pub download_path: Option<String>,
    pub category: Option<String>,
    
    // Progress tracking
    pub downloaded_bytes: Option<i64>,
    pub upload_bytes: Option<i64>,
    pub download_speed: Option<u64>, // bytes per second
    pub upload_speed: Option<u64>,   // bytes per second
    pub eta_seconds: Option<i64>,    // estimated time remaining
    pub seeders: Option<i32>,
    pub leechers: Option<i32>,
    
    // Error information
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub max_retries: i32,
    
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl QueueItem {
    /// Create a new queue item
    pub fn new(
        movie_id: Uuid,
        release_id: Uuid,
        title: String,
        download_url: String,
    ) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id: Uuid::new_v4(),
            movie_id,
            release_id,
            title,
            download_url,
            magnet_url: None,
            size_bytes: None,
            status: QueueStatus::default(),
            priority: QueuePriority::default(),
            progress: 0.0,
            download_client_id: None,
            download_path: None,
            category: None,
            downloaded_bytes: None,
            upload_bytes: None,
            download_speed: None,
            upload_speed: None,
            eta_seconds: None,
            seeders: None,
            leechers: None,
            error_message: None,
            retry_count: 0,
            max_retries: 3,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }
    
    /// Update the queue item status
    pub fn update_status(&mut self, status: QueueStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
        
        // Set timestamps based on status
        match status {
            QueueStatus::Downloading => {
                if self.started_at.is_none() {
                    self.started_at = Some(self.updated_at);
                }
            }
            QueueStatus::Completed | QueueStatus::Seeding => {
                if self.completed_at.is_none() {
                    self.completed_at = Some(self.updated_at);
                }
                self.progress = 1.0;
            }
            QueueStatus::Failed => {
                self.retry_count += 1;
            }
            _ => {}
        }
    }
    
    /// Update download progress
    pub fn update_progress(
        &mut self,
        progress: f64,
        downloaded_bytes: Option<i64>,
        download_speed: Option<u64>,
        eta_seconds: Option<i64>,
    ) {
        self.progress = progress.clamp(0.0, 1.0);
        self.downloaded_bytes = downloaded_bytes;
        self.download_speed = download_speed;
        self.eta_seconds = eta_seconds;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Update seeding information
    pub fn update_seeding_info(
        &mut self,
        upload_bytes: Option<i64>,
        upload_speed: Option<u64>,
        seeders: Option<i32>,
        leechers: Option<i32>,
    ) {
        self.upload_bytes = upload_bytes;
        self.upload_speed = upload_speed;
        self.seeders = seeders;
        self.leechers = leechers;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Set error message and increment retry count
    pub fn set_error(&mut self, error_message: String) {
        self.error_message = Some(error_message);
        self.status = QueueStatus::Failed;
        self.retry_count += 1;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Check if the item can be retried
    pub fn can_retry(&self) -> bool {
        self.status == QueueStatus::Failed && self.retry_count < self.max_retries
    }
    
    /// Reset for retry
    pub fn reset_for_retry(&mut self) {
        self.status = QueueStatus::Queued;
        self.error_message = None;
        self.progress = 0.0;
        self.downloaded_bytes = None;
        self.download_speed = None;
        self.upload_speed = None;
        self.eta_seconds = None;
        self.updated_at = chrono::Utc::now();
        // Don't reset started_at to preserve first attempt time
    }
    
    /// Set download client ID
    pub fn set_download_client_id(&mut self, client_id: String) {
        self.download_client_id = Some(client_id);
        self.updated_at = chrono::Utc::now();
    }
    
    /// Set download path and category
    pub fn set_download_info(&mut self, path: Option<String>, category: Option<String>) {
        self.download_path = path;
        self.category = category;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Get human readable size
    pub fn human_readable_size(&self) -> Option<String> {
        self.size_bytes.map(|bytes| {
            const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
            let mut size = bytes as f64;
            let mut unit_index = 0;
            
            while size >= 1024.0 && unit_index < UNITS.len() - 1 {
                size /= 1024.0;
                unit_index += 1;
            }
            
            format!("{:.1} {}", size, UNITS[unit_index])
        })
    }
    
    /// Get human readable download speed
    pub fn human_readable_download_speed(&self) -> Option<String> {
        self.download_speed.map(|speed| {
            const UNITS: &[&str] = &["B/s", "KB/s", "MB/s", "GB/s"];
            let mut size = speed as f64;
            let mut unit_index = 0;
            
            while size >= 1024.0 && unit_index < UNITS.len() - 1 {
                size /= 1024.0;
                unit_index += 1;
            }
            
            format!("{:.1} {}", size, UNITS[unit_index])
        })
    }
    
    /// Get human readable ETA
    pub fn human_readable_eta(&self) -> Option<String> {
        self.eta_seconds.and_then(|eta| {
            if eta <= 0 {
                return None;
            }
            
            let hours = eta / 3600;
            let minutes = (eta % 3600) / 60;
            let seconds = eta % 60;
            
            if hours > 0 {
                Some(format!("{}h {}m", hours, minutes))
            } else if minutes > 0 {
                Some(format!("{}m {}s", minutes, seconds))
            } else {
                Some(format!("{}s", seconds))
            }
        })
    }
    
    /// Check if the download is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            QueueStatus::Queued | QueueStatus::Downloading | QueueStatus::Seeding
        )
    }
    
    /// Check if the download is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, QueueStatus::Completed | QueueStatus::Seeding)
    }
    
    /// Check if the download has failed
    pub fn is_failed(&self) -> bool {
        self.status == QueueStatus::Failed
    }
}

/// Queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total number of items in queue
    pub total_count: i64,
    /// Number of items currently downloading
    pub downloading_count: i64,
    /// Number of items queued for download
    pub queued_count: i64,
    /// Number of completed items
    pub completed_count: i64,
    /// Number of failed items
    pub failed_count: i64,
    /// Number of paused items
    pub paused_count: i64,
    /// Total download speed across all active downloads
    pub total_download_speed: u64,
    /// Total upload speed across all active uploads
    pub total_upload_speed: u64,
    /// Total size of all items in queue
    pub total_size_bytes: i64,
    /// Total downloaded bytes across all items
    pub total_downloaded_bytes: i64,
}

impl Default for QueueStats {
    fn default() -> Self {
        Self {
            total_count: 0,
            downloading_count: 0,
            queued_count: 0,
            completed_count: 0,
            failed_count: 0,
            paused_count: 0,
            total_download_speed: 0,
            total_upload_speed: 0,
            total_size_bytes: 0,
            total_downloaded_bytes: 0,
        }
    }
}

// Implement Display for enum serialization to string
impl std::fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueStatus::Queued => write!(f, "queued"),
            QueueStatus::Downloading => write!(f, "downloading"),
            QueueStatus::Completed => write!(f, "completed"),
            QueueStatus::Failed => write!(f, "failed"),
            QueueStatus::Cancelled => write!(f, "cancelled"),
            QueueStatus::Paused => write!(f, "paused"),
            QueueStatus::Stalled => write!(f, "stalled"),
            QueueStatus::Seeding => write!(f, "seeding"),
        }
    }
}

impl std::fmt::Display for QueuePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueuePriority::Low => write!(f, "low"),
            QueuePriority::Normal => write!(f, "normal"),
            QueuePriority::High => write!(f, "high"),
            QueuePriority::VeryHigh => write!(f, "very_high"),
        }
    }
}