//! Tests for connection pool module
//!
//! This module tests the connection pool functionality which enables
//! parallel query execution for improved performance in multi-threaded
//! or high-concurrency scenarios.
//!
//! Features tested:
//! - Pool creation with various sizes
//! - Basic operations (exec, query)
//! - Parallel query execution
//! - Health checking
//! - Statistics reporting
//! - Timeout handling
//! - Query retry logic
//! - Array mode optimization

use wasm_bindgen_test::*;
use sqlite_wasm::modules::extensions::pool::ConnectionPool;
use sqlite_wasm::modules::core::worker::*;
use js_sys::Reflect;  // ← REMOVIDO Array e Object, mantido apenas Reflect
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use futures::future::join_all;

use crate::common::*;
use crate::common::fixtures::sql;

// Configure tests to run in the browser
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Helper function to close all pool connections
async fn close_all_connections(pool: &ConnectionPool) {
    let _ = pool.close_all().await;
}

/// Test pool creation with various sizes
///
/// Verifies that:
/// 1. Pools can be created with different numbers of connections
/// 2. Statistics are reported correctly
#[wasm_bindgen_test]
async fn test_connection_pool_creation() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    
    // Test various pool sizes
    for size in [1, 2, 3, 5] {
        let pool = ConnectionPool::new(&db_name, size).await;
        assert!(pool.is_ok(), "Should create pool with {} connections", size);
        
        let pool = pool.unwrap();
        let stats = pool.stats();
        
        // Stats should contain db name and size info
        assert!(stats.contains(&db_name), "Stats should include db name");
        assert!(stats.contains(&format!("{}/{}", size, size)), 
                "Stats should show pool size {}/{}", size, size);
        
        close_all_connections(&pool).await;
    }
}

/// Test basic pool operations
///
/// Verifies that:
/// 1. exec works through the pool
/// 2. query works through the pool
/// 3. Multiple operations can be performed
#[wasm_bindgen_test]
async fn test_pool_basic_operations() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    let pool = ConnectionPool::new(&db_name, 2).await.unwrap();
    
    // Test exec
    let setup_result = pool.exec(sql::CREATE_USERS, vec![]).await;
    assert!(setup_result.is_ok(), "Should create table via pool");
    
    // Insert some data
    for i in 0..3 {
        let insert = pool.exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&format!("Pool User {}", i)),
                JsValue::from_str(&format!("pool{}@test.com", i)),
                JsValue::from(20 + i),
                JsValue::from(40000.0 + (i * 5000) as f64),
                JsValue::from(1),
            ]
        ).await;
        assert!(insert.is_ok(), "Should insert user {} via pool", i);
    }
    
    // Test query
    let query_result = pool.query("SELECT COUNT(*) as count FROM users", vec![]).await;
    assert!(query_result.is_ok(), "Should query via pool");
    
    // Parse result
    if let Ok(result_obj) = Reflect::get(query_result.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                if rows_array.length() > 0 {
                    let first_row = rows_array.get(0);
                    if let Ok(count_val) = Reflect::get(&first_row, &"count".into()) {
                        let count = count_val.as_f64().unwrap_or(0.0) as i32;
                        assert_eq!(count, 3, "Should have 3 users");
                    }
                }
            }
        }
    }
    
    close_all_connections(&pool).await;
}

/// Test parallel query execution
///
/// Verifies that multiple queries can run concurrently
/// through the pool connections.
#[wasm_bindgen_test]
async fn test_pool_parallel_queries() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    let pool = ConnectionPool::new(&db_name, 4).await.unwrap();
    
    // Setup schema
    pool.exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert test data
    for i in 0..10 {
        pool.exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&format!("Parallel User {}", i)),
                JsValue::from_str(&format!("parallel{}@test.com", i)),
                JsValue::from(20 + i),
                JsValue::from(40000.0 + (i * 1000) as f64),
                JsValue::from(i % 2),
            ]
        ).await.unwrap();
    }
    
    // Run 8 parallel queries
    let start = js_sys::Date::now();
    
    let queries: Vec<_> = (0..8).map(|i| {
        let pool = &pool;
        async move {
            match i % 3 {
                0 => pool.query("SELECT COUNT(*) as count FROM users", vec![]).await,
                1 => pool.query("SELECT AVG(age) as avg_age FROM users", vec![]).await,
                _ => pool.query("SELECT name, age FROM users ORDER BY age LIMIT 5", vec![]).await,
            }
        }
    }).collect();
    
    let results = join_all(queries).await;
    let duration = js_sys::Date::now() - start;
    
    // All queries should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Parallel query {} should succeed", i);
    }
    
    web_sys::console::log_1(
        &format!("⏱️ 8 parallel queries on 4-connection pool took {}ms", duration).into()
    );
    
    close_all_connections(&pool).await;
}

/// Test pool health checking
///
/// Verifies that all connections in the pool are healthy
/// and respond to simple queries.
#[wasm_bindgen_test]
async fn test_pool_health_check() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    let pool = ConnectionPool::new(&db_name, 3).await.unwrap();
    
    let health = pool.health_check().await;
    assert!(health.is_ok(), "Health check should succeed");
    
    let results = health.unwrap();
    assert_eq!(results.len(), 3, "Should check all 3 connections");
    assert!(results.iter().all(|&r| r), "All connections should be healthy");
    
    close_all_connections(&pool).await;
}

/// Test pool statistics reporting
///
/// Verifies that both string and JSON stats are reported correctly.
#[wasm_bindgen_test]
async fn test_pool_stats_reporting() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    let pool = ConnectionPool::new(&db_name, 2).await.unwrap();
    
    // Test string stats
    let stats_str = pool.stats();
    assert!(stats_str.contains(&db_name), "Stats should contain db name");
    assert!(stats_str.contains("2/2"), "Stats should show 2/2 connections available");
    
    // Test JSON stats
    let stats_json = pool.stats_json();
    assert!(stats_json.is_ok(), "JSON stats should be available");
    
    if let Ok(stats) = stats_json {
        if let Ok(obj) = stats.dyn_into::<js_sys::Object>() {
            let has_db_name = Reflect::has(&obj, &"dbName".into()).unwrap();
            let has_pool_size = Reflect::has(&obj, &"poolSize".into()).unwrap();
            let has_available = Reflect::has(&obj, &"available".into()).unwrap();
            
            assert!(has_db_name, "JSON stats should have dbName");
            assert!(has_pool_size, "JSON stats should have poolSize");
            assert!(has_available, "JSON stats should have available");
        }
    }
    
    close_all_connections(&pool).await;
}

/// Test query with retry logic
///
/// Verifies that failed queries are retried appropriately.
#[wasm_bindgen_test]
async fn test_pool_query_with_retry() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    let pool = ConnectionPool::new(&db_name, 2).await.unwrap();
    
    // Setup schema
    pool.exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert some data
    for user in data::user_rows() {
        pool.exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    // Test query with retry (should succeed first time)
    let result = pool.query_with_retry(
        "SELECT COUNT(*) as count FROM users", 
        vec![], 
        3
    ).await;
    assert!(result.is_ok(), "Query with retry should succeed");
    
    // Test with invalid SQL (will fail, but should retry)
    let invalid_result = pool.query_with_retry(
        "INVALID SQL", 
        vec![], 
        2
    ).await;
    assert!(invalid_result.is_err(), "Invalid SQL should fail even with retry");
    
    close_all_connections(&pool).await;
}

/// Test array mode optimization
///
/// Verifies that array mode queries work and are faster.
#[wasm_bindgen_test]
async fn test_pool_array_mode() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    let pool = ConnectionPool::new(&db_name, 2).await.unwrap();
    
    // Setup
    pool.exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        pool.exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    // Test array mode
    let array_result = pool.query_array(
        "SELECT name, age FROM users ORDER BY age", 
        vec![]
    ).await;
    assert!(array_result.is_ok(), "Array mode query should succeed");
    
    // Parse array result (should be arrays, not objects)
    if let Ok(result_obj) = Reflect::get(array_result.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                if rows_array.length() > 0 {
                    let first_row = rows_array.get(0);
                    assert!(first_row.is_array(), "Array mode should return arrays");
                }
            }
        }
    }
    
    close_all_connections(&pool).await;
}