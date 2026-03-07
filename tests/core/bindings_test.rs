//! Tests for bindings module

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::bindings::*;
use js_sys::Reflect;

use crate::common::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_get_api() {
    setup();
    
    let api = get_api();
    let api_js = JsValue::from(api);
    
    let exec_method = js_sys::Reflect::get(&api_js, &"exec".into());
    assert!(exec_method.is_ok(), "API should have exec method");
    assert!(exec_method.unwrap().is_function(), "exec should be a function");
    
    let query_method = js_sys::Reflect::get(&api_js, &"query".into());
    assert!(query_method.is_ok(), "API should have query method");
    assert!(query_method.unwrap().is_function(), "query should be a function");
}

#[wasm_bindgen_test]
async fn test_initialize_bindings_idempotent() {
    setup();
    
    let first = initialize_bindings();
    let second = initialize_bindings();
    
    assert!(!first.is_undefined(), "First call should return object");
    assert!(!second.is_undefined(), "Second call should return object");
    
    let global = js_sys::global();
    let wasm_global = Reflect::get(&global, &"wasm".into()).unwrap();
    assert!(!wasm_global.is_undefined(), "window.wasm should exist");
}

#[wasm_bindgen_test]
async fn test_multiple_api_instances() {
    setup();
    
    let api1 = get_api();
    let api2 = get_api();
    
    let api1_js = JsValue::from(api1);
    let api2_js = JsValue::from(api2);
    
    assert!(!js_sys::Object::is(&api1_js, &api2_js),
            "Different get_api calls should return different objects");
    
    let methods = ["exec", "query"];
    for method in methods {
        let m1 = Reflect::get(&api1_js, &method.into()).unwrap();
        let m2 = Reflect::get(&api2_js, &method.into()).unwrap();
        assert!(m1.is_function(), "api1 should have {}", method);
        assert!(m2.is_function(), "api2 should have {}", method);
    }
}

#[wasm_bindgen_test]
async fn test_bindings_expose_essential_functions() {
    setup();
    
    let global = js_sys::global();
    let wasm = Reflect::get(&global, &"wasm".into()).unwrap_or(JsValue::UNDEFINED);
    let wasm_bindings = Reflect::get(&global, &"wasmBindings".into()).unwrap_or(JsValue::UNDEFINED);
    
    let bindings_obj = if !wasm.is_undefined() && !wasm.is_null() {
        wasm
    } else if !wasm_bindings.is_undefined() && !wasm_bindings.is_null() {
        wasm_bindings
    } else {
        panic!("No bindings object found");
    };
    
    let essential_functions = ["open", "close"];
    for fn_name in essential_functions.iter() {
        let has_fn = Reflect::has(&bindings_obj, &(*fn_name).into()).unwrap();
        assert!(has_fn, "Bindings should have function '{}'", fn_name);
        
        let fn_val = Reflect::get(&bindings_obj, &(*fn_name).into()).unwrap();
        assert!(fn_val.is_function(), "'{}' should be a function", fn_name);
    }
}

#[wasm_bindgen_test]
async fn test_global_wasm_exists() {
    setup();
    
    let global = js_sys::global();
    let wasm = Reflect::get(&global, &"wasm".into()).unwrap_or(JsValue::UNDEFINED);
    let wasm_bindings = Reflect::get(&global, &"wasmBindings".into()).unwrap_or(JsValue::UNDEFINED);
    
    let has_wasm = !wasm.is_undefined() && !wasm.is_null();
    let has_wasm_bindings = !wasm_bindings.is_undefined() && !wasm_bindings.is_null();
    
    assert!(has_wasm || has_wasm_bindings, "Global bindings object should exist");
}