//! Core module tests

use sqlite_wasm::autostart_embedded;

// Declara common com path explícito
#[path = "common.rs"]
pub mod common;

#[path = "core/worker_test.rs"]
pub mod worker_test;

#[path = "core/database_test.rs"]
pub mod database_test;

#[path = "core/bindings_test.rs"]
pub mod bindings_test;

#[path = "core/blobs_test.rs"]
pub mod blobs_test;

// Inicializa o worker UMA ÚNICA VEZ antes de qualquer teste
#[wasm_bindgen_test::wasm_bindgen_test]
async fn init_worker_once() {
    autostart_embedded().await.unwrap();
}

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[allow(dead_code)]
fn main() {}
