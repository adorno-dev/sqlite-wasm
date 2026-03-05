use wasm_bindgen::prelude::*;
use js_sys::{Array, Uint8Array};
use web_sys::{Blob, Url, BlobPropertyBag};

const SQLITE_JS: &str = include_str!("../../../static/sqlite.org/sqlite3.js");
const SQLITE_WORKER: &str = include_str!("../../../static/sqlite.org/sqlite3-worker1.js");
const SQLITE_OPFS: &str = include_str!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");
const SQLITE_WASM: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.wasm");

fn blob_js(code: &str) -> Result<String, JsValue> {
    let arr = Array::new();
    arr.push(&JsValue::from_str(code));

    let mut opts = BlobPropertyBag::new();
    opts.type_("text/javascript"); // corrige temporary value dropped

    let blob = Blob::new_with_str_sequence_and_options(&arr, &opts)?;
    Url::create_object_url_with_blob(&blob)
}

fn blob_wasm(bytes: &[u8]) -> Result<String, JsValue> {
    let arr = Array::new();
    arr.push(&Uint8Array::from(bytes));

    let mut opts = BlobPropertyBag::new();
    opts.type_("application/wasm");

    let blob = Blob::new_with_u8_array_sequence_and_options(&arr, &opts)?;
    Url::create_object_url_with_blob(&blob)
}

/// Retorna um Blob URL pronto para `autostart()`
/// que funciona mesmo com OPFS.
#[wasm_bindgen]
pub fn sqlite_worker_path() -> Result<String, JsValue> {
    // cria blobs para os arquivos
    let sqlite_js_url = blob_js(SQLITE_JS)?;
    let wasm_url = blob_wasm(SQLITE_WASM)?;
    
    // patch opfs proxy para carregar os blobs corretos
    let patched_opfs = SQLITE_OPFS
        .replace("sqlite3.js", &sqlite_js_url)
        .replace("sqlite3.wasm", &wasm_url);
    let opfs_blob = blob_js(&patched_opfs)?;

    // patch worker1 para apontar para o opfs proxy blob e sqlite js
    let patched_worker = SQLITE_WORKER
        .replace("sqlite3.js", &sqlite_js_url)
        .replace("sqlite3-opfs-async-proxy.js", &opfs_blob);
    let worker_blob = blob_js(&patched_worker)?;

    // bootstrap worker: define locateFile e importa worker patchado
    let bootstrap = format!(
        r#"
self.sqlite3InitModule = {{
    locateFile: () => "{}"
}};
self.OPFS_PROXY_URL = "{}";

importScripts("{}");
"#,
        wasm_url,
        opfs_blob,
        worker_blob
    );

    blob_js(&bootstrap)
}
