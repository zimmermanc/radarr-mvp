//! Integration layer for import pipeline with Radarr's download management
//!
//! This module provides high-level integration between the import pipeline
//! and Radarr's download management system, handling post-download processing
//! and library organization.

use chrono::Utc;
use radarr_core::{
    domain::repositories::{DownloadRepository, MovieRepository},
    models::{Download, DownloadStatus, Movie},
    retry::{retry_with_backoff, RetryConfig, RetryPolicy},
    RadarrError, Result,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{error, info, instrument, warn};

use crate::{
    AnalyzedFile, FileScanner, ImportConfig, ImportPipeline, ImportResult, ImportStats,
    RenameEngine,
};

/// Configuration for integrated import operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedImportConfig {
    /// Base import pipeline configuration
    pub import_config: ImportConfig,
    /// Root directory for downloads
    pub download_directory: PathBuf,
    /// Root directory for media library
    pub library_directory: PathBuf,
    /// Whether to delete source files after successful import
    pub delete_after_import: bool,
    /// Whether to update movie records after import
    pub update_movie_records: bool,
    /// Whether to send notifications on import
    pub send_notifications: bool,
}

impl Default for IntegratedImportConfig {
    fn default() -> Self {
        Self {
            import_config: ImportConfig::default(),
            download_directory: PathBuf::from("/downloads/complete"),
            library_directory: PathBuf::from("/movies"),
            delete_after_import: false,
            update_movie_records: true,
            send_notifications: true,
        }
    }
}

/// Result of an integrated import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedImportResult {
    /// Download that was processed
    pub download_id: uuid::Uuid,
    /// Movie that was imported to
    pub movie_id: Option<uuid::Uuid>,
    /// Base import results
    pub import_results: Vec<ImportResult>,
    /// Overall statistics
    pub stats: ImportStats,
    /// Final library path of the movie
    pub library_path: Option<PathBuf>,
    /// Whether the operation was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Service for handling post-download import operations
pub struct ImportService<M: MovieRepository, D: DownloadRepository> {
    config: IntegratedImportConfig,
    pipeline: ImportPipeline,
    movie_repo: M,
    download_repo: D,
}

impl<M: MovieRepository, D: DownloadRepository> ImportService<M, D> {
    /// Create a new import service
    pub fn new(config: IntegratedImportConfig, movie_repo: M, download_repo: D) -> Self {
        let pipeline = ImportPipeline::new(config.import_config.clone());
        Self {
            config,
            pipeline,
            movie_repo,
            download_repo,
        }
    }

    /// Process a completed download
    #[instrument(skip(self))]
    pub async fn process_download(
        &self,
        download_id: uuid::Uuid,
    ) -> Result<IntegratedImportResult> {
        info!("Processing download: {}", download_id);

        // Get download record
        let mut download = self
            .download_repo
            .find_by_id(download_id)
            .await?
            .ok_or_else(|| RadarrError::NotFound {
                resource: format!("Download {} not found", download_id),
            })?;

        // Verify download is complete
        if download.status != DownloadStatus::Completed {
            return Err(RadarrError::ValidationError {
                field: "status".to_string(),
                message: format!(
                    "Download {} is not completed (status: {:?})",
                    download_id, download.status
                ),
            });
        }

        // Get associated movie
        let movie = self.movie_repo.find_by_id(download.movie_id).await?;

        // Determine source and destination paths
        let source_path = self.get_download_path(&download)?;
        let dest_path = self.get_library_path(&download, &movie)?;

        info!(
            "Importing from {} to {}",
            source_path.display(),
            dest_path.display()
        );

        // Run import pipeline with retry logic for transient failures
        let retry_config = RetryConfig::slow(); // Use slow retries for import operations
        let (stats, import_results) = retry_with_backoff(
            retry_config,
            RetryPolicy::Transient,
            "import_directory",
            || {
                self.pipeline
                    .import_directory_with_results(&source_path, &dest_path)
            },
        )
        .await?;

        // Update download status with retry
        let import_success = stats.successful_imports > 0;
        download.status = if import_success {
            DownloadStatus::Imported
        } else {
            DownloadStatus::Failed
        };

        let retry_config = RetryConfig::quick();
        retry_with_backoff(
            retry_config,
            RetryPolicy::Transient,
            "update_download_status",
            || self.download_repo.update(&download),
        )
        .await?;

        // Update movie record if successful
        if import_success {
            if let Some(mut movie) = movie {
                self.update_movie_record(&mut movie, &dest_path, &import_results)
                    .await?;
            }
        }

        // Clean up source files if configured
        if self.config.delete_after_import && import_success {
            self.cleanup_source_files(&source_path).await?;
        }

        Ok(IntegratedImportResult {
            download_id,
            movie_id: Some(download.movie_id),
            import_results,
            stats,
            library_path: Some(dest_path),
            success: import_success,
            error: None,
        })
    }

    /// Process all pending downloads
    #[instrument(skip(self))]
    pub async fn process_all_pending(&self) -> Result<Vec<IntegratedImportResult>> {
        info!("Processing all pending downloads");

        // Get all completed downloads that haven't been imported
        let pending_downloads = self
            .download_repo
            .find_by_status(DownloadStatus::Completed)
            .await?;

        info!("Found {} pending downloads", pending_downloads.len());

        let mut results = Vec::new();
        for download in pending_downloads {
            match self.process_download(download.id).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Failed to process download {}: {}", download.id, e);
                    if !self.config.import_config.continue_on_error {
                        return Err(e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get the download path for a download
    fn get_download_path(&self, download: &Download) -> Result<PathBuf> {
        let mut path = self.config.download_directory.clone();

        // Use title as the directory name
        path.push(&download.title);

        if !path.exists() {
            return Err(RadarrError::NotFound {
                resource: format!("Download path does not exist: {}", path.display()),
            });
        }

        Ok(path)
    }

    /// Get the library path for a movie
    fn get_library_path(&self, _download: &Download, movie: &Option<Movie>) -> Result<PathBuf> {
        let mut path = self.config.library_directory.clone();

        if let Some(movie) = movie {
            // Create movie folder: "Movie Title (Year)"
            let folder_name = if let Some(year) = movie.year {
                format!("{} ({})", movie.title, year)
            } else {
                movie.title.clone()
            };
            path.push(folder_name);
        } else {
            // Fallback to download title
            path.push(&_download.title);
        }

        Ok(path)
    }

    /// Update movie record after successful import
    async fn update_movie_record(
        &self,
        movie: &mut Movie,
        library_path: &Path,
        import_results: &[ImportResult],
    ) -> Result<()> {
        info!("Updating movie record for: {}", movie.title);

        // Update movie metadata to include the path
        let mut metadata = movie.metadata.as_object().cloned().unwrap_or_default();
        metadata.insert(
            "path".to_string(),
            serde_json::json!(library_path.to_string_lossy()),
        );

        // Find the main video file
        if let Some(main_file) = import_results
            .iter()
            .filter(|r| r.success)
            .find(|r| r.analyzed_file.is_some())
        {
            if let Some(analyzed) = &main_file.analyzed_file {
                // Update movie quality info if available
                let quality = &analyzed.quality;
                metadata.insert(
                    "quality".to_string(),
                    serde_json::json!({
                        "resolution": quality.resolution,
                        "source": quality.source,
                        "codec": quality.codec,
                        "audio": quality.audio,
                        "hdr": quality.hdr,
                    }),
                );
            }
        }

        movie.metadata = serde_json::Value::Object(metadata);

        // Update movie status
        movie.has_file = true;
        movie.monitored = true;
        movie.updated_at = Utc::now();

        self.movie_repo.update(movie).await?;
        Ok(())
    }

    /// Clean up source files after successful import
    async fn cleanup_source_files(&self, source_path: &Path) -> Result<()> {
        if !self.config.delete_after_import {
            return Ok(());
        }

        info!("Cleaning up source files at: {}", source_path.display());

        if source_path.is_dir() {
            tokio::fs::remove_dir_all(source_path).await.map_err(|e| {
                RadarrError::ExternalServiceError {
                    service: "filesystem".to_string(),
                    error: format!("Failed to remove source directory: {}", e),
                }
            })?;
        } else {
            tokio::fs::remove_file(source_path).await.map_err(|e| {
                RadarrError::ExternalServiceError {
                    service: "filesystem".to_string(),
                    error: format!("Failed to remove source file: {}", e),
                }
            })?;
        }

        Ok(())
    }

    /// Calculate statistics from import results
    fn calculate_stats(&self, results: &[ImportResult]) -> ImportStats {
        let successful_imports = results.iter().filter(|r| r.success).count();
        let failed_imports = results.iter().filter(|r| !r.success).count();
        let total_size = results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.detected_file.size)
            .sum();
        let total_duration = results
            .iter()
            .map(|r| r.duration)
            .fold(Duration::from_secs(0), |acc, d| acc + d);
        let hardlinks_created = results
            .iter()
            .filter_map(|r| r.hardlink_result.as_ref())
            .filter(|h| h.is_hardlink)
            .count();
        let files_copied = results
            .iter()
            .filter_map(|r| r.hardlink_result.as_ref())
            .filter(|h| !h.is_hardlink)
            .count();

        ImportStats {
            files_scanned: results.len(),
            files_analyzed: results.iter().filter(|r| r.analyzed_file.is_some()).count(),
            successful_imports,
            failed_imports,
            skipped_files: 0,
            total_size,
            total_duration,
            hardlinks_created,
            files_copied,
        }
    }
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_import_config_defaults() {
        let config = IntegratedImportConfig::default();

        assert_eq!(
            config.download_directory,
            PathBuf::from("/downloads/complete")
        );
        assert_eq!(config.library_directory, PathBuf::from("/movies"));
        assert!(!config.delete_after_import);
        assert!(config.update_movie_records);
        assert!(config.send_notifications);
    }
}
