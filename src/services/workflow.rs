//! Workflow coordination and orchestration
//!
//! This module provides workflow management for complex multi-step operations
//! that span multiple services and components.

use std::sync::Arc;
use std::collections::HashMap;
use radarr_core::{RadarrError, Result};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, instrument};

/// Workflow step definition
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    /// Step ID
    pub id: String,
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Whether this step is required
    pub required: bool,
    /// Estimated duration in seconds
    pub estimated_duration: u64,
}

/// Workflow step execution result
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Step ID
    pub step_id: String,
    /// Whether the step succeeded
    pub success: bool,
    /// Step output data
    pub data: Option<serde_json::Value>,
    /// Error message if step failed
    pub error: Option<String>,
    /// Step execution duration
    pub duration_ms: u64,
    /// Timestamp when step completed
    pub completed_at: DateTime<Utc>,
}

/// Workflow execution status
#[derive(Debug, Clone)]
pub enum WorkflowStatus {
    /// Workflow is pending execution
    Pending,
    /// Workflow is currently running
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow was cancelled
    Cancelled,
    /// Workflow is paused waiting for user input
    AwaitingInput,
}

/// Complete workflow execution context
#[derive(Debug, Clone)]
pub struct WorkflowExecution {
    /// Workflow ID
    pub id: Uuid,
    /// Workflow name
    pub name: String,
    /// Current status
    pub status: WorkflowStatus,
    /// Current step index
    pub current_step: usize,
    /// All workflow steps
    pub steps: Vec<WorkflowStep>,
    /// Completed step results
    pub step_results: Vec<StepResult>,
    /// Overall progress (0.0 to 1.0)
    pub progress: f32,
    /// Workflow input parameters
    pub input: serde_json::Value,
    /// Workflow output data
    pub output: Option<serde_json::Value>,
    /// Error message if workflow failed
    pub error_message: Option<String>,
    /// Workflow start time
    pub started_at: DateTime<Utc>,
    /// Workflow completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

/// Workflow manager for coordinating complex operations
pub struct WorkflowManager {
    /// Active workflow executions
    executions: Arc<RwLock<HashMap<Uuid, WorkflowExecution>>>,
}

impl WorkflowManager {
    /// Create a new workflow manager
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start a new workflow execution
    #[instrument(skip(self))]
    pub async fn start_workflow(
        &self,
        name: String,
        steps: Vec<WorkflowStep>,
        input: serde_json::Value,
    ) -> Result<Uuid> {
        let workflow_id = Uuid::new_v4();
        let now = Utc::now();
        
        let execution = WorkflowExecution {
            id: workflow_id,
            name: name.clone(),
            status: WorkflowStatus::Pending,
            current_step: 0,
            steps,
            step_results: Vec::new(),
            progress: 0.0,
            input,
            output: None,
            error_message: None,
            started_at: now,
            completed_at: None,
            updated_at: now,
        };
        
        let mut executions = self.executions.write().await;
        executions.insert(workflow_id, execution);
        
        info!("Started workflow '{}' with ID {}", name, workflow_id);
        Ok(workflow_id)
    }
    
    /// Update workflow status
    #[instrument(skip(self))]
    pub async fn update_workflow_status(
        &self,
        workflow_id: Uuid,
        status: WorkflowStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(execution) = executions.get_mut(&workflow_id) {
            execution.status = status;
            execution.error_message = error_message;
            execution.updated_at = Utc::now();
            
            // Set completion time if workflow is finished
            match execution.status {
                WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled => {
                    execution.completed_at = Some(Utc::now());
                    execution.progress = 1.0;
                }
                _ => {}
            }
            
            debug!("Updated workflow {} status to {:?}", workflow_id, execution.status);
            Ok(())
        } else {
            Err(RadarrError::NotFoundError {
                entity: "Workflow".to_string(),
                id: workflow_id.to_string(),
            })
        }
    }
    
    /// Complete a workflow step
    #[instrument(skip(self))]
    pub async fn complete_step(
        &self,
        workflow_id: Uuid,
        step_result: StepResult,
    ) -> Result<()> {
        let mut executions = self.executions.write().await;
        
        if let Some(execution) = executions.get_mut(&workflow_id) {
            execution.step_results.push(step_result.clone());
            
            // Update progress
            execution.progress = execution.step_results.len() as f32 / execution.steps.len() as f32;
            execution.updated_at = Utc::now();
            
            // Move to next step if this one succeeded
            if step_result.success {
                execution.current_step += 1;
                
                // Check if workflow is complete
                if execution.current_step >= execution.steps.len() {
                    execution.status = WorkflowStatus::Completed;
                    execution.completed_at = Some(Utc::now());
                    info!("Workflow {} completed successfully", workflow_id);
                } else {
                    debug!("Workflow {} advanced to step {}", workflow_id, execution.current_step);
                }
            } else {
                // Step failed
                execution.status = WorkflowStatus::Failed;
                execution.error_message = step_result.error.clone();
                execution.completed_at = Some(Utc::now());
                warn!("Workflow {} failed at step {}: {:?}", workflow_id, step_result.step_id, step_result.error);
            }
            
            Ok(())
        } else {
            Err(RadarrError::NotFoundError {
                entity: "Workflow".to_string(),
                id: workflow_id.to_string(),
            })
        }
    }
    
    /// Get workflow execution by ID
    pub async fn get_workflow(&self, workflow_id: Uuid) -> Result<Option<WorkflowExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.get(&workflow_id).cloned())
    }
    
    /// Get all workflow executions
    pub async fn get_all_workflows(&self) -> Result<Vec<WorkflowExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.values().cloned().collect())
    }
    
    /// Get workflows by status
    pub async fn get_workflows_by_status(&self, status: WorkflowStatus) -> Result<Vec<WorkflowExecution>> {
        let executions = self.executions.read().await;
        Ok(executions.values()
            .filter(|e| matches!(e.status, status))
            .cloned()
            .collect())
    }
    
    /// Cancel a workflow
    #[instrument(skip(self))]
    pub async fn cancel_workflow(&self, workflow_id: Uuid) -> Result<()> {
        self.update_workflow_status(
            workflow_id,
            WorkflowStatus::Cancelled,
            Some("Workflow cancelled by user".to_string()),
        ).await
    }
    
    /// Clean up completed workflows older than specified duration
    #[instrument(skip(self))]
    pub async fn cleanup_old_workflows(&self, max_age_hours: i64) -> Result<usize> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut executions = self.executions.write().await;
        
        let initial_count = executions.len();
        
        // Remove workflows that are completed and older than cutoff
        executions.retain(|_, execution| {
            match (&execution.status, &execution.completed_at) {
                (WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled, Some(completed_at)) => {
                    completed_at > &cutoff_time
                }
                _ => true, // Keep running or pending workflows
            }
        });
        
        let removed_count = initial_count - executions.len();
        
        if removed_count > 0 {
            info!("Cleaned up {} old workflow executions", removed_count);
        }
        
        Ok(removed_count)
    }
    
    /// Get workflow statistics
    pub async fn get_statistics(&self) -> Result<WorkflowStatistics> {
        let executions = self.executions.read().await;
        
        let total_workflows = executions.len();
        let mut completed = 0;
        let mut failed = 0;
        let mut running = 0;
        let mut pending = 0;
        let mut cancelled = 0;
        
        for execution in executions.values() {
            match execution.status {
                WorkflowStatus::Completed => completed += 1,
                WorkflowStatus::Failed => failed += 1,
                WorkflowStatus::Running => running += 1,
                WorkflowStatus::Pending => pending += 1,
                WorkflowStatus::Cancelled => cancelled += 1,
                WorkflowStatus::AwaitingInput => running += 1, // Count as running
            }
        }
        
        Ok(WorkflowStatistics {
            total_workflows,
            completed,
            failed,
            running,
            pending,
            cancelled,
        })
    }
}

/// Workflow execution statistics
#[derive(Debug, Clone)]
pub struct WorkflowStatistics {
    /// Total number of workflows
    pub total_workflows: usize,
    /// Number of completed workflows
    pub completed: usize,
    /// Number of failed workflows
    pub failed: usize,
    /// Number of currently running workflows
    pub running: usize,
    /// Number of pending workflows
    pub pending: usize,
    /// Number of cancelled workflows
    pub cancelled: usize,
}

impl Default for WorkflowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Movie workflow helper functions
pub mod movie_workflows {
    use super::*;
    
    /// Create a standard movie search workflow
    pub fn create_movie_search_workflow() -> Vec<WorkflowStep> {
        vec![
            WorkflowStep {
                id: "validate_input".to_string(),
                name: "Validate Input".to_string(),
                description: "Validate movie search parameters".to_string(),
                required: true,
                estimated_duration: 1,
            },
            WorkflowStep {
                id: "search_indexers".to_string(),
                name: "Search Indexers".to_string(),
                description: "Search configured indexers for releases".to_string(),
                required: true,
                estimated_duration: 10,
            },
            WorkflowStep {
                id: "filter_results".to_string(),
                name: "Filter Results".to_string(),
                description: "Apply quality profile and filters to results".to_string(),
                required: true,
                estimated_duration: 2,
            },
            WorkflowStep {
                id: "rank_releases".to_string(),
                name: "Rank Releases".to_string(),
                description: "Rank releases by quality and preferences".to_string(),
                required: true,
                estimated_duration: 1,
            },
        ]
    }
    
    /// Create a download workflow
    pub fn create_download_workflow() -> Vec<WorkflowStep> {
        vec![
            WorkflowStep {
                id: "validate_release".to_string(),
                name: "Validate Release".to_string(),
                description: "Validate release selection and parameters".to_string(),
                required: true,
                estimated_duration: 1,
            },
            WorkflowStep {
                id: "add_to_client".to_string(),
                name: "Add to Download Client".to_string(),
                description: "Add torrent to download client".to_string(),
                required: true,
                estimated_duration: 5,
            },
            WorkflowStep {
                id: "monitor_progress".to_string(),
                name: "Monitor Download".to_string(),
                description: "Monitor download progress".to_string(),
                required: false,
                estimated_duration: 3600, // 1 hour average
            },
        ]
    }
    
    /// Create an import workflow
    pub fn create_import_workflow() -> Vec<WorkflowStep> {
        vec![
            WorkflowStep {
                id: "scan_downloads".to_string(),
                name: "Scan Downloads".to_string(),
                description: "Scan download directory for completed files".to_string(),
                required: true,
                estimated_duration: 10,
            },
            WorkflowStep {
                id: "analyze_files".to_string(),
                name: "Analyze Files".to_string(),
                description: "Analyze and identify media files".to_string(),
                required: true,
                estimated_duration: 30,
            },
            WorkflowStep {
                id: "create_hardlinks".to_string(),
                name: "Create Hardlinks".to_string(),
                description: "Create hardlinks to media library".to_string(),
                required: true,
                estimated_duration: 60,
            },
            WorkflowStep {
                id: "rename_files".to_string(),
                name: "Rename Files".to_string(),
                description: "Rename files according to naming scheme".to_string(),
                required: true,
                estimated_duration: 5,
            },
            WorkflowStep {
                id: "update_database".to_string(),
                name: "Update Database".to_string(),
                description: "Update movie database with import results".to_string(),
                required: true,
                estimated_duration: 2,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::movie_workflows::*;
    
    #[tokio::test]
    async fn test_workflow_manager() {
        let manager = WorkflowManager::new();
        let steps = create_movie_search_workflow();
        let input = serde_json::json!({"title": "The Matrix", "year": 1999});
        
        let workflow_id = manager.start_workflow(
            "Movie Search".to_string(),
            steps,
            input,
        ).await.unwrap();
        
        let workflow = manager.get_workflow(workflow_id).await.unwrap().unwrap();
        assert_eq!(workflow.name, "Movie Search");
        assert!(matches!(workflow.status, WorkflowStatus::Pending));
    }
    
    #[tokio::test]
    async fn test_workflow_step_completion() {
        let manager = WorkflowManager::new();
        let steps = vec![
            WorkflowStep {
                id: "test_step".to_string(),
                name: "Test Step".to_string(),
                description: "A test step".to_string(),
                required: true,
                estimated_duration: 10,
            },
        ];
        
        let workflow_id = manager.start_workflow(
            "Test Workflow".to_string(),
            steps,
            serde_json::json!({}),
        ).await.unwrap();
        
        let step_result = StepResult {
            step_id: "test_step".to_string(),
            success: true,
            data: Some(serde_json::json!({"result": "success"})),
            error: None,
            duration_ms: 100,
            completed_at: Utc::now(),
        };
        
        manager.complete_step(workflow_id, step_result).await.unwrap();
        
        let workflow = manager.get_workflow(workflow_id).await.unwrap().unwrap();
        assert!(matches!(workflow.status, WorkflowStatus::Completed));
        assert_eq!(workflow.step_results.len(), 1);
    }
    
    #[tokio::test]
    async fn test_workflow_statistics() {
        let manager = WorkflowManager::new();
        
        // Start a few workflows
        for i in 0..3 {
            let steps = vec![WorkflowStep {
                id: format!("step_{}", i),
                name: format!("Step {}", i),
                description: "Test step".to_string(),
                required: true,
                estimated_duration: 10,
            }];
            
            manager.start_workflow(
                format!("Workflow {}", i),
                steps,
                serde_json::json!({}),
            ).await.unwrap();
        }
        
        let stats = manager.get_statistics().await.unwrap();
        assert_eq!(stats.total_workflows, 3);
        assert_eq!(stats.pending, 3);
    }
}