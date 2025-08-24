//! Download domain model

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Download status in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum DownloadStatus {
    #[default]
    Queued,
    Downloading,
    Completed,
    Failed,
    Warning,
    Paused,
    Importing,
    Imported,
    Removed,
}


impl std::fmt::Display for DownloadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadStatus::Queued => write!(f, "queued"),
            DownloadStatus::Downloading => write!(f, "downloading"),
            DownloadStatus::Completed => write!(f, "completed"),
            DownloadStatus::Failed => write!(f, "failed"),
            DownloadStatus::Warning => write!(f, "warning"),
            DownloadStatus::Paused => write!(f, "paused"),
            DownloadStatus::Importing => write!(f, "importing"),
            DownloadStatus::Imported => write!(f, "imported"),
            DownloadStatus::Removed => write!(f, "removed"),
        }
    }
}

/// Core download entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: Uuid,
    pub movie_id: Uuid,
    pub download_client_id: i32,
    pub indexer_id: Option<i32>,
    
    // Download identification
    pub download_id: String, // Client-specific ID (torrent hash, nzb id, etc.)
    pub title: String,
    pub category: Option<String>,
    
    // Status and progress
    pub status: DownloadStatus,
    pub size_bytes: Option<i64>,
    pub size_left: Option<i64>,
    
    // Quality information
    pub quality: serde_json::Value,
    
    // Timestamps
    pub download_time: Option<chrono::DateTime<chrono::Utc>>,
    pub completion_time: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    
    // Import status
    pub imported: bool,
    pub import_time: Option<chrono::DateTime<chrono::Utc>>,
    
    // Metadata
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Download {
    /// Create a new download
    pub fn new(
        movie_id: Uuid,
        download_client_id: i32,
        download_id: String,
        title: String,
    ) -> Self {
        let now = chrono::Utc::now();
        
        Self {
            id: Uuid::new_v4(),
            movie_id,
            download_client_id,
            indexer_id: None,
            download_id,
            title,
            category: None,
            status: DownloadStatus::default(),
            size_bytes: None,
            size_left: None,
            quality: serde_json::json!({}),
            download_time: None,
            completion_time: None,
            error_message: None,
            imported: false,
            import_time: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Update download status
    pub fn update_status(&mut self, status: DownloadStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now();
        
        // Set timestamps based on status
        match status {
            DownloadStatus::Downloading if self.download_time.is_none() => {
                self.download_time = Some(chrono::Utc::now());
            }
            DownloadStatus::Completed if self.completion_time.is_none() => {
                self.completion_time = Some(chrono::Utc::now());
            }
            DownloadStatus::Imported if self.import_time.is_none() => {
                self.imported = true;
                self.import_time = Some(chrono::Utc::now());
            }
            _ => {}
        }
    }
    
    /// Update download progress
    pub fn update_progress(&mut self, size_left: Option<i64>) {
        self.size_left = size_left;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.status = DownloadStatus::Failed;
        self.updated_at = chrono::Utc::now();
    }
    
    /// Calculate progress percentage
    pub fn progress_percentage(&self) -> Option<f64> {
        if let (Some(total), Some(left)) = (self.size_bytes, self.size_left) {
            if total > 0 {
                let completed = total - left;
                Some((completed as f64 / total as f64) * 100.0)
            } else {
                None
            }
        } else {
            // If we don't have size info, estimate based on status
            match self.status {
                DownloadStatus::Queued => Some(0.0),
                DownloadStatus::Downloading => None, // Unknown without size info
                DownloadStatus::Completed | DownloadStatus::Importing | DownloadStatus::Imported => Some(100.0),
                DownloadStatus::Failed | DownloadStatus::Warning => None,
                DownloadStatus::Paused => None,
                DownloadStatus::Removed => None,
            }
        }
    }
    
    /// Check if download is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            DownloadStatus::Queued | DownloadStatus::Downloading | DownloadStatus::Importing
        )
    }
    
    /// Check if download is completed
    pub fn is_completed(&self) -> bool {
        matches!(self.status, DownloadStatus::Completed | DownloadStatus::Imported)
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
    
    /// Get ETA in seconds if downloading
    pub fn eta_seconds(&self) -> Option<u64> {
        if self.status == DownloadStatus::Downloading {
            if let Some(progress) = self.progress_percentage() {
                if progress > 0.0 && progress < 100.0 {
                    // Very basic ETA estimation - would need download speed in real implementation
                    let remaining_percent = 100.0 - progress;
                    let elapsed = self.download_time?
                        .signed_duration_since(chrono::Utc::now())
                        .num_seconds() as f64;
                    
                    if elapsed > 0.0 {
                        Some((elapsed * remaining_percent / progress) as u64)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}