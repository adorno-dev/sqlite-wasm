use wasm_bindgen::prelude::*;
use std::fs;
use std::path::{PathBuf};
use std::io::Write;
use std::sync::Once;

// Inclui os arquivos dentro da crate
const SQLITE_JS: &str = include_str!("../../..//static/sqlite.org/sqlite3.js");
const SQLITE_WORKER: &str = include_str!("../../../static/sqlite.org/sqlite3-worker1.js");
const SQLITE_OPFS: &str = include_str!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");
const SQLITE_WASM: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.wasm");

// Garante que a pasta temporária seja criada apenas uma vez
static INIT: Once = Once::new();

fn get_temp_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push("wasm-sqlite-assets");
    dir
}

fn write_file(path: &PathBuf, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

/// Retorna o path do worker principal pronto para `autostart()`.
/// Cria os arquivos temporários na primeira execução.
#[wasm_bindgen]
pub fn sqlite_worker_path() -> Result<String, JsValue> {
    let temp_dir = get_temp_dir();

    // Garante criação apenas uma vez
    INIT.call_once(|| {
        let _ = write_file(&temp_dir.join("sqlite3.js"), SQLITE_JS.as_bytes());
        let _ = write_file(&temp_dir.join("sqlite3-worker1.js"), SQLITE_WORKER.as_bytes());
        let _ = write_file(&temp_dir.join("sqlite3-opfs-async-proxy.js"), SQLITE_OPFS.as_bytes());
        let _ = write_file(&temp_dir.join("sqlite3.wasm"), SQLITE_WASM);
    });

    // Retorna o path do worker principal
    let worker_path = temp_dir.join("sqlite3-worker1.js");
    worker_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| JsValue::from_str("Failed to convert worker path to string"))
}
