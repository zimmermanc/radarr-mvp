# Radarr Import Pipeline

Comprehensive media import pipeline for the Radarr movie automation system. This crate provides sophisticated file scanning, analysis, hardlink management, and renaming capabilities to organize downloaded media files into your library structure.

## Features

- **File Scanner**: Recursive discovery of media files with configurable filters
- **File Analyzer**: Intelligent quality detection and metadata extraction from filenames
- **Hardlink Manager**: Safe hardlink creation with fallback to copying
- **Rename Engine**: Flexible filename generation based on configurable templates
- **Import Pipeline**: Orchestrated workflow combining all import stages
- **Integration Service**: High-level service for database integration
- **Error Recovery**: Comprehensive error handling with rollback capabilities
- **Progress Tracking**: Real-time progress reporting for long-running operations
- **Validation**: File integrity checks and duplicate detection

## Key Dependencies

- **tokio**: Async runtime with filesystem and process support
- **futures**: Stream processing for parallel operations
- **regex**: Pattern matching for filename parsing and validation
- **chrono**: Date/time handling for file timestamps
- **once_cell**: Static initialization for compiled regex patterns
- **which**: External tool detection for media analysis
- **uuid**: Unique identifiers for import operations
- **serde_json**: Configuration serialization

## File Scanner

### Basic File Discovery

```rust
use radarr_import::{FileScanner, ScanConfig, DetectedFile, MediaType};
use std::path::Path;

// Configure scanner
let config = ScanConfig {
    extensions: vec!["mkv", "mp4", "avi", "m4v"],
    min_size_bytes: 50 * 1024 * 1024, // 50MB minimum
    max_depth: Some(3),                // Limit recursion depth
    follow_symlinks: false,
    exclude_patterns: vec![r"\.partial$", r"sample"],
};

let scanner = FileScanner::new(config);

// Scan directory
let scan_path = Path::new("/downloads/movies");
let detected_files = scanner.scan_directory(scan_path).await?;

for file in detected_files {
    println!("Found: {} ({})", 
        file.path.display(), 
        file.size_bytes / 1_024_000
    );
    
    match file.media_type {
        MediaType::Movie => println!("  Type: Movie"),
        MediaType::Subtitle => println!("  Type: Subtitle"),
        MediaType::Extra => println!("  Type: Extra"),
        MediaType::Unknown => println!("  Type: Unknown"),
    }
}
```

### Advanced Filtering

```rust
use radarr_import::{ScanConfig, FileFilter};

let config = ScanConfig {
    extensions: vec!["mkv", "mp4", "avi"],
    min_size_bytes: 100 * 1024 * 1024, // 100MB minimum
    filters: vec![
        FileFilter::ExcludePattern(r"(?i)sample".to_string()),
        FileFilter::ExcludePattern(r"(?i)trailer".to_string()),
        FileFilter::RequirePattern(r"(?i)\.(mkv|mp4|avi)$".to_string()),
    ],
    parallel_scan: true,
    max_concurrent: 10,
    ..Default::default()
};
```

## File Analyzer

### Quality Detection and Metadata Extraction

```rust
use radarr_import::{FileAnalyzer, AnalyzedFile, QualityInfo};

let analyzer = FileAnalyzer::new();

// Analyze file from path
let file_path = Path::new("/downloads/The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
let analyzed = analyzer.analyze_file(file_path).await?;

println!("Title: {}", analyzed.parsed_title);
println!("Year: {:?}", analyzed.year);
println!("Quality: {:?}", analyzed.quality);
println!("Source: {:?}", analyzed.source);
println!("Codec: {:?}", analyzed.codec);
println!("Resolution: {:?}", analyzed.resolution);

// Quality information details
match analyzed.quality {
    Some(quality_info) => {
        println!("Quality Score: {}", quality_info.score);
        println!("Source Confidence: {:.2}", quality_info.source_confidence);
        println!("Resolution Confidence: {:.2}", quality_info.resolution_confidence);
    }
    None => println!("Quality detection failed"),
}
```

### Batch Analysis

```rust
use futures::StreamExt;

// Analyze multiple files concurrently
let analyzer = FileAnalyzer::new();
let file_paths = vec![file1, file2, file3];

let analyzed_stream = analyzer.analyze_batch(file_paths).await?;
let analyzed_files: Vec<AnalyzedFile> = analyzed_stream.collect().await;

for analyzed in analyzed_files {
    match analyzed {
        Ok(file) => println!("Analyzed: {}", file.original_path.display()),
        Err(e) => println!("Analysis failed: {}", e),
    }
}
```

## Hardlink Manager

### Safe File Operations with Hardlinks

```rust
use radarr_import::{HardlinkManager, HardlinkConfig, HardlinkResult};

let config = HardlinkConfig {
    prefer_hardlinks: true,
    verify_after_operation: true,
    preserve_permissions: true,
    create_parent_dirs: true,
    overwrite_existing: false,
    backup_existing: true,
};

let manager = HardlinkManager::new(config);

// Create hardlink or copy file
let source = Path::new("/downloads/movie.mkv");
let destination = Path::new("/movies/The Matrix (1999)/The Matrix (1999) 1080p BluRay.mkv");

match manager.hardlink_or_copy(source, destination).await? {
    HardlinkResult::Hardlinked { saved_bytes } => {
        println!("Hardlinked successfully, saved {} bytes", saved_bytes);
    }
    HardlinkResult::Copied { bytes_copied } => {
        println!("Copied {} bytes (hardlink not possible)", bytes_copied);
    }
    HardlinkResult::AlreadyExists => {
        println!("Destination already exists");
    }
}
```

### Batch Operations with Progress

```rust
use radarr_import::HardlinkStats;

let operations = vec![
    (source1, dest1),
    (source2, dest2),
    (source3, dest3),
];

let mut progress_rx = manager.batch_hardlink_with_progress(operations).await?;

while let Some(stats) = progress_rx.recv().await {
    println!("Progress: {}/{} files, {} bytes saved", 
        stats.completed_operations,
        stats.total_operations,
        stats.total_bytes_saved
    );
}
```

## Rename Engine

### Template-Based Renaming

```rust
use radarr_import::{RenameEngine, RenameConfig, RenameResult};

let config = RenameConfig {
    movie_template: "{Movie Title} ({Release Year}) {Quality} {Source}".to_string(),
    folder_template: "{Movie Title} ({Release Year})".to_string(),
    replace_illegal_chars: true,
    max_filename_length: Some(255),
    reserved_words: vec!["CON", "PRN", "AUX"],
};

let engine = RenameEngine::new(config);

// Generate new filename
let analyzed_file = analyzer.analyze_file(file_path).await?;
let rename_result = engine.generate_filename(&analyzed_file)?;

match rename_result {
    RenameResult::Success { new_name, folder_name } => {
        println!("New filename: {}", new_name);
        println!("Folder: {}", folder_name);
    }
    RenameResult::Warning { new_name, warnings } => {
        println!("Renamed with warnings: {}", new_name);
        for warning in warnings {
            println!("  Warning: {}", warning);
        }
    }
    RenameResult::Failed { reason } => {
        println!("Rename failed: {}", reason);
    }
}
```

### Custom Templates

```rust
// Advanced template with conditional formatting
let template = r#"
{Movie Title} ({Release Year})
{if Quality != Unknown} - {Quality}{endif}
{if Source != Unknown} [{Source}]{endif}
{if Group} - {Group}{endif}
"#;

// Template variables available:
// - {Movie Title}: Parsed movie title
// - {Release Year}: Release year
// - {Quality}: Quality (1080p, 720p, etc.)
// - {Source}: Source (BluRay, WebDL, HDTV)
// - {Codec}: Video codec (x264, x265, HEVC)
// - {Audio}: Audio codec (DTS, AC3, AAC)
// - {Group}: Release group name
// - {Edition}: Edition (Director's Cut, Extended, etc.)
```

## Import Pipeline

### Complete Import Workflow

```rust
use radarr_import::{ImportPipeline, ImportConfig, ImportResult, ImportStats};

let config = ImportConfig {
    scan_config: ScanConfig::default(),
    analyzer_config: AnalyzerConfig::default(),
    hardlink_config: HardlinkConfig::default(),
    rename_config: RenameConfig::default(),
    dry_run: false,
    skip_existing: true,
    verify_imports: true,
};

let pipeline = ImportPipeline::new(config);

// Import entire directory
let source_dir = Path::new("/downloads/movies");
let destination_dir = Path::new("/movies");

let result = pipeline.import_directory(source_dir, destination_dir).await?;

println!("Import Results:");
println!("  Successful imports: {}", result.stats.successful_imports);
println!("  Failed imports: {}", result.stats.failed_imports);
println!("  Skipped files: {}", result.stats.skipped_files);
println!("  Total bytes processed: {}", result.stats.total_bytes_processed);
println!("  Bytes saved (hardlinks): {}", result.stats.bytes_saved_hardlinks);

// Individual file results
for file_result in result.file_results {
    match file_result {
        Ok(success) => {
            println!("✓ {}", success.destination_path.display());
        }
        Err(error) => {
            println!("✗ {} - {}", error.source_path.display(), error.reason);
        }
    }
}
```

### Streaming Import with Progress

```rust
use futures::StreamExt;

let mut import_stream = pipeline.import_directory_streaming(source_dir, destination_dir).await?;

while let Some(progress) = import_stream.next().await {
    match progress {
        ImportProgress::FileStarted { path } => {
            println!("Starting: {}", path.display());
        }
        ImportProgress::FileCompleted { path, result } => {
            match result {
                Ok(_) => println!("✓ Completed: {}", path.display()),
                Err(e) => println!("✗ Failed: {} - {}", path.display(), e),
            }
        }
        ImportProgress::ScanCompleted { files_found } => {
            println!("Scan completed: {} files found", files_found);
        }
        ImportProgress::ImportCompleted { stats } => {
            println!("Import completed: {}", stats);
            break;
        }
    }
}
```

## Integration Service

### High-Level Database Integration

```rust
use radarr_import::{ImportService, IntegratedImportConfig, IntegratedImportResult};
use radarr_core::{MovieRepository, Movie};

let integrated_config = IntegratedImportConfig {
    import_config: ImportConfig::default(),
    update_database: true,
    create_missing_movies: true,
    match_threshold: 0.8, // 80% confidence for auto-matching
};

let import_service = ImportService::new(
    movie_repository,
    integrated_config,
);

// Import with database integration
let result = import_service.import_and_update_database(
    source_dir,
    destination_dir,
).await?;

match result {
    IntegratedImportResult::Success { 
        imported_files, 
        updated_movies, 
        created_movies 
    } => {
        println!("Imported {} files", imported_files.len());
        println!("Updated {} movies", updated_movies.len());
        println!("Created {} new movies", created_movies.len());
    }
    IntegratedImportResult::PartialSuccess { 
        successes, 
        failures 
    } => {
        println!("Partial success: {} succeeded, {} failed", 
                 successes.len(), failures.len());
    }
    IntegratedImportResult::Failed { reason } => {
        println!("Import failed: {}", reason);
    }
}
```

## Error Handling and Recovery

### Comprehensive Error Types

```rust
use radarr_import::{ImportError, ImportErrorType};

pub enum ImportError {
    FileSystem { path: PathBuf, error: std::io::Error },
    Analysis { path: PathBuf, reason: String },
    Hardlink { source: PathBuf, dest: PathBuf, error: std::io::Error },
    Rename { original_name: String, reason: String },
    Validation { path: PathBuf, reason: String },
    Database { operation: String, error: String },
    Configuration { setting: String, reason: String },
}

// Error recovery strategies
match import_result {
    Err(ImportError::Hardlink { source, dest, .. }) => {
        // Fallback to copy operation
        println!("Hardlink failed, falling back to copy");
        manager.force_copy(&source, &dest).await?;
    }
    Err(ImportError::Analysis { path, reason }) => {
        // Manual analysis or skip
        println!("Analysis failed for {}: {}", path.display(), reason);
        // Could prompt user for manual classification
    }
    Err(e) => return Err(e),
}
```

### Rollback Capabilities

```rust
use radarr_import::ImportTransaction;

// Transactional imports with rollback
let transaction = ImportTransaction::new();

let result = transaction.execute(|| async {
    // Perform import operations
    let imported_files = pipeline.import_directory(source, dest).await?;
    
    // Update database
    for file in &imported_files {
        repository.update_movie_file(&file).await?;
    }
    
    Ok(imported_files)
}).await;

match result {
    Ok(files) => {
        transaction.commit().await?;
        println!("Import committed successfully");
    }
    Err(e) => {
        transaction.rollback().await?;
        println!("Import rolled back due to error: {}", e);
    }
}
```

## Testing

### Unit Tests

```bash
# Run import pipeline tests
cargo test -p radarr-import

# Test specific modules
cargo test -p radarr-import file_scanner::tests
cargo test -p radarr-import pipeline::tests
```

### Integration Testing with Temporary Files

```rust
use tempfile::TempDir;
use radarr_import::*;

#[tokio::test]
async fn test_complete_import_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("downloads");
    let dest_dir = temp_dir.path().join("movies");
    
    // Create test file structure
    create_test_movie_file(&source_dir, "The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv").await;
    
    let config = ImportConfig::default();
    let pipeline = ImportPipeline::new(config);
    
    let result = pipeline.import_directory(&source_dir, &dest_dir).await.unwrap();
    
    assert_eq!(result.stats.successful_imports, 1);
    assert!(dest_dir.join("The Matrix (1999)").exists());
}
```

## Configuration Examples

### Production Configuration

```json
{
  "scanConfig": {
    "extensions": ["mkv", "mp4", "avi", "m4v"],
    "minSizeBytes": 104857600,
    "excludePatterns": ["(?i)sample", "(?i)trailer", "\\.partial$"],
    "maxDepth": 5,
    "parallelScan": true,
    "maxConcurrent": 8
  },
  "hardlinkConfig": {
    "preferHardlinks": true,
    "verifyAfterOperation": true,
    "createParentDirs": true,
    "preservePermissions": true
  },
  "renameConfig": {
    "movieTemplate": "{Movie Title} ({Release Year}) - {Quality} [{Source}]",
    "folderTemplate": "{Movie Title} ({Release Year})",
    "maxFilenameLength": 250
  }
}
```

## Performance Optimization

- **Parallel Processing**: Concurrent file operations with configurable limits
- **Stream Processing**: Memory-efficient handling of large directories
- **Lazy Loading**: On-demand analysis to reduce memory usage
- **Connection Pooling**: Database connection reuse for batch operations
- **Hardlink Detection**: Intelligent hardlink vs copy decisions
- **Progress Streaming**: Real-time progress updates without blocking
- **Error Isolation**: Individual file failures don't stop batch operations