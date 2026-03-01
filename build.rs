// build.rs
use std::fs;
use std::path::Path;
use std::env;
use std::process::Command;

fn main() {
    copy_sqlite_files();
    
    if env::var("PROFILE").unwrap() == "release" {
        minify_all_js();
    }
}

fn copy_sqlite_files() {
    let src_dir = Path::new("sqlite-wasm");
    let out_dir = env::var("OUT_DIR").unwrap();
    
    let dest_out = Path::new(&out_dir).join("sqlite-wasm");
    fs::create_dir_all(&dest_out).unwrap();
    
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let _ = fs::copy(entry.path(), dest_out.join(entry.file_name()));
    }
    
    let dest_pkg = Path::new("pkg").join("sqlite-wasm");
    if fs::create_dir_all(&dest_pkg).is_ok() {
        for entry in fs::read_dir(src_dir).unwrap() {
            let entry = entry.unwrap();
            let _ = fs::copy(entry.path(), dest_pkg.join(entry.file_name()));
        }
    }
    
    println!("cargo:rerun-if-changed=sqlite-wasm/");
}

fn minify_all_js() {
    println!("🔧 Minifying JavaScript files...");
    
    let pkg_dir = Path::new("pkg");
    
    // 1. MINIFICA O GLUE CODE (sqlite_wasm.js)
    let root_js = pkg_dir.join("sqlite_wasm.js");
    if root_js.exists() {
        println!("  📄 Minifying: sqlite_wasm.js");
        minify_file(&root_js, "sqlite_wasm.js");
    }
    
    // 2. MINIFICA OS ARQUIVOS DO SQLITE ENGINE
    let sqlite_dir = pkg_dir.join("sqlite-wasm");
    if sqlite_dir.exists() {
        let sqlite_files = [
            "sqlite3.js",
            "sqlite3-worker1.js", 
            "sqlite3-opfs-async-proxy.js"
        ];
        
        for &file in &sqlite_files {
            let file_path = sqlite_dir.join(file);
            if file_path.exists() {
                println!("  📄 Minifying: sqlite-wasm/{}", file);
                minify_file(&file_path, file);
            } else {
                println!("cargo:warning=⚠ File not found: sqlite-wasm/{}", file);
            }
        }
    }
}

fn minify_file(path: &Path, name: &str) {
    // Backup
    let backup = path.with_extension("bak");
    let _ = fs::copy(path, &backup);
    
    let output = Command::new("minhtml")
        .arg("--minify-js")
        .arg(path.to_str().unwrap())
        .output();
    
    match output {
        Ok(out) if out.status.success() => {
            if let Err(e) = fs::write(path, out.stdout) {
                println!("cargo:warning=❌ Failed to write {}: {}", name, e);
                let _ = fs::rename(&backup, path);
            } else {
                let _ = fs::remove_file(&backup);
                
                // Mostra economia
                if let Ok(metadata) = fs::metadata(path) {
                    let size = metadata.len();
                    println!("    ✅ Minified: {} ({} bytes)", name, size);
                }
            }
        }
        Ok(out) => {
            let _ = fs::rename(&backup, path);
            let err = String::from_utf8_lossy(&out.stderr);
            println!("cargo:warning=❌ Failed to minify {}: {}", name, err);
        }
        Err(e) => {
            let _ = fs::rename(&backup, path);
            println!("cargo:warning=❌ Error minifying {}: {}", name, e);
            println!("cargo:warning=   Tip: Install minhtml with: cargo install minhtml");
        }
    }
}
