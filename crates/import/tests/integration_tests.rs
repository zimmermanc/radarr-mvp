//! Integration tests for the import pipeline

use radarr_import::{ImportPipeline, ImportConfig};
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_complete_import_workflow() {
    // Setup test directories
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("downloads");
    let dest_dir = temp_dir.path().join("movies");
    
    fs::create_dir_all(&source_dir).await.unwrap();
    fs::create_dir_all(&dest_dir).await.unwrap();
    
    // Create test movie files with realistic names
    let movie_files = vec![
        "The.Matrix.1999.1080p.BluRay.x264.DTS-GROUP.mkv",
        "Inception.2010.2160p.UHD.BluRay.x265.HDR.Atmos-SCENE.mkv",
        "sample-movie.mkv", // This should be skipped
        "Blade.Runner.2049.2017.720p.WEB-DL.DD5.1.H264-FGT.mp4",
    ];
    
    for filename in &movie_files {
        let file_path = source_dir.join(filename);
        // Create files with sufficient size (200MB minimum by default)
        fs::write(&file_path, vec![0u8; 200 * 1024 * 1024]).await.unwrap();
    }
    
    // Configure import pipeline
    let mut config = ImportConfig::default();
    config.dry_run = false;
    config.min_confidence = 0.2; // Lower threshold for testing
    config.skip_samples = true;
    config.scan_config.min_file_size = 100 * 1024 * 1024; // 100MB minimum
    
    // Use copy fallback for testing (hardlinks might fail in test environment)
    config.hardlink_config.enable_hardlinks = false;
    config.hardlink_config.copy_fallback = true;
    
    let pipeline = ImportPipeline::new(config);
    
    // Execute the import
    let stats = pipeline.import_directory(&source_dir, &dest_dir).await.unwrap();
    
    // Verify results
    assert_eq!(stats.files_scanned, 4); // All files were scanned
    assert_eq!(stats.skipped_files, 1); // Sample file was skipped
    assert!(stats.successful_imports >= 2); // At least 2 good movies imported
    assert!(stats.files_copied > 0); // Files were copied (not hardlinked in test)
    
    // Check that files were properly organized
    let matrix_path = dest_dir.join("1999").join("The Matrix (1999)");
    assert!(matrix_path.exists());
    
    let inception_path = dest_dir.join("2010").join("Inception (2010)");
    assert!(inception_path.exists());
    
    println!("Import completed successfully!");
    println!("Files scanned: {}", stats.files_scanned);
    println!("Successful imports: {}", stats.successful_imports);
    println!("Files copied: {}", stats.files_copied);
    println!("Total size: {} bytes", stats.total_size);
}

#[tokio::test]
async fn test_single_file_import() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("The.Godfather.1972.1080p.BluRay.x264-CLASSIC.mkv");
    let dest_dir = temp_dir.path().join("movies");
    
    fs::create_dir_all(&dest_dir).await.unwrap();
    fs::write(&source_file, vec![0u8; 200 * 1024 * 1024]).await.unwrap();
    
    let mut config = ImportConfig::default();
    config.dry_run = false;
    config.hardlink_config.enable_hardlinks = false;
    config.hardlink_config.copy_fallback = true;
    
    let pipeline = ImportPipeline::new(config);
    
    let result = pipeline.import_file(&source_file, &dest_dir).await.unwrap();
    
    assert!(result.success);
    assert!(result.analyzed_file.is_some());
    assert!(result.hardlink_result.is_some());
    assert!(result.rename_result.is_some());
    
    let analyzed = result.analyzed_file.unwrap();
    assert_eq!(analyzed.title, Some("The Godfather".to_string()));
    assert_eq!(analyzed.year, Some(1972));
    
    // Check that the file was organized properly
    let expected_path = dest_dir.join("1972").join("The Godfather (1972)");
    assert!(expected_path.exists());
}

#[tokio::test]
async fn test_dry_run_mode() {
    let temp_dir = TempDir::new().unwrap();
    let source_dir = temp_dir.path().join("downloads");
    let dest_dir = temp_dir.path().join("movies");
    
    fs::create_dir_all(&source_dir).await.unwrap();
    fs::create_dir_all(&dest_dir).await.unwrap();
    
    let movie_file = source_dir.join("Casablanca.1942.1080p.BluRay.x264-CRITERION.mkv");
    fs::write(&movie_file, vec![0u8; 200 * 1024 * 1024]).await.unwrap();
    
    let mut config = ImportConfig::default();
    config.dry_run = true; // Don't actually move files
    config.min_confidence = 0.1;
    
    let pipeline = ImportPipeline::new(config);
    
    let stats = pipeline.import_directory(&source_dir, &dest_dir).await.unwrap();
    
    // In dry run mode, files should be analyzed but not moved
    assert_eq!(stats.files_scanned, 1);
    assert_eq!(stats.successful_imports, 1);
    assert_eq!(stats.files_copied, 0); // No actual copies in dry run
    
    // Original file should still exist in source
    assert!(movie_file.exists());
    
    // No files should have been created in destination
    let mut dest_entries = fs::read_dir(&dest_dir).await.unwrap();
    let dest_contents = dest_entries.next_entry().await.unwrap();
    assert!(dest_contents.is_none());
}

#[tokio::test]
async fn test_configuration_validation() {
    // Test invalid confidence threshold
    let mut config = ImportConfig::default();
    config.min_confidence = 1.5; // Invalid (> 1.0)
    
    let pipeline = ImportPipeline::new(config);
    assert!(pipeline.validate_config().is_err());
    
    // Test invalid max_parallel
    let mut config = ImportConfig::default();
    config.max_parallel = 0;
    
    let pipeline = ImportPipeline::new(config);
    assert!(pipeline.validate_config().is_err());
    
    // Test valid configuration
    let config = ImportConfig::default();
    let pipeline = ImportPipeline::new(config);
    assert!(pipeline.validate_config().is_ok());
}

#[tokio::test]
async fn test_custom_naming_template() {
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("Alien.1979.DIRECTORS.CUT.1080p.BluRay.x264-HDMANIACS.mkv");
    let dest_dir = temp_dir.path().join("movies");
    
    fs::create_dir_all(&dest_dir).await.unwrap();
    fs::write(&source_file, vec![0u8; 200 * 1024 * 1024]).await.unwrap();
    
    let mut config = ImportConfig::default();
    config.dry_run = true; // Just test the naming
    config.rename_config.movie_template = "{title}.{year}.{quality}".to_string();
    config.rename_config.folder_template = "{title}".to_string();
    config.rename_config.year_folders = false;
    
    let pipeline = ImportPipeline::new(config);
    
    let result = pipeline.import_file(&source_file, &dest_dir).await.unwrap();
    
    assert!(result.success);
    let rename_result = result.rename_result.unwrap();
    
    // Check that custom template was applied
    let filename = rename_result.new_path.file_name().unwrap().to_str().unwrap();
    assert!(filename.contains("Alien.1979"));
    assert!(filename.contains("1080P"));
    
    // Check folder structure (no year folder)
    let folder_name = rename_result.folder_path.file_name().unwrap().to_str().unwrap();
    assert_eq!(folder_name, "Alien");
}