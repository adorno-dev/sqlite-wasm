mod bindings;
mod database;
mod worker;

pub use bindings::{initialize_bindings, WasmApi};
pub use database::{close, db_id, exec, is_open, open, query};
pub use worker::{initialize_worker, wait_for_worker};

use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

/// One-stop initialization: creates worker, waits for ready, opens database, exposes bindings
#[wasm_bindgen(js_name = "autostart")]
pub async fn autostart(worker_path: &str, database_name: &str) -> Result<WasmApi, JsValue> {
    worker::initialize_worker(worker_path).await?;
    worker::wait_for_worker().await?;
    database::open(database_name).await?;
    bindings::initialize_bindings();
    Ok(bindings::get_api())
}
