//! End-to-end integration tests

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::autostart_embedded;
use sqlite_wasm::modules::core::database::*;
use sqlite_wasm::modules::extensions::prepared::PreparedStatement;
use crate::common::*;
use crate::common::fixtures::{sql, data};

#[wasm_bindgen_test]
async fn test_complete_workflow() {
    setup();
    
    let db = autostart_embedded().await;
    assert!(db.is_ok(), "Autostart should succeed");
    
    let db_name = test_db_name();
    let open_result = open(&db_name).await;
    assert!(open_result.is_ok(), "Should open database");
    
    let create_users = exec(sql::CREATE_USERS, vec![]).await;
    assert!(create_users.is_ok(), "Should create users table");
    
    for user in data::user_rows().iter().take(2) {
        let insert = exec(sql::INSERT_USER, user.clone()).await;
        assert!(insert.is_ok(), "Should insert user");
    }
    
    let all_users = query(sql::SELECT_ALL_USERS, vec![]).await;
    assert!(all_users.is_ok(), "Should query all users");
    
    let prep_stmt = PreparedStatement::prepare(sql::SELECT_USER_BY_ID).await.unwrap();
    let user = prep_stmt.query(vec![JsValue::from(1)]).await;
    assert!(user.is_ok(), "Prepared query should succeed");
    
    let close_result = close().await;
    assert!(close_result.is_ok(), "Should close database");
}
