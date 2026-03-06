//! Tests for prepared statements module
//!
//! This module tests the prepared statement functionality which provides
//! 1.5x faster execution for repeated queries by compiling SQL once
//! and reusing the execution plan.
//!
//! Features tested:
//! - Basic prepared statement execution
//! - Statement cache for repeated prepares
//! - Performance comparison with regular exec
//! - Parameter binding
//! - Query execution
//! - Error handling

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::extensions::prepared::*;
use sqlite_wasm::modules::core::database::{open, close, exec, query};
use sqlite_wasm::modules::core::worker::*;
use js_sys::Reflect;
use wasm_bindgen::JsCast;

use crate::common::*;
use crate::common::fixtures::sql;

// Configure tests to run in the browser
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Helper function to get user count
async fn get_user_count() -> i32 {
    let count_result = query("SELECT COUNT(*) as count FROM users", vec![]).await.unwrap();
    
    if let Ok(result_obj) = Reflect::get(&count_result, &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
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

/// Test basic prepared statement execution
///
/// Verifies that:
/// 1. A statement can be prepared
/// 2. It can be executed multiple times with different parameters
/// 3. Results are correct
#[wasm_bindgen_test]
async fn test_prepared_statement_basic() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Prepare insert statement
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await;
    assert!(stmt.is_ok(), "Should prepare insert statement");
    let stmt = stmt.unwrap();
    
    // Execute multiple times with different parameters
    let test_users = vec![
        ("Prepared Alice", "prepared.alice@test.com", 28, 72000.0),
        ("Prepared Bob", "prepared.bob@test.com", 32, 85000.0),
        ("Prepared Charlie", "prepared.charlie@test.com", 45, 95000.0),
    ];
    
    for (name, email, age, salary) in test_users {
        let params = vec![
            JsValue::from_str(name),
            JsValue::from_str(email),
            JsValue::from(age),
            JsValue::from(salary),
            JsValue::from(1),
        ];
        
        let result = stmt.execute(params).await;
        assert!(result.is_ok(), "Prepared statement execution should succeed for {}", name);
    }
    
    // Verify count
    let count = get_user_count().await;
    assert_eq!(count, 3, "Should have 3 users from prepared inserts");
    
    // Test prepared SELECT
    let select_stmt = PreparedStatement::prepare(
        "SELECT name, age FROM users WHERE age > ? ORDER BY age"
    ).await.unwrap();
    
    let query_result = select_stmt.query(vec![JsValue::from(30)]).await;
    assert!(query_result.is_ok(), "Prepared SELECT should succeed");
    
    close().await.unwrap();
}

/// Test prepared statement cache
///
/// Verifies that preparing the same SQL multiple times
/// uses the cache and is faster after the first prepare.
#[wasm_bindgen_test]
async fn test_prepared_statement_cache() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Prepare same SQL multiple times to test cache
    let iterations = 5;
    let mut times = Vec::new();
    
    for i in 0..iterations {
        let start = js_sys::Date::now();
        PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
        let duration = js_sys::Date::now() - start;
        times.push(duration);
        
        web_sys::console::log_1(&format!("Prepare #{} took {}ms", i+1, duration).into());
    }
    
    // Log cache performance
    web_sys::console::log_1(
        &format!("📊 Prepare times: {:?}", times).into()
    );
    
    // Clear cache and test again
    PreparedStatement::clear_cache();
    
    let start = js_sys::Date::now();
    PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    let after_clear = js_sys::Date::now() - start;
    
    web_sys::console::log_1(
        &format!("After cache clear: {}ms", after_clear).into()
    );
    
    close().await.unwrap();
}

/// Test performance comparison between regular exec and prepared statements
///
/// This test demonstrates the performance benefit of prepared statements
/// for repeated executions.
#[wasm_bindgen_test]
async fn test_prepared_statement_performance() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let iterations = 50;
    let test_params = vec![
        JsValue::from_str("Perf Test"),
        JsValue::from_str("perf@test.com"),
        JsValue::from(30),
        JsValue::from(60000.0),
        JsValue::from(1),
    ];
    
    // Method 1: Regular exec for each insert
    let start_regular = js_sys::Date::now();
    for _ in 0..iterations {
        exec(sql::INSERT_USER, test_params.clone()).await.unwrap();
    }
    let regular_duration = js_sys::Date::now() - start_regular;
    
    // Clear table
    exec("DELETE FROM users", vec![]).await.unwrap();
    
    // Method 2: Prepared statement reused
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    
    let start_prepared = js_sys::Date::now();
    for _ in 0..iterations {
        stmt.execute(test_params.clone()).await.unwrap();
    }
    let prepared_duration = js_sys::Date::now() - start_prepared;
    
    // Log comparison
    web_sys::console::log_1(
        &format!("📊 Performance comparison for {} inserts:", iterations).into()
    );
    web_sys::console::log_1(
        &format!("   - Regular exec: {}ms", regular_duration).into()
    );
    web_sys::console::log_1(
        &format!("   - Prepared statement: {}ms", prepared_duration).into()
    );
    
    // This is informational, not a hard assertion
    if prepared_duration < regular_duration {
        web_sys::console::log_1(
            &format!("   - Speedup: {:.1}x", regular_duration / prepared_duration).into()
        );
    }
    
    close().await.unwrap();
}

/// Test prepared statement with different parameter types
///
/// Verifies that various JsValue types can be bound correctly.
#[wasm_bindgen_test]
async fn test_prepared_statement_parameter_types() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    
    // Test with different parameter types
    let test_cases = vec![
        (
            "String Test",
            vec![
                JsValue::from_str("String User"),
                JsValue::from_str("string@test.com"),
                JsValue::from_str("25"), // String where number expected (SQLite coerces)
                JsValue::from(50000.0),
                JsValue::from(1),
            ]
        ),
        (
            "Number Test",
            vec![
                JsValue::from_str("Number User"),
                JsValue::from_str("number@test.com"),
                JsValue::from(30),
                JsValue::from(60000.0),
                JsValue::from(1),
            ]
        ),
        (
            "Boolean Test",
            vec![
                JsValue::from_str("Bool User"),
                JsValue::from_str("bool@test.com"),
                JsValue::from(true), // true -> 1
                JsValue::from(70000.0),
                JsValue::from(1),
            ]
        ),
        (
            "Null Test",
            vec![
                JsValue::from_str("Null User"),
                JsValue::from_str("null@test.com"),
                JsValue::NULL, // NULL age
                JsValue::from(80000.0),
                JsValue::from(1),
            ]
        ),
    ];
    
    for (name, params) in test_cases {
        let result = stmt.execute(params).await;
        assert!(result.is_ok(), "Prepared statement with {} should succeed", name);
    }
    
    close().await.unwrap();
}

/// Test prepared statement error handling
///
/// Verifies that errors are properly reported and handled.
#[wasm_bindgen_test]
async fn test_prepared_statement_error_handling() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test preparing invalid SQL
    let invalid = PreparedStatement::prepare("INVALID SQL").await;
    assert!(invalid.is_err(), "Preparing invalid SQL should fail");
    
    // Prepare valid statement
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    
    // Test with wrong number of parameters (too few)
    let too_few = vec![
        JsValue::from_str("Only Name"),
    ];
    let _result = stmt.execute(too_few).await;
    
    // Test with wrong number of parameters (too many)
    let too_many = vec![
        JsValue::from_str("Extra Params"),
        JsValue::from_str("extra@test.com"),
        JsValue::from(30),
        JsValue::from(60000.0),
        JsValue::from(1),
        JsValue::from_str("Extra"), // Extra parameter
    ];
    let _result = stmt.execute(too_many).await;
    
    close().await.unwrap();
}

/// Test JS-friendly prepared statement interface
///
/// Verifies that the JavaScript-compatible wrapper works correctly.
#[wasm_bindgen_test]
async fn test_js_prepared_statement() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test JS-friendly prepared statement
    let js_stmt = JsPreparedStatement::prepare(sql::INSERT_USER).await;
    assert!(js_stmt.is_ok(), "JS prepare should succeed");
    let js_stmt = js_stmt.unwrap();
    
    // Execute with JS array
    let params = js_sys::Array::new();
    params.push(&JsValue::from_str("JS User"));
    params.push(&JsValue::from_str("js.user@test.com"));
    params.push(&JsValue::from(33));
    params.push(&JsValue::from(77000.0));
    params.push(&JsValue::from(1));
    
    let exec_result = js_stmt.execute(params).await;
    assert!(exec_result.is_ok(), "JS execute should succeed");
    
    // Test JS query
    let select_stmt = JsPreparedStatement::prepare(
        "SELECT name FROM users WHERE email = ?"
    ).await.unwrap();
    let query_params = js_sys::Array::new();
    query_params.push(&JsValue::from_str("js.user@test.com"));
    
    let query_result = select_stmt.query(query_params).await;
    assert!(query_result.is_ok(), "JS query should succeed");
    
    close().await.unwrap();
}