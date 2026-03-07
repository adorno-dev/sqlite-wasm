//! Tests for prepared statements module

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::{
    autostart_embedded,
    modules::core::database::{open, exec, close},
    modules::extensions::prepared::*
};
use std::sync::Once;

use crate::common::*;
use crate::common::fixtures::sql;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

static WORKER_INIT: Once = Once::new();

async fn ensure_worker() {
    WORKER_INIT.call_once(|| {
        wasm_bindgen_futures::spawn_local(async {
            autostart_embedded().await.expect("Failed to initialize worker");
        });
    });
    // Dá tempo pro worker inicializar
    crate::common::wait(200).await;
}

/// Helper to create unique email
fn unique_email(prefix: &str) -> String {
    format!("{}_{}@test.com", prefix, js_sys::Date::now() as u64)
}

#[wasm_bindgen_test]
async fn test_prepared_statement_basic() {
    setup();
    ensure_worker().await;
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await;
    assert!(stmt.is_ok(), "Should prepare statement");
    let stmt = stmt.unwrap();
    
    let params = vec![
        JsValue::from_str("Basic User"),
        JsValue::from_str(&unique_email("basic")),
        JsValue::from(30),
        JsValue::from(75000.0),
        JsValue::from(1),
    ];
    
    let result = stmt.execute(params).await;
    assert!(result.is_ok(), "Should execute prepared statement");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_prepared_statement_cache() {
    setup();
    ensure_worker().await;
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let start1 = js_sys::Date::now();
    let stmt1 = PreparedStatement::prepare(sql::INSERT_USER).await;
    assert!(stmt1.is_ok(), "First prepare should succeed");
    let time1 = js_sys::Date::now() - start1;
    
    let start2 = js_sys::Date::now();
    let stmt2 = PreparedStatement::prepare(sql::INSERT_USER).await;
    assert!(stmt2.is_ok(), "Second prepare should succeed");
    let time2 = js_sys::Date::now() - start2;
    
    web_sys::console::log_1(&format!("First prepare: {}ms, Cached: {}ms", time1, time2).into());
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_prepared_statement_performance() {
    setup();
    ensure_worker().await;
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let iterations = 20;
    let base_email = unique_email("perf");
    
    let test_params = vec![
        JsValue::from_str("Performance User"),
        JsValue::from_str(&base_email),
        JsValue::from(30),
        JsValue::from(60000.0),
        JsValue::from(1),
    ];
    
    let start_regular = js_sys::Date::now();
    for i in 0..iterations {
        let mut params = test_params.clone();
        params[1] = JsValue::from_str(&format!("{}_{}", base_email, i));
        sqlite_wasm::modules::core::database::exec(
            sql::INSERT_USER, 
            params
        ).await.unwrap();
    }
    let regular_duration = js_sys::Date::now() - start_regular;
    
    exec("DELETE FROM users", vec![]).await.unwrap();
    
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    
    let start_prepared = js_sys::Date::now();
    for i in 0..iterations {
        let mut params = test_params.clone();
        params[1] = JsValue::from_str(&format!("{}_{}", base_email, i));
        stmt.execute(params).await.unwrap();
    }
    let prepared_duration = js_sys::Date::now() - start_prepared;
    
    web_sys::console::log_1(
        &format!("📊 Performance ({} inserts): Regular={}ms, Prepared={}ms", 
                 iterations, regular_duration, prepared_duration).into()
    );
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_prepared_statement_parameter_types() {
    setup();
    ensure_worker().await;
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    
    let string_params = vec![
        JsValue::from_str("String User"),
        JsValue::from_str(&unique_email("string")),
        JsValue::from_str("25"),
        JsValue::from(50000.0),
        JsValue::from(1),
    ];
    let result = stmt.execute(string_params).await;
    assert!(result.is_ok(), "String parameters should work");
    
    let bool_params = vec![
        JsValue::from_str("Bool User"),
        JsValue::from_str(&unique_email("bool")),
        JsValue::from(true),
        JsValue::from(60000.0),
        JsValue::from(1),
    ];
    let result = stmt.execute(bool_params).await;
    assert!(result.is_ok(), "Boolean parameters should work");
    
    let null_params = vec![
        JsValue::from_str("Null User"),
        JsValue::from_str(&unique_email("null")),
        JsValue::NULL,
        JsValue::from(70000.0),
        JsValue::from(1),
    ];
    let result = stmt.execute(null_params).await;
    assert!(result.is_ok(), "NULL parameters should work");
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_prepared_statement_error_handling() {
    setup();
    ensure_worker().await;
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let invalid = PreparedStatement::prepare("INVALID SQL").await;
    assert!(invalid.is_err(), "Invalid SQL should fail to prepare");
    
    let stmt = PreparedStatement::prepare(sql::INSERT_USER).await.unwrap();
    
    let too_few = vec![JsValue::from_str("Only Name")];
    let _result = stmt.execute(too_few).await;
    
    let too_many = vec![
        JsValue::from_str("Extra Params"),
        JsValue::from_str(&unique_email("extra")),
        JsValue::from(30),
        JsValue::from(60000.0),
        JsValue::from(1),
        JsValue::from_str("Extra"),
    ];
    let _result = stmt.execute(too_many).await;
    
    close().await.unwrap();
}

#[wasm_bindgen_test]
async fn test_js_prepared_statement() {
    setup();
    ensure_worker().await;
    
    let db_name = test_db_name();
    open(&db_name).await.unwrap();
    exec(sql::CREATE_USERS, vec![]).await.unwrap();
    
    let js_stmt = JsPreparedStatement::prepare(sql::INSERT_USER).await;
    assert!(js_stmt.is_ok(), "JS prepare should succeed");
    let js_stmt = js_stmt.unwrap();
    
    let params = js_sys::Array::new();
    params.push(&JsValue::from_str("JS User"));
    params.push(&JsValue::from_str(&unique_email("js")));
    params.push(&JsValue::from(33));
    params.push(&JsValue::from(77000.0));
    params.push(&JsValue::from(1));
    
    let exec_result = js_stmt.execute(params).await;
    assert!(exec_result.is_ok(), "JS execute should succeed");
    
    let select_stmt = JsPreparedStatement::prepare(
        "SELECT name FROM users WHERE email = ?"
    ).await.unwrap();
    
    let query_params = js_sys::Array::new();
    query_params.push(&JsValue::from_str(&unique_email("js")));
    
    let query_result = select_stmt.query(query_params).await;
    assert!(query_result.is_ok(), "JS query should succeed");
    
    close().await.unwrap();
}