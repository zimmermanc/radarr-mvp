//! Hardlink manager for preserving seedbox files
//! 
//! This module provides functionality to create hardlinks for imported files,
//! preserving the original files for seeding while organizing them in the
//! media library structure.

use std::path::{Path, PathBuf};
use std::fs;
use radarr_core::RadarrError;
use serde::{Deserialize, Serialize};
use tokio::fs as async_fs;
use tracing::{debug, info, warn, error};

/// Configuration for hardlink operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardlinkConfig {
    /// Whether to use hardlinks (if false, will copy files)
    pub enable_hardlinks: bool,
    /// Whether to allow copy fallback when hardlinks fail
    pub copy_fallback: bool,
    /// Verify hardlink creation with file size check
    pub verify_links: bool,
    /// Maximum file size for copy operations (bytes, 0 = no limit)
    pub max_copy_size: u64,
}

impl Default for HardlinkConfig {
    fn default() -> Self {
        Self {
            enable_hardlinks: true,
            copy_fallback: true,
            verify_links: true,
            max_copy_size: 50 * 1024 * 1024 * 1024, // 50GB limit for copies
        }
    }
}

/// Result of a hardlink operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardlinkResult {
    /// Source file path
    pub source: PathBuf,
    /// Destination file path
    pub destination: PathBuf,
    /// Whether a hardlink was created (vs copy)
    pub is_hardlink: bool,
    /// Size of the file that was linked/copied
    pub file_size: u64,
    /// Time taken for the operation in milliseconds
    pub duration_ms: u64,
}

/// Statistics for a batch of hardlink operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardlinkStats {
    /// Total number of files processed
    pub total_files: usize,
    /// Number of successful hardlinks
    pub hardlinks_created: usize,
    /// Number of fallback copies
    pub copies_created: usize,
    /// Number of failed operations
    pub failed_operations: usize,
    /// Total size of all processed files
    pub total_size: u64,
    /// Total time taken for all operations
    pub total_duration_ms: u64,
}

/// Hardlink manager for creating file links
pub struct HardlinkManager {
    config: HardlinkConfig,
}

impl HardlinkManager {
    /// Create a new hardlink manager with the given configuration
    pub fn new(config: HardlinkConfig) -> Self {
        Self { config }
    }

    /// Create a hardlink manager with default configuration
    pub fn default() -> Self {
        Self::new(HardlinkConfig::default())
    }

    /// Create a hardlink from source to destination
    pub async fn create_hardlink(
        &self,
        source: &Path,
        destination: &Path,
    ) -> Result<HardlinkResult, RadarrError> {
        let start_time = std::time::Instant::now();
        
        debug!("Creating hardlink: {} -> {}", source.display(), destination.display());

        // Validate source file exists
        if !source.exists() {
            return Err(RadarrError::ValidationError {
                field: "source".to_string(),
                message: format!("Source file does not exist: {}", source.display()),
            });
        }

        // Get source file metadata
        let source_metadata = async_fs::metadata(source).await.map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "filesystem".to_string(),
                error: format!("Failed to read source metadata: {}", e),
            }
        })?;

        let file_size = source_metadata.len();

        // Create destination directory if it doesn't exist
        if let Some(dest_dir) = destination.parent() {
            async_fs::create_dir_all(dest_dir).await.map_err(|e| {
                RadarrError::ExternalServiceError {
                    service: "filesystem".to_string(),
                    error: format!("Failed to create destination directory: {}", e),
                }
            })?;
        }

        // Remove destination if it already exists
        if destination.exists() {
            async_fs::remove_file(destination).await.map_err(|e| {
                RadarrError::ExternalServiceError {
                    service: "filesystem".to_string(),
                    error: format!("Failed to remove existing destination: {}", e),
                }
            })?;
        }

        let mut is_hardlink = false;

        // Try hardlink first if enabled
        if self.config.enable_hardlinks {
            match self.try_hardlink(source, destination).await {
                Ok(()) => {
                    is_hardlink = true;
                    info!("Successfully created hardlink: {}", destination.display());
                }
                Err(e) => {
                    warn!("Hardlink failed: {}. Will try copy fallback.", e);
                    
                    if !self.config.copy_fallback {
                        return Err(RadarrError::ExternalServiceError {
                            service: "filesystem".to_string(),
                            error: format!("Hardlink failed and copy fallback disabled: {}", e),
                        });
                    }
                }
            }
        }

        // Fall back to copying if hardlink wasn't created
        if !is_hardlink {
            self.copy_file(source, destination, file_size).await?;
            info!("Successfully copied file: {}", destination.display());
        }

        // Verify the operation if enabled
        if self.config.verify_links {
            self.verify_file(destination, file_size).await?;
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(HardlinkResult {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            is_hardlink,
            file_size,
            duration_ms,
        })
    }

    /// Attempt to create a hardlink using the system call
    async fn try_hardlink(&self, source: &Path, destination: &Path) -> Result<(), std::io::Error> {
        // Use tokio::task::spawn_blocking for the blocking fs::hard_link call
        let source = source.to_path_buf();
        let destination = destination.to_path_buf();
        
        tokio::task::spawn_blocking(move || {
            fs::hard_link(&source, &destination)
        }).await.map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Task join error: {}", e))
        })?
    }

    /// Copy file when hardlink is not possible
    async fn copy_file(&self, source: &Path, destination: &Path, file_size: u64) -> Result<(), RadarrError> {
        // Check file size limits
        if self.config.max_copy_size > 0 && file_size > self.config.max_copy_size {
            return Err(RadarrError::ValidationError {
                field: "file_size".to_string(),
                message: format!(
                    "File too large for copy operation: {} bytes (limit: {} bytes)",
                    file_size, self.config.max_copy_size
                ),
            });
        }

        async_fs::copy(source, destination).await.map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "filesystem".to_string(),
                error: format!("Failed to copy file: {}", e),
            }
        })?;

        Ok(())
    }

    /// Verify that the destination file was created correctly
    async fn verify_file(&self, destination: &Path, expected_size: u64) -> Result<(), RadarrError> {
        let dest_metadata = async_fs::metadata(destination).await.map_err(|e| {
            RadarrError::ExternalServiceError {
                service: "filesystem".to_string(),
                error: format!("Failed to verify destination file: {}", e),
            }
        })?;

        if dest_metadata.len() != expected_size {
            return Err(RadarrError::ValidationError {
                field: "file_size".to_string(),
                message: format!(
                    "File size mismatch: expected {} bytes, got {} bytes",
                    expected_size, dest_metadata.len()
                ),
            });
        }

        debug!("File verification successful: {} bytes", expected_size);
        Ok(())
    }

    /// Process multiple files in batch
    pub async fn process_batch(
        &self,
        files: &[(PathBuf, PathBuf)], // (source, destination) pairs
    ) -> Result<HardlinkStats, RadarrError> {
        let start_time = std::time::Instant::now();
        let total_files = files.len();
        
        info!("Processing batch of {} files", total_files);

        let mut hardlinks_created = 0;
        let mut copies_created = 0;
        let mut failed_operations = 0;
        let mut total_size = 0;

        for (source, destination) in files {
            match self.create_hardlink(source, destination).await {
                Ok(result) => {
                    total_size += result.file_size;
                    if result.is_hardlink {
                        hardlinks_created += 1;
                    } else {
                        copies_created += 1;
                    }
                }
                Err(e) => {
                    error!("Failed to process {}: {}", source.display(), e);
                    failed_operations += 1;
                }
            }
        }

        let total_duration_ms = start_time.elapsed().as_millis() as u64;

        let stats = HardlinkStats {
            total_files,
            hardlinks_created,
            copies_created,
            failed_operations,
            total_size,
            total_duration_ms,
        };

        info!("Batch processing complete: {} hardlinks, {} copies, {} failures", 
              hardlinks_created, copies_created, failed_operations);

        Ok(stats)
    }

    /// Check if two paths are on the same filesystem (for hardlink compatibility)
    pub async fn can_hardlink(&self, source: &Path, destination: &Path) -> bool {
        if !self.config.enable_hardlinks {
            return false;
        }

        // Try to get filesystem information for both paths
        let source_fs = self.get_filesystem_id(source).await;
        let dest_parent = destination.parent().unwrap_or(destination);
        let dest_fs = self.get_filesystem_id(dest_parent).await;

        match (source_fs, dest_fs) {
            (Some(source_id), Some(dest_id)) => source_id == dest_id,
            _ => {
                // If we can't determine filesystem info, assume it might work
                warn!("Could not determine filesystem compatibility, assuming hardlinks possible");
                true
            }
        }
    }

    /// Get filesystem identifier for a path (platform-specific)
    async fn get_filesystem_id(&self, path: &Path) -> Option<u64> {
        // This is a simplified version - real implementation would use
        // platform-specific system calls to get device ID
        match async_fs::metadata(path).await {
            Ok(metadata) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    Some(metadata.dev())
                }
                #[cfg(not(unix))]
                {
                    // On Windows, we could use GetVolumeInformation API
                    // For now, return None to indicate unknown
                    None
                }
            }
            Err(_) => None,
        }
    }

    /// Check available space at destination
    pub async fn check_available_space(&self, destination: &Path, required_size: u64) -> Result<bool, RadarrError> {
        let dest_dir = destination.parent().unwrap_or(destination);
        
        // This is a simplified check - real implementation would use
        // platform-specific APIs to get free space
        match async_fs::metadata(dest_dir).await {
            Ok(_) => {
                // Assume we have enough space for now
                // Real implementation would call statvfs on Unix or GetDiskFreeSpace on Windows
                debug!("Space check passed for {} bytes at {}", required_size, dest_dir.display());
                Ok(true)
            }
            Err(e) => Err(RadarrError::ExternalServiceError {
                service: "filesystem".to_string(),
                error: format!("Failed to check available space: {}", e),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::io::Write;

    #[tokio::test]
    async fn test_create_hardlink_success() {
        let temp_dir = TempDir::new().unwrap();
        let manager = HardlinkManager::default();
        
        // Create source file
        let source = temp_dir.path().join("source.txt");
        let mut file = std::fs::File::create(&source).unwrap();
        file.write_all(b"test content").unwrap();
        
        let destination = temp_dir.path().join("destination.txt");
        
        let result = manager.create_hardlink(&source, &destination).await.unwrap();
        
        assert_eq!(result.source, source);
        assert_eq!(result.destination, destination);
        assert!(destination.exists());
        assert_eq!(result.file_size, 12); // "test content" length
    }

    #[tokio::test]
    async fn test_copy_fallback() {
        let config = HardlinkConfig {
            enable_hardlinks: false, // Force copy mode
            copy_fallback: true,
            verify_links: true,
            max_copy_size: 1024 * 1024, // 1MB limit
        };
        let manager = HardlinkManager::new(config);
        
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let mut file = std::fs::File::create(&source).unwrap();
        file.write_all(b"test content").unwrap();
        
        let destination = temp_dir.path().join("destination.txt");
        
        let result = manager.create_hardlink(&source, &destination).await.unwrap();
        
        assert!(!result.is_hardlink); // Should be a copy
        assert!(destination.exists());
    }

    #[tokio::test]
    async fn test_file_too_large() {
        let config = HardlinkConfig {
            enable_hardlinks: false,
            copy_fallback: true,
            verify_links: false,
            max_copy_size: 5, // Very small limit
        };
        let manager = HardlinkManager::new(config);
        
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("large.txt");
        let mut file = std::fs::File::create(&source).unwrap();
        file.write_all(b"this content is too large").unwrap();
        
        let destination = temp_dir.path().join("destination.txt");
        
        let result = manager.create_hardlink(&source, &destination).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let temp_dir = TempDir::new().unwrap();
        let manager = HardlinkManager::default();
        
        // Create multiple source files
        let files = vec![
            (temp_dir.path().join("source1.txt"), temp_dir.path().join("dest1.txt")),
            (temp_dir.path().join("source2.txt"), temp_dir.path().join("dest2.txt")),
        ];
        
        for (source, _) in &files {
            let mut file = std::fs::File::create(source).unwrap();
            file.write_all(b"test").unwrap();
        }
        
        let stats = manager.process_batch(&files).await.unwrap();
        
        assert_eq!(stats.total_files, 2);
        assert_eq!(stats.failed_operations, 0);
        assert!(stats.hardlinks_created + stats.copies_created == 2);
    }

    #[tokio::test]
    async fn test_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let manager = HardlinkManager::default();
        
        let source = temp_dir.path().join("nonexistent.txt");
        let destination = temp_dir.path().join("destination.txt");
        
        let result = manager.create_hardlink(&source, &destination).await;
        assert!(result.is_err());
    }
}