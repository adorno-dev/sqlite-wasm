//! Tests for database module
//!
//! This module tests the core database operations including:
//! - Opening and closing databases
//! - Creating tables
//! - CRUD operations (Create, Read, Update, Delete)
//! - Parameterized queries
//! - Transaction behavior
//! - Error handling and constraint violations
//! - Concurrency and persistence
//! - Edge cases and boundary conditions

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::database::*;
use sqlite_wasm::modules::core::worker::*;
#[allow(unused_imports)]
use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsCast;  // ← ADICIONADO! Necessário para dyn_into()
use crate::common::*;
use crate::common::fixtures::sql;

// Configure tests to run in the browser
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Helper function to get user count from database
async fn get_user_count() -> i32 {
    let count_result = query("SELECT COUNT(*) as count FROM users", vec![]).await.unwrap();
    
    let mut count = 0;
    if let Ok(result_obj) = js_sys::Reflect::get(&count_result, &"result".into()) {
        if let Ok(rows) = js_sys::Reflect::get(&result_obj, &"resultRows".into()) {
            // CORRIGIDO: Adicionada anotação de tipo
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                if rows_array.length() > 0 {
                    let first_row = rows_array.get(0);
                    if let Ok(count_val) = js_sys::Reflect::get(&first_row, &"count".into()) {
                        count = count_val.as_f64().unwrap_or(0.0) as i32;
                    }
                }
            }
        }
    }
    count
}

/// Helper function to verify user exists by email
async fn user_exists(email: &str) -> bool {
    let select_result = query(
        "SELECT * FROM users WHERE email = ?",
        vec![JsValue::from_str(email)]
    ).await.unwrap();
    
    if let Ok(result_obj) = js_sys::Reflect::get(&select_result, &"result".into()) {
        if let Ok(rows) = js_sys::Reflect::get(&result_obj, &"resultRows".into()) {
            // CORRIGIDO: Adicionada anotação de tipo
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                return rows_array.length() > 0;
            }
        }
    }
    false
}

/// Helper function to get user field value
async fn get_user_field(email: &str, field: &str) -> Option<JsValue> {
    let select_result = query(
        &format!("SELECT {} FROM users WHERE email = ?", field),
        vec![JsValue::from_str(email)]
    ).await.ok()?;
    
    if let Ok(result_obj) = js_sys::Reflect::get(&select_result, &"result".into()) {
        if let Ok(rows) = js_sys::Reflect::get(&result_obj, &"resultRows".into()) {
            // CORRIGIDO: Adicionada anotação de tipo
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                if rows_array.length() > 0 {
                    let row = rows_array.get(0);
                    return js_sys::Reflect::get(&row, &field.into()).ok();
                }
            }
        }
    }
    None
}

// =============================================================================
// BASIC DATABASE LIFECYCLE TESTS
// =============================================================================

/// Test complete database lifecycle
///
/// Verifies that:
/// 1. Database can be opened
/// 2. Tables can be created
/// 3. Data can be inserted
/// 4. Data can be queried
/// 5. Database can be closed
#[wasm_bindgen_test]
async fn test_database_lifecycle() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    
    // Test 1: Open database
    let open_result = open(&db_name).await;
    assert!(open_result.is_ok(), "Should open database successfully");
    assert!(is_open(), "Database should be marked as open");
    
    // Test 2: Verify database ID is set
    let id = db_id();
    assert!(!id.is_null(), "Should have valid database ID");
    assert!(!id.is_undefined(), "Database ID should not be undefined");
    
    // Test 3: Create table
    let exec_result = exec(sql::CREATE_USERS, vec![]).await;
    assert!(exec_result.is_ok(), "Should create users table");
    
    // Test 4: Insert single row
    let insert_result = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Test User"),
            JsValue::from_str("test@example.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(insert_result.is_ok(), "Should insert row");
    
    // Test 5: Query data
    let select_result = query(sql::SELECT_ALL_USERS, vec![]).await;
    assert!(select_result.is_ok(), "Should query data");
    
    // Parse results to verify data
    let mut row_count = 0;
    let mut has_test_user = false;
    
    if let Ok(result_obj) = js_sys::Reflect::get(select_result.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = js_sys::Reflect::get(&result_obj, &"resultRows".into()) {
            // CORRIGIDO: Adicionada anotação de tipo
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                row_count = rows_array.length();
                
                if row_count > 0 {
                    let first_row = rows_array.get(0);
                    if let Ok(name_val) = js_sys::Reflect::get(&first_row, &"name".into()) {
                        if let Some(name) = name_val.as_string() {
                            has_test_user = name.contains("Test User");
                        }
                    }
                }
            }
        }
    }
    
    assert_eq!(row_count, 1, "Should have exactly 1 row");
    assert!(has_test_user, "Should contain the test user");
    
    // Test 6: Close database
    let close_result = close().await;
    assert!(close_result.is_ok(), "Should close database");
    assert!(!is_open(), "Database should be closed");
}

/// Test opening database without worker initialized
///
/// Verifies that attempting to open a database before worker
/// initialization fails with an appropriate error.
#[wasm_bindgen_test]
async fn test_open_without_worker() {
    setup();
    
    // Try to open without initializing worker first
    let result = open("test.sqlite3").await;
    assert!(result.is_err(), "Should fail without worker initialized");
    
    if let Err(e) = result {
        let error_str = format!("{:?}", e);
        assert!(
            error_str.contains("worker") || error_str.contains("Worker"),
            "Error should mention worker: {}",
            error_str
        );
    }
}

/// Test database operations without opening database
///
/// Verifies that exec and query fail appropriately when no
/// database is open.
#[wasm_bindgen_test]
async fn test_operations_without_open() {
    setup();
    
    // Initialize worker but don't open database
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    // Try exec without open
    let exec_result = exec("SELECT 1", vec![]).await;
    assert!(exec_result.is_err(), "Exec should fail without open database");
    
    if let Err(e) = exec_result {
        let error_str = format!("{:?}", e);
        assert!(
            error_str.contains("open") || error_str.contains("Open"),
            "Error should mention opening database: {}",
            error_str
        );
    }
    
    // Try query without open
    let query_result = query("SELECT 1", vec![]).await;
    assert!(query_result.is_err(), "Query should fail without open database");
}

// =============================================================================
// CRUD OPERATIONS TESTS
// =============================================================================

/// Test parameterized queries for insert and select
#[wasm_bindgen_test]
async fn test_parameterized_queries() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Create table
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test 1: Parameterized insert with multiple parameters
    let insert_result = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Bob Johnson"),
            JsValue::from_str("bob.johnson@test.com"),
            JsValue::from(25),
            JsValue::from(65000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(insert_result.is_ok(), "Parameterized insert should work");
    
    // Test 2: Insert with different parameter values
    let insert_result2 = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Alice Smith"),
            JsValue::from_str("alice.smith@test.com"),
            JsValue::from(30),
            JsValue::from(75000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(insert_result2.is_ok(), "Second insert should work");
    
    // Test 3: Query with single parameter
    let select_result = query(
        "SELECT * FROM users WHERE age > ?",
        vec![JsValue::from(20)]
    ).await;
    assert!(select_result.is_ok(), "Parameterized query with single param should work");
    
    let count = get_user_count().await;
    assert_eq!(count, 2, "Should find both users with age > 20");
    
    // Test 4: Query with multiple parameters (BETWEEN)
    let select_result2 = query(
        "SELECT * FROM users WHERE age BETWEEN ? AND ?",
        vec![JsValue::from(25), JsValue::from(30)]
    ).await;
    assert!(select_result2.is_ok(), "Query with multiple params should work");
    
    // Test 5: Query with string parameter (exact match)
    assert!(user_exists("bob.johnson@test.com").await, 
            "Should find user by email");
    
    // Test 6: Query with NULL parameter handling
    let null_test = query(
        "SELECT * FROM users WHERE ? IS NULL",
        vec![JsValue::NULL]
    ).await;
    assert!(null_test.is_ok(), "Query with NULL parameter should work");
    
    close().await.unwrap();
}

/// Test inserting multiple rows
#[wasm_bindgen_test]
async fn test_multiple_inserts() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert 5 users with different data
    let test_users = vec![
        ("User1", "user1@test.com", 20, 40000.0),
        ("User2", "user2@test.com", 25, 50000.0),
        ("User3", "user3@test.com", 30, 60000.0),
        ("User4", "user4@test.com", 35, 70000.0),
        ("User5", "user5@test.com", 40, 80000.0),
    ];
    
    for (name, email, age, salary) in test_users {
        let insert = exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(name),
                JsValue::from_str(email),
                JsValue::from(age),
                JsValue::from(salary),
                JsValue::from(1),
            ]
        ).await;
        assert!(insert.is_ok(), "Should insert user {}", name);
    }
    
    // Verify count
    let count = get_user_count().await;
    assert_eq!(count, 5, "Should have 5 users");
    
    // Verify each user exists
    assert!(user_exists("user1@test.com").await, "User1 should exist");
    assert!(user_exists("user2@test.com").await, "User2 should exist");
    assert!(user_exists("user3@test.com").await, "User3 should exist");
    assert!(user_exists("user4@test.com").await, "User4 should exist");
    assert!(user_exists("user5@test.com").await, "User5 should exist");
    
    close().await.unwrap();
}

/// Test update operations
#[wasm_bindgen_test]
async fn test_update_operations() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert test user
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Update Test"),
            JsValue::from_str("update@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    // Test 1: Update single field
    let update_result = exec(
        "UPDATE users SET age = ? WHERE email = ?",
        vec![JsValue::from(26), JsValue::from_str("update@test.com")]
    ).await;
    assert!(update_result.is_ok(), "Single field update should succeed");
    
    // Verify single field update
    if let Some(age_val) = get_user_field("update@test.com", "age").await {
        let age = age_val.as_f64().unwrap_or(0.0) as i32;
        assert_eq!(age, 26, "Age should be updated to 26");
    } else {
        panic!("Could not retrieve age after update");
    }
    
    // Test 2: Update multiple fields
    let update_result2 = exec(
        "UPDATE users SET age = ?, salary = ? WHERE email = ?",
        vec![
            JsValue::from(27),
            JsValue::from(55000.0),
            JsValue::from_str("update@test.com")
        ]
    ).await;
    assert!(update_result2.is_ok(), "Multi-field update should succeed");
    
    // Verify multi-field update
    if let Some(age_val) = get_user_field("update@test.com", "age").await {
        let age = age_val.as_f64().unwrap_or(0.0) as i32;
        assert_eq!(age, 27, "Age should be updated to 27");
    }
    
    if let Some(salary_val) = get_user_field("update@test.com", "salary").await {
        let salary = salary_val.as_f64().unwrap_or(0.0);
        assert_eq!(salary, 55000.0, "Salary should be updated to 55000");
    }
    
    // Test 3: Update non-existent record (should succeed but affect 0 rows)
    let update_none = exec(
        "UPDATE users SET age = ? WHERE email = ?",
        vec![JsValue::from(30), JsValue::from_str("nonexistent@test.com")]
    ).await;
    assert!(update_none.is_ok(), "Update on non-existent record should still return OK");
    
    // Test 4: Update with NULL value
    let update_null = exec(
        "UPDATE users SET age = NULL WHERE email = ?",
        vec![JsValue::from_str("update@test.com")]
    ).await;
    assert!(update_null.is_ok(), "Update with NULL should succeed");
    
    // Verify NULL update
    if let Some(age_val) = get_user_field("update@test.com", "age").await {
        assert!(age_val.is_null() || age_val.is_undefined(), 
                "Age should be NULL after update");
    }
    
    close().await.unwrap();
}

/// Test delete operations
#[wasm_bindgen_test]
async fn test_delete_operations() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert test users
    for i in 0..5 {
        exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&format!("Delete Test {}", i)),
                JsValue::from_str(&format!("delete{}@test.com", i)),
                JsValue::from(20 + i),
                JsValue::from(40000.0 + (i * 5000) as f64),
                JsValue::from(i % 2), // Alternate active status
            ]
        ).await.unwrap();
    }
    
    let initial_count = get_user_count().await;
    assert_eq!(initial_count, 5, "Should start with 5 users");
    
    // Test 1: Delete single user by email
    let delete_result = exec(
        "DELETE FROM users WHERE email = ?",
        vec![JsValue::from_str("delete0@test.com")]
    ).await;
    assert!(delete_result.is_ok(), "Single delete should succeed");
    
    let count_after_first = get_user_count().await;
    assert_eq!(count_after_first, 4, "Should have 4 users after first delete");
    assert!(!user_exists("delete0@test.com").await, "Deleted user should not exist");
    
    // Test 2: Delete multiple users with condition
    let delete_result2 = exec(
        "DELETE FROM users WHERE age > ?",
        vec![JsValue::from(22)]
    ).await;
    assert!(delete_result2.is_ok(), "Conditional delete should succeed");
    
    let count_after_second = get_user_count().await;
    assert_eq!(count_after_second, 2, "Should have 2 users after conditional delete");
    
    // Test 3: Delete with invalid condition (should succeed but affect 0 rows)
    let delete_none = exec(
        "DELETE FROM users WHERE age > ?",
        vec![JsValue::from(100)]
    ).await;
    assert!(delete_none.is_ok(), "Delete with no matches should still return OK");
    
    let final_count = get_user_count().await;
    assert_eq!(final_count, 2, "Count should remain unchanged");
    
    // Test 4: Delete all remaining users
    let delete_all = exec(
        "DELETE FROM users",
        vec![]
    ).await;
    assert!(delete_all.is_ok(), "Delete all should succeed");
    
    let empty_count = get_user_count().await;
    assert_eq!(empty_count, 0, "Should have 0 users after delete all");
    
    close().await.unwrap();
}

// =============================================================================
// TRANSACTION TESTS
// =============================================================================

/// Test transaction behavior with COMMIT and ROLLBACK
#[wasm_bindgen_test]
async fn test_transaction_behavior() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test 1: Successful transaction with COMMIT
    exec("BEGIN TRANSACTION", vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Transaction User 1"),
            JsValue::from_str("tx1@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Transaction User 2"),
            JsValue::from_str("tx2@test.com"),
            JsValue::from(30),
            JsValue::from(60000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec("COMMIT", vec![]).await.unwrap();
    
    // Verify both inserted
    let count_after_commit = get_user_count().await;
    assert_eq!(count_after_commit, 2, "Both users should be inserted after COMMIT");
    
    // Test 2: Transaction with ROLLBACK
    exec("BEGIN TRANSACTION", vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Rollback User"),
            JsValue::from_str("rollback@test.com"),
            JsValue::from(35),
            JsValue::from(70000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec("ROLLBACK", vec![]).await.unwrap();
    
    // Verify count hasn't changed
    let count_after_rollback = get_user_count().await;
    assert_eq!(count_after_rollback, 2, "Count should be unchanged after ROLLBACK");
    assert!(!user_exists("rollback@test.com").await, "Rollback user should not exist");
    
    // Test 3: Nested transactions (SQLite treats nested BEGIN as no-op)
    exec("BEGIN", vec![]).await.unwrap();
    exec("BEGIN", vec![]).await.unwrap(); // Second BEGIN is ignored
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Nested User"),
            JsValue::from_str("nested@test.com"),
            JsValue::from(40),
            JsValue::from(80000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec("COMMIT", vec![]).await.unwrap(); // This commits
    
    let count_after_nested = get_user_count().await;
    assert_eq!(count_after_nested, 3, "Nested transaction should commit");
    
    // Test 4: Transaction with SAVEPOINT (real nested transactions)
    exec("SAVEPOINT sp1", vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Savepoint User"),
            JsValue::from_str("savepoint@test.com"),
            JsValue::from(45),
            JsValue::from(90000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec("ROLLBACK TO SAVEPOINT sp1", vec![]).await.unwrap();
    
    let count_after_savepoint = get_user_count().await;
    assert_eq!(count_after_savepoint, 3, "Savepoint rollback should revert insert");
    
    exec("RELEASE SAVEPOINT sp1", vec![]).await.unwrap();
    
    close().await.unwrap();
}

/// Test transaction with constraint violation and automatic rollback
#[wasm_bindgen_test]
async fn test_transaction_constraint_violation() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert initial user
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Original User"),
            JsValue::from_str("original@test.com"),
            JsValue::from(30),
            JsValue::from(60000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    // Start transaction
    exec("BEGIN", vec![]).await.unwrap();
    
    // Insert valid user
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Valid User"),
            JsValue::from_str("valid@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    // Try to insert duplicate email (violates UNIQUE constraint)
    let duplicate_result = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Duplicate User"),
            JsValue::from_str("original@test.com"), // Duplicate email
            JsValue::from(35),
            JsValue::from(70000.0),
            JsValue::from(1),
        ]
    ).await;
    
    assert!(duplicate_result.is_err(), "Duplicate insert should fail");
    
    // In SQLite, constraint violation does NOT automatically rollback the transaction
    // The transaction remains active but in a failed state
    
    // We need to explicitly rollback
    exec("ROLLBACK", vec![]).await.unwrap();
    
    // Verify only original user remains
    let count = get_user_count().await;
    assert_eq!(count, 1, "Only original user should remain after rollback");
    assert!(user_exists("original@test.com").await, "Original user should exist");
    assert!(!user_exists("valid@test.com").await, "Valid user should be rolled back");
    
    close().await.unwrap();
}

// =============================================================================
// ERROR HANDLING AND CONSTRAINT TESTS
// =============================================================================

/// Test error handling for invalid SQL
#[wasm_bindgen_test]
async fn test_invalid_sql_errors() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test 1: Completely invalid SQL
    let invalid_result = exec("THIS IS NOT VALID SQL", vec![]).await;
    assert!(invalid_result.is_err(), "Invalid SQL should error");
    
    // Test 2: Valid SQL but non-existent table
    let no_table_result = query("SELECT * FROM non_existent_table", vec![]).await;
    assert!(no_table_result.is_err(), "Query on non-existent table should error");
    
    // Test 3: Syntax error in SQL
    let syntax_error = exec("CREATE TABLE (id INTEGER)", vec![]).await; // Missing table name
    assert!(syntax_error.is_err(), "Syntax error should be caught");
    
    // Test 4: Invalid column name
    let bad_column = query("SELECT invalid_column FROM users", vec![]).await;
    assert!(bad_column.is_err(), "Invalid column should error");
    
    // Test 5: Wrong number of parameters
    let _wrong_params = query(
        "SELECT * FROM users WHERE age > ? AND name = ?",
        vec![JsValue::from(25)] // Missing second parameter
    ).await;
    // SQLite might handle this (missing params treated as NULL) or error
    // We just verify it doesn't crash
    
    close().await.unwrap();
}

/// Test constraint violations (UNIQUE, NOT NULL, etc.)
#[wasm_bindgen_test]
async fn test_constraint_violations() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    // Create table with constraints
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert first user
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Unique Test"),
            JsValue::from_str("unique@test.com"), // UNIQUE email
            JsValue::from(30),
            JsValue::from(60000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    // Test 1: Duplicate UNIQUE constraint
    let duplicate = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Duplicate Test"),
            JsValue::from_str("unique@test.com"), // Duplicate email
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(duplicate.is_err(), "Duplicate UNIQUE constraint should error");
    
    // Test 2: NULL in NOT NULL column (if we had one - our schema doesn't enforce NOT NULL on all)
    // SQLite is lenient with NULLs, but we can test with a modified schema
    
    // Create a table with explicit NOT NULL
    exec(
        "CREATE TABLE not_null_test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        vec![]
    ).await.unwrap();
    
    let null_insert = exec(
        "INSERT INTO not_null_test (name) VALUES (?)",
        vec![JsValue::NULL]
    ).await;
    assert!(null_insert.is_err(), "NULL in NOT NULL column should error");
    
    // Test 3: PRIMARY KEY violation
    exec(
        "INSERT INTO not_null_test (id, name) VALUES (1, 'Test1')",
        vec![]
    ).await.unwrap();
    
    let pk_violation = exec(
        "INSERT INTO not_null_test (id, name) VALUES (1, 'Test2')",
        vec![]
    ).await;
    assert!(pk_violation.is_err(), "Duplicate PRIMARY KEY should error");
    
    close().await.unwrap();
}

/// Test type coercion and handling
#[wasm_bindgen_test]
async fn test_type_handling() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test 1: Insert with string where number expected
    // SQLite is lenient - it will try to convert
    let string_as_number = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Type Test"),
            JsValue::from_str("type@test.com"),
            JsValue::from_str("30"), // String instead of number
            JsValue::from(60000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(string_as_number.is_ok(), "String where number expected should be coerced");
    
    // Test 2: Insert with boolean (JS boolean)
    let boolean_test = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Bool Test"),
            JsValue::from_str("bool@test.com"),
            JsValue::from(true), // true -> 1
            JsValue::from(60000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(boolean_test.is_ok(), "Boolean should be converted to number");
    
    // Test 3: Verify coerced values
    if let Some(age_val) = get_user_field("type@test.com", "age").await {
        let age = age_val.as_f64().unwrap_or(0.0) as i32;
        assert_eq!(age, 30, "String '30' should be coerced to number 30");
    }
    
    if let Some(age_val) = get_user_field("bool@test.com", "age").await {
        let age = age_val.as_f64().unwrap_or(0.0) as i32;
        assert_eq!(age, 1, "Boolean true should be coerced to 1");
    }
    
    close().await.unwrap();
}

// =============================================================================
// PERSISTENCE AND REOPEN TESTS
// =============================================================================

/// Test database reopen and data persistence
#[wasm_bindgen_test]
async fn test_reopen_persistence() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    
    // ===== FIRST SESSION =====
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert test data
    let test_users = vec![
        ("Persist User 1", "persist1@test.com", 25, 50000.0),
        ("Persist User 2", "persist2@test.com", 30, 60000.0),
        ("Persist User 3", "persist3@test.com", 35, 70000.0),
    ];
    
    for (name, email, age, salary) in test_users {
        exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(name),
                JsValue::from_str(email),
                JsValue::from(age),
                JsValue::from(salary),
                JsValue::from(1),
            ]
        ).await.unwrap();
    }
    
    let first_count = get_user_count().await;
    assert_eq!(first_count, 3, "Should have 3 users in first session");
    
    close().await.unwrap();
    
    // Small delay to ensure file is flushed to OPFS
    wait(200).await;
    
    // ===== SECOND SESSION - REOPEN =====
    open(&db_name).await.unwrap();
    
    // Verify data persisted
    let second_count = get_user_count().await;
    assert_eq!(second_count, 3, "Data should persist after reopen");
    
    // Verify each user exists
    assert!(user_exists("persist1@test.com").await, "User1 should persist");
    assert!(user_exists("persist2@test.com").await, "User2 should persist");
    assert!(user_exists("persist3@test.com").await, "User3 should persist");
    
    // Verify we can still do operations
    let insert_after_reopen = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Post-Reopen User"),
            JsValue::from_str("postreopen@test.com"),
            JsValue::from(40),
            JsValue::from(80000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(insert_after_reopen.is_ok(), "Should insert after reopen");
    
    let final_count = get_user_count().await;
    assert_eq!(final_count, 4, "Should have 4 users after second session insert");
    
    close().await.unwrap();
    
    // ===== THIRD SESSION - VERIFY FINAL STATE =====
    open(&db_name).await.unwrap();
    
    let final_verify_count = get_user_count().await;
    assert_eq!(final_verify_count, 4, "Final state should have 4 users");
    assert!(user_exists("postreopen@test.com").await, "Post-reopen user should persist");
    
    close().await.unwrap();
}

/// Test that database is automatically closed when dropped
/// (This is more of a conceptual test - we can't directly test Drop)
#[wasm_bindgen_test]
async fn test_auto_close_on_drop() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    
    // Open in a scope
    {
        open(&db_name).await.unwrap();
        assert!(is_open(), "Database should be open in scope");
        // Let it go out of scope - should close automatically
    }
    
    // Small delay
    wait(100).await;
    
    // Should be closed
    assert!(!is_open(), "Database should be closed after scope ends");
    
    // Should be able to reopen
    open(&db_name).await.unwrap();
    assert!(is_open(), "Should reopen successfully");
    
    close().await.unwrap();
}

// =============================================================================
// CONCURRENCY TESTS
// =============================================================================

/// Test concurrent operations on the same database
/// SQLite handles concurrency via its own locking mechanism
#[wasm_bindgen_test]
async fn test_concurrent_operations() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Run multiple inserts concurrently
    let mut futures = Vec::new();
    
    for i in 0..10 {
        // CORRIGIDO: Clone i para usar dentro da async block
        let i_clone = i;
        let future = async move {
            exec(
                sql::INSERT_USER,
                vec![
                    JsValue::from_str(&format!("Concurrent User {}", i_clone)),
                    JsValue::from_str(&format!("concurrent{}@test.com", i_clone)),
                    JsValue::from(20 + i_clone),
                    JsValue::from(40000.0 + (i_clone * 1000) as f64),
                    JsValue::from(i_clone % 2),
                ]
            ).await
        };
        futures.push(future);
    }
    
    let results = futures::future::join_all(futures).await;
    
    // All operations should succeed (SQLite handles concurrency)
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Concurrent insert {} should succeed", i);
    }
    
    // Verify count
    let count = get_user_count().await;
    assert_eq!(count, 10, "Should have 10 users after concurrent inserts");
    
    close().await.unwrap();
}

/// Test interleaved read/write operations
#[wasm_bindgen_test]
async fn test_interleaved_read_write() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert initial data
    for i in 0..5 {
        exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&format!("Initial User {}", i)),
                JsValue::from_str(&format!("initial{}@test.com", i)),
                JsValue::from(20 + i),
                JsValue::from(40000.0),
                JsValue::from(1),
            ]
        ).await.unwrap();
    }
    
    // Interleave reads and writes
    for i in 0..5 {
        // Write
        let write = exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&format!("Interleaved User {}", i)),
                JsValue::from_str(&format!("interleaved{}@test.com", i)),
                JsValue::from(30 + i),
                JsValue::from(50000.0),
                JsValue::from(1),
            ]
        ).await;
        assert!(write.is_ok(), "Interleaved write {} should succeed", i);
        
        // Read
        let read = query(
            "SELECT * FROM users WHERE age > ?",
            vec![JsValue::from(25)]
        ).await;
        assert!(read.is_ok(), "Interleaved read {} should succeed", i);
    }
    
    // Final count should be 5 initial + 5 interleaved = 10
    let final_count = get_user_count().await;
    assert_eq!(final_count, 10, "Should have 10 total users");
    
    close().await.unwrap();
}

// =============================================================================
// EDGE CASE TESTS
// =============================================================================

/// Test with empty database
#[wasm_bindgen_test]
async fn test_empty_database() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Query empty table
    let empty_result = query(sql::SELECT_ALL_USERS, vec![]).await.unwrap();
    
    let mut row_count = 0;
    if let Ok(result_obj) = js_sys::Reflect::get(&empty_result, &"result".into()) {
        if let Ok(rows) = js_sys::Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                row_count = rows_array.length();
            }
        }
    }
    
    assert_eq!(row_count, 0, "Empty table should return 0 rows");
    
    close().await.unwrap();
}

/// Test with very large data (stress test)
#[wasm_bindgen_test]
async fn test_large_data() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Insert 100 users
    let start = js_sys::Date::now();
    
    for i in 0..100 {
        let name = format!("Large Data User {}", i);
        let email = format!("large{}@test.com", i);
        
        exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&name),
                JsValue::from_str(&email),
                JsValue::from(20 + (i % 50)),
                JsValue::from(40000.0 + (i * 100) as f64),
                JsValue::from(i % 2),
            ]
        ).await.unwrap();
    }
    
    let duration = js_sys::Date::now() - start;
    web_sys::console::log_1(&format!("⏱️ Inserted 100 users in {}ms", duration).into());
    
    // Verify count
    let count = get_user_count().await;
    assert_eq!(count, 100, "Should have 100 users");
    
    // Query all
    let query_start = js_sys::Date::now();
    let _all_users = query(sql::SELECT_ALL_USERS, vec![]).await.unwrap();
    let query_duration = js_sys::Date::now() - query_start;
    
    web_sys::console::log_1(&format!("⏱️ Queried 100 users in {}ms", query_duration).into());
    
    close().await.unwrap();
}

/// Test with special characters in strings
#[wasm_bindgen_test]
async fn test_special_characters() {
    setup();
    
    initialize_worker("/sqlite.org/sqlite3-worker1.js").await.unwrap();
    wait_for_worker().await.unwrap();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    // Test strings with special characters
    let special_names = vec![
        "O'Connor",           // Apostrophe
        "José María",         // Accented characters
        "张伟",               // Unicode (Chinese)
        "🐘 SQLite",          // Emoji
        "Line\nBreak",        // Newline
        "Tab\tSeparated",     // Tab
        "Quote \"Double\"",   // Double quotes
        "Back\\Slash",        // Backslash
        "100% Complete",      // Percent sign
        "Under_score",        // Underscore
    ];
    
    for (i, name) in special_names.iter().enumerate() {
        let email = format!("special{}@test.com", i);
        
        let insert = exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(name),
                JsValue::from_str(&email),
                JsValue::from(30),
                JsValue::from(50000.0),
                JsValue::from(1),
            ]
        ).await;
        assert!(insert.is_ok(), "Should insert name with special chars: {}", name);
    }
    
    // Verify we can retrieve them
    for (i, original_name) in special_names.iter().enumerate() {
        let email = format!("special{}@test.com", i);
        
        if let Some(name_val) = get_user_field(&email, "name").await {
            if let Some(retrieved_name) = name_val.as_string() {
                assert_eq!(&retrieved_name, original_name, 
                          "Retrieved name should match original");
            } else {
                panic!("Could not convert retrieved name to string for {}", email);
            }
        } else {
            panic!("Could not retrieve user with email {}", email);
        }
    }
    
    close().await.unwrap();
}
