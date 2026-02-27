use std::fs;
use std::path::Path;
use std::env;

fn main() {
    let src_dir = Path::new("jswasm");
    let out_dir = env::var("OUT_DIR").unwrap();
    
    // Copia para OUT_DIR
    let dest_out = Path::new(&out_dir).join("jswasm");
    fs::create_dir_all(&dest_out).unwrap();
    
    for entry in fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), dest_out.join(entry.file_name())).unwrap();
    }
    
    // EXPORTA O CAMINHO!
    println!("cargo:rustc-env=SQLITE_WASM_OUT_DIR={}", out_dir);
    println!("cargo:rerun-if-changed=jswasm/");
}

// commandos para compilar o projeto
// cargo build --target wasm32-unknown-unknown
// wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/debug/sqlite_wasm.wasm
// node webserver.js
