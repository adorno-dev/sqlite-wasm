//! # sqlite-wasm
//!
//! A high-performance SQLite wrapper for WebAssembly with OPFS support.
//! This crate provides a clean, type-safe API for using SQLite in the browser
//! through a dedicated Web Worker, with full support for persistent storage
//! using the Origin Private File System (OPFS).
//!
//! ## Features
//!
//! * **Zero-cost abstractions** - Worker management with `OnceLock` and atomic counters
//! * **OPFS persistence** - Databases survive page reloads and browser restarts
//! * **Async/await API** - All operations return Promises/futures
//! * **Automatic worker management** - Singleton worker with ready-state signaling
//! * **No manual sleeps** - Proper event-based waiting for initialization
//! * **JavaScript bindings** - Global `window.wasm` object with camelCase methods
//!
//! ## Architecture
//!
//! The crate is organized into three core modules:
//!
//! * **`worker`** - Web Worker lifecycle and message passing
//! * **`database`** - SQLite operations (open, exec, query, close)
//! * **`bindings`** - JavaScript API exposure and type-safe Rust wrapper
//!
//! ## Quick Start
//!
//! ```no_run
//! # async fn example() -> Result<(), wasm_bindgen::JsValue> {
//! use sqlite_wasm::{autostart, open, WasmApi};
//!
//! // One-line initialization (worker only)
//! let db: WasmApi = autostart("/sqlite.org/sqlite3-worker1.js").await?;
//!     
//! // Open specific database
//! open("myapp.sqlite3").await?;
//!     
//! // Create table
//! db.exec("CREATE TABLE users (id INTEGER, name TEXT)", vec![]).await?;
//!     
//! // Insert data
//! db.exec(
//!     "INSERT INTO users VALUES (?, ?)",
//!     vec![1.into(), "Alice".into()]
//! ).await?;
//!     
//! // Query data
//! let result = db.query("SELECT * FROM users", vec![]).await?;
//! # Ok(())
//! # }
//! ```

pub mod modules;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

use crate::modules::core::{bindings, database, worker};

pub use bindings::{WasmApi, initialize_bindings};
pub use database::{close, db_id, exec, is_open, open, query};
pub use worker::{initialize_worker, wait_for_worker};

/// One-stop initialization: creates worker, waits for ready, exposes bindings
///
/// This is the recommended way to initialize the SQLite WASM system. It performs
/// all necessary steps in the correct order:
///
/// 1. Creates the Web Worker
/// 2. Waits for the worker to be ready (receives 'worker1-ready' message)
/// 3. Exposes the global `window.wasm` object with camelCase methods
///
/// # Arguments
/// * `worker_path` - Path to the SQLite worker script.
///   Typically this points to the official SQLite worker, e.g.:
///   - `"/sqlite.org/sqlite3-worker1.js"`
///   - `"/static/sqlite3-worker1.js"`
///
/// # Returns
/// * `Ok(WasmApi)` - A type-safe Rust wrapper with `exec` and `query` methods.
///   The same API is also available globally as `window.wasm` in JavaScript.
///
/// * `Err(JsValue)` - Initialization failed at any step. The error value
///   contains details about which step failed.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> Result<(), wasm_bindgen::JsValue> {
/// use sqlite_wasm::{autostart, open, WasmApi};
///
/// // Rust usage - initialize worker
/// let db: WasmApi = autostart("/sqlite.org/sqlite3-worker1.js").await?;
///
/// // Open a database (separate step)
/// open("app.sqlite3").await?;
///
/// // Now execute queries
/// db.exec("CREATE TABLE users (id INTEGER)", vec![]).await?;
/// # Ok(())
/// # }
/// ```
///
/// ```javascript
/// // JavaScript usage (after initialization)
/// await wasm.initializeWorker("/sqlite.org/sqlite3-worker1.js");
/// await wasm.open("app.sqlite3");
/// await wasm.exec("CREATE TABLE users (id INTEGER)", []);
/// const users = await wasm.query("SELECT * FROM users", []);
/// ```
///
/// # Idempotency
/// The worker is a singleton. Subsequent calls to `autostart`
/// will return `Ok(WasmApi)` immediately after the first successful initialization.
///
/// # Performance
/// This function uses event-based waiting rather than polling or sleeps,
/// ensuring optimal performance. The overhead after initialization is zero.
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "autostart")]
pub async fn autostart(worker_path: &str) -> Result<WasmApi, wasm_bindgen::JsValue> {
    worker::initialize_worker(worker_path).await?;
    worker::wait_for_worker().await?;
    bindings::initialize_bindings();
    Ok(bindings::get_api())
}

// #[wasm_bindgen::prelude::wasm_bindgen(js_name = "autostart_embedded")]
// pub async fn autostart_embedded() -> Result<WasmApi, wasm_bindgen::JsValue> {
//     // 🔥 GARANTE QUE O RUNTIME WASM FOI INICIALIZADO
//     #[cfg(target_arch = "wasm32")]
//     wasm_bindgen_futures::spawn_local(async move {});
//     
//     let assets = crate::modules::core::blobs::EmbeddedAssets::new()?;
//     let worker_url = crate::modules::core::blobs::create_embedded_worker(&assets)?;
//
//     worker::initialize_worker(&worker_url).await?;
//     worker::wait_for_worker().await?;
//
//     bindings::initialize_bindings();
//     Ok(bindings::get_api())
// }


// #[wasm_bindgen::prelude::wasm_bindgen(js_name = "autostart_embedded")]
// pub async fn autostart_embedded() -> Result<WasmApi, wasm_bindgen::JsValue> {
//     let assets = crate::modules::core::blobs::EmbeddedAssets::new()?;
//     let worker_url = crate::modules::core::blobs::create_embedded_worker(&assets)?;
//     worker::initialize_worker(&worker_url).await?;
//     worker::wait_for_worker().await?;
//     bindings::initialize_bindings();
//     Ok(bindings::get_api())
// }

// #[wasm_bindgen::prelude::wasm_bindgen(js_name = "autostart_embedded")]
// pub async fn autostart_embedded() -> Result<WasmApi, wasm_bindgen::JsValue> {
//     let assets = crate::modules::core::blobs::EmbeddedAssets::new()?;
//     
//     // 🔥 AGORA USA AWAIT!
//     let worker_url = crate::modules::core::blobs::create_embedded_worker(&assets).await?;
//     
//     worker::initialize_worker(&worker_url).await?;
//     worker::wait_for_worker().await?;
//     bindings::initialize_bindings();
//     Ok(bindings::get_api())
// }

#[wasm_bindgen(js_name = "autostart_embedded")]
pub async fn autostart_embedded() -> Result<WasmApi, JsValue> {
    // Inicializa o worker com os assets embutidos
    worker::initialize_embedded_worker().await?;
    worker::wait_for_worker().await?;
    bindings::initialize_bindings();
    Ok(bindings::get_api())
}
