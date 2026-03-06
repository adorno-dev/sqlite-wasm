✨ SQLite WASM ✨
====================================================================

[![crates.io](https://img.shields.io/crates/v/sqlite-wasm.svg)](https://crates.io/crates/sqlite-wasm)
[![docs.rs](https://docs.rs/sqlite-wasm/badge.svg)](https://docs.rs/sqlite-wasm)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rust-lang.org)
[![WASM](https://img.shields.io/badge/target-wasm32-purple.svg)](https://webassembly.org)

A high-performance SQLite wrapper for WebAssembly with OPFS support
Zero external files · Plug-and-play · Works in Chrome and Firefox

--------------------------------------------------------------------

## ✨ Features

| Icon | Feature | Description |
|------|---------|-------------|
| 🚀 | **Zero-cost abstractions** | Worker management with `OnceLock` and atomic counters |
| 🔒 | **OPFS persistence** | Databases survive page reloads and browser restarts |
| 🧵 | **Web Worker** | Database operations run in a separate thread |
| 📦 | **Auto-minification** | JS glue code automatically minified (Brotli/Gzip) |
| 🔐 | **COOP/COEP Headers** | Proper headers for SharedArrayBuffer support |
| 🎯 | **Type Safe** | Strongly typed Rust API with proper error handling |
| 🔄 | **Async/Await** | Promise-based API for JavaScript |
| ⚡ | **Atomic operations** | Lock-free message passing with `AtomicU32` |


--------------------------------------------------------------------

🚀 Test It Live
====================================================================

👉 Try SQLite Studio Online: https://sqlite-wasm.adorno-dev.workers.dev/
No installation needed · Opens directly in your browser

--------------------------------------------------------------------

📦 Installation
====================================================================

Add to your Cargo.toml:

[dependencies]
sqlite-wasm = "0.1.3"

--------------------------------------------------------------------

🔨 Building
====================================================================

trunk build --release

--------------------------------------------------------------------

🏗️ Architecture
====================================================================

                    ┌─────────────────┐
                    │   Your App      │
                    │   (Rust/JS)     │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   sqlite-wasm   │
                    │   ┌────────────┐│
                    │   │   Worker   ││  Singleton, auto-managed
                    │   │   Manager  ││
                    │   └────────────┘│
                    │   ┌────────────┐│
                    │   │  Embedded  ││  Files via include_bytes!
                    │   │   Assets   ││  (Blob/Data URLs)
                    │   └────────────┘│
                    │   ┌────────────┐│
                    │   │    OPFS    ││  Persistent storage
                    │   │   Storage  ││
                    │   └────────────┘│
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │  SQLite Worker  │
                    │  (JavaScript)   │
                    └─────────────────┘

--------------------------------------------------------------------

📋 API REFERENCE
====================================================================

autostart_embedded() → Initializes worker with embedded files

autostart(path) → Legacy: initializes with external file

open(name) → Opens or creates a database

exec(sql, params) → Executes SQL without returning rows

query(sql, params) → Executes SELECT and returns rows

close() → Closes the current database

is_open() → Checks if a database is open

====================================================================

🦀 Rust Usage
====================================================================

Add to your Cargo.toml:

[dependencies]
sqlite-wasm = "0.1.3"

In your Rust code:

use sqlite_wasm::{autostart_embedded, open, exec, query, close};

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    // 1. Initialize worker with embedded files
    autostart_embedded().await?;
    
    // 2. Open database
    open("myapp.sqlite3").await?;
    
    // 3. Create table
    exec("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT)", vec![]).await?;
    
    // 4. Insert data
    exec("INSERT INTO users VALUES (1, 'Alice')", vec![]).await?;
    
    // 5. Query data
    let users = query("SELECT * FROM users", vec![]).await?;
    
    // 6. Close database
    close().await?;
    
    Ok(())
}

--------------------------------------------------------------------

🌐 JavaScript Usage
====================================================================

import init, { 
    autostart_embedded, 
    open, 
    exec, 
    query, 
    close 
} from '/sqlite-wasm.js';

async function start() {
    await init();
    
    // 1. Initialize worker with embedded files
    await autostart_embedded();
    
    // 2. Open database
    await open('myapp.sqlite3');
    
    // 3. Create table
    await exec('CREATE TABLE users (id INTEGER, name TEXT)', []);
    
    // 4. Insert data
    await exec('INSERT INTO users VALUES (1, "Alice")', []);
    
    // 5. Query data
    const result = await query('SELECT * FROM users', []);
    console.log('Users:', result.resultRows);
    
    // 6. Close database
    await close();
}

start();

--------------------------------------------------------------------

🌍 Browser Requirements
====================================================================

- WebAssembly support
- Web Workers
- OPFS (Origin Private File System)
- COOP/COEP headers (Cross-Origin Isolation)

Works in Chrome, Firefox, and other modern browsers.

--------------------------------------------------------------------

🧪 How It Works
====================================================================

1. Embedding: All SQLite files are embedded at compile time using include_bytes!
2. URL creation: Files become Blob URLs (Chrome) or Data URLs (Firefox)
3. Worker wrapper: Intercepts importScripts and fetch
4. File mapping: Original filenames map to embedded URLs
5. Zero external files: Everything runs from memory

--------------------------------------------------------------------

⚖️ License
====================================================================

MIT © adorno-dev (https://github.com/adorno-dev)

--------------------------------------------------------------------

Built with 🦀 and ❤️
Made in Rust · Runs in Browser · Powered by SQLite
