# SQLite WASM

[![crates.io](https://img.shields.io/crates/v/sqlite-wasm.svg)](https://crates.io/crates/sqlite-wasm)
[![docs.rs](https://docs.rs/sqlite-wasm/badge.svg)](https://docs.rs/sqlite-wasm)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rust-lang.org)
[![WASM](https://img.shields.io/badge/target-wasm32-purple.svg)](https://webassembly.org)

**A high-performance SQLite wrapper for WebAssembly with OPFS support**  
Zero-cost abstractions • Type-safe API • Persistent storage • Single-threaded by design

---

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

---

## 🚀 Test It Live

**👉 [Try SQLite Studio Online](https://sqlite-wasm.netlify.app)**  
No installation needed. Opens directly in your browser.

---

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sqlite-wasm = "0.1"
```

Or via CLI:

```bash
cargo add sqlite-wasm
```

---

## 🔨 Building

```bash
trunk build --release
trunk serve --release
```

---

## 🏗️ Architecture

```
┌─────────────────┐
│   Your App      │
│   (Rust/JS)     │
└────────┬────────┘
         │
┌────────▼────────┐
│   sqlite-wasm   │
│   ┌────────────┐│
│   │  Worker    ││  ── Singleton, auto-managed
│   │  Manager   ││
│   └────────────┘│
│   ┌────────────┐│
│   │   Message  ││  ── Atomic counters, oneshot channels
│   │   Passing  ││
│   └────────────┘│
│   ┌────────────┐│
│   │    OPFS    ││  ── Persistent, high-performance
│   │   Storage  ││
│   └────────────┘│
└────────┬────────┘
         │
┌────────▼────────┐
│  SQLite Worker  │
│  (JavaScript)   │
└─────────────────┘
```

---

### API Reference

| Function | Description |
|----------|-------------|
| `autostart(path)` | Initializes the SQLite worker (call once) |
| `open(name)` | Opens or creates a database |
| `exec(sql, params)` | Executes SQL without returning rows |
| `query(sql, params)` | Executes SELECT and returns rows |
| `close()` | Closes the current database |
| `is_open()` | Checks if a database is open |
| `db_id()` | Returns current database ID (debug) |

---

## 🦀 Rust Usage

### Quick Start

```rust
use sqlite_wasm::{autostart, open, close};

#[wasm_bindgen]
pub async fn example() -> Result<(), JsValue> {
    // 1. Initialize worker
    let db = autostart("/static/sqlite.org/sqlite3-worker1.js").await?;
    
    // 2. Open database
    open("myapp.sqlite3").await?;
    
    // 3. Create table
    db.exec(
        "CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT)",
        vec![]
    ).await?;
    
    // 4. Insert data
    db.exec(
        "INSERT INTO users VALUES (?, ?)",
        vec![1.into(), "Alice".into()]
    ).await?;
    
    // 5. Query data
    let users = db.query("SELECT * FROM users", vec![]).await?;
    
    // 6. Close database
    close().await?;
    
    Ok(())
}
```

---

## 🌐 JavaScript Usage

After initialization, the global `wasm` object is available with all methods.

### Quick Start

```javascript
import init from './pkg/sqlite_wasm.js';

async function start() {
    // 1. Load WASM module
    await init();
    
    // 2. Initialize worker
    await wasm.autostart('/sqlite.org/sqlite3-worker1.js');
    
    // 3. Open database
    await wasm.open('app.db');
    
    // 4. Create table
    await wasm.exec(
        `CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT UNIQUE
        )`,
        []
    );
    
    // 5. Insert data
    await wasm.exec(
        "INSERT INTO users (name, email) VALUES (?, ?)",
        ["Alice", "alice@example.com"]
    );
    
    // 6. Query data
    const result = await wasm.query("SELECT * FROM users", []);
    console.log('Users:', result.resultRows);
    
    // 7. Close database
    await wasm.close();
}

start().catch(console.error);
```

### JavaScript API

| Method | Description | Example |
|--------|-------------|---------|
| `autostart(path)` | Initializes worker | `await wasm.autostart('/sqlite.org/sqlite3-worker1.js')` |
| `open(name)` | Opens/creates database | `await wasm.open('mydb.sqlite3')` |
| `exec(sql, params)` | Executes SQL | `await wasm.exec("INSERT INTO users VALUES (?)", ["João"])` |
| `query(sql, params)` | Runs SELECT | `const res = await wasm.query("SELECT * FROM users", [])` |
| `close()` | Closes database | `await wasm.close()` |

---

## 🌍 Browser Requirements

- WebAssembly support
- Web Workers
- OPFS (Origin Private File System)
- COOP/COEP headers (Cross-Origin Isolation)

**Recommended:** Latest Chrome, Edge, or other Chromium-based browsers.

---

## 📄 License

MIT © [adorno-dev](https://github.com/adorno-dev)

---

Built with 🦀 and ❤️ for maximum performance  
Made in Rust · Runs in Browser · Powered by SQLite