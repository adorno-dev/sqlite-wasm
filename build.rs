// build.rs

//! Build script for sqlite-wasm crate.
//! 
//! This build script handles copying of static assets and optional
//! minification of JavaScript assets in release builds.
//! 
//! # Functions
//! 
//! * Copies static assets from configured source paths to:
//!   - `OUT_DIR` for compilation
//!   - `pkg/` for distribution
//! 
//! Currently copies:
//!   - `static/sqlite.org/` → `pkg/static/sqlite.org/` (SQLite native files and WASM modules)
//!   - `static/coi-serviceworker/` → `pkg/static/coi-serviceworker/` (COI service worker files)
//! 
//! # Configuration
//! 
//! To add more static assets, extend the `STATIC_ASSETS` array with `(source_path, dest_name)` tuples:
//! ```rust
//! const STATIC_ASSETS: &[(&str, &str)] = &[
//!     ("static/sqlite.org", "sqlite.org"),
//!     ("static/coi-serviceworker", "coi-serviceworker"),
//!     ("node_modules/some-library/dist", "some-library"),  // Copy from node_modules
//!     ("third-party/assets", "vendor/assets"),             // Custom source paths
//! ];
//! ```
//! 
//! * In release mode, minifies JavaScript files using `minhtml`:
//!   - All JavaScript files in the copied directories
//!   - Including main glue code (`sqlite_wasm.js`)
//!   - SQLite engine files (`sqlite3.js`, worker files, proxy)
//!   - Service worker files
//! 
//! # Dependencies
//! 
//! Requires `minhtml` to be installed for minification:
//! ```bash
//! cargo install minhtml
//! ```

use std::fs;
use std::path::Path;
use std::env;
use std::process::Command;

/// List of (source_path, dest_name) tuples to copy.
/// 
/// Each entry specifies:
/// - Source path relative to project root
/// - Destination directory name under `static/` in OUT_DIR and pkg/
/// 
/// This flexible format allows copying from anywhere in the project
/// to a structured location in the output directories.
const STATIC_ASSETS: &[(&str, &str)] = &[
    ("static/sqlite-wasm", "sqlite-wasm"),
    ("static/sqlite.org", "sqlite.org"),
    // Add more assets here as needed
    // Example: ("node_modules/something/dist", "vendor/something"),
];

/// Main build script entry point.
/// 
/// # Steps
/// 1. Copies all configured static assets to OUT_DIR and pkg/
/// 2. In release mode, minifies all JavaScript files in pkg/static/
fn main() {
    for (src, dest_name) in STATIC_ASSETS {
        copy_asset(src, dest_name)
            .unwrap_or_else(|e| panic!("Failed to copy asset '{}' to '{}': {}", src, dest_name, e));
    }
    
    if env::var("PROFILE").unwrap() == "release" {
        minify_all_js().expect("Failed to minify JavaScript files");
    }
}

/// Copies a static asset from source path to both OUT_DIR and pkg directory.
/// 
/// # Arguments
/// * `src_path` - Source path relative to project root (e.g., "static/sqlite.org")
/// * `dest_name` - Destination directory name under `static/` (e.g., "sqlite.org")
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error details
/// 
/// # Behavior
/// - Source path can be anywhere in the project (not limited to static/)
/// - Copies to `{OUT_DIR}/static/{dest_name}`
/// - Copies to `pkg/static/{dest_name}` if pkg directory exists/can be created
/// - Adds cargo rerun trigger for the source path
/// - Skips gracefully if source doesn't exist (with warning)
fn copy_asset(src_path: &str, dest_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let src_dir = Path::new(src_path);
    
    if !src_dir.exists() {
        println!("cargo:warning=Source path '{}' does not exist, skipping", src_dir.display());
        return Ok(());
    }
    
    let out_dir = env::var("OUT_DIR")?;
    
    // Copy to OUT_DIR
    let dest_out = Path::new(&out_dir).join("static").join(dest_name);
    fs::create_dir_all(&dest_out)?;
    copy_directory(src_dir, &dest_out)?;
    
    // Copy to pkg directory (if it exists/will exist)
    let dest_pkg = Path::new("dist").join("static").join(dest_name);
    if fs::create_dir_all(&dest_pkg).is_ok() {
        copy_directory(src_dir, &dest_pkg)?;
    }
    
    println!("cargo:rerun-if-changed={}", src_path);
    Ok(())
}

/// Recursively copies a directory and all its contents.
/// 
/// # Arguments
/// * `src` - Source directory path
/// * `dst` - Destination directory path
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error details
/// 
/// # Note
/// Creates destination directory if it doesn't exist and preserves
/// directory structure.
fn copy_directory(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_file() {
            fs::copy(&src_path, &dst_path)?;
        } else if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_directory(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Minifies all JavaScript files in the pkg/static directory recursively.
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error details
/// 
/// # Note
/// Only runs in release mode and if pkg/static directory exists.
fn minify_all_js() -> Result<(), Box<dyn std::error::Error>> {
    let pkg_static = Path::new("pkg").join("static");
    
    if pkg_static.exists() {
        minify_directory(&pkg_static)?;
    }
    
    Ok(())
}

/// Recursively minifies JavaScript files in a directory.
/// 
/// # Arguments
/// * `dir` - Directory path to scan for JavaScript files
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error details
/// 
/// # Note
/// Only processes files with `.js` extension.
fn minify_directory(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "js" {
                    minify_file(&path)?;
                }
            }
        } else if path.is_dir() {
            minify_directory(&path)?;
        }
    }
    Ok(())
}

/// Minifies a single JavaScript file using minhtml.
/// 
/// # Arguments
/// * `path` - Path to the JavaScript file to minify
/// 
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error details
/// 
/// # Dependencies
/// Requires `minhtml` to be installed and available in PATH.
/// 
/// # Panics
/// Returns error if minhtml command fails or is not found.
fn minify_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("minhtml")
        .arg(path)
        .arg("--output")
        .arg(path)
        .status()?;
    
    if !status.success() {
        return Err(format!("minhtml failed on file: {}", path.display()).into());
    }
    
    Ok(())
}
