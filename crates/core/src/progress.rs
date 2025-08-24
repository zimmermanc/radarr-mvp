//! Progress tracking for long-running operations

use serde::{Deserialize, Serialize};
use std::time::Duration;
// use std::time::Instant; // Currently unused
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Types of operations that can be tracked
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OperationType {
    Download,
    Import,
    IndexerSearch,
    LibraryScan,
    QueueProcessing,
    Backup,
    Update,
}

/// Status of a tracked operation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

/// Progress information for an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Unique identifier for this operation
    pub id: Uuid,
    /// Type of operation
    pub operation_type: OperationType,
    /// Current status
    pub status: OperationStatus,
    /// Human-readable title
    pub title: String,
    /// Current progress message
    pub message: String,
    /// Progress percentage (0-100)
    pub percentage: f32,
    /// Current step number
    pub current_step: Option<u32>,
    /// Total number of steps
    pub total_steps: Option<u32>,
    /// Bytes processed (for downloads/uploads)
    pub bytes_processed: Option<u64>,
    /// Total bytes (for downloads/uploads)
    pub total_bytes: Option<u64>,
    /// Items processed (for batch operations)
    pub items_processed: Option<u32>,
    /// Total items (for batch operations)
    pub total_items: Option<u32>,
    /// When the operation started
    pub started_at: DateTime<Utc>,
    /// When the operation was last updated
    pub updated_at: DateTime<Utc>,
    /// When the operation completed (if applicable)
    pub completed_at: Option<DateTime<Utc>>,
    /// Estimated time remaining
    pub eta_seconds: Option<u64>,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ProgressInfo {
    /// Create a new progress info
    pub fn new(operation_type: OperationType, title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            operation_type,
            status: OperationStatus::Pending,
            title: title.into(),
            message: String::new(),
            percentage: 0.0,
            current_step: None,
            total_steps: None,
            bytes_processed: None,
            total_bytes: None,
            items_processed: None,
            total_items: None,
            started_at: now,
            updated_at: now,
            completed_at: None,
            eta_seconds: None,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Update progress percentage and message
    pub fn update(&mut self, percentage: f32, message: impl Into<String>) {
        self.percentage = percentage.clamp(0.0, 100.0);
        self.message = message.into();
        self.updated_at = Utc::now();
        self.status = OperationStatus::InProgress;

        // Calculate ETA if we have enough data
        if percentage > 0.0 && percentage < 100.0 {
            let elapsed = (self.updated_at - self.started_at).num_seconds() as f32;
            let remaining = (elapsed / percentage) * (100.0 - percentage);
            self.eta_seconds = Some(remaining as u64);
        }
    }

    /// Update with step information
    pub fn update_step(&mut self, current: u32, total: u32, message: impl Into<String>) {
        self.current_step = Some(current);
        self.total_steps = Some(total);
        self.percentage = (current as f32 / total as f32) * 100.0;
        self.message = message.into();
        self.updated_at = Utc::now();
        self.status = OperationStatus::InProgress;
    }

    /// Update with byte progress (for downloads)
    pub fn update_bytes(&mut self, processed: u64, total: u64) {
        self.bytes_processed = Some(processed);
        self.total_bytes = Some(total);
        self.percentage = (processed as f32 / total as f32) * 100.0;
        self.updated_at = Utc::now();
        self.status = OperationStatus::InProgress;

        // Auto-generate message
        self.message = format!(
            "{}/{} ({:.1}%)",
            format_bytes(processed),
            format_bytes(total),
            self.percentage
        );
    }

    /// Mark as completed
    pub fn complete(&mut self, message: impl Into<String>) {
        self.status = OperationStatus::Completed;
        self.percentage = 100.0;
        self.message = message.into();
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.eta_seconds = None;
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = OperationStatus::Failed;
        self.error = Some(error.into());
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.eta_seconds = None;
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Progress tracker for managing multiple operations
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    operations: Arc<RwLock<HashMap<Uuid, ProgressInfo>>>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a new operation
    pub async fn start_operation(
        &self,
        operation_type: OperationType,
        title: impl Into<String>,
    ) -> Uuid {
        let progress = ProgressInfo::new(operation_type, title);
        let id = progress.id;

        let mut ops = self.operations.write().await;
        ops.insert(id, progress);

        id
    }

    /// Update an operation's progress
    pub async fn update_progress(&self, id: Uuid, percentage: f32, message: impl Into<String>) {
        let mut ops = self.operations.write().await;
        if let Some(progress) = ops.get_mut(&id) {
            progress.update(percentage, message);
        }
    }

    /// Update with step information
    pub async fn update_step(
        &self,
        id: Uuid,
        current: u32,
        total: u32,
        message: impl Into<String>,
    ) {
        let mut ops = self.operations.write().await;
        if let Some(progress) = ops.get_mut(&id) {
            progress.update_step(current, total, message);
        }
    }

    /// Update byte progress
    pub async fn update_bytes(&self, id: Uuid, processed: u64, total: u64) {
        let mut ops = self.operations.write().await;
        if let Some(progress) = ops.get_mut(&id) {
            progress.update_bytes(processed, total);
        }
    }

    /// Complete an operation
    pub async fn complete_operation(&self, id: Uuid, message: impl Into<String>) {
        let mut ops = self.operations.write().await;
        if let Some(progress) = ops.get_mut(&id) {
            progress.complete(message);
        }
    }

    /// Fail an operation
    pub async fn fail_operation(&self, id: Uuid, error: impl Into<String>) {
        let mut ops = self.operations.write().await;
        if let Some(progress) = ops.get_mut(&id) {
            progress.fail(error);
        }
    }

    /// Get progress for a specific operation
    pub async fn get_progress(&self, id: Uuid) -> Option<ProgressInfo> {
        let ops = self.operations.read().await;
        ops.get(&id).cloned()
    }

    /// Get all active operations
    pub async fn get_active_operations(&self) -> Vec<ProgressInfo> {
        let ops = self.operations.read().await;
        ops.values()
            .filter(|p| {
                matches!(
                    p.status,
                    OperationStatus::InProgress | OperationStatus::Pending
                )
            })
            .cloned()
            .collect()
    }

    /// Get all operations
    pub async fn get_all_operations(&self) -> Vec<ProgressInfo> {
        let ops = self.operations.read().await;
        ops.values().cloned().collect()
    }

    /// Clean up completed operations older than the specified duration
    pub async fn cleanup_old_operations(&self, older_than: Duration) {
        let mut ops = self.operations.write().await;
        let cutoff = Utc::now()
            - chrono::Duration::from_std(older_than).unwrap_or_else(|_| chrono::Duration::hours(1));

        ops.retain(|_, progress| {
            if let Some(completed_at) = progress.completed_at {
                completed_at > cutoff
            } else {
                true // Keep incomplete operations
            }
        });
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_update() {
        let mut progress = ProgressInfo::new(OperationType::Download, "Test Download");

        progress.update(50.0, "Halfway there");
        assert_eq!(progress.percentage, 50.0);
        assert_eq!(progress.message, "Halfway there");
        assert_eq!(progress.status, OperationStatus::InProgress);

        progress.complete("Done!");
        assert_eq!(progress.percentage, 100.0);
        assert_eq!(progress.status, OperationStatus::Completed);
        assert!(progress.completed_at.is_some());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[tokio::test]
    async fn test_progress_tracker() {
        let tracker = ProgressTracker::new();

        let id = tracker
            .start_operation(OperationType::Import, "Test Import")
            .await;

        tracker.update_progress(id, 25.0, "Processing files").await;

        let progress = tracker.get_progress(id).await.unwrap();
        assert_eq!(progress.percentage, 25.0);

        tracker.complete_operation(id, "Import completed").await;

        let progress = tracker.get_progress(id).await.unwrap();
        assert_eq!(progress.status, OperationStatus::Completed);
    }
}
