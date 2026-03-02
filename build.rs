// build.rs

//! Build script for sqlite-wasm crate.
//! 
//! This build script handles copying of SQLite native files and optional
//! minification of JavaScript assets in release builds.
//! 
//! # Functions
//! 
//! * Copies SQLite files from `sqlite.org/` to:
//!   - `OUT_DIR` for compilation
//!   - `pkg/sqlite.org/` for distribution
//! 
//! * In release mode, minifies JavaScript files using `minhtml`:
//!   - The main glue code (`sqlite_wasm.js`)
//!   - SQLite engine files (`sqlite3.js`, worker files, proxy)
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

/// Main build script entry point.
fn main() {
    copy_sqlite_files().expect("Failed to copy SQLite files");
    
    if env::var("PROFILE").unwrap() == "release" {
        minify_all_js().expect("Failed to minify JavaScript files");
    }
}

/// Copies SQLite native files to both OUT_DIR and pkg directory.
fn copy_sqlite_files() -> Result<(), Box<dyn std::error::Error>> {
    let src_dir = Path::new("sqlite.org");
    let out_dir = env::var("OUT_DIR")?;
    
    // Copy to OUT_DIR
    let dest_out = Path::new(&out_dir).join("sqlite.org");
    fs::create_dir_all(&dest_out)?;
    copy_directory(src_dir, &dest_out)?;
    
    // Copy to pkg directory (if it exists/will exist)
    let dest_pkg = Path::new("pkg").join("sqlite.org");
    if fs::create_dir_all(&dest_pkg).is_ok() {
        copy_directory(src_dir, &dest_pkg)?;
    }
    
    println!("cargo:rerun-if-changed=sqlite.org/");
    Ok(())
}

/// Recursively copies a directory.
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

/// Minifies JavaScript files in the `pkg` directory.
fn minify_all_js() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Minifying JavaScript files...");
    
    let pkg_dir = Path::new("pkg");
    
    // Minify glue code
    let root_js = pkg_dir.join("sqlite_wasm.js");
    if root_js.exists() {
        println!("  📄 Minifying: sqlite_wasm.js");
        minify_file(&root_js)?;
    }
    
    // Minify SQLite engine files
    let sqlite_dir = pkg_dir.join("sqlite.org");
    if sqlite_dir.exists() {
        minify_sqlite_files(&sqlite_dir)?;
    }
    
    Ok(())
}

/// Minifies SQLite engine files in the specified directory.
fn minify_sqlite_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let sqlite_files = [
        "sqlite3.js",
        "sqlite3-worker1.js", 
        "sqlite3-opfs-async-proxy.js"
    ];
    
    for &file in &sqlite_files {
        let file_path = dir.join(file);
        if file_path.exists() {
            println!("  📄 Minifying: sqlite.org/{}", file);
            
            if let Err(e) = minify_file(&file_path) {
                eprintln!("cargo:warning=⚠ Failed to minify {}: {}", file, e);
            }
        } else {
            eprintln!("cargo:warning=⚠ File not found: sqlite.org/{}", file);
        }
    }
    
    Ok(())
}

/// Minifies a single JavaScript file using `minhtml`.
fn minify_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Create backup
    let backup = path.with_extension("js.bak");
    fs::copy(path, &backup)?;
    
    // Run minifier
    let output = Command::new("minhtml")
        .arg("--minify-js")
        .arg(path.to_str().unwrap())
        .output()
        .map_err(|e| format!("Failed to execute minhtml: {}. Install with: cargo install minhtml", e))?;
    
    if !output.status.success() {
        // Restore backup on failure
        fs::rename(&backup, path)?;
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Minification failed: {}", error_msg).into());
    }
    
    // Write minified content
    fs::write(path, output.stdout)?;
    
    // Remove backup on success
    let _ = fs::remove_file(&backup);
    
    // Show size reduction
    let size = fs::metadata(path)?.len();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    println!("    ✅ Minified: {} ({} bytes)", file_name, size);
    
    Ok(())
}
