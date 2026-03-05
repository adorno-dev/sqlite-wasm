use wasm_bindgen::prelude::*;
use js_sys::{Array, Uint8Array, Object};
use web_sys::{Blob, Url, BlobPropertyBag};

// Embeds dos arquivos SQLite
const SQLITE_JS: &str = include_str!("../../../static/sqlite.org/sqlite3.js");
const SQLITE_WORKER: &str = include_str!("../../../static/sqlite.org/sqlite3-worker1.js");
const SQLITE_OPFS: &str = include_str!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");
const SQLITE_WASM: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.wasm");

// Cria blob JS
fn blob_js(code: &str) -> Result<String, JsValue> {
    let arr = Array::new();
    arr.push(&JsValue::from_str(code));
    let mut opts = BlobPropertyBag::new();
    opts.type_("text/javascript");
    let blob = Blob::new_with_str_sequence_and_options(&arr, &opts)?;
    Url::create_object_url_with_blob(&blob)
}

// Cria blob WASM
fn blob_wasm(bytes: &[u8]) -> Result<String, JsValue> {
    let arr = Array::new();
    arr.push(&Uint8Array::from(bytes));
    let mut opts = BlobPropertyBag::new();
    opts.type_("application/wasm");
    let blob = Blob::new_with_u8_array_sequence_and_options(&arr, &opts)?;
    Url::create_object_url_with_blob(&blob)
}

/// Retorna um blob URL pronto para `autostart()`
/// Funciona com OPFS, workers, e não precisa de replace()
#[wasm_bindgen]
pub fn sqlite_worker_path() -> Result<String, JsValue> {
    // Cria blobs para todos os arquivos
    let sqlite_js_url = blob_js(SQLITE_JS)?;
    let wasm_url = blob_wasm(SQLITE_WASM)?;
    let opfs_blob = blob_js(&format!(
        "const SQLITE_WASM_URL = '{}';\n{}",
        wasm_url, SQLITE_OPFS
    ))?;
    let worker_blob = blob_js(&format!(
        "const OPFS_PROXY_URL = '{}';\nconst SQLITE_JS_URL = '{}';\n{}",
        opfs_blob, sqlite_js_url, SQLITE_WORKER
    ))?;

    // Bootstrap worker que redefine importScripts para ler de blobs
    let bootstrap = format!(
        r#"
const SQLITE_WASM_URL = "{}";
const WORKER_BLOB_URL = "{}";
const OPFS_BLOB_URL = "{}";
const SQLITE_JS_BLOB_URL = "{}";

function patchImportScripts() {{
    const originalImport = self.importScripts;
    self.importScripts = function(...urls) {{
        const remap = {{
            'sqlite3.js': SQLITE_JS_BLOB_URL,
            'sqlite3-opfs-async-proxy.js': OPFS_BLOB_URL,
            'sqlite3.wasm': SQLITE_WASM_URL
        }};
        const mapped = urls.map(u => remap[u] || u);
        return originalImport.apply(self, mapped);
    }};
}}

patchImportScripts();
importScripts(WORKER_BLOB_URL);
"#,
        wasm_url, worker_blob, opfs_blob, sqlite_js_url
    );

    blob_js(&bootstrap)
}
