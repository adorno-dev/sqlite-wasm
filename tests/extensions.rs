//! Extensions module tests entry point

// Declara common com path explícito (mesmo padrão do core.rs)
#[path = "common.rs"]
pub mod common;

#[path = "extensions/batch_test.rs"]
pub mod batch_test;

#[path = "extensions/pool_test.rs"]
pub mod pool_test;

#[path = "extensions/prepared_test.rs"]
pub mod prepared_test;

#[path = "extensions/queries_test.rs"]
pub mod queries_test;


wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[allow(dead_code)]
fn main() {}
