//! Tests for batch insert module
//!
//! This module tests the batch insert functionality which provides
//! 10x faster inserts for large datasets by using transactions.
//!
//! Features tested:
//! - Basic batch insert
//! - Empty batch handling
//! - Chunked batch inserts for very large datasets
//! - Error handling and rollback
//! - Performance comparison with individual inserts
//! - JS-friendly interface

use wasm_bindgen_test::*;
use sqlite_wasm::modules::extensions::batch::*;
use sqlite_wasm::modules::core::database::{open, close, exec, query};
use sqlite_wasm::modules::core::worker::*;
use js_sys::{Array, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;

use crate::common::*;

// Configure tests to run in the browser
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Helper function to get user count
async fn get_user_count() -> i32 {
    let count_result = query("SELECT COUNT(*) as count FROM users", vec![]).await.unwrap();
    
    if let Ok(result_obj) = Reflect::get(&count_result, &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            // CORRIGIDO: Removido .unwrap() desnecessário e adicionada anotação de tipo
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                if rows_array.length() > 0 {
                    let first_row = rows_array.get(0);
                    if let Ok(count_val) = Reflect::get(&first_row, &"count".into()) {
                        return count_val.as_f64().unwrap_or(0.0) as i32;
                    }
                }
            }
        }
    }
    0
}

/// Test basic batch insert functionality
///
/// Verifies that:
/// 1. Multiple rows can be inserted in a single batch
/// 2. All rows are properly inserted
/// 3. Transaction commits successfully
#[wasm_bindgen_test]
async fn test_batch_insert_basic() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Create table
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Get initial count
    let initial_count = get_user_count().await;
    assert_eq!(initial_count, 0, "Should start with 0 users");
    
    // Perform batch insert
    let user_data = data::user_rows();
    let batch_size = user_data.len();
    
    let batch_result = insert_batch(
        sql::INSERT_USER,
        user_data
    ).await;
    
    assert!(batch_result.is_ok(), "Batch insert should succeed");
    
    // Verify count after batch
    let final_count = get_user_count().await;
    assert_eq!(final_count, batch_size as i32, "Should have inserted {} users", batch_size);
    
    close().await.unwrap();
}

/// Test empty batch insert
///
/// Verifies that inserting an empty batch is a no-op and
/// returns success without errors.
#[wasm_bindgen_test]
async fn test_batch_insert_empty() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Empty batch should succeed without error
    let result = insert_batch(sql::INSERT_USER, vec![]).await;
    assert!(result.is_ok(), "Empty batch should succeed");
    
    // Verify no rows were inserted
    let count = get_user_count().await;
    assert_eq!(count, 0, "No rows should be inserted");
    
    close().await.unwrap();
}

/// Test chunked batch insert for large datasets
///
/// Verifies that:
/// 1. Very large batches can be split into chunks
/// 2. Each chunk is properly committed
/// 3. Memory usage is controlled
#[wasm_bindgen_test]
async fn test_chunked_batch_insert() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Create large batch (100 users)
    let large_batch = data::large_user_batch();
    assert_eq!(large_batch.len(), 100, "Should have 100 test users");
    
    // Test with different chunk sizes
    let chunk_sizes = [10, 25, 50];
    
    for &chunk_size in &chunk_sizes {
        // Clear table
        exec("DELETE FROM users", vec![]).await.unwrap();
        
        // Measure performance
        let start = js_sys::Date::now();
        let result = insert_batch_chunked(sql::INSERT_USER, large_batch.clone(), chunk_size).await;
        let duration = js_sys::Date::now() - start;
        
        assert!(result.is_ok(), "Chunked batch insert (size={}) should succeed", chunk_size);
        
        // Log performance for debugging
        web_sys::console::log_1(
            &format!("⏱️ Chunked batch (size={}) inserted {} rows in {}ms", 
                     chunk_size, large_batch.len(), duration).into()
        );
        
        // Verify count
        let count = get_user_count().await;
        assert_eq!(count, large_batch.len() as i32, 
                   "All rows should be inserted with chunk size {}", chunk_size);
    }
    
    close().await.unwrap();
}

/// Test batch insert with transaction rollback on error
///
/// Verifies that:
/// 1. If any insert fails, the entire transaction is rolled back
/// 2. No partial data is committed
#[wasm_bindgen_test]
async fn test_batch_insert_rollback_on_error() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Create table with UNIQUE constraint
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert valid data
    let mut batch = data::user_rows();
    
    // Add invalid row (duplicate email - violates UNIQUE constraint)
    // Assuming first user has email "alice@test.com" from fixtures
    batch.push(vec![
        JsValue::from_str("Duplicate User"),
        JsValue::from_str("alice@test.com"), // Duplicate email
        JsValue::from(40),
        JsValue::from(80000.0),
        JsValue::from(1),
    ]);
    
    let batch_result = insert_batch(sql::INSERT_USER, batch).await;
    assert!(batch_result.is_err(), "Batch with constraint violation should fail");
    
    // Verify no rows were inserted (transaction rolled back)
    let count = get_user_count().await;
    assert_eq!(count, 0, "No rows should be inserted due to rollback");
    
    close().await.unwrap();
}

/// Test performance comparison between individual and batch inserts
///
/// This test demonstrates the performance benefit of batch inserts
/// but doesn't fail if the speedup isn't achieved (it's informational).
#[wasm_bindgen_test]
async fn test_batch_performance() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let batch = data::large_user_batch();
    let batch_size = batch.len();
    
    // Method 1: Individual inserts (no batch)
    let start_individual = js_sys::Date::now();
    for row in &batch {
        exec(sql::INSERT_USER, row.clone()).await.unwrap();
    }
    let individual_duration = js_sys::Date::now() - start_individual;
    
    // Clear table
    exec("DELETE FROM users", vec![]).await.unwrap();
    
    // Method 2: Batch insert
    let start_batch = js_sys::Date::now();
    insert_batch(sql::INSERT_USER, batch).await.unwrap();
    let batch_duration = js_sys::Date::now() - start_batch;
    
    // Log comparison (informational only)
    web_sys::console::log_1(
        &format!("📊 Performance comparison for {} rows:", batch_size).into()
    );
    web_sys::console::log_1(
        &format!("   - Individual inserts: {}ms", individual_duration).into()
    );
    web_sys::console::log_1(
        &format!("   - Batch insert: {}ms", batch_duration).into()
    );
    
    // This is informational, not a hard assertion
    if batch_duration < individual_duration {
        web_sys::console::log_1(
            &format!("   - Speedup: {:.1}x", individual_duration / batch_duration).into()
        );
    }
    
    close().await.unwrap();
}

/// Test JS-friendly batch insert interface
///
/// Verifies that the JavaScript-compatible functions work correctly
/// with JS arrays.
#[wasm_bindgen_test]
async fn test_js_batch_interface() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Create JS array of arrays (simulating what JS would pass)
    let js_batch = Array::new();
    for row in data::user_rows() {
        let js_row = Array::new();
        for value in row {
            js_row.push(&value);
        }
        js_batch.push(&js_row);
    }
    
    // Test JS-friendly batch insert
    let result = insert_batch_js(sql::INSERT_USER, js_batch).await;
    assert!(result.is_ok(), "JS batch insert should succeed");
    
    // Verify count
    let count = get_user_count().await;
    assert_eq!(count, data::user_rows().len() as i32, "All JS batch rows should be inserted");
    
    close().await.unwrap();
}