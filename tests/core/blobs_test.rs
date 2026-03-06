//! Tests for blobs module
//!
//! This module tests the embedded asset handling and Blob/Data URL creation
//! functionality. These are critical for the self-contained nature of the library,
//! allowing it to run without external file dependencies.

use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::blobs::*;
use crate::common::*;

// Configure tests to run in the browser (required for Blob/URL APIs)
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Test that all embedded assets are available at compile time
///
/// This test verifies that the include_bytes! macros in blobs.rs successfully
/// embedded all required SQLite files. If any of these assertions fail,
/// it means the build process is missing critical files.
#[wasm_bindgen_test]
fn test_assets_available() {
    setup();
    
    // SQLite main JavaScript glue code
    assert!(!assets::SQLITE_JS.is_empty(), "SQLITE_JS should not be empty");
    assert!(assets::SQLITE_JS.len() > 1000, "SQLITE_JS should be substantial size (>1KB)");
    
    // SQLite WebAssembly binary
    assert!(!assets::SQLITE_WASM.is_empty(), "SQLITE_WASM should not be empty");
    assert!(assets::SQLITE_WASM.len() > 100000, "SQLITE_WASM should be substantial size (>100KB)");
    
    // SQLite worker script
    assert!(!assets::SQLITE_WORKER.is_empty(), "SQLITE_WORKER should not be empty");
    assert!(assets::SQLITE_WORKER.len() > 1000, "SQLITE_WORKER should be substantial size (>1KB)");
    
    // OPFS async proxy worker
    assert!(!assets::SQLITE_PROXY.is_empty(), "SQLITE_PROXY should not be empty");
    assert!(assets::SQLITE_PROXY.len() > 10000, "SQLITE_PROXY should be substantial size (>10KB)");
}

/// Test creating Blob URLs from binary data
///
/// Blob URLs are the preferred method for serving embedded files as they're
/// more efficient than Data URLs. This test verifies that:
/// 1. Blob URLs can be created successfully
/// 2. They have the correct format (blob: prefix)
/// 3. Each URL is unique
#[wasm_bindgen_test]
fn test_create_blob_url() {
    setup();
    
    // Test with simple text data
    let test_data = b"Hello, World! This is test data for blob URL creation.";
    let result = create_blob_url(test_data, "text/plain");
    
    assert!(result.is_ok(), "Should create blob URL successfully");
    let url = result.unwrap();
    
    // Verify URL format
    assert!(url.starts_with("blob:"), "URL should start with blob:");
    assert!(url.len() > 10, "URL should have reasonable length");
    
    // Test with binary data
    let binary_data = vec![0x00, 0x01, 0x02, 0x03, 0xFF];
    let binary_result = create_blob_url(&binary_data, "application/octet-stream");
    assert!(binary_result.is_ok(), "Should create blob URL for binary data");
    
    // Test with different MIME types
    let html_result = create_blob_url(b"<html></html>", "text/html");
    assert!(html_result.is_ok(), "Should create blob URL for HTML");
    
    let js_result = create_blob_url(b"console.log('test');", "application/javascript");
    assert!(js_result.is_ok(), "Should create blob URL for JavaScript");
}

/// Test creating Data URLs from binary data
///
/// Data URLs are used as a fallback when Blob URLs are not available
/// (though modern browsers all support Blob URLs). This test verifies:
/// 1. Data URLs can be created
/// 2. They have the correct format (data: prefix with base64)
/// 3. They include the correct MIME type
#[wasm_bindgen_test]
fn test_create_data_url() {
    setup();
    
    let test_data = b"Hello, World! This is test data for data URL creation.";
    let url = create_data_url(test_data, "text/plain");
    
    // Verify data URL format
    assert!(url.starts_with("data:"), "URL should start with data:");
    assert!(url.contains("base64"), "Should be base64 encoded");
    assert!(url.len() > 50, "Data URL should have reasonable length");
    
    // Test with different MIME types
    let html_url = create_data_url(b"<html></html>", "text/html");
    assert!(html_url.starts_with("data:text/html;base64,"), 
            "HTML should use correct MIME type: {}", html_url);
    
    let js_url = create_data_url(b"console.log('test');", "application/javascript");
    assert!(js_url.starts_with("data:application/javascript;base64,"), 
            "JS should use correct MIME type: {}", js_url);
    
    let wasm_url = create_data_url(&[0x00, 0x61, 0x73, 0x6D], "application/wasm");
    assert!(wasm_url.starts_with("data:application/wasm;base64,"), 
            "WASM should use correct MIME type: {}", wasm_url);
}

/// Test creating asset URLs for all embedded file types
///
/// This test verifies that the create_asset_url function works correctly
/// for all SQLite asset types (JS, WASM, worker, proxy) and returns valid URLs.
#[wasm_bindgen_test]
fn test_create_asset_url() {
    setup();
    
    // Test with various asset types
    let js_result = create_asset_url(assets::SQLITE_JS, "text/javascript");
    assert!(js_result.is_ok(), "Should create URL for SQLite JS");
    
    let wasm_result = create_asset_url(assets::SQLITE_WASM, "application/wasm");
    assert!(wasm_result.is_ok(), "Should create URL for SQLite WASM");
    
    let worker_result = create_asset_url(assets::SQLITE_WORKER, "text/javascript");
    assert!(worker_result.is_ok(), "Should create URL for SQLite worker");
    
    let proxy_result = create_asset_url(assets::SQLITE_PROXY, "text/javascript");
    assert!(proxy_result.is_ok(), "Should create URL for SQLite proxy");
    
    // Verify URLs are valid
    let urls = [
        js_result.unwrap(),
        wasm_result.unwrap(),
        worker_result.unwrap(),
        proxy_result.unwrap()
    ];
    
    for url in urls {
        assert!(
            url.starts_with("blob:") || url.starts_with("data:"),
            "URL should be blob: or data: format, got: {}",
            url
        );
    }
}

/// Test creating an embedded worker URL
///
/// This is the main entry point for creating self-contained workers.
/// It should produce a URL that can be used with new Worker() to
/// create a fully functional SQLite worker with all assets embedded.
#[wasm_bindgen_test]
async fn test_create_embedded_worker() {
    setup();
    
    let result = create_embedded_worker().await;
    assert!(result.is_ok(), "Should create embedded worker URL");
    
    let url = result.unwrap();
    assert!(
        url.starts_with("blob:") || url.starts_with("data:"),
        "Worker URL should be blob: or data:, got: {}",
        url
    );
    
    // Verify the URL is not empty and has reasonable length
    assert!(url.len() > 50, "Worker URL should have reasonable length");
    
    // Try to create a worker with this URL (should not panic)
    // Note: Worker creation might fail in test environment due to CSP,
    // but we just verify it doesn't crash
    let worker_result = web_sys::Worker::new(&url);
    web_sys::console::log_1(&format!("Worker creation attempt: {:?}", worker_result).into());
}

/// Test creating large blobs
///
/// Verifies that the blob creation functions can handle larger data sizes
/// that might be encountered in real-world scenarios.
#[wasm_bindgen_test]
fn test_large_blob_creation() {
    setup();
    
    // Create a larger test blob (1MB)
    let large_data = vec![0x42u8; 1024 * 1024]; // 1MB of 'B' characters
    
    let result = create_blob_url(&large_data, "application/octet-stream");
    assert!(result.is_ok(), "Should create blob URL for 1MB data");
    
    let url = result.unwrap();
    assert!(url.starts_with("blob:"), "Large blob URL should be created");
    
    // Test with Data URL as fallback
    let data_url = create_data_url(&large_data, "application/octet-stream");
    assert!(data_url.starts_with("data:"), "Large data URL should be created");
    assert!(data_url.len() > large_data.len(), "Data URL should be larger than raw data");
}

/// Test that URLs are unique
///
/// Each call to create_blob_url should generate a unique URL.
/// This is important for isolating different resources.
#[wasm_bindgen_test]
fn test_url_uniqueness() {
    setup();
    
    let test_data = b"test data";
    
    let url1 = create_blob_url(test_data, "text/plain").unwrap();
    let url2 = create_blob_url(test_data, "text/plain").unwrap();
    
    assert_ne!(url1, url2, "Each blob URL should be unique");
    
    // Data URLs are deterministic, so they should be the same
    let data_url1 = create_data_url(test_data, "text/plain");
    let data_url2 = create_data_url(test_data, "text/plain");
    
    assert_eq!(data_url1, data_url2, "Data URLs with same data should be identical");
}
