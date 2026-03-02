//src/modules/core.rs

//! Core database functionality for SQLite in the browser.
//! 
//! This module aggregates all the core components needed for SQLite
//! database operations in WebAssembly. It provides a clean interface
//! for the worker lifecycle, database operations, and JavaScript bindings.
//! 
//! # Submodules
//! 
//! The core module consists of three main components:
//! 
//! ## `worker`
//! 
//! Manages the Web Worker lifecycle including initialization, ready-state
//! signaling, and request-response messaging. This is the foundation that
//! all database operations build upon.
//! 
//! ## `database`
//! 
//! Provides the main API for database operations: open, close, exec, and query.
//! These functions communicate with the worker using the messaging system and
//! handle the database ID and request counters.
//! 
//! ## `bindings`
//! 
//! Creates a convenient JavaScript API surface by exposing a global `window.wasm`
//! object with camelCase method names. Also provides a type-safe Rust wrapper
//! (`WasmApi`) for internal use.
//! 
//! # Architecture
//! 
//! ```text
//! ┌─────────────────┐
//! │    bindings     │  ← JavaScript API + Rust wrapper
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │    database     │  ← SQL operations (open, exec, query)
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │     worker      │  ← Worker lifecycle + messaging
//! └─────────────────┘
//! ```
//! 
//! # Usage
//! 
//! Internal imports within the crate should use:
//! 
//! ```rust
//! use crate::modules::core::{worker, database, bindings};
//! 
//! // Worker operations
//! worker::initialize_worker("/worker.js").await?;
//! worker::wait_for_worker().await?;
//! 
//! // Database operations
//! database::open("myapp.db").await?;
//! let result = database::query("SELECT * FROM users", vec![]).await?;
//! 
//! // Bindings (usually handled by autostart)
//! bindings::initialize_bindings();
//! let api = bindings::get_api();
//! ```

pub mod worker;
pub mod database;
pub mod bindings;