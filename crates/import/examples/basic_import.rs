//! Basic usage example for the Radarr import pipeline
//! 
//! This example demonstrates how to set up and use the import pipeline
//! to scan, analyze, and organize media files.

use radarr_import::{ImportPipeline, ImportConfig};
use std::path::Path;
// use tracing_subscriber; // Not available in dependencies

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (tracing_subscriber not available)
    // tracing_subscriber::init();

    // Create import configuration
    let mut config = ImportConfig::default();
    
    // Configure for demonstration (dry run mode to avoid moving files)
    config.dry_run = true;
    config.min_confidence = 0.3;
    config.skip_samples = true;
    
    // Customize naming template
    config.rename_config.movie_template = "{title} ({year}) [{quality}] - {release_group}".to_string();
    config.rename_config.folder_template = "{title} ({year})".to_string();
    config.rename_config.year_folders = true;
    
    // Remember dry_run setting before moving config
    let is_dry_run = config.dry_run;
    
    // Create the import pipeline
    let pipeline = ImportPipeline::new(config);
    
    // Validate configuration
    pipeline.validate_config()?;
    
    println!("Radarr Import Pipeline Example");
    println!("==============================");
    
    // Example: Import from downloads to movies directory
    let source_dir = Path::new("/downloads/movies");
    let dest_dir = Path::new("/movies");
    
    if source_dir.exists() {
        println!("Scanning directory: {}", source_dir.display());
        
        match pipeline.import_directory(source_dir, dest_dir).await {
            Ok(stats) => {
                println!("\nImport Results:");
                println!("- Files scanned: {}", stats.files_scanned);
                println!("- Files analyzed: {}", stats.files_analyzed);
                println!("- Successful imports: {}", stats.successful_imports);
                println!("- Failed imports: {}", stats.failed_imports);
                println!("- Skipped files: {}", stats.skipped_files);
                println!("- Total size: {:.2} GB", stats.total_size as f64 / (1024.0 * 1024.0 * 1024.0));
                println!("- Hardlinks created: {}", stats.hardlinks_created);
                println!("- Files copied: {}", stats.files_copied);
                println!("- Duration: {:.2}s", stats.total_duration.as_secs_f64());
                
                if is_dry_run {
                    println!("\n(DRY RUN MODE - No files were actually moved)");
                }
            }
            Err(e) => {
                eprintln!("Import failed: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("Source directory not found: {}", source_dir.display());
        println!("This is just a demonstration - no actual import performed.");
    }
    
    // Example: Import a single file
    let single_file = Path::new("/downloads/The.Matrix.1999.1080p.BluRay.x264-GROUP.mkv");
    
    if single_file.exists() {
        println!("\nImporting single file: {}", single_file.display());
        
        match pipeline.import_file(single_file, dest_dir).await {
            Ok(result) => {
                if result.success {
                    println!("Single file import successful!");
                    if let Some(analyzed) = result.analyzed_file {
                        println!("- Title: {:?}", analyzed.title);
                        println!("- Year: {:?}", analyzed.year);
                        println!("- Quality: {:?}", analyzed.quality.resolution);
                        println!("- Release Group: {:?}", analyzed.release_group);
                        println!("- Confidence: {:.2}", analyzed.confidence);
                    }
                } else {
                    println!("Single file import failed: {:?}", result.error);
                }
            }
            Err(e) => {
                eprintln!("Single file import error: {}", e);
            }
        }
    } else {
        println!("\nSample file not found: {}", single_file.display());
        println!("This is just a demonstration.");
    }
    
    println!("\nExample completed!");
    Ok(())
}