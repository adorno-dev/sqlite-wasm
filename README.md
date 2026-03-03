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

**👉 [Try SQLite Studio Online](https://adorno-dev.github.io/sqlite-wasm)**  
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

## 🛠️ Build Dependencies

**1. Rust WASM target**
```bash
rustup target add wasm32-unknown-unknown
```

**2. wasm-pack**
```bash
cargo install wasm-pack
```

**3. minix (JS minification)**
```bash
cargo install minix
```

**4. Compression tools**

Ubuntu / Debian:
```bash
sudo apt install brotli gzip
```

macOS:
```bash
brew install brotli
```

These tools compile to `wasm32`, generate bindings, minify JS, and compress the final output.

---

## 🔨 Building

```bash
# Make it executable (first time only)
chmod +x build

# Build everything
./build
```

The script automatically:
- ✅ Compiles Rust to WASM
- ✅ Generates bindings
- ✅ Minifies JS glue code
- ✅ Applies Brotli/Gzip compression
- ✅ Produces optimized artifacts in `pkg/`

---

## 🦀 Rust Usage

### Quick Start

```rust
use sqlite_wasm::{autostart, open, close};

#[wasm_bindgen]
pub async fn example() -> Result<(), JsValue> {
    // 1. Initialize worker
    let db = autostart("/sqlite.org/sqlite3-worker1.js").await?;
    
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

## ⚙️ Performance Optimizations

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link time optimization
codegen-units = 1    # Maximum optimization
panic = "abort"      # Remove unwinding
strip = "debuginfo"  # Remove debug symbols
```

| File | Original | Brotli | Reduction |
|------|----------|--------|-----------|
| sqlite_wasm.js | 16KB | 12KB | 25% |
| sqlite_wasm_bg.wasm | 47KB | 17KB | 64% |
| sqlite3.wasm | 835KB | 336KB | 60% |

---

## 📁 Project Structure

```
sqlite-wasm/
├── src/               # Rust source code
│   ├── lib.rs
│   └── modules/
│       └── core/
│           ├── worker.rs
│           ├── database.rs
│           └── bindings.rs
├── sqlite.org/        # Official SQLite files
├── pkg/               # Generated WASM package
├── build              # Build script
├── webserver.js       # Development server
└── index.html         # Test playground
```

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