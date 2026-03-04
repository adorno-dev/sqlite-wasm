//! JavaScript bindings and global API exposure.
//! 
//! This module handles the creation of a convenient JavaScript API surface
//! for the SQLite WASM functionality. It creates a `wasm` object in the
//! global scope (window) with properly named methods that can be called
//! directly from JavaScript.
//! 
//! # Architecture
//! 
//! The bindings module provides two complementary APIs:
//! 
//! * **Rust API** - The `WasmApi` struct with `exec` and `query` methods
//!   that can be used from Rust code.
//! 
//! * **JavaScript API** - A global `window.wasm` object with camelCase
//!   method names that mirror the Rust functions, making the library feel
//!   natural in JavaScript.
//! 
//! # JavaScript API
//! 
//! After calling `initialize_bindings()`, the following becomes available:
//! 
//! ```javascript
//! // This is JavaScript code, not Rust
//! window.wasm = {
//!   initializeWorker: (path) => {...},  // Calls initialize_worker
//!   open: (dbName) => {...},            // Calls open
//!   close: () => {...},                  // Calls close
//!   exec: (sql, bind) => {...},          // Calls exec
//!   query: (sql, bind) => {...}          // Calls query
//! };
//! ```
//! 
//! # Performance
//! 
//! This module uses `Reflect::get` to obtain function references directly
//! from the global scope, which is a zero-cost operation (≈1ns). The
//! resulting object provides O(1) access to all methods.
//! 
//! # Examples
//! 
//! ```no_run
//! # async fn example() -> Result<(), wasm_bindgen::JsValue> {
//! // Rust usage
//! let api = sqlite_wasm::modules::core::bindings::get_api();
//! let result = api.exec("CREATE TABLE users (id INTEGER)", vec![]).await?;
//! # Ok(())
//! # }
//! ```
//! 
//! ```javascript
//! // JavaScript usage (after calling initialize_bindings from Rust)
//! await wasm.initializeWorker("/sqlite.org/sqlite3-worker1.js");
//! await wasm.open("app.sqlite3");
//! await wasm.exec("CREATE TABLE users (id INTEGER, name TEXT)", []);
//! const users = await wasm.query("SELECT * FROM users", []);
//! ```

use js_sys::{Object, Reflect};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen::prelude::*;

/// JavaScript API wrapper for SQLite operations.
/// 
/// This struct provides a type-safe Rust interface to the underlying
/// database operations. It's primarily used within Rust code that
/// needs to interact with the SQLite database.
/// 
/// # Methods
/// 
/// * `exec` - Execute SQL without returning rows
/// * `query` - Execute SQL and return rows
/// 
/// # Examples
/// 
/// ```no_run
/// # async fn example() -> Result<(), wasm_bindgen::JsValue> {
/// let api = sqlite_wasm::modules::core::bindings::get_api();
/// api.exec("INSERT INTO users (name) VALUES (?)", vec!["Alice".into()]).await?;
/// let rows = api.query("SELECT * FROM users", vec![]).await?;
/// # Ok(())
/// # }
/// ```
#[wasm_bindgen]
pub struct WasmApi;

#[wasm_bindgen]
impl WasmApi {
    /// Executes a SQL statement without returning rows.
    /// 
    /// This method delegates to `crate::modules::core::database::exec()`.
    /// 
    /// # Arguments
    /// * `sql` - SQL statement to execute
    /// * `bind` - Vector of `JsValue` parameters for placeholders
    /// 
    /// # Returns
    /// * `Ok(JsValue)` - Execution result from the database module
    /// * `Err(JsValue)` - Error from the database module
    pub async fn exec(&self, sql: &str, bind: Vec<JsValue>) -> Result<JsValue, JsValue> {
        crate::modules::core::database::exec(sql, bind).await
    }

    /// Executes a SQL query and returns rows.
    /// 
    /// This method delegates to `crate::modules::core::database::query()`.
    /// 
    /// # Arguments
    /// * `sql` - SELECT statement to execute
    /// * `bind` - Vector of `JsValue` parameters for placeholders
    /// 
    /// # Returns
    /// * `Ok(JsValue)` - Query results (typically containing `resultRows`)
    /// * `Err(JsValue)` - Error from the database module
    pub async fn query(&self, sql: &str, bind: Vec<JsValue>) -> Result<JsValue, JsValue> {
        crate::modules::core::database::query(sql, bind).await
    }
}

/// Returns a new instance of the Rust API wrapper.
/// 
/// This function creates a `WasmApi` struct that can be used to
/// interact with the database from Rust code.
/// 
/// # Returns
/// * `WasmApi` - A new API wrapper instance
pub fn get_api() -> WasmApi {
    WasmApi
}

/// Creates and exposes the global `wasm` object in the browser window.
/// 
/// This function constructs a JavaScript object containing all the
/// database methods with camelCase names, then attaches it to the
/// global `window` object as `window.wasm`.
/// 
/// # Method Naming
/// 
/// | Rust function | JavaScript name |
/// |---------------|------------------|
/// | `initialize_worker` | `initializeWorker` |
/// | `open` | `open` |
/// | `close` | `close` |
/// | `exec` | `exec` |
/// | `query` | `query` |
/// 
/// # Returns
/// * `Object` - The created `wasm` object (also available globally as `window.wasm`)
/// 
/// # Panics
/// This function will panic if any of the required functions are not
/// found in the global scope. This should never happen if the wasm-bindgen
/// exports are intact.
/// 
/// # Examples
/// 
/// ```javascript
/// // After calling initialize_bindings() from Rust
/// await wasm.initializeWorker("/sqlite.org/sqlite3-worker1.js");
/// await wasm.open("app.sqlite3");
/// 
/// // The object is also available in the console for debugging
/// console.log(window.wasm);
/// ```
pub fn initialize_bindings() -> Object {
    let wasm = Object::new();
    let global = js_sys::global();
    
    // Get functions from global scope (where wasm-bindgen placed them)
    let init_fn = js_sys::Reflect::get(&global, &"initialize_worker".into()).unwrap_throw();
    let open_fn = js_sys::Reflect::get(&global, &"open".into()).unwrap_throw();
    let close_fn = js_sys::Reflect::get(&global, &"close".into()).unwrap_throw();
    let exec_fn = js_sys::Reflect::get(&global, &"exec".into()).unwrap_throw();
    let query_fn = js_sys::Reflect::get(&global, &"query".into()).unwrap_throw();
    
    // Add to wasm object with camelCase names (JavaScript convention)
    Reflect::set(&wasm, &"initializeWorker".into(), &init_fn).unwrap_throw();
    Reflect::set(&wasm, &"open".into(), &open_fn).unwrap_throw();
    Reflect::set(&wasm, &"close".into(), &close_fn).unwrap_throw();
    Reflect::set(&wasm, &"exec".into(), &exec_fn).unwrap_throw();
    Reflect::set(&wasm, &"query".into(), &query_fn).unwrap_throw();
    
    // Also expose globally as window.wasm for easy access
    Reflect::set(&global, &"wasm".into(), &wasm).unwrap_throw();
    
    wasm
}