//src/lib.rs

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
//! ```rust
//! use sqlite_wasm::autostart;
//! 
//! #[wasm_bindgen]
//! pub async fn main() -> Result<(), JsValue> {
//!     // One-line initialization
//!     let db = autostart(
//!         "/sqlite.org/sqlite3-worker1.js",
//!         "myapp.sqlite3"
//!     ).await?;
//!     
//!     // Create table
//!     db.exec("CREATE TABLE users (id INTEGER, name TEXT)", vec![]).await?;
//!     
//!     // Insert data
//!     db.exec(
//!         "INSERT INTO users VALUES (?, ?)",
//!         vec![1.into(), "Alice".into()]
//!     ).await?;
//!     
//!     // Query data
//!     let result = db.query("SELECT * FROM users", vec![]).await?;
//!     Ok(())
//! }
//! ```

pub mod modules;

use crate::modules::core::{worker, database, bindings};

pub use worker::{initialize_worker, wait_for_worker};
pub use database::{close, db_id, exec, is_open, open, query};
pub use bindings::{initialize_bindings, WasmApi};

/// One-stop initialization: creates worker, waits for ready, opens database, exposes bindings
/// 
/// This is the recommended way to initialize the SQLite WASM system. It performs
/// all necessary steps in the correct order:
/// 
/// 1. Creates the Web Worker
/// 2. Waits for the worker to be ready (receives 'worker1-ready' message)
/// 3. Opens the specified database with OPFS
/// 4. Exposes the global `window.wasm` object with camelCase methods
/// 
/// # Arguments
/// * `worker_path` - Path to the SQLite worker script.
///   Typically this points to the official SQLite worker, e.g.:
///   - `"/sqlite.org/sqlite3-worker1.js"`
///   - `"/static/sqlite3-worker1.js"`
/// 
/// * `database_name` - Name of the database file (e.g., `"myapp.sqlite3"`).
///   The file will be created in OPFS if it doesn't exist.
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
/// ```rust
/// // Rust usage
/// let db = autostart("/sqlite.org/sqlite3-worker1.js", "app.sqlite3").await?;
/// db.exec("CREATE TABLE users (id INTEGER)", vec![]).await?;
/// ```
/// 
/// ```javascript
/// // JavaScript usage (after calling autostart)
/// await wasm.exec("CREATE TABLE users (id INTEGER)", []);
/// const users = await wasm.query("SELECT * FROM users", []);
/// ```
/// 
/// # Idempotency
/// The worker and database are singletons. Subsequent calls to `autostart`
/// will return `Ok(WasmApi)` immediately after the first successful initialization.
/// 
/// # Performance
/// This function uses event-based waiting rather than polling or sleeps,
/// ensuring optimal performance. The overhead after initialization is zero.
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "autostart")]
// pub async fn autostart(worker_path: &str, database_name: &str) -> Result<WasmApi, wasm_bindgen::JsValue> {
pub async fn autostart(worker_path: &str) -> Result<WasmApi, wasm_bindgen::JsValue> {
    worker::initialize_worker(worker_path).await?;
    worker::wait_for_worker().await?;
    // database::open(database_name).await?;
    bindings::initialize_bindings();
    Ok(bindings::get_api())
}
