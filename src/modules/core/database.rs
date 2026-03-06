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
//! ```no_run
//! # async fn example() -> Result<(), wasm_bindgen::JsValue> {
//! use sqlite_wasm::modules::core::database::{open, exec, query, close};
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
//! # Ok(())
//! # }
//! ```

use js_sys::{Array, Object, Reflect};
use std::
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    }
;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::modules::core::worker::w_msg;

// ==================== GLOBAL STATE ====================

/// Global database ID (singleton).
///
/// Stores the unique identifier returned by the worker after a successful
/// `open()` operation. This ID is required for all subsequent database commands.
static DB_UID: Mutex<Option<JsValue>> = Mutex::new(None);

/// Request counter for message IDs.
///
/// Provides a monotonically increasing ID for each database operation.
static COUNTER: AtomicU32 = AtomicU32::new(0);

// ==================== ACCESSOR FUNCTIONS ====================

/// Gets the current database ID (for internal use)
pub fn get_db_uid() -> Option<JsValue> {
    DB_UID.lock().unwrap().clone()
}

/// Checks if database is open (for internal use)
pub fn is_db_open() -> bool {
    DB_UID.lock().unwrap().is_some()
}

/// Gets next message ID (for internal use)
pub fn next_message_id() -> u32 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ==================== CORE DATABASE FUNCTIONS ====================

/// Opens a database with OPFS (Origin Private File System).
///
/// This function initializes a new SQLite database in the browser's
/// OPFS storage. The database is persistent across page reloads.
///
/// # Arguments
/// * `database_name` - Name of the database file (e.g., "app.sqlite3").
///
/// # Returns
/// * `Ok(())` - Database opened successfully and UID stored.
/// * `Err(JsValue)` - Opening failed.
#[wasm_bindgen]
pub async fn open(database_name: &str) -> Result<(), JsValue> {
    // If already opened, close it first
    if DB_UID.lock().unwrap().is_some() {
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
/// stored UID. After closing, a new `open()` call is required.
///
/// # Returns
/// * `Ok(())` - Database closed successfully (or was already closed).
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
/// * `sql` - SQL statement to execute
/// * `args` - Vector of `JsValue` parameters for placeholders
///
/// # Returns
/// * `Ok(JsValue)` - Execution succeeded.
/// * `Err(JsValue)` - Execution failed.
#[wasm_bindgen]
pub async fn exec(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if !is_db_open() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let id = next_message_id();

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

/// Executes a SQL query and returns rows as objects.
///
/// Use this for SELECT statements that return data.
///
/// # Arguments
/// * `sql` - SELECT statement to execute
/// * `args` - Vector of `JsValue` parameters for placeholders
///
/// # Returns
/// * `Ok(JsValue)` - Query results with `resultRows` array.
/// * `Err(JsValue)` - Query failed.
#[wasm_bindgen]
pub async fn query(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if !is_db_open() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let id = next_message_id();

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
/// * `false` - No database is currently open.
#[wasm_bindgen]
pub fn is_open() -> bool {
    DB_UID.lock().unwrap().is_some()
}

/// Gets the current database ID (for debugging).
///
/// # Returns
/// * `JsValue` - The current database UID, or `NULL` if no database is open.
#[wasm_bindgen]
pub fn db_id() -> JsValue {
    DB_UID.lock().unwrap().clone().unwrap_or(JsValue::NULL)
}
