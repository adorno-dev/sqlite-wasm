//! Tests for bindings module
//!
//! This module tests the JavaScript API exposure functionality.
//! The bindings module creates a global `window.wasm` object with
//! camelCase methods that wrap the Rust functions, making the library
//! feel natural in JavaScript.

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use sqlite_wasm::modules::core::bindings::*;
use js_sys::Reflect;
use wasm_bindgen::JsCast;

use crate::common::*;

// Configure tests to run in the browser (required for window object)
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Test that the Rust API wrapper (WasmApi) has the expected methods
///
/// The WasmApi struct provides a type-safe Rust interface to the
/// database operations. This test verifies that it exposes the
/// required exec and query methods.
#[wasm_bindgen_test]
fn test_get_api() {
    setup();
    
    let api = get_api();
    let api_js = JsValue::from(api);
    
    // Test exec method exists and is a function
    let exec_method = js_sys::Reflect::get(&api_js, &"exec".into());
    assert!(exec_method.is_ok(), "API should have exec method");
    let exec_fn = exec_method.unwrap();
    assert!(exec_fn.is_function(), "exec should be a function");
    
    // Test query method exists and is a function
    let query_method = js_sys::Reflect::get(&api_js, &"query".into());
    assert!(query_method.is_ok(), "API should have query method");
    let query_fn = query_method.unwrap();
    assert!(query_fn.is_function(), "query should be a function");
    
    // Verify there are no unexpected enumerable properties
    // CORRIGIDO: Adicionada anotação de tipo para Object::keys
    let api_obj = api_js.dyn_into::<js_sys::Object>().unwrap();
    let own_keys = js_sys::Object::keys(&api_obj);
    assert_eq!(own_keys.length(), 2, "API should only have exec and query methods");
}

/// Test that initialize_bindings creates the global wasm object correctly
///
/// This test verifies that:
/// 1. The returned object has all required methods
/// 2. The methods follow JavaScript camelCase naming convention
/// 3. The global window.wasm object is created
/// 4. All methods are functions
#[wasm_bindgen_test]
fn test_initialize_bindings() {
    setup();
    
    let wasm_obj = initialize_bindings();
    
    // Expected methods with their JavaScript names
    let methods = [
        ("initializeWorker", "initialize_worker"),
        ("open", "open"),
        ("close", "close"),
        ("exec", "exec"),
        ("query", "query"),
    ];
    
    // Test each method exists and is a function
    for (js_name, rust_name) in methods.iter() {
        // CORRIGIDO: Usando `(*js_name).into()` para evitar &&str
        let has_method = Reflect::has(&wasm_obj, &(*js_name).into()).unwrap();
        assert!(has_method, "Returned object should have {}", js_name);
        
        let method = Reflect::get(&wasm_obj, &(*js_name).into()).unwrap();
        assert!(method.is_function(), "{} should be a function", js_name);
        
        // Verify the underlying Rust function exists in global scope
        let global = js_sys::global();
        let has_rust = Reflect::has(&global, &(*rust_name).into()).unwrap();
        assert!(has_rust, "Rust function {} should exist in global scope", rust_name);
    }
    
    // Test global window.wasm object exists
    let global = js_sys::global();
    let wasm_global = Reflect::get(&global, &"wasm".into()).unwrap();
    
    assert!(!wasm_global.is_undefined(), "window.wasm should exist");
    assert!(!wasm_global.is_null(), "window.wasm should not be null");
    
    // Verify global object has same methods
    for (js_name, _) in methods.iter() {
        let has_method = Reflect::has(&wasm_global, &(*js_name).into()).unwrap();
        assert!(has_method, "window.wasm should have {}", js_name);
        
        let method = Reflect::get(&wasm_global, &(*js_name).into()).unwrap();
        assert!(method.is_function(), "window.wasm.{} should be a function", js_name);
    }
}

/// Test that initialize_bindings is idempotent
///
/// Calling initialize_bindings multiple times should be safe and
/// should not cause errors or duplicate objects.
#[wasm_bindgen_test]
fn test_initialize_bindings_idempotent() {
    setup();
    
    let first = initialize_bindings();
    let second = initialize_bindings();
    
    // Both calls should return objects (not undefined/null)
    assert!(!first.is_undefined(), "First call should return object");
    assert!(!second.is_undefined(), "Second call should return object");
    
    // They should have the same methods
    let methods = ["initializeWorker", "open", "close", "exec", "query"];
    for method in methods.iter() {
        // CORRIGIDO: Usando `(*method).into()` para evitar &&str
        let first_has = Reflect::has(&first, &(*method).into()).unwrap();
        let second_has = Reflect::has(&second, &(*method).into()).unwrap();
        assert!(first_has, "First object has {}", method);
        assert!(second_has, "Second object has {}", method);
    }
    
    // Global wasm object should exist after both calls
    let global = js_sys::global();
    let wasm_global = Reflect::get(&global, &"wasm".into()).unwrap();
    assert!(!wasm_global.is_undefined(), "window.wasm should exist");
}

/// Test that API methods fail gracefully before worker initialization
///
/// The API methods should return proper errors (not panic) when called
/// before the worker is initialized. This test verifies that error
/// handling works correctly.
#[wasm_bindgen_test]
async fn test_api_methods_before_init() {
    setup();
    
    let api = get_api();
    
    // Test exec fails with appropriate error
    let exec_result = api.exec("SELECT 1", vec![]).await;
    assert!(exec_result.is_err(), "Exec should fail without worker");
    
    if let Err(e) = exec_result {
        let error_str = format!("{:?}", e);
        assert!(
            error_str.contains("worker") || error_str.contains("Worker") || error_str.contains("initialize"),
            "Error should mention worker initialization: {}",
            error_str
        );
    }
    
    // Test query fails with appropriate error
    let query_result = api.query("SELECT 1", vec![]).await;
    assert!(query_result.is_err(), "Query should fail without worker");
    
    if let Err(e) = query_result {
        let error_str = format!("{:?}", e);
        assert!(
            error_str.contains("worker") || error_str.contains("Worker") || error_str.contains("initialize"),
            "Error should mention worker initialization: {}",
            error_str
        );
    }
}

/// Test that the bindings correctly map Rust function names to JS names
///
/// This test verifies the naming convention mapping:
/// Rust snake_case -> JavaScript camelCase
#[wasm_bindgen_test]
fn test_naming_convention() {
    setup();
    
    let wasm_obj = initialize_bindings();
    
    // Verify that the JS names follow camelCase convention
    let js_names = ["initializeWorker", "open", "close", "exec", "query"];
    
    for name in js_names.iter() {
        // First character should be lowercase (camelCase)
        let first_char = &name[0..1];
        assert_eq!(first_char.to_lowercase(), first_char, 
                   "JS name '{}' should start with lowercase (camelCase)", name);
        
        // Method should exist
        // CORRIGIDO: Usando `(*name).into()` para evitar &&str
        let has_method = Reflect::has(&wasm_obj, &(*name).into()).unwrap();
        assert!(has_method, "Method {} should exist", name);
    }
    
    // Verify that the Rust functions exist with snake_case names
    let global = js_sys::global();
    let rust_names = ["initialize_worker", "open", "close", "exec", "query"];
    
    for name in rust_names.iter() {
        // CORRIGIDO: Usando `(*name).into()` para evitar &&str
        let has_method = Reflect::has(&global, &(*name).into()).unwrap();
        assert!(has_method, "Rust function {} should exist in global scope", name);
    }
}

/// Test that multiple calls to get_api return independent instances
///
/// Each call to get_api should return a new WasmApi instance.
/// While they wrap the same functionality, they should be distinct objects.
#[wasm_bindgen_test]
fn test_multiple_api_instances() {
    setup();
    
    let api1 = get_api();
    let api2 = get_api();
    
    let api1_js = JsValue::from(api1);
    let api2_js = JsValue::from(api2);
    
    // CORRIGIDO: Comparar diretamente em vez de usar strict_equal
    // Eles são diferentes objetos JS
    assert!(!js_sys::Object::is(&api1_js, &api2_js),
            "Different get_api calls should return different objects");
    
    // But they should have the same methods
    let methods = ["exec", "query"];
    for method in methods.iter() {
        let m1 = Reflect::get(&api1_js, &(*method).into()).unwrap();
        let m2 = Reflect::get(&api2_js, &(*method).into()).unwrap();
        
        // The method functions themselves might be the same (they're from the same prototype)
        // But we just verify they exist in both
        assert!(m1.is_function(), "api1 should have {}", method);
        assert!(m2.is_function(), "api2 should have {}", method);
    }
}