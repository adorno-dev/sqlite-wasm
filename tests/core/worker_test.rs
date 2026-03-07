//! Tests for worker module

use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::worker::{
    initialize_embedded_worker, wait_for_worker
};

use crate::common::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_initialize_worker() {
    setup();
    
    let result = initialize_embedded_worker().await;
    assert!(result.is_ok(), "Worker should initialize successfully");
}

#[wasm_bindgen_test]
async fn test_initialize_embedded_worker() {
    setup();
    
    let result = initialize_embedded_worker().await;
    assert!(result.is_ok(), "Embedded worker should initialize");
}

#[wasm_bindgen_test]
async fn test_wait_for_worker_timeout() {
    setup();
    
    let result = wait_for_worker().await;
    assert!(result.is_err(), "wait_for_worker should timeout when worker not initialized");
}