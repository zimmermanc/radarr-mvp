use crate::models::Movie;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};
// use tracing::debug; // Currently unused
use uuid::Uuid;

/// List synchronization scheduler that manages periodic syncs
#[derive(Clone)]
pub struct ListSyncScheduler {
    jobs: Arc<RwLock<HashMap<Uuid, SyncJob>>>,
    running_jobs: Arc<Mutex<HashMap<Uuid, RunningJob>>>,
    sync_handler: Arc<dyn SyncHandler>,
}

/// Represents a scheduled sync job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncJob {
    pub id: Uuid,
    pub list_id: Uuid,
    pub list_name: String,
    pub source_type: String,
    pub enabled: bool,
    pub sync_interval: Duration,
    pub next_sync: DateTime<Utc>,
    pub last_sync: Option<DateTime<Utc>>,
    pub priority: u8,
    pub retry_count: u32,
    pub max_retries: u32,
}

/// Represents a currently running sync job
#[derive(Debug)]
struct RunningJob {
    job_id: Uuid,
    started_at: DateTime<Utc>,
    cancel_token: tokio::sync::oneshot::Sender<()>,
}

/// Trait for handling sync operations
#[async_trait::async_trait]
pub trait SyncHandler: Send + Sync {
    /// Execute a sync job
    async fn execute_sync(&self, job: &SyncJob) -> Result<SyncResult, SyncError>;

    /// Handle sync conflicts
    async fn resolve_conflict(&self, existing: &Movie, new: &Movie) -> ConflictResolution;

    /// Store sync results
    async fn store_results(&self, results: &SyncResult) -> Result<(), SyncError>;
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub job_id: Uuid,
    pub list_id: Uuid,
    pub status: SyncStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub items_found: usize,
    pub items_added: usize,
    pub items_updated: usize,
    pub items_excluded: usize,
    pub items_conflicted: usize,
    pub error_message: Option<String>,
    pub provenance: Vec<MovieProvenance>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncStatus {
    Success,
    Partial,
    Failed,
    Cancelled,
}

/// How to resolve conflicts when a movie already exists
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConflictResolution {
    Keep,   // Keep existing movie unchanged
    Update, // Update with new information
    Merge,  // Merge metadata (prefer newer/better quality)
    Skip,   // Skip this item
}

/// Tracks where a movie came from
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieProvenance {
    pub movie_id: Uuid,
    pub list_id: Uuid,
    pub list_name: String,
    pub source_type: String,
    pub added_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("List not found: {0}")]
    ListNotFound(Uuid),

    #[error("Sync already running for list: {0}")]
    AlreadyRunning(Uuid),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl ListSyncScheduler {
    /// Create a new scheduler with the given sync handler
    pub fn new(sync_handler: Arc<dyn SyncHandler>) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            running_jobs: Arc::new(Mutex::new(HashMap::new())),
            sync_handler,
        }
    }

    /// Add a new sync job to the scheduler
    pub async fn add_job(&self, job: SyncJob) -> Result<(), SyncError> {
        let mut jobs = self.jobs.write().await;
        info!(
            "Adding sync job for list {} ({})",
            job.list_name, job.list_id
        );
        jobs.insert(job.id, job);
        Ok(())
    }

    /// Remove a sync job from the scheduler
    pub async fn remove_job(&self, job_id: Uuid) -> Result<(), SyncError> {
        let mut jobs = self.jobs.write().await;

        // Cancel if running
        let mut running = self.running_jobs.lock().await;
        if let Some(running_job) = running.remove(&job_id) {
            let _ = running_job.cancel_token.send(());
            info!("Cancelled running job {}", job_id);
        }

        jobs.remove(&job_id);
        info!("Removed sync job {}", job_id);
        Ok(())
    }

    /// Enable or disable a sync job
    pub async fn set_job_enabled(&self, job_id: Uuid, enabled: bool) -> Result<(), SyncError> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.enabled = enabled;
            info!("Set job {} enabled={}", job_id, enabled);
            Ok(())
        } else {
            Err(SyncError::ListNotFound(job_id))
        }
    }

    /// Manually trigger a sync job
    pub async fn trigger_sync(&self, job_id: Uuid) -> Result<(), SyncError> {
        let jobs = self.jobs.read().await;
        if let Some(job) = jobs.get(&job_id) {
            let job = job.clone();
            drop(jobs); // Release read lock before executing

            self.execute_job(job).await
        } else {
            Err(SyncError::ListNotFound(job_id))
        }
    }

    /// Start the scheduler loop
    pub async fn start(&self) {
        info!("Starting list sync scheduler");
        let mut interval = interval(TokioDuration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            // Check for jobs that need to run
            let jobs = self.jobs.read().await;
            let now = Utc::now();

            for job in jobs.values() {
                if job.enabled && job.next_sync <= now {
                    let job = job.clone();
                    let scheduler = self.clone();

                    // Spawn async task for each job
                    tokio::spawn(async move {
                        if let Err(e) = scheduler.execute_job(job.clone()).await {
                            error!("Failed to execute sync job {}: {}", job.id, e);

                            // Handle retry logic
                            if job.retry_count < job.max_retries {
                                let mut jobs = scheduler.jobs.write().await;
                                if let Some(job) = jobs.get_mut(&job.id) {
                                    job.retry_count += 1;
                                    job.next_sync = Utc::now() + Duration::minutes(5); // Retry in 5 minutes
                                    warn!(
                                        "Scheduled retry {} of {} for job {}",
                                        job.retry_count, job.max_retries, job.id
                                    );
                                }
                            }
                        }
                    });
                }
            }
        }
    }

    /// Execute a sync job
    async fn execute_job(&self, job: SyncJob) -> Result<(), SyncError> {
        // Check if already running
        {
            let running = self.running_jobs.lock().await;
            if running.contains_key(&job.id) {
                return Err(SyncError::AlreadyRunning(job.id));
            }
        }

        info!(
            "Executing sync job for list {} ({})",
            job.list_name, job.list_id
        );
        let start = Utc::now();

        // Create cancel token
        let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();

        // Mark as running
        {
            let mut running = self.running_jobs.lock().await;
            running.insert(
                job.id,
                RunningJob {
                    job_id: job.id,
                    started_at: start,
                    cancel_token: cancel_tx,
                },
            );
        }

        // Execute sync with cancellation support
        let sync_future = self.sync_handler.execute_sync(&job);
        let result = tokio::select! {
            result = sync_future => result,
            _ = &mut cancel_rx => {
                info!("Sync job {} was cancelled", job.id);
                Err(SyncError::Unknown("Job cancelled".to_string()))
            }
        };

        // Remove from running jobs
        {
            let mut running = self.running_jobs.lock().await;
            running.remove(&job.id);
        }

        // Process result
        match result {
            Ok(sync_result) => {
                info!(
                    "Sync job {} completed successfully: {} items found, {} added, {} updated",
                    job.id,
                    sync_result.items_found,
                    sync_result.items_added,
                    sync_result.items_updated
                );

                // Store results
                if let Err(e) = self.sync_handler.store_results(&sync_result).await {
                    error!("Failed to store sync results: {}", e);
                }

                // Update job with next sync time
                let mut jobs = self.jobs.write().await;
                if let Some(job) = jobs.get_mut(&job.id) {
                    job.last_sync = Some(Utc::now());
                    job.next_sync = Utc::now() + job.sync_interval;
                    job.retry_count = 0; // Reset retry count on success
                    info!(
                        "Next sync for {} scheduled at {}",
                        job.list_name, job.next_sync
                    );
                }

                Ok(())
            }
            Err(e) => {
                error!("Sync job {} failed: {}", job.id, e);
                Err(e)
            }
        }
    }

    /// Get status of all jobs
    pub async fn get_job_statuses(&self) -> Vec<JobStatus> {
        let jobs = self.jobs.read().await;
        let running = self.running_jobs.lock().await;

        jobs.values()
            .map(|job| {
                let is_running = running.contains_key(&job.id);
                let running_since = running.get(&job.id).map(|r| r.started_at);

                JobStatus {
                    job_id: job.id,
                    list_id: job.list_id,
                    list_name: job.list_name.clone(),
                    enabled: job.enabled,
                    is_running,
                    running_since,
                    last_sync: job.last_sync,
                    next_sync: job.next_sync,
                    retry_count: job.retry_count,
                }
            })
            .collect()
    }
}

/// Status information for a sync job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatus {
    pub job_id: Uuid,
    pub list_id: Uuid,
    pub list_name: String,
    pub enabled: bool,
    pub is_running: bool,
    pub running_since: Option<DateTime<Utc>>,
    pub last_sync: Option<DateTime<Utc>>,
    pub next_sync: DateTime<Utc>,
    pub retry_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSyncHandler;

    #[async_trait::async_trait]
    impl SyncHandler for MockSyncHandler {
        async fn execute_sync(&self, job: &SyncJob) -> Result<SyncResult, SyncError> {
            Ok(SyncResult {
                job_id: job.id,
                list_id: job.list_id,
                status: SyncStatus::Success,
                started_at: Utc::now(),
                completed_at: Utc::now(),
                duration_ms: 1000,
                items_found: 10,
                items_added: 5,
                items_updated: 2,
                items_excluded: 3,
                items_conflicted: 0,
                error_message: None,
                provenance: vec![],
            })
        }

        async fn resolve_conflict(&self, _existing: &Movie, _new: &Movie) -> ConflictResolution {
            ConflictResolution::Keep
        }

        async fn store_results(&self, _results: &SyncResult) -> Result<(), SyncError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_add_and_trigger_job() {
        let handler = Arc::new(MockSyncHandler);
        let scheduler = ListSyncScheduler::new(handler);

        let job = SyncJob {
            id: Uuid::new_v4(),
            list_id: Uuid::new_v4(),
            list_name: "Test List".to_string(),
            source_type: "test".to_string(),
            enabled: true,
            sync_interval: Duration::hours(6),
            next_sync: Utc::now() + Duration::hours(1),
            last_sync: None,
            priority: 5,
            retry_count: 0,
            max_retries: 3,
        };

        scheduler.add_job(job.clone()).await.unwrap();
        scheduler.trigger_sync(job.id).await.unwrap();

        let statuses = scheduler.get_job_statuses().await;
        assert_eq!(statuses.len(), 1);
        assert!(statuses[0].last_sync.is_some());
    }
}
