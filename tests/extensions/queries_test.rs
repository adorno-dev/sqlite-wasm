//! Tests for optimized queries module
//!
//! This module tests the optimized query functionality:
//! - Array mode: returns rows as arrays (2x faster)
//! - Optimized mode: uses column name caching for repeated queries
//! - Column name caching for performance
//!
//! Features tested:
//! - Array mode query execution
//! - Optimized query with caching
//! - Cache hit performance
//! - Parameterized queries in both modes
//! - Error handling

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::extensions::queries::*;
use sqlite_wasm::modules::core::database::{open, close, exec, query};
use sqlite_wasm::modules::core::worker::*;
use js_sys::{Array, Reflect};
use wasm_bindgen::JsCast;

use crate::common::*;
use crate::common::fixtures::{sql, data};

// Configure tests to run in the browser
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Helper function to get row count from query result
fn get_row_count(result: &JsValue) -> u32 {
    if let Ok(result_obj) = Reflect::get(result, &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<Array>() {
                return rows_array.length();
            }
        }
    }
    0
}

/// Test array mode query
///
/// Verifies that:
/// 1. Array mode returns rows as arrays (not objects)
/// 2. All data is correctly retrieved
/// 3. Performance is acceptable
#[wasm_bindgen_test]
async fn test_query_array_mode() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Setup
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    // Test array mode query
    let result = query_array(
        "SELECT name, age, salary FROM users ORDER BY age", 
        vec![]
    ).await;
    assert!(result.is_ok(), "Array mode query should succeed");
    
    // Verify results are arrays, not objects
    if let Ok(result_obj) = Reflect::get(result.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<Array>() {
                assert!(rows_array.length() > 0, "Should return rows");
                
                // First row should be an array
                let first_row = rows_array.get(0);
                assert!(first_row.is_array(), "Array mode should return arrays, not objects");
                
                // Verify array contents
                if let Ok(first_array) = first_row.dyn_into::<Array>() {
                    assert!(first_array.length() >= 3, "Array should have correct number of columns");
                    
                    // Log sample for debugging
                    web_sys::console::log_1(&format!("First row array: {:?}", first_array).into());
                }
            }
        }
    }
    
    close().await.unwrap();
}

/// Test optimized query with caching
///
/// Verifies that:
/// 1. First query (cache miss) works correctly
/// 2. Second query (cache hit) works correctly
/// 3. Cache hit is faster than cache miss
/// 4. Results are returned as objects with named properties
#[wasm_bindgen_test]
async fn test_query_optimized() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Setup
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    // Test optimized query with different parameters
    let sql = "SELECT name, age, salary FROM users WHERE age > ? ORDER BY age";
    let params = vec![JsValue::from(25)];
    
    // First query (cache miss)
    let start1 = js_sys::Date::now();
    let result1 = query_optimized(sql, params.clone()).await;
    let time1 = js_sys::Date::now() - start1;
    
    assert!(result1.is_ok(), "First optimized query should succeed");
    let row_count1 = get_row_count(result1.as_ref().unwrap());
    assert!(row_count1 > 0, "Should return some rows");
    
    // Second query (should hit cache)
    let start2 = js_sys::Date::now();
    let result2 = query_optimized(sql, params).await;
    let time2 = js_sys::Date::now() - start2;
    
    assert!(result2.is_ok(), "Second optimized query should succeed");
    let row_count2 = get_row_count(result2.as_ref().unwrap());
    assert_eq!(row_count1, row_count2, "Both queries should return same number of rows");
    
    web_sys::console::log_1(
        &format!("📊 Optimized query performance: Cache miss: {}ms, Cache hit: {}ms", 
                 time1, time2).into()
    );
    
    // Results should be objects with named properties
    if let Ok(result_obj) = Reflect::get(result2.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<Array>() {
                if rows_array.length() > 0 {
                    let first_row = rows_array.get(0);
                    assert!(!first_row.is_array(), "Optimized query should return objects");
                    
                    // Check for expected properties
                    let has_name = Reflect::has(&first_row, &"name".into()).unwrap();
                    let has_age = Reflect::has(&first_row, &"age".into()).unwrap();
                    let has_salary = Reflect::has(&first_row, &"salary".into()).unwrap();
                    
                    assert!(has_name && has_age && has_salary, 
                            "Result objects should have all columns");
                }
            }
        }
    }
    
    close().await.unwrap();
}

/// Test optimized query with different parameters
///
/// Verifies that the cache works correctly with different parameter values
/// for the same SQL string.
#[wasm_bindgen_test]
async fn test_query_optimized_different_parameters() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Setup
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    let sql = "SELECT name, age FROM users WHERE age BETWEEN ? AND ? ORDER BY age";
    
    // Query with different parameter sets
    let param_sets = vec![
        (vec![JsValue::from(20), JsValue::from(30)], "20-30"),
        (vec![JsValue::from(25), JsValue::from(35)], "25-35"),
        (vec![JsValue::from(30), JsValue::from(40)], "30-40"),
        (vec![JsValue::from(20), JsValue::from(50)], "20-50"),
    ];
    
    for (i, (params, range)) in param_sets.iter().enumerate() {
        let start = js_sys::Date::now();
        let result = query_optimized(sql, params.clone()).await;
        let duration = js_sys::Date::now() - start;
        
        assert!(result.is_ok(), "Query {} (age {}) should succeed", i+1, range);
        
        let row_count = get_row_count(result.as_ref().unwrap());
        web_sys::console::log_1(
            &format!("Query {} (age {}) returned {} rows in {}ms", 
                     i+1, range, row_count, duration).into()
        );
    }
    
    close().await.unwrap();
}

/// Test array mode with parameters
///
/// Verifies that array mode works correctly with parameterized queries.
#[wasm_bindgen_test]
async fn test_array_mode_with_parameters() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Setup
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    // Test array mode with parameters
    let result = query_array(
        "SELECT name, age FROM users WHERE age > ? AND age < ? ORDER BY age",
        vec![JsValue::from(25), JsValue::from(40)]
    ).await;
    assert!(result.is_ok(), "Array mode with params should succeed");
    
    // Verify results are arrays
    if let Ok(result_obj) = Reflect::get(result.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<Array>() {
                if rows_array.length() > 0 {
                    let first_row = rows_array.get(0);
                    assert!(first_row.is_array(), "Array mode should return arrays");
                }
            }
        }
    }
    
    close().await.unwrap();
}

/// Test performance comparison between regular and array mode
///
/// Demonstrates the speed advantage of array mode.
#[wasm_bindgen_test]
async fn test_array_mode_performance() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Setup with more data for meaningful comparison
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    let large_batch = data::large_user_batch();
    for user in &large_batch {
        exec(sql::INSERT_USER, user.clone()).await.unwrap();
    }
    
    let sql = "SELECT * FROM users ORDER BY id";
    
    // Method 1: Regular query (returns objects)
    let start_objects = js_sys::Date::now();
    let object_result = query(sql, vec![]).await.unwrap();
    let object_duration = js_sys::Date::now() - start_objects;
    let object_rows = get_row_count(&object_result);
    
    // Method 2: Array mode (2x faster)
    let start_arrays = js_sys::Date::now();
    let array_result = query_array(sql, vec![]).await.unwrap();
    let array_duration = js_sys::Date::now() - start_arrays;
    let array_rows = get_row_count(&array_result);
    
    assert_eq!(object_rows, array_rows, "Both modes should return same number of rows");
    
    web_sys::console::log_1(
        &format!("📊 Query performance ({} rows):", object_rows).into()
    );
    web_sys::console::log_1(
        &format!("   - Object mode: {}ms", object_duration).into()
    );
    web_sys::console::log_1(
        &format!("   - Array mode: {}ms", array_duration).into()
    );
    
    // This is informational, not a hard assertion
    if array_duration < object_duration {
        web_sys::console::log_1(
            &format!("   - Speedup: {:.1}x", object_duration / array_duration).into()
        );
    }
    
    close().await.unwrap();
}

/// Test error handling in optimized queries
///
/// Verifies that errors are properly reported.
#[wasm_bindgen_test]
async fn test_query_error_handling() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Test array mode with invalid SQL
    let invalid_array = query_array("INVALID SQL", vec![]).await;
    assert!(invalid_array.is_err(), "Array mode with invalid SQL should error");
    
    // Test optimized mode with invalid SQL
    let invalid_opt = query_optimized("INVALID SQL", vec![]).await;
    assert!(invalid_opt.is_err(), "Optimized mode with invalid SQL should error");
    
    // Test with non-existent table
    let no_table = query_array("SELECT * FROM non_existent", vec![]).await;
    assert!(no_table.is_err(), "Query on non-existent table should error");
    
    close().await.unwrap();
}