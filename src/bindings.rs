use js_sys::{Object, Reflect};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmApi;

#[wasm_bindgen]
impl WasmApi {
    pub async fn exec(&self, sql: &str, bind: Vec<JsValue>) -> Result<JsValue, JsValue> {
        crate::database::exec(sql, bind).await  // ← bind, não args
    }
    
    pub async fn query(&self, sql: &str, bind: Vec<JsValue>) -> Result<JsValue, JsValue> {
        crate::database::query(sql, bind).await  // ← bind, não args
    }
}

pub fn get_api() -> WasmApi {
    WasmApi
}

/// Creates and exposes the global `wasm` object in the browser window
pub fn initialize_bindings() -> Object {
    let wasm = Object::new();
    let global = js_sys::global();
    
    // Get functions from global scope (where wasm-bindgen placed them)
    let init_fn = js_sys::Reflect::get(&global, &"initialize_worker".into()).unwrap_throw();
    let open_fn = js_sys::Reflect::get(&global, &"open".into()).unwrap_throw();
    let close_fn = js_sys::Reflect::get(&global, &"close".into()).unwrap_throw();
    let exec_fn = js_sys::Reflect::get(&global, &"exec".into()).unwrap_throw();
    let query_fn = js_sys::Reflect::get(&global, &"query".into()).unwrap_throw();
    
    // Add to wasm object with camelCase names (JavaScript convention)
    Reflect::set(&wasm, &"initializeWorker".into(), &init_fn).unwrap_throw();
    Reflect::set(&wasm, &"open".into(), &open_fn).unwrap_throw();
    Reflect::set(&wasm, &"close".into(), &close_fn).unwrap_throw();
    Reflect::set(&wasm, &"exec".into(), &exec_fn).unwrap_throw();
    Reflect::set(&wasm, &"query".into(), &query_fn).unwrap_throw();
    
    // Also expose globally as window.wasm for easy access
    Reflect::set(&global, &"wasm".into(), &wasm).unwrap_throw();
    
    wasm
}
