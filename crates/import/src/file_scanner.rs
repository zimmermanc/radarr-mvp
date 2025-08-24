//! File scanner for detecting and cataloging media files
//!
//! This module provides functionality to recursively scan directories
//! for video files and extract metadata about them.

use radarr_core::RadarrError;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::SystemTime;
use tokio::fs;
use tracing::{debug, info, warn};

/// Configuration for file scanning operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Maximum depth to scan directories
    pub max_depth: u8,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Minimum file size in bytes to consider
    pub min_file_size: u64,
    /// Maximum file size in bytes to consider (0 = no limit)
    pub max_file_size: u64,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            follow_symlinks: false,
            min_file_size: 100 * 1024 * 1024, // 100MB minimum
            max_file_size: 0,                 // No limit
        }
    }
}

/// Detected media file with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedFile {
    /// Full path to the file
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File extension (lowercase)
    pub extension: String,
    /// Last modified timestamp
    pub modified: SystemTime,
    /// Detected media type
    pub media_type: MediaType,
    /// Whether this appears to be a sample file
    pub is_sample: bool,
}

/// Type of media content detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    /// Movie file
    Movie,
    /// TV show episode
    TvShow,
    /// Unknown/ambiguous content
    Unknown,
}

/// Video file extensions supported for scanning
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp", "3g2", "mxf",
    "roq", "nsv", "f4v", "f4p", "f4a", "f4b",
];

/// Sample file indicators in filename
const SAMPLE_INDICATORS: &[&str] = &["sample", "trailer", "preview", "rarbg", "proof"];

/// File scanner for discovering media files
pub struct FileScanner {
    config: ScanConfig,
}

impl FileScanner {
    /// Create a new file scanner with the given configuration
    pub fn new(config: ScanConfig) -> Self {
        Self { config }
    }

    /// Create a file scanner with default configuration
    pub fn default() -> Self {
        Self::new(ScanConfig::default())
    }

    /// Scan a directory for media files
    pub async fn scan_directory(&self, path: &Path) -> Result<Vec<DetectedFile>, RadarrError> {
        info!("Starting scan of directory: {}", path.display());

        if !path.exists() {
            return Err(RadarrError::ValidationError {
                field: "path".to_string(),
                message: format!("Directory does not exist: {}", path.display()),
            });
        }

        if !path.is_dir() {
            return Err(RadarrError::ValidationError {
                field: "path".to_string(),
                message: format!("Path is not a directory: {}", path.display()),
            });
        }

        let mut detected_files = Vec::new();
        self.scan_recursive(path, 0, &mut detected_files).await?;

        info!("Scan complete. Found {} media files", detected_files.len());
        Ok(detected_files)
    }

    /// Recursively scan directories up to max_depth
    fn scan_recursive<'a>(
        &'a self,
        path: &'a Path,
        current_depth: u8,
        detected_files: &'a mut Vec<DetectedFile>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RadarrError>> + 'a>> {
        Box::pin(async move {
            if current_depth >= self.config.max_depth {
                debug!(
                    "Reached max depth {} at {}",
                    self.config.max_depth,
                    path.display()
                );
                return Ok(());
            }

            let mut entries = match fs::read_dir(path).await {
                Ok(entries) => entries,
                Err(e) => {
                    warn!("Failed to read directory {}: {}", path.display(), e);
                    return Ok(()); // Continue scanning other directories
                }
            };

            while let Some(entry) =
                entries
                    .next_entry()
                    .await
                    .map_err(|e| RadarrError::ExternalServiceError {
                        service: "filesystem".to_string(),
                        error: e.to_string(),
                    })?
            {
                let entry_path = entry.path();

                if entry_path.is_dir() {
                    // Recursively scan subdirectories
                    self.scan_recursive(&entry_path, current_depth + 1, detected_files)
                        .await?;
                } else if entry_path.is_file() {
                    // Check if this is a video file
                    if let Some(detected) = self.analyze_file(&entry_path).await? {
                        detected_files.push(detected);
                    }
                }
            }

            Ok(())
        })
    }

    /// Analyze a single file to see if it's a valid media file
    async fn analyze_file(&self, path: &Path) -> Result<Option<DetectedFile>, RadarrError> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .unwrap_or_default();

        // Check if it's a video file
        if !VIDEO_EXTENSIONS.contains(&extension.as_str()) {
            return Ok(None);
        }

        let metadata = match fs::metadata(path).await {
            Ok(metadata) => metadata,
            Err(e) => {
                warn!("Failed to get metadata for {}: {}", path.display(), e);
                return Ok(None);
            }
        };

        let size = metadata.len();
        let modified = metadata
            .modified()
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "filesystem".to_string(),
                error: e.to_string(),
            })?;

        // Check file size constraints
        if size < self.config.min_file_size {
            debug!("File too small: {} ({} bytes)", path.display(), size);
            return Ok(None);
        }

        if self.config.max_file_size > 0 && size > self.config.max_file_size {
            debug!("File too large: {} ({} bytes)", path.display(), size);
            return Ok(None);
        }

        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Detect if it's a sample file
        let is_sample = SAMPLE_INDICATORS
            .iter()
            .any(|indicator| filename.contains(indicator));

        // Basic media type detection (refined in file_analyzer)
        let media_type = self.detect_media_type(&filename);

        debug!(
            "Detected media file: {} ({} bytes, {})",
            path.display(),
            size,
            extension
        );

        Ok(Some(DetectedFile {
            path: path.to_path_buf(),
            size,
            extension,
            modified,
            media_type,
            is_sample,
        }))
    }

    /// Basic media type detection based on filename patterns
    fn detect_media_type(&self, filename: &str) -> MediaType {
        // Look for TV show patterns (S01E01, 1x01, etc.)
        if filename.contains("s01e")
            || filename.contains("s1e")
            || filename.contains("1x01")
            || filename.contains("episode")
        {
            return MediaType::TvShow;
        }

        // Look for movie year patterns
        if filename.contains("2024")
            || filename.contains("2023")
            || filename.contains("2022")
            || filename.contains("2021")
            || filename.contains("2020")
            || filename.contains("1080p")
            || filename.contains("2160p")
            || filename.contains("720p")
        {
            return MediaType::Movie;
        }

        MediaType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = FileScanner::default();

        let files = scanner.scan_directory(temp_dir.path()).await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_scan_with_video_files() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = FileScanner::default();

        // Create test video files
        let video_path = temp_dir.path().join("test_movie_2023.mkv");
        fs::write(&video_path, vec![0u8; 200 * 1024 * 1024])
            .await
            .unwrap(); // 200MB

        let sample_path = temp_dir.path().join("sample.mp4");
        fs::write(&sample_path, vec![0u8; 150 * 1024 * 1024])
            .await
            .unwrap(); // 150MB

        let files = scanner.scan_directory(temp_dir.path()).await.unwrap();
        assert_eq!(files.len(), 2);

        let movie_file = files.iter().find(|f| f.path == video_path).unwrap();
        assert_eq!(movie_file.media_type, MediaType::Movie);
        assert!(!movie_file.is_sample);

        let sample_file = files.iter().find(|f| f.path == sample_path).unwrap();
        assert!(sample_file.is_sample);
    }

    #[tokio::test]
    async fn test_file_size_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let config = ScanConfig {
            min_file_size: 500 * 1024 * 1024, // 500MB minimum
            ..Default::default()
        };
        let scanner = FileScanner::new(config);

        // Create small file that should be filtered out
        let small_file = temp_dir.path().join("small.mkv");
        fs::write(&small_file, vec![0u8; 100 * 1024 * 1024])
            .await
            .unwrap(); // 100MB

        let files = scanner.scan_directory(temp_dir.path()).await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_nonexistent_directory() {
        let scanner = FileScanner::default();
        let result = scanner.scan_directory(Path::new("/nonexistent/path")).await;
        assert!(result.is_err());
    }
}
