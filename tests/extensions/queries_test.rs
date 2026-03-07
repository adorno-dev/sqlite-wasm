//! Tests for optimized queries module

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::extensions::queries::*;
use sqlite_wasm::modules::core::database::{open, close, exec};
use js_sys::{Array, Reflect};
use wasm_bindgen::JsCast;

use crate::common::*;
use crate::common::fixtures::{sql, data};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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

#[wasm_bindgen_test]
async fn test_query_array_mode() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    let result = query_array("SELECT name, age FROM users ORDER BY age", vec![]).await;
    assert!(result.is_ok(), "Array mode query should succeed");
    
    if let Ok(result_obj) = Reflect::get(result.as_ref().unwrap(), &"result".into()) {
        if let Ok(rows) = Reflect::get(&result_obj, &"resultRows".into()) {
            if let Ok(rows_array) = rows.dyn_into::<Array>() {
                assert!(rows_array.length() > 0, "Should return rows");
                let first_row = rows_array.get(0);
                assert!(first_row.is_array(), "Array mode should return arrays");
            }
        }
    }
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_query_optimized() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    let sql = "SELECT name, age, salary FROM users WHERE age > ? ORDER BY age";
    let params = vec![JsValue::from(25)];
    
    let start1 = js_sys::Date::now();
    let result1 = query_optimized(sql, params.clone()).await;
    let time1 = js_sys::Date::now() - start1;
    
    assert!(result1.is_ok(), "First optimized query should succeed");
    let row_count1 = get_row_count(result1.as_ref().unwrap());
    assert!(row_count1 > 0, "Should return some rows");
    
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
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_query_optimized_different_parameters() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    let sql = "SELECT name, age FROM users WHERE age BETWEEN ? AND ? ORDER BY age";
    
    let param_sets = vec![
        (vec![JsValue::from(20), JsValue::from(30)], "20-30"),
        (vec![JsValue::from(25), JsValue::from(35)], "25-35"),
        (vec![JsValue::from(30), JsValue::from(40)], "30-40"),
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

#[wasm_bindgen_test]
async fn test_array_mode_with_parameters() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    for user in data::user_rows() {
        exec(sql::INSERT_USER, user).await.unwrap();
    }
    
    let result = query_array(
        "SELECT name, age FROM users WHERE age > ? AND age < ? ORDER BY age",
        vec![JsValue::from(25), JsValue::from(40)]
    ).await;
    assert!(result.is_ok(), "Array mode with params should succeed");
    
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

#[wasm_bindgen_test]
async fn test_array_mode_performance() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    let large_batch = data::large_user_batch();
    for user in &large_batch {
        exec(sql::INSERT_USER, user.clone()).await.unwrap();
    }
    
    let sql = "SELECT * FROM users ORDER BY id";
    
    let start_objects = js_sys::Date::now();
    // CORRIGIDO: sqlite_wasm::modules em vez de crate::modules
    let object_result = sqlite_wasm::modules::core::database::query(sql, vec![]).await.unwrap();
    let object_duration = js_sys::Date::now() - start_objects;
    let object_rows = get_row_count(&object_result);
    
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
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_query_error_handling() {
    setup();
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let invalid_array = query_array("INVALID SQL", vec![]).await;
    assert!(invalid_array.is_err(), "Array mode with invalid SQL should error");
    
    let invalid_opt = query_optimized("INVALID SQL", vec![]).await;
    assert!(invalid_opt.is_err(), "Optimized mode with invalid SQL should error");
    
    let no_table = query_array("SELECT * FROM non_existent", vec![]).await;
    assert!(no_table.is_err(), "Query on non-existent table should error");
    
    close().await.unwrap();
}