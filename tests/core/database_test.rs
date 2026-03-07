//! Tests for database module

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::database::{
    open, close, exec, query, is_open, db_id
};
use js_sys::Reflect;
use wasm_bindgen::JsCast;

use crate::common::*;
use crate::common::fixtures::sql;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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

async fn get_table_names() -> Vec<String> {
    let result = query(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
        vec![]
    ).await.unwrap();
    
    let mut tables = Vec::new();
    
    if let Ok(result_obj) = Reflect::get(&result, &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                for i in 0..rows_array.length() {
                    let row = rows_array.get(i);
                    if let Ok(name_val) = Reflect::get(&row, &"name".into()) {
                        if let Some(name) = name_val.as_string() {
                            tables.push(name);
                        }
                    }
                }
            }
        }
    }
    
    tables
}

#[wasm_bindgen_test]
async fn test_database_lifecycle() {
    setup();
    
    let db_name = test_db_name();
    
    let open_result = open(&db_name).await;
    assert!(open_result.is_ok(), "Should open database");
    assert!(is_open(), "Database should be open");
    
    let id = db_id();
    assert!(!id.is_null(), "Should have valid database ID");
    
    let create_result = exec(sql::CREATE_USERS, vec![]).await;
    assert!(create_result.is_ok(), "Should create table");
    
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
    
    let select_result = query(sql::SELECT_ALL_USERS, vec![]).await;
    assert!(select_result.is_ok(), "Should query data");
    
    let close_result = close().await;
    assert!(close_result.is_ok(), "Should close database");
    assert!(!is_open(), "Database should be closed");
}

#[wasm_bindgen_test]
async fn test_parameterized_queries() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let insert_result = exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Bob"),
            JsValue::from_str("bob@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await;
    assert!(insert_result.is_ok(), "Parameterized insert should work");
    
    let select_result = query(
        "SELECT * FROM users WHERE age > ?",
        vec![JsValue::from(20)]
    ).await;
    assert!(select_result.is_ok(), "Parameterized query should work");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_multiple_inserts() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    for i in 0..3 {
        let insert = exec(
            sql::INSERT_USER,
            vec![
                JsValue::from_str(&format!("User {}", i)),
                JsValue::from_str(&format!("user{}@test.com", i)),
                JsValue::from(20 + i),
                JsValue::from(40000.0 + (i * 5000) as f64),
                JsValue::from(1),
            ]
        ).await;
        assert!(insert.is_ok(), "Should insert user {}", i);
    }
    
    let count = get_user_count().await;
    assert_eq!(count, 3, "Should have 3 users");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_update_operations() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
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
    
    let update_result = exec(
        "UPDATE users SET age = ? WHERE email = ?",
        vec![JsValue::from(26), JsValue::from_str("update@test.com")]
    ).await;
    assert!(update_result.is_ok(), "Update should succeed");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_delete_operations() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Delete Test"),
            JsValue::from_str("delete@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    let delete_result = exec(
        "DELETE FROM users WHERE email = ?",
        vec![JsValue::from_str("delete@test.com")]
    ).await;
    assert!(delete_result.is_ok(), "Delete should succeed");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_invalid_sql_errors() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let invalid_result = exec("INVALID SQL", vec![]).await;
    assert!(invalid_result.is_err(), "Invalid SQL should error");
    
    let no_table_result = query("SELECT * FROM non_existent", vec![]).await;
    assert!(no_table_result.is_err(), "Non-existent table should error");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_transaction_behavior() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    exec("BEGIN", vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Transaction User"),
            JsValue::from_str("tx@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec("COMMIT", vec![]).await.unwrap();
    
    let count = get_user_count().await;
    assert_eq!(count, 1, "User should exist after commit");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_rollback() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    exec("BEGIN", vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Rollback User"),
            JsValue::from_str("rollback@test.com"),
            JsValue::from(25),
            JsValue::from(50000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    exec("ROLLBACK", vec![]).await.unwrap();
    
    let count = get_user_count().await;
    assert_eq!(count, 0, "User should not exist after rollback");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_database_persistence() {
    setup();
    
    let db_name = test_db_name();
    
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    exec(
        sql::INSERT_USER,
        vec![
            JsValue::from_str("Persist Test"),
            JsValue::from_str("persist@test.com"),
            JsValue::from(30),
            JsValue::from(60000.0),
            JsValue::from(1),
        ]
    ).await.unwrap();
    
    close().await.unwrap();
    
    wait(100).await;
    
    open(&db_name).await.unwrap();
    
    let count = get_user_count().await;
    assert_eq!(count, 1, "Data should persist after reopen");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_large_text() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec("CREATE TABLE large (id INTEGER PRIMARY KEY, data TEXT)", vec![]).await.unwrap();
    
    let large_text = "X".repeat(10 * 1024);
    
    exec(
        "INSERT INTO large (data) VALUES (?)",
        vec![JsValue::from_str(&large_text)]
    ).await.unwrap();
    
    let result = query("SELECT data FROM large WHERE id = 1", vec![]).await.unwrap();
    
    if let Ok(result_obj) = Reflect::get(&result, &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                assert_eq!(rows_array.length(), 1, "Should have 1 row");
                if rows_array.length() > 0 {
                    let row = rows_array.get(0);
                    if let Ok(data) = Reflect::get(&row, &"data".into()) {
                        assert_eq!(data.as_string(), Some(large_text));
                    }
                }
            }
        }
    }
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_schema_introspection() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    exec(sql::CREATE_PRODUCTS, vec![]).await.unwrap();
    
    let tables = get_table_names().await;
    
    assert!(tables.contains(&"users".to_string()), "Should find users table");
    assert!(tables.contains(&"products".to_string()), "Should find products table");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_null_handling() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec("CREATE TABLE null_test (id INTEGER, value TEXT)", vec![]).await.unwrap();
    
    exec(
        "INSERT INTO null_test (id, value) VALUES (?, ?)",
        vec![JsValue::from(1), JsValue::NULL]
    ).await.unwrap();
    
    let result = query("SELECT value FROM null_test WHERE id = 1", vec![]).await.unwrap();
    
    if let Ok(result_obj) = Reflect::get(&result, &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<js_sys::Array>() {
                assert_eq!(rows_array.length(), 1, "Should have 1 row");
                if rows_array.length() > 0 {
                    let row = rows_array.get(0);
                    if let Ok(val) = Reflect::get(&row, &"value".into()) {
                        assert!(val.is_null(), "Value should be NULL");
                    }
                }
            }
        }
    }
    
    close().await.unwrap();
}