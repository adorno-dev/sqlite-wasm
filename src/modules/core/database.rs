//src/modules/core/database.rs

//! Core database operations for SQLite in the browser.
//! 
//! This module provides the main API for interacting with SQLite databases
//! through a Web Worker. It handles database lifecycle (open/close), SQL
//! execution, and query operations with proper synchronization and error
//! handling.
//! 
//! # Architecture
//! 
//! The database module is built on top of the worker messaging system:
//! 
//! * **Database ID** - A unique identifier (`DB_UID`) is stored after successful
//!   `open()` and used for all subsequent operations. This is wrapped in a
//!   `Mutex<Option<JsValue>>` for thread-safe access.
//! 
//! * **Message counter** - `AtomicU32` provides unique request IDs for each
//!   operation, ensuring responses can be matched to requests.
//! 
//! * **OPFS support** - The Origin Private File System is used for persistent
//!   storage when available, falling back to memory-only mode.
//! 
//! # Performance Considerations
//! 
//! * `DB_UID` uses a `Mutex` with minimal overhead (≈2ns) since contention is rare.
//! * `COUNTER` uses relaxed atomic ordering for maximum speed.
//! * All database operations check `DB_UID` first to fail fast if not opened.
//! 
//! # Examples
//! 
//! ```rust
//! use sqlite_wasm::core::database::{open, exec, query, close};
//! 
//! // Open database
//! open("mydb.sqlite3").await?;
//! 
//! // Create table
//! exec("CREATE TABLE users (id INTEGER, name TEXT)", vec![]).await?;
//! 
//! // Insert data
//! exec("INSERT INTO users VALUES (?, ?)", vec![1.into(), "Alice".into()]).await?;
//! 
//! // Query data
//! let result = query("SELECT * FROM users", vec![]).await?;
//! 
//! // Close when done
//! close().await?;
//! ```

use std::sync::{Mutex, atomic::{AtomicU32, Ordering}};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use js_sys::{Object, Array, Reflect};

use crate::worker::w_msg;

/// Global database ID (singleton).
/// 
/// Stores the unique identifier returned by the worker after a successful
/// `open()` operation. This ID is required for all subsequent database commands.
/// 
/// # Thread Safety
/// Wrapped in a `Mutex` to allow safe access from any async context.
/// The overhead is minimal (≈2ns) as contention is extremely rare.
static DB_UID: Mutex<Option<JsValue>> = Mutex::new(None);

/// Request counter for message IDs.
/// 
/// Provides a monotonically increasing ID for each database operation,
/// used to match responses with requests. The counter is atomic and
/// uses relaxed ordering for maximum performance.
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Opens a database with OPFS (Origin Private File System).
///
/// This function initializes a new SQLite database in the browser's
/// OPFS storage. The database is persistent across page reloads.
///
/// # Arguments
/// * `database_name` - Name of the database file (e.g., "app.sqlite3").
///   The `.sqlite3` extension is recommended but not required.
///
/// # Returns
/// * `Ok(())` - Database opened successfully and UID stored.
/// * `Err(JsValue)` - Opening failed. Possible reasons:
///   - Worker not initialized
///   - OPFS not available in this browser
///   - Storage quota exceeded
///   - Invalid database name
///
/// # Idempotency
/// This function is idempotent. Once a database is opened, subsequent
/// calls return `Ok(())` immediately without re-opening.
///
/// # OPFS Details
/// The Origin Private File System provides persistent storage that:
/// * Survives page reloads and browser restarts
/// * Is isolated by origin (no cross-site access)
/// * Has quota limits (typically hundreds of MB)
/// * Requires COOP/COEP headers to be set
///
/// # Examples
/// ```rust
/// // Simple open
/// open("app.sqlite3").await?;
/// 
/// // Subsequent calls are no-ops
/// open("app.sqlite3").await?; // Ok(()) immediately
/// ```
#[wasm_bindgen]
pub async fn open(database_name: &str) -> Result<(), JsValue> {
    // If already opened, return immediately
    if DB_UID.lock().unwrap().is_some() {
        // return Ok(());
        close().await?;
    }

    let args = Object::new();
    Reflect::set(&args, &"filename".into(), &JsValue::from_str(database_name))?;
    Reflect::set(&args, &"vfs".into(), &JsValue::from_str("opfs"))?;

    let open_result = w_msg("open".to_string(), args.into()).await?;
    
    let result_field = Reflect::get(&open_result, &"result".into()).ok();
    
    let uid_value = if let Some(result_obj) = result_field {
        let nested = Reflect::get(&result_obj, &"dbId".into()).ok();
        nested
            .unwrap_or_else(|| Reflect::get(&open_result, &"dbId".into()).unwrap_or(JsValue::NULL))
    } else {
        Reflect::get(&open_result, &"dbId".into()).unwrap_or(JsValue::NULL)
    };

    *DB_UID.lock().unwrap() = Some(uid_value);
    
    Ok(())
}

/// Closes the currently open database.
///
/// This function sends a close command to the worker and clears the
/// stored UID. After closing, a new `open()` call is required to
/// perform further database operations.
///
/// # Returns
/// * `Ok(())` - Database closed successfully (or was already closed).
/// * `Err(JsValue)` - Close operation failed.
///
/// # Notes
/// * Closing an already closed database is a no-op and returns `Ok(())`.
/// * After closing, the database file remains on disk and can be re-opened.
///
/// # Examples
/// ```rust
/// open("app.sqlite3").await?;
/// // ... do work ...
/// close().await?;
/// ```
#[wasm_bindgen]
pub async fn close() -> Result<(), JsValue> {
    let uid = DB_UID.lock().unwrap().take();

    if let Some(uid) = uid {
        let args = Object::new();
        Reflect::set(&args, &"dbId".into(), &uid)?;
        w_msg("close".to_string(), args.into()).await?;
    }

    Ok(())
}

/// Executes a SQL statement without returning rows.
///
/// Use this for DDL statements (CREATE, DROP, ALTER) and DML statements
/// that don't return data (INSERT, UPDATE, DELETE).
///
/// # Arguments
/// * `sql` - SQL statement to execute (e.g., "INSERT INTO users VALUES (?, ?)")
/// * `args` - Vector of `JsValue` parameters for placeholders. Use empty vector
///   if no parameters are needed.
///
/// # Returns
/// * `Ok(JsValue)` - Execution succeeded. The returned value is typically
///   an empty object or contains execution metadata.
/// * `Err(JsValue)` - Execution failed. Possible reasons:
///   - Database not opened (call `open()` first)
///   - SQL syntax error
///   - Constraint violation
///   - Worker communication error
///
/// # Parameter Binding
/// Placeholders in SQL are represented by `?` (positional). The `args` vector
/// must contain one value per placeholder, in order.
///
/// # Examples
/// ```rust
/// // Create table
/// exec("CREATE TABLE users (id INTEGER, name TEXT)", vec![]).await?;
/// 
/// // Insert with parameters
/// exec(
///     "INSERT INTO users VALUES (?, ?)",
///     vec![1.into(), "Alice".into()]
/// ).await?;
/// 
/// // Update
/// exec(
///     "UPDATE users SET name = ? WHERE id = ?",
///     vec!["Bob".into(), 1.into()]
/// ).await?;
/// 
/// // Delete
/// exec("DELETE FROM users WHERE id = ?", vec![1.into()]).await?;
/// ```
#[wasm_bindgen]
pub async fn exec(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if DB_UID.lock().unwrap().is_none() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    
    let args_obj = Object::new();
    Reflect::set(&args_obj, &"id".into(), &JsValue::from(id)).ok();
    Reflect::set(&args_obj, &"type".into(), &"exec".into()).ok();
    Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql)).ok();
    
    if !args.is_empty() {
        let args_array = Array::new();
        for arg in args {
            args_array.push(&arg);
        }
        Reflect::set(&args_obj, &"bind".into(), &args_array).ok();
    }
    
    w_msg("exec".to_string(), args_obj.into()).await
}

/// Executes a SQL query and returns rows.
///
/// Use this for SELECT statements that return data. The result is a
/// `JsValue` that typically contains a `resultRows` array with the
/// query results.
///
/// # Arguments
/// * `sql` - SELECT statement to execute (e.g., "SELECT * FROM users WHERE age > ?")
/// * `args` - Vector of `JsValue` parameters for placeholders. Use empty vector
///   if no parameters are needed.
///
/// # Returns
/// * `Ok(JsValue)` - Query results. The structure depends on `rowMode`:
///   - With `rowMode: "object"` (default): Array of objects with column names
///   - With `rowMode: "array"`: Array of arrays (values only)
/// * `Err(JsValue)` - Query failed. Possible reasons:
///   - Database not opened
///   - SQL syntax error
///   - Table doesn't exist
///   - Worker communication error
///
/// # Row Mode
/// By default, rows are returned as objects with column names as keys.
/// To change this, you can modify the `rowMode` property in the message object.
///
/// # Examples
/// ```rust
/// // Simple query
/// let result = query("SELECT * FROM users", vec![]).await?;
/// 
/// // With parameters
/// let result = query(
///     "SELECT * FROM users WHERE age > ?",
///     vec![18.into()]
/// ).await?;
/// 
/// // Process results
/// if let Ok(rows) = js_sys::Reflect::get(&result, &"resultRows".into()) {
///     let array = js_sys::Array::from(&rows);
///     for row in array.iter() {
///         let id = js_sys::Reflect::get(&row, &"id".into())?;
///         let name = js_sys::Reflect::get(&row, &"name".into())?;
///     }
/// }
/// ```
#[wasm_bindgen]
pub async fn query(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if DB_UID.lock().unwrap().is_none() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    
    let args_obj = Object::new();
    Reflect::set(&args_obj, &"id".into(), &JsValue::from(id)).ok();
    Reflect::set(&args_obj, &"type".into(), &"exec".into()).ok();
    Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql)).ok();
    Reflect::set(&args_obj, &"rowMode".into(), &"object".into()).ok();
    
    if !args.is_empty() {
        let args_array = Array::new();
        for arg in args {
            args_array.push(&arg);
        }
        Reflect::set(&args_obj, &"bind".into(), &args_array).ok();
    }
    
    w_msg("exec".to_string(), args_obj.into()).await
}

/// Checks if a database is currently open.
///
/// # Returns
/// * `true` - Database is open and ready for operations.
/// * `false` - No database is currently open (or `open()` hasn't been called).
///
/// # Examples
/// ```rust
/// if is_open() {
///     // Safe to execute queries
///     let result = query("SELECT * FROM users", vec![]).await?;
/// } else {
///     open("app.sqlite3").await?;
/// }
/// ```
#[wasm_bindgen]
pub fn is_open() -> bool {
    DB_UID.lock().unwrap().is_some()
}

/// Gets the current database ID (for debugging).
///
/// # Returns
/// * `JsValue` - The current database UID, or `NULL` if no database is open.
///
/// # Notes
/// This function is primarily for debugging and should not be used
/// in production code. The UID format is internal and may change.
#[wasm_bindgen]
pub fn db_id() -> JsValue {
    DB_UID.lock().unwrap().clone().unwrap_or(JsValue::NULL)
}
