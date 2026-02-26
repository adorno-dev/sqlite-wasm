cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/debug/sqlite_wasm.wasm
node webserver.js
