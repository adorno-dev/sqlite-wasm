//src/bindings.rs

pub fn initialize_bindings() {
    // Pega o objeto global (window em browsers)
    let global = js_sys::global();
    
    // Cria o objeto wasm
    let wasm = js_sys::Object::new();
    
    // Pega as funções do escopo global
    let init_fn = js_sys::Reflect::get(&global, &"initialize_worker".into()).unwrap();
    let open_fn = js_sys::Reflect::get(&global, &"open".into()).unwrap();
    let close_fn = js_sys::Reflect::get(&global, &"close".into()).unwrap();
    let exec_fn = js_sys::Reflect::get(&global, &"exec".into()).unwrap();
    let query_fn = js_sys::Reflect::get(&global, &"query".into()).unwrap();
    
    // Adiciona as funções ao objeto wasm (com nomes em camelCase)
    js_sys::Reflect::set(&wasm, &"initializeWorker".into(), &init_fn).unwrap();
    js_sys::Reflect::set(&wasm, &"open".into(), &open_fn).unwrap();
    js_sys::Reflect::set(&wasm, &"close".into(), &close_fn).unwrap();
    js_sys::Reflect::set(&wasm, &"exec".into(), &exec_fn).unwrap();
    js_sys::Reflect::set(&wasm, &"query".into(), &query_fn).unwrap();
    
    // Expõe o objeto wasm globalmente
    js_sys::Reflect::set(&global, &"wasm".into(), &wasm).unwrap();
}
