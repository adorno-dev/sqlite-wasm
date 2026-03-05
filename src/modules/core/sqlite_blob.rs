use wasm_bindgen::prelude::*;
use js_sys::{Array, Uint8Array};
use web_sys::{Blob, Url};

const SQLITE_JS: &str =
    include_str!("../../../static/sqlite.org/sqlite3.js");

const SQLITE_WORKER: &str =
    include_str!("../../../static/sqlite.org/sqlite3-worker1.js");

const SQLITE_OPFS: &str =
    include_str!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");

const SQLITE_WASM: &[u8] =
    include_bytes!("../../../static/sqlite.org/sqlite3.wasm");

use web_sys::BlobPropertyBag;

fn blob_url(code: &str) -> Result<String, JsValue> {
    let parts = Array::new();
    parts.push(&JsValue::from_str(code));

    let mut opts = BlobPropertyBag::new();
    opts.set_type("text/javascript");

    let blob = Blob::new_with_str_sequence_and_options(&parts, &opts)?;

    Url::create_object_url_with_blob(&blob)
}

fn blob_url_bytes(bytes: &[u8]) -> Result<String, JsValue> {
    let arr = Uint8Array::from(bytes);

    let parts = Array::new();
    parts.push(&arr);

    let mut opts = BlobPropertyBag::new();
    opts.set_type("application/wasm");

    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &opts)?;

    Url::create_object_url_with_blob(&blob)
}

/// Retorna um blob URL para sqlite3-worker1.js com todos os
/// assets do SQLite embutidos.
///
/// Esse valor pode ser passado diretamente para `autostart(worker_path)`
#[wasm_bindgen]
pub fn sqlite_worker_path() -> Result<String, JsValue> {
    let sqlite_js_url = blob_url(SQLITE_JS)?;
    let opfs_js_url = blob_url(SQLITE_OPFS)?;
    let wasm_url = blob_url_bytes(SQLITE_WASM)?;
    let worker_url = blob_url(SQLITE_WORKER)?;

    let bootstrap = format!(
        r#"
self.SQLITE_JS_URL = "{sqlite_js}";
self.SQLITE_WASM_URL = "{wasm}";
self.SQLITE_OPFS_PROXY_URL = "{opfs}";

import("{worker}");
"#,
        sqlite_js = sqlite_js_url,
        wasm = wasm_url,
        opfs = opfs_js_url,
        worker = worker_url
    );

    blob_url(&bootstrap)
}
