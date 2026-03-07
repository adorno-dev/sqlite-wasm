//! Common utilities and helpers for testing

#[path = "common/fixtures.rs"]
pub mod fixtures;

pub use fixtures::*;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::JsValue;
use std::sync::Once;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

static INIT: Once = Once::new();

pub fn setup() {
    INIT.call_once(|| {
        console_error_panic_hook::set_once();
        web_sys::console::log_1(&"🧪 Test environment initialized".into());
    });
}

pub fn test_db_name() -> String {
    // let timestamp = js_sys::Date::now() as u64;
    // format!("test_db_{}.sqlite3", timestamp)
    "test_db.sqlite3".to_string()
}

pub fn js_array_to_vec(array: &Array) -> Vec<JsValue> {
    let mut vec = Vec::new();
    for i in 0..array.length() {
        vec.push(array.get(i));
    }
    vec
}

pub fn create_test_user(id: i32, name: &str) -> Object {
    let obj = Object::new();
    Reflect::set(&obj, &"id".into(), &JsValue::from(id)).unwrap();
    Reflect::set(&obj, &"name".into(), &JsValue::from_str(name)).unwrap();
    obj
}

pub fn assert_js_eq(actual: &JsValue, expected: &JsValue) {
    assert_eq!(
        format!("{:?}", actual),
        format!("{:?}", expected),
        "JS values not equal"
    );
}

pub async fn wait(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
}

pub fn get_current_db_id() -> Result<JsValue, JsValue> {
    use sqlite_wasm::modules::core::database::db_id;
    Ok(db_id())
}

pub async fn cleanup_test_db(_db_name: &str) {
    use sqlite_wasm::modules::core::database::close;
    let _ = close().await;
    web_sys::console::log_1(&"🔒 Database closed".into());
}