//! Rename engine for generating organized file names
//!
//! This module provides functionality to rename imported media files
//! according to configurable templates and naming conventions.

use crate::file_analyzer::AnalyzedFile;
use once_cell::sync::Lazy;
use radarr_core::RadarrError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Configuration for file renaming operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameConfig {
    /// Template for movie file names
    pub movie_template: String,
    /// Template for movie folder names
    pub folder_template: String,
    /// Whether to replace existing files
    pub replace_existing: bool,
    /// Characters to replace in filenames
    pub invalid_chars: HashMap<char, String>,
    /// Maximum filename length
    pub max_filename_length: usize,
    /// Whether to create year-based folders
    pub year_folders: bool,
}

impl Default for RenameConfig {
    fn default() -> Self {
        let mut invalid_chars = HashMap::new();
        invalid_chars.insert('<', "".to_string());
        invalid_chars.insert('>', "".to_string());
        invalid_chars.insert(':', " -".to_string());
        invalid_chars.insert('"', "'".to_string());
        invalid_chars.insert('|', " -".to_string());
        invalid_chars.insert('?', "".to_string());
        invalid_chars.insert('*', "".to_string());
        invalid_chars.insert('/', " -".to_string());
        invalid_chars.insert('\\', " -".to_string());

        Self {
            movie_template: "{title} ({year}) [{quality}] - {release_group}".to_string(),
            folder_template: "{title} ({year})".to_string(),
            replace_existing: false,
            invalid_chars,
            max_filename_length: 255,
            year_folders: true,
        }
    }
}

/// Result of a rename operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameResult {
    /// Original file path
    pub original_path: PathBuf,
    /// New file path (what it should be renamed to)
    pub new_path: PathBuf,
    /// Whether the rename was executed or just planned
    pub executed: bool,
    /// Whether the file already existed at destination
    pub file_existed: bool,
    /// Generated folder path for organization
    pub folder_path: PathBuf,
}

/// Template variables available for renaming
#[derive(Debug, Clone)]
pub struct TemplateVariables {
    pub title: String,
    pub year: String,
    pub quality: String,
    pub codec: String,
    pub source: String,
    pub release_group: String,
    pub resolution: String,
    pub audio: String,
    pub extension: String,
}

/// Regular expressions for template parsing
static TEMPLATE_VAR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{([^}]+)\}").unwrap());

/// File rename engine
pub struct RenameEngine {
    config: RenameConfig,
}

impl RenameEngine {
    /// Create a new rename engine with the given configuration
    pub fn new(config: RenameConfig) -> Self {
        Self { config }
    }

    /// Create a rename engine with default configuration
    pub fn default() -> Self {
        Self::new(RenameConfig::default())
    }

    /// Generate a new filename based on the analyzed file
    pub fn generate_filename(
        &self,
        analyzed_file: &AnalyzedFile,
        base_path: &Path,
    ) -> Result<RenameResult, RadarrError> {
        debug!("Generating filename for: {}", analyzed_file.path.display());

        // Extract template variables from analyzed file
        let variables = self.extract_template_variables(analyzed_file)?;

        // Generate the folder name
        let folder_name = self.apply_template(&self.config.folder_template, &variables)?;
        let folder_name = self.sanitize_filename(&folder_name)?;

        // Generate full folder path
        let mut folder_path = base_path.to_path_buf();

        // Add year-based subfolder if enabled
        if self.config.year_folders && !variables.year.is_empty() {
            folder_path.push(&variables.year);
        }

        folder_path.push(folder_name);

        // Generate the new filename
        let new_filename = self.apply_template(&self.config.movie_template, &variables)?;
        let new_filename = self.sanitize_filename(&new_filename)?;

        // Add file extension
        let final_filename = format!("{}.{}", new_filename, variables.extension);
        let new_path = folder_path.join(final_filename);

        // Check if file already exists
        let file_existed = new_path.exists();

        debug!(
            "Generated path: {} -> {}",
            analyzed_file.path.display(),
            new_path.display()
        );

        Ok(RenameResult {
            original_path: analyzed_file.path.clone(),
            new_path,
            executed: false, // Just planning by default
            file_existed,
            folder_path,
        })
    }

    /// Execute the rename operation
    pub async fn execute_rename(
        &self,
        rename_result: &mut RenameResult,
    ) -> Result<(), RadarrError> {
        if rename_result.executed {
            return Ok(());
        }

        // Check if destination already exists and we're not replacing
        if rename_result.file_existed && !self.config.replace_existing {
            return Err(RadarrError::ValidationError {
                field: "destination".to_string(),
                message: format!("File already exists: {}", rename_result.new_path.display()),
            });
        }

        // Create destination directory
        if let Some(parent_dir) = rename_result.new_path.parent() {
            tokio::fs::create_dir_all(parent_dir).await.map_err(|e| {
                RadarrError::ExternalServiceError {
                    service: "filesystem".to_string(),
                    error: format!("Failed to create directory: {}", e),
                }
            })?;
        }

        // Perform the rename
        tokio::fs::rename(&rename_result.original_path, &rename_result.new_path)
            .await
            .map_err(|e| RadarrError::ExternalServiceError {
                service: "filesystem".to_string(),
                error: format!("Failed to rename file: {}", e),
            })?;

        rename_result.executed = true;
        debug!(
            "Successfully renamed file: {}",
            rename_result.new_path.display()
        );

        Ok(())
    }

    /// Process multiple files for renaming
    pub fn plan_batch_rename(
        &self,
        analyzed_files: &[AnalyzedFile],
        base_path: &Path,
    ) -> Result<Vec<RenameResult>, RadarrError> {
        let mut results = Vec::with_capacity(analyzed_files.len());

        for analyzed_file in analyzed_files {
            match self.generate_filename(analyzed_file, base_path) {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!(
                        "Failed to generate filename for {}: {}",
                        analyzed_file.path.display(),
                        e
                    );
                    // Continue with other files
                }
            }
        }

        Ok(results)
    }

    /// Execute batch rename operations
    pub async fn execute_batch_rename(
        &self,
        rename_results: &mut [RenameResult],
    ) -> Result<usize, RadarrError> {
        let mut success_count = 0;

        for result in rename_results.iter_mut() {
            match self.execute_rename(result).await {
                Ok(()) => success_count += 1,
                Err(e) => {
                    warn!("Failed to rename {}: {}", result.original_path.display(), e);
                    // Continue with other files
                }
            }
        }

        Ok(success_count)
    }

    /// Extract template variables from analyzed file
    fn extract_template_variables(
        &self,
        analyzed_file: &AnalyzedFile,
    ) -> Result<TemplateVariables, RadarrError> {
        let title = analyzed_file
            .title
            .clone()
            .unwrap_or_else(|| "Unknown Movie".to_string());
        let year = analyzed_file
            .year
            .map(|y| y.to_string())
            .unwrap_or_default();

        let quality = self.format_quality_string(&analyzed_file.quality)?;
        let codec = analyzed_file.quality.codec.clone().unwrap_or_default();
        let source = analyzed_file.quality.source.clone().unwrap_or_default();
        let resolution = analyzed_file.quality.resolution.clone().unwrap_or_default();
        let audio = analyzed_file.quality.audio.clone().unwrap_or_default();

        let release_group = analyzed_file
            .release_group
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());

        let extension = analyzed_file
            .path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("mkv")
            .to_string();

        Ok(TemplateVariables {
            title,
            year,
            quality,
            codec,
            source,
            release_group,
            resolution,
            audio,
            extension,
        })
    }

    /// Format quality information into a readable string
    fn format_quality_string(
        &self,
        quality: &crate::file_analyzer::QualityInfo,
    ) -> Result<String, RadarrError> {
        let mut parts = Vec::new();

        if let Some(ref resolution) = quality.resolution {
            parts.push(resolution.clone());
        }

        if let Some(ref source) = quality.source {
            parts.push(source.clone());
        }

        if let Some(ref codec) = quality.codec {
            parts.push(codec.clone());
        }

        if let Some(ref hdr) = quality.hdr {
            parts.push(hdr.clone());
        }

        if parts.is_empty() {
            Ok("Unknown Quality".to_string())
        } else {
            Ok(parts.join(" "))
        }
    }

    /// Apply template variables to a template string
    fn apply_template(
        &self,
        template: &str,
        variables: &TemplateVariables,
    ) -> Result<String, RadarrError> {
        let mut result = template.to_string();

        // Replace all template variables
        result = TEMPLATE_VAR_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = &caps[1];
                match var_name {
                    "title" => &variables.title,
                    "year" => &variables.year,
                    "quality" => &variables.quality,
                    "codec" => &variables.codec,
                    "source" => &variables.source,
                    "release_group" => &variables.release_group,
                    "resolution" => &variables.resolution,
                    "audio" => &variables.audio,
                    "extension" => &variables.extension,
                    _ => {
                        warn!("Unknown template variable: {}", var_name);
                        ""
                    }
                }
            })
            .to_string();

        // Clean up extra spaces and brackets with empty content
        result = result
            .replace("[]", "")
            .replace("()", "")
            .replace("  ", " ")
            .trim()
            .to_string();

        Ok(result)
    }

    /// Sanitize filename by replacing invalid characters
    fn sanitize_filename(&self, filename: &str) -> Result<String, RadarrError> {
        let mut sanitized = filename.to_string();

        // Replace invalid characters
        for (invalid_char, replacement) in &self.config.invalid_chars {
            sanitized = sanitized.replace(*invalid_char, replacement);
        }

        // Remove leading/trailing dots and spaces (problematic on Windows)
        sanitized = sanitized.trim_matches(|c| c == '.' || c == ' ').to_string();

        // Ensure filename isn't too long
        if sanitized.len() > self.config.max_filename_length {
            let truncate_at = self.config.max_filename_length.saturating_sub(3);
            sanitized = format!("{}...", &sanitized[..truncate_at]);
        }

        // Ensure filename isn't empty
        if sanitized.is_empty() {
            sanitized = "Unknown".to_string();
        }

        Ok(sanitized)
    }

    /// Preview what a filename would look like without executing
    pub fn preview_rename(
        &self,
        analyzed_file: &AnalyzedFile,
        base_path: &Path,
    ) -> Result<String, RadarrError> {
        let result = self.generate_filename(analyzed_file, base_path)?;
        Ok(result.new_path.to_string_lossy().to_string())
    }

    /// Validate that a template is correctly formatted
    pub fn validate_template(&self, template: &str) -> Result<(), RadarrError> {
        // Check for unmatched braces
        let mut brace_count = 0;
        for ch in template.chars() {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count < 0 {
                        return Err(RadarrError::ValidationError {
                            field: "template".to_string(),
                            message: "Unmatched closing brace".to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        if brace_count != 0 {
            return Err(RadarrError::ValidationError {
                field: "template".to_string(),
                message: "Unmatched opening brace".to_string(),
            });
        }

        // Check for valid variable names
        for caps in TEMPLATE_VAR_REGEX.captures_iter(template) {
            let var_name = &caps[1];
            if !matches!(
                var_name,
                "title"
                    | "year"
                    | "quality"
                    | "codec"
                    | "source"
                    | "release_group"
                    | "resolution"
                    | "audio"
                    | "extension"
            ) {
                warn!("Unknown template variable: {}", var_name);
            }
        }

        Ok(())
    }
}

impl Default for RenameEngine {
    fn default() -> Self {
        Self::new(RenameConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_analyzer::{AnalyzedFile, QualityInfo};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_analyzed_file() -> AnalyzedFile {
        AnalyzedFile {
            path: PathBuf::from("/downloads/The.Matrix.1999.1080p.BluRay.x264.DTS-GROUP.mkv"),
            title: Some("The Matrix".to_string()),
            year: Some(1999),
            quality: QualityInfo {
                resolution: Some("1080P".to_string()),
                codec: Some("X264".to_string()),
                audio: Some("DTS".to_string()),
                source: Some("BLURAY".to_string()),
                hdr: None,
            },
            release_group: Some("GROUP".to_string()),
            is_sample: false,
            confidence: 0.9,
            original_filename: "The.Matrix.1999.1080p.BluRay.x264.DTS-GROUP".to_string(),
        }
    }

    #[test]
    fn test_generate_filename() {
        let engine = RenameEngine::default();
        let analyzed_file = create_test_analyzed_file();
        let base_path = Path::new("/movies");

        let result = engine.generate_filename(&analyzed_file, base_path).unwrap();

        assert!(result.new_path.to_string_lossy().contains("The Matrix"));
        assert!(result.new_path.to_string_lossy().contains("1999"));
        assert!(result.folder_path.to_string_lossy().contains("1999")); // Year folder
        assert!(result
            .folder_path
            .to_string_lossy()
            .contains("The Matrix (1999)"));
    }

    #[test]
    fn test_sanitize_filename() {
        let engine = RenameEngine::default();

        let result = engine
            .sanitize_filename("Movie: The Sequel <2023>")
            .unwrap();
        assert_eq!(result, "Movie - The Sequel 2023");

        let result = engine.sanitize_filename("...Movie...").unwrap();
        assert_eq!(result, "Movie");

        let result = engine.sanitize_filename("").unwrap();
        assert_eq!(result, "Unknown");
    }

    #[test]
    fn test_template_validation() {
        let engine = RenameEngine::default();

        assert!(engine.validate_template("{title} ({year})").is_ok());
        assert!(engine.validate_template("{title} {year").is_err()); // Unmatched brace
        assert!(engine.validate_template("title} ({year})").is_err()); // Unmatched brace
    }

    #[test]
    fn test_apply_template() {
        let engine = RenameEngine::default();
        let analyzed_file = create_test_analyzed_file();
        let variables = engine.extract_template_variables(&analyzed_file).unwrap();

        let result = engine
            .apply_template("{title} ({year}) [{quality}]", &variables)
            .unwrap();
        assert!(result.contains("The Matrix"));
        assert!(result.contains("1999"));
        assert!(result.contains("1080P"));
    }

    #[test]
    fn test_format_quality_string() {
        let engine = RenameEngine::default();

        let quality = QualityInfo {
            resolution: Some("1080P".to_string()),
            codec: Some("X264".to_string()),
            source: Some("BLURAY".to_string()),
            hdr: Some("HDR10".to_string()),
            audio: None,
        };

        let result = engine.format_quality_string(&quality).unwrap();
        assert!(result.contains("1080P"));
        assert!(result.contains("BLURAY"));
        assert!(result.contains("X264"));
        assert!(result.contains("HDR10"));
    }

    #[tokio::test]
    async fn test_plan_batch_rename() {
        let engine = RenameEngine::default();
        let analyzed_files = vec![create_test_analyzed_file()];
        let temp_dir = TempDir::new().unwrap();

        let results = engine
            .plan_batch_rename(&analyzed_files, temp_dir.path())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].executed);
    }

    #[test]
    fn test_preview_rename() {
        let engine = RenameEngine::default();
        let analyzed_file = create_test_analyzed_file();
        let temp_dir = TempDir::new().unwrap();

        let preview = engine
            .preview_rename(&analyzed_file, temp_dir.path())
            .unwrap();
        assert!(preview.contains("The Matrix"));
        assert!(preview.contains("1999"));
    }
}
