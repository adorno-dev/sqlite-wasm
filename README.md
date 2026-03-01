# SQLite WASM with Rust

A high-performance SQLite wrapper for WebAssembly using Rust. This project provides a safe and efficient way to use SQLite in the browser with OPFS (Origin Private File System) support.

## ✨ Features

- 🚀 **High Performance**: Written in Rust, compiled to WASM
- 🔒 **OPFS Support**: Persistent storage using Origin Private File System
- 🧵 **Web Worker**: Database operations run in a separate thread
- 📦 **Auto-minification**: JS glue code automatically minified
- 🔐 **COOP/COEP Headers**: Proper headers for SharedArrayBuffer support
- 📊 **Compression**: Brotli/Gzip compression for WASM files
- 🎯 **Type Safe**: Strongly typed Rust API
- 🔄 **Async/Await**: Promise-based API for JavaScript

## 📋 Prerequisites

- Rust (stable) with wasm32-unknown-unknown target
- Node.js (for development server)
- wasm-pack
- minix (for JS minification)
- brotli & gzip (for WASM compression)

## 🛠️ Installation

```bash
# Install Rust target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack

# Install minix for JS minification
cargo install minix

# Install compression tools (Ubuntu/Debian)
sudo apt install brotli gzip

# Install compression tools (MacOS)
brew install brotli
```

## 🚀 Usage

### 1. Clone and build

```bash
git clone https://github.com/yourusername/sqlite-wasm
cd sqlite-wasm
chmod +x build.sh
./build.sh
```

### 2. Start the server

```bash
NODE_ENV=production node webserver.js
```

### 3. Open in browser

```
http://localhost:8080
```

## 📁 Project Structure

```
.
├── src/
│   └── lib.rs           # Rust source code
├── pkg/                  # Generated WASM package
│   ├── sqlite_wasm.js    # Glue code (minified)
│   ├── sqlite_wasm_bg.wasm
│   └── sqlite-wasm/      # SQLite JS files
├── sqlite-wasm/          # Original SQLite files
├── index.html            # Test page
├── webserver.js          # Express server
├── build.sh              # Build script
├── Cargo.toml            # Rust dependencies
└── Trunk.toml            # Trunk configuration
```

## 📝 API Reference

### Initialize Worker
```javascript
await wasm.initialize_worker("/sqlite-wasm/sqlite3-worker1.js");
```

### Open Database
```javascript
await wasm.open();
```

### Execute SQL (no return)
```javascript
await wasm.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", []);
await wasm.exec("INSERT INTO users (name) VALUES (?)", ["Alice"]);
```

### Query SQL (with return)
```javascript
const users = await wasm.query("SELECT * FROM users", []);
console.log(users);
```

### Close Database
```javascript
await wasm.close();
```

## 🏗️ Build Script Features

The `build.sh` script automatically:

1. Compiles Rust to WASM with `wasm-pack`
2. Minifies the glue code (`sqlite_wasm.js`)
3. Copies SQLite JS files to the package
4. Compresses WASM files with Brotli and Gzip
5. Shows final sizes of all generated files

## ⚙️ Configuration Files

### `Cargo.toml` (optimized for size)
```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
```

### `webserver.js` (with compression)
```javascript
app.use(compression({ level: 9 }));
app.use((req, res, next) => {
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  next();
});
```

## 📊 Performance

| File | Original | Compressed | Reduction |
|------|----------|------------|-----------|
| sqlite_wasm.js | 16KB | 12KB | 25% |
| sqlite_wasm_bg.wasm | 47KB | 17KB (Brotli) | 64% |
| sqlite3.wasm | 835KB | 336KB (Brotli) | 60% |

## 🤝 Contributing

1. Fork the project
2. Create your feature branch (`git checkout -b feature/amazing`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- [SQLite](https://sqlite.org) for the amazing database
- [Rust](https://rust-lang.org) for the performance
- [wasm-pack](https://github.com/rustwasm/wasm-pack) for the tooling

## 📧 Contact

Project Link: [https://github.com/yourusername/sqlite-wasm](https://github.com/adorno-dev/sqlite-wasm)

---

**Note**: This project requires COOP/COEP headers to be set on the server for OPFS support with SharedArrayBuffer.
