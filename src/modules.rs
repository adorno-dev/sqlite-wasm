//! Module organization for the SQLite WASM crate.
//! 
//! This module serves as the root of the module hierarchy, re-exporting
//! the core functionality and providing a clean import structure.
//! 
//! # Module Structure
//! 
//! The crate is organized into the following modules:
//! 
//! * **`core`** - Core database functionality including worker management,
//!   database operations, and JavaScript bindings.
//! 
//! # Usage
//! 
//! Internal imports within the crate should use this module hierarchy:
//! 
//! ```rust
//! use sqlite_wasm::modules::core::{worker, database, bindings};
//! ```
//! 
//! External users should import directly from the crate root:
//! 
//! ```rust
//! use sqlite_wasm::{autostart, WasmApi};
//! ```

pub mod core;
