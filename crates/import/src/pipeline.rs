//! Import pipeline orchestrator
//! 
//! This module provides the main ImportPipeline that coordinates all import
//! operations including scanning, analysis, hardlinking, and renaming.

use std::path::Path;
use std::time::{Duration, Instant};
use radarr_core::RadarrError;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, debug, instrument};

use crate::{
    file_scanner::{FileScanner, ScanConfig, DetectedFile},
    file_analyzer::{FileAnalyzer, AnalyzedFile},
    hardlink_manager::{HardlinkManager, HardlinkConfig, HardlinkResult},
    rename_engine::{RenameEngine, RenameConfig, RenameResult},
};

/// Complete configuration for the import pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    /// File scanning configuration
    pub scan_config: ScanConfig,
    /// Hardlink management configuration
    pub hardlink_config: HardlinkConfig,
    /// File renaming configuration
    pub rename_config: RenameConfig,
    /// Whether to actually move files or just plan the operation
    pub dry_run: bool,
    /// Minimum confidence score to accept analyzed files
    pub min_confidence: f32,
    /// Skip sample files during import
    pub skip_samples: bool,
    /// Continue on errors instead of stopping
    pub continue_on_error: bool,
    /// Maximum parallel operations
    pub max_parallel: usize,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            scan_config: ScanConfig::default(),
            hardlink_config: HardlinkConfig::default(),
            rename_config: RenameConfig::default(),
            dry_run: false,
            min_confidence: 0.3,
            skip_samples: true,
            continue_on_error: true,
            max_parallel: 4,
        }
    }
}

/// Result of importing a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// Original detected file
    pub detected_file: DetectedFile,
    /// Analysis results
    pub analyzed_file: Option<AnalyzedFile>,
    /// Hardlink operation result
    pub hardlink_result: Option<HardlinkResult>,
    /// Rename operation result
    pub rename_result: Option<RenameResult>,
    /// Whether the import was successful
    pub success: bool,
    /// Error message if import failed
    pub error: Option<String>,
    /// Time taken for this import
    pub duration: Duration,
}

/// Statistics for a complete import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStats {
    /// Total files scanned
    pub files_scanned: usize,
    /// Files that passed analysis
    pub files_analyzed: usize,
    /// Files successfully imported
    pub successful_imports: usize,
    /// Files that failed import
    pub failed_imports: usize,
    /// Files skipped (samples, low confidence, etc.)
    pub skipped_files: usize,
    /// Total size of imported files
    pub total_size: u64,
    /// Total time for the entire operation
    pub total_duration: Duration,
    /// Number of hardlinks created
    pub hardlinks_created: usize,
    /// Number of files copied
    pub files_copied: usize,
}

/// Main import pipeline orchestrator
pub struct ImportPipeline {
    config: ImportConfig,
    file_scanner: FileScanner,
    file_analyzer: FileAnalyzer,
    hardlink_manager: HardlinkManager,
    rename_engine: RenameEngine,
}

impl ImportPipeline {
    /// Create a new import pipeline with the given configuration
    pub fn new(config: ImportConfig) -> Self {
        let file_scanner = FileScanner::new(config.scan_config.clone());
        let file_analyzer = FileAnalyzer::new();
        let hardlink_manager = HardlinkManager::new(config.hardlink_config.clone());
        let rename_engine = RenameEngine::new(config.rename_config.clone());

        Self {
            config,
            file_scanner,
            file_analyzer,
            hardlink_manager,
            rename_engine,
        }
    }

    /// Create an import pipeline with default configuration
    pub fn default() -> Self {
        Self::new(ImportConfig::default())
    }

    /// Import all files from source directory to destination
    #[instrument(skip(self), fields(source = %source_dir.display(), dest = %dest_dir.display()))]
    pub async fn import_directory(
        &self,
        source_dir: &Path,
        dest_dir: &Path,
    ) -> Result<ImportStats, RadarrError> {
        let start_time = Instant::now();
        info!("Starting import from {} to {}", source_dir.display(), dest_dir.display());

        // Phase 1: Scan for media files
        let detected_files = self.scan_phase(source_dir).await?;
        info!("Scan phase complete: {} files detected", detected_files.len());

        // Phase 2: Analyze detected files
        let analyzed_files = self.analyze_phase(&detected_files).await?;
        info!("Analysis phase complete: {} files analyzed", analyzed_files.len());

        // Phase 3: Import files (hardlink + rename)
        let import_results = self.import_phase(&analyzed_files, dest_dir).await?;
        info!("Import phase complete: {} files processed", import_results.len());

        // Generate statistics
        let stats = self.generate_stats(&detected_files, &import_results, start_time.elapsed());
        info!("Import operation complete: {} successful, {} failed, {} skipped", 
              stats.successful_imports, stats.failed_imports, stats.skipped_files);

        Ok(stats)
    }

    /// Import all files from source directory to destination, returning both stats and individual results
    #[instrument(skip(self), fields(source = %source_dir.display(), dest = %dest_dir.display()))]
    pub async fn import_directory_with_results(
        &self,
        source_dir: &Path,
        dest_dir: &Path,
    ) -> Result<(ImportStats, Vec<ImportResult>), RadarrError> {
        let start_time = Instant::now();
        info!("Starting import from {} to {}", source_dir.display(), dest_dir.display());

        // Phase 1: Scan for media files
        let detected_files = self.scan_phase(source_dir).await?;
        info!("Scan phase complete: {} files detected", detected_files.len());

        // Phase 2: Analyze detected files
        let analyzed_files = self.analyze_phase(&detected_files).await?;
        info!("Analysis phase complete: {} files analyzed", analyzed_files.len());

        // Phase 3: Import files (hardlink + rename)
        let import_results = self.import_phase(&analyzed_files, dest_dir).await?;
        info!("Import phase complete: {} files processed", import_results.len());

        // Generate statistics
        let stats = self.generate_stats(&detected_files, &import_results, start_time.elapsed());
        info!("Import operation complete: {} successful, {} failed, {} skipped", 
              stats.successful_imports, stats.failed_imports, stats.skipped_files);

        Ok((stats, import_results))
    }

    /// Import a single file from source to destination
    #[instrument(skip(self))]
    pub async fn import_file(
        &self,
        source_path: &Path,
        dest_dir: &Path,
    ) -> Result<ImportResult, RadarrError> {
        let start_time = Instant::now();
        debug!("Importing single file: {}", source_path.display());

        // Analyze the single file
        let analyzed_file = match self.file_analyzer.analyze_file(source_path) {
            Ok(analyzed) => {
                if analyzed.confidence < self.config.min_confidence {
                    return Ok(ImportResult {
                        detected_file: DetectedFile {
                            path: source_path.to_path_buf(),
                            size: 0,
                            extension: String::new(),
                            modified: std::time::SystemTime::now(),
                            media_type: crate::file_scanner::MediaType::Unknown,
                            is_sample: false,
                        },
                        analyzed_file: Some(analyzed),
                        hardlink_result: None,
                        rename_result: None,
                        success: false,
                        error: Some("Confidence too low".to_string()),
                        duration: start_time.elapsed(),
                    });
                }
                analyzed
            }
            Err(e) => {
                return Ok(ImportResult {
                    detected_file: DetectedFile {
                        path: source_path.to_path_buf(),
                        size: 0,
                        extension: String::new(),
                        modified: std::time::SystemTime::now(),
                        media_type: crate::file_scanner::MediaType::Unknown,
                        is_sample: false,
                    },
                    analyzed_file: None,
                    hardlink_result: None,
                    rename_result: None,
                    success: false,
                    error: Some(e.to_string()),
                    duration: start_time.elapsed(),
                });
            }
        };

        // Execute the import
        Ok(self.import_single_file(&analyzed_file, dest_dir, start_time).await)
    }

    /// Scan phase: discover all media files
    async fn scan_phase(&self, source_dir: &Path) -> Result<Vec<DetectedFile>, RadarrError> {
        debug!("Starting scan phase");
        self.file_scanner.scan_directory(source_dir).await
    }

    /// Analysis phase: analyze all detected files
    async fn analyze_phase(&self, detected_files: &[DetectedFile]) -> Result<Vec<AnalyzedFile>, RadarrError> {
        debug!("Starting analysis phase for {} files", detected_files.len());
        
        let mut analyzed_files = Vec::new();
        
        for detected_file in detected_files {
            // Skip samples if configured
            if self.config.skip_samples && detected_file.is_sample {
                debug!("Skipping sample file: {}", detected_file.path.display());
                continue;
            }

            match self.file_analyzer.analyze_file(&detected_file.path) {
                Ok(analyzed) => {
                    if analyzed.confidence >= self.config.min_confidence {
                        analyzed_files.push(analyzed);
                    } else {
                        debug!("Skipping file with low confidence {}: {}", 
                               analyzed.confidence, detected_file.path.display());
                    }
                }
                Err(e) => {
                    if self.config.continue_on_error {
                        warn!("Failed to analyze {}: {}", detected_file.path.display(), e);
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        
        Ok(analyzed_files)
    }

    /// Import phase: hardlink and rename files
    async fn import_phase(
        &self,
        analyzed_files: &[AnalyzedFile],
        dest_dir: &Path,
    ) -> Result<Vec<ImportResult>, RadarrError> {
        debug!("Starting import phase for {} files", analyzed_files.len());
        
        let mut results = Vec::new();
        
        // Process files in batches to control parallelism
        for chunk in analyzed_files.chunks(self.config.max_parallel) {
            let mut batch_futures = Vec::new();
            
            for analyzed_file in chunk {
                let future = self.import_single_file(analyzed_file, dest_dir, Instant::now());
                batch_futures.push(future);
            }
            
            // Wait for all files in this batch to complete
            let batch_results = futures::future::join_all(batch_futures).await;
            results.extend(batch_results);
        }
        
        Ok(results)
    }

    /// Import a single analyzed file
    async fn import_single_file(
        &self,
        analyzed_file: &AnalyzedFile,
        dest_dir: &Path,
        start_time: Instant,
    ) -> ImportResult {
        debug!("Importing file: {}", analyzed_file.path.display());

        let detected_file = DetectedFile {
            path: analyzed_file.path.clone(),
            size: 0, // Would need to get from filesystem
            extension: analyzed_file.path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("unknown")
                .to_string(),
            modified: std::time::SystemTime::now(),
            media_type: crate::file_scanner::MediaType::Movie, // Simplified
            is_sample: analyzed_file.is_sample,
        };

        // Step 1: Generate rename plan
        let mut rename_result = match self.rename_engine.generate_filename(analyzed_file, dest_dir) {
            Ok(result) => result,
            Err(e) => {
                return ImportResult {
                    detected_file,
                    analyzed_file: Some(analyzed_file.clone()),
                    hardlink_result: None,
                    rename_result: None,
                    success: false,
                    error: Some(format!("Rename planning failed: {}", e)),
                    duration: start_time.elapsed(),
                };
            }
        };

        // Step 2: Create hardlink/copy to new location
        let hardlink_result = if !self.config.dry_run {
            match self.hardlink_manager.create_hardlink(&analyzed_file.path, &rename_result.new_path).await {
                Ok(result) => Some(result),
                Err(e) => {
                    return ImportResult {
                        detected_file,
                        analyzed_file: Some(analyzed_file.clone()),
                        hardlink_result: None,
                        rename_result: Some(rename_result),
                        success: false,
                        error: Some(format!("Hardlink failed: {}", e)),
                        duration: start_time.elapsed(),
                    };
                }
            }
        } else {
            info!("DRY RUN: Would create hardlink {} -> {}", 
                  analyzed_file.path.display(), rename_result.new_path.display());
            None
        };

        // Mark rename as executed if we're not in dry run mode
        if !self.config.dry_run {
            rename_result.executed = true;
        }

        ImportResult {
            detected_file,
            analyzed_file: Some(analyzed_file.clone()),
            hardlink_result,
            rename_result: Some(rename_result),
            success: true,
            error: None,
            duration: start_time.elapsed(),
        }
    }

    /// Generate comprehensive statistics for the import operation
    fn generate_stats(
        &self,
        detected_files: &[DetectedFile],
        import_results: &[ImportResult],
        total_duration: Duration,
    ) -> ImportStats {
        let files_scanned = detected_files.len();
        let files_analyzed = import_results.iter().filter(|r| r.analyzed_file.is_some()).count();
        let successful_imports = import_results.iter().filter(|r| r.success).count();
        let failed_imports = import_results.iter().filter(|r| !r.success).count();
        let skipped_files = files_scanned - import_results.len();

        let total_size = import_results.iter()
            .filter_map(|r| r.hardlink_result.as_ref())
            .map(|hr| hr.file_size)
            .sum();

        let hardlinks_created = import_results.iter()
            .filter_map(|r| r.hardlink_result.as_ref())
            .filter(|hr| hr.is_hardlink)
            .count();

        let files_copied = import_results.iter()
            .filter_map(|r| r.hardlink_result.as_ref())
            .filter(|hr| !hr.is_hardlink)
            .count();

        ImportStats {
            files_scanned,
            files_analyzed,
            successful_imports,
            failed_imports,
            skipped_files,
            total_size,
            total_duration,
            hardlinks_created,
            files_copied,
        }
    }

    /// Get configuration for this pipeline
    pub fn config(&self) -> &ImportConfig {
        &self.config
    }

    /// Update pipeline configuration
    pub fn update_config(&mut self, config: ImportConfig) {
        self.config = config.clone();
        self.file_scanner = FileScanner::new(config.scan_config);
        self.hardlink_manager = HardlinkManager::new(config.hardlink_config);
        self.rename_engine = RenameEngine::new(config.rename_config);
    }

    /// Validate that the pipeline is properly configured
    pub fn validate_config(&self) -> Result<(), RadarrError> {
        // Validate rename template
        self.rename_engine.validate_template(&self.config.rename_config.movie_template)?;
        self.rename_engine.validate_template(&self.config.rename_config.folder_template)?;

        // Validate confidence threshold
        if self.config.min_confidence < 0.0 || self.config.min_confidence > 1.0 {
            return Err(RadarrError::ValidationError {
                field: "min_confidence".to_string(),
                message: "Confidence must be between 0.0 and 1.0".to_string(),
            });
        }

        // Validate max_parallel
        if self.config.max_parallel == 0 {
            return Err(RadarrError::ValidationError {
                field: "max_parallel".to_string(),
                message: "max_parallel must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ImportPipeline {
    fn default() -> Self {
        Self::new(ImportConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_pipeline_creation() {
        let config = ImportConfig::default();
        let pipeline = ImportPipeline::new(config);
        assert!(pipeline.validate_config().is_ok());
    }

    #[tokio::test]
    async fn test_import_directory_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let source_dir = temp_dir.path().join("source");
        let dest_dir = temp_dir.path().join("dest");
        
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();
        
        // Create test movie file
        let movie_file = source_dir.join("The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
        fs::write(&movie_file, vec![0u8; 200 * 1024 * 1024]).unwrap(); // 200MB
        
        let mut config = ImportConfig::default();
        config.dry_run = true;
        config.min_confidence = 0.1; // Lower threshold for testing
        
        let pipeline = ImportPipeline::new(config);
        
        let stats = pipeline.import_directory(&source_dir, &dest_dir).await.unwrap();
        
        assert_eq!(stats.files_scanned, 1);
        // In dry run, no actual files should be moved
    }

    #[test]
    fn test_config_validation() {
        let mut config = ImportConfig::default();
        config.min_confidence = 1.5; // Invalid confidence
        
        let pipeline = ImportPipeline::new(config);
        assert!(pipeline.validate_config().is_err());
    }

    #[test]
    fn test_stats_generation() {
        let pipeline = ImportPipeline::default();
        let detected_files = vec![];
        let import_results = vec![];
        let duration = Duration::from_secs(10);
        
        let stats = pipeline.generate_stats(&detected_files, &import_results, duration);
        
        assert_eq!(stats.files_scanned, 0);
        assert_eq!(stats.successful_imports, 0);
        assert_eq!(stats.total_duration, duration);
    }
}