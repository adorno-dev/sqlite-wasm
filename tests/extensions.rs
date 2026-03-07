//! Extensions module tests entry point

#[path = "common.rs"]
pub mod common;

#[path = "extensions/prepared_test.rs"]
pub mod prepared_test;

#[path = "extensions/queries_test.rs"]
pub mod queries_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[allow(dead_code)]
fn main() {}