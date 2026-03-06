//! Integration tests entry point

#[path = "common.rs"]
pub mod common;

#[path = "integration/end_to_end_test.rs"]
pub mod end_to_end_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[allow(dead_code)]
fn main() {}
