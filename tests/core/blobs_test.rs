//! Tests for blobs module

use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::blobs::*;

use crate::common::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_assets_available() {
    setup();
    
    assert!(!assets::SQLITE_JS.is_empty(), "SQLITE_JS should not be empty");
    assert!(!assets::SQLITE_WASM.is_empty(), "SQLITE_WASM should not be empty");
    assert!(!assets::SQLITE_WORKER.is_empty(), "SQLITE_WORKER should not be empty");
    assert!(!assets::SQLITE_PROXY.is_empty(), "SQLITE_PROXY should not be empty");
}

#[wasm_bindgen_test]
fn test_create_blob_url() {
    setup();
    
    let test_data = b"Hello, World!";
    let result = create_blob_url(test_data, "text/plain");
    
    assert!(result.is_ok(), "Should create blob URL");
    let url = result.unwrap();
    assert!(url.starts_with("blob:"), "URL should start with blob:");
}

#[wasm_bindgen_test]
fn test_create_data_url() {
    setup();
    
    let test_data = b"Hello, World!";
    let url = create_data_url(test_data, "text/plain");
    
    assert!(url.starts_with("data:"), "URL should start with data:");
    assert!(url.contains("base64"), "Should be base64 encoded");
}

#[wasm_bindgen_test]
fn test_create_asset_url() {
    setup();
    
    let js_result = create_asset_url(assets::SQLITE_JS, "text/javascript");
    assert!(js_result.is_ok(), "Should create URL for SQLite JS");
    
    let wasm_result = create_asset_url(assets::SQLITE_WASM, "application/wasm");
    assert!(wasm_result.is_ok(), "Should create URL for SQLite WASM");
}

#[wasm_bindgen_test]
async fn test_create_embedded_worker() {
    setup();
    
    let result = create_embedded_worker().await;
    assert!(result.is_ok(), "Should create embedded worker URL");
    
    let url = result.unwrap();
    assert!(
        url.starts_with("blob:") || url.starts_with("data:"),
        "URL should be blob: or data:"
    );
}

#[wasm_bindgen_test]
fn test_large_blob_creation() {
    setup();
    
    let large_data = vec![0x42u8; 1024 * 1024];
    let result = create_blob_url(&large_data, "application/octet-stream");
    assert!(result.is_ok(), "Should create blob URL for 1MB data");
}

#[wasm_bindgen_test]
fn test_url_uniqueness() {
    setup();
    
    let test_data = b"test data";
    
    let url1 = create_blob_url(test_data, "text/plain").unwrap();
    let url2 = create_blob_url(test_data, "text/plain").unwrap();
    
    assert_ne!(url1, url2, "Each blob URL should be unique");
}