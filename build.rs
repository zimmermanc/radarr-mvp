use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Only build web assets in release builds or when explicitly requested
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let build_web = env::var("BUILD_WEB").is_ok() || profile == "release" || profile == "dist";
    
    if build_web {
        println!("cargo:rerun-if-changed=web/");
        println!("cargo:rerun-if-changed=web/src/");
        println!("cargo:rerun-if-changed=web/package.json");
        println!("cargo:rerun-if-changed=web/package-lock.json");
        
        let web_dir = Path::new("web");
        
        if web_dir.exists() {
            println!("cargo:warning=Building web UI assets...");
            
            // Run the web build script
            let output = Command::new("./build-web.sh")
                .status()
                .expect("Failed to execute build-web.sh");
                
            if !output.success() {
                panic!("Web build failed!");
            }
            
            // Check if dist directory exists
            let dist_dir = web_dir.join("dist");
            if !dist_dir.exists() {
                panic!("Web build succeeded but dist directory not found!");
            }
            
            println!("cargo:warning=Web UI build completed successfully");
        } else {
            println!("cargo:warning=Web directory not found, skipping web build");
        }
    } else {
        println!("cargo:warning=Skipping web build (debug mode)");
    }
}