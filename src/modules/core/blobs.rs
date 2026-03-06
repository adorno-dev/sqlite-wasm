//! Management of Blob/Data URLs for embedded files
//!
//! This module provides functions to create URLs from embedded data,
//! automatically choosing between Blob (Chrome) and Data URL (Firefox).

use wasm_bindgen::prelude::*;
use web_sys::{Blob, Url};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

// ==================== EMBEDDED ASSETS ====================

/// Module with all static files embedded via include_bytes!
pub mod assets {
    pub const SQLITE_JS: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.js");
    pub const SQLITE_WASM: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.wasm");
    pub const SQLITE_WORKER: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3-worker1.js");
    pub const SQLITE_PROXY: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");
}

// ==================== BROWSER DETECTION ====================

/// Detects if the browser is Firefox (needs Data URL)
fn is_firefox() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
        .contains("Firefox")
}

// ==================== URL CREATION ====================

/// Creates a Blob URL from binary data (ideal for Chrome)
pub fn create_blob_url(data: &[u8], mime_type: &str) -> Result<String, JsValue> {
    let array = js_sys::Uint8Array::from(data);
    let js_array = js_sys::Array::of1(&array.into());
    
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_type);
    
    let blob = Blob::new_with_u8_array_sequence_and_options(&js_array, &options)?;
    Ok(Url::create_object_url_with_blob(&blob)?)
}

/// Creates a Data URL from binary data (works in Firefox)
pub fn create_data_url(data: &[u8], mime_type: &str) -> String {
    let base64 = STANDARD.encode(data);
    format!("data:{};base64,{}", mime_type, base64)
}

/// Creates the appropriate URL based on browser
pub fn create_asset_url(data: &[u8], mime_type: &str) -> Result<String, JsValue> {
    // if is_firefox() {
    //     Ok(create_data_url(data, mime_type))
    // } else {
    //     create_blob_url(data, mime_type)
    // }
    create_blob_url(data, mime_type)
}

// ==================== WORKER WRAPPER ====================

/// Creates the wrapper code that intercepts importScripts and fetch
fn create_wrapper_code(
    js_url: &str,
    wasm_url: &str,
    proxy_url: &str,
    worker_url: &str,
    is_ff: bool,
) -> String {
    let load_method = if is_ff { "import" } else { "importScripts" };
    
    // For Firefox, the proxy needs special handling to inherit page origin
    let proxy_handler = if is_ff {
        r#"
        // Firefox: proxy needs to inherit page origin
        const originalWorker = Worker;
        Worker = function(url, options) {
            if (url === 'sqlite3-opfs-async-proxy.js') {
                // Use a blob with the SAME ORIGIN as the page
                const proxyBlob = new Blob(
                    [`importScripts('${FILES[url]}');`], 
                    { type: 'text/javascript' }
                );
                const proxyBlobUrl = URL.createObjectURL(proxyBlob);
                
                // Return a CLASSIC worker (not module)
                return new originalWorker(proxyBlobUrl, { type: 'classic' });
            }
            const mappedUrl = FILES[url] || url;
            return new originalWorker(mappedUrl, options);
        };
        "#
    } else {
        r#"
        // Chrome: works directly
        const originalWorker = Worker;
        Worker = function(url, options) {
            const mappedUrl = FILES[url] || url;
            return new originalWorker(mappedUrl, options);
        };
        "#
    };
    
    format!(
        r#"
        const FILES = {{
            'sqlite3.js': '{}',
            'sqlite3.wasm': '{}',
            'sqlite3-opfs-async-proxy.js': '{}',
            'sqlite3-worker1.js': '{}'
        }};
        
        const originalImportScripts = importScripts;
        importScripts = function(...urls) {{
            const mapped = urls.map(url => FILES[url] || url);
            return originalImportScripts.apply(this, mapped);
        }};
        
        const originalFetch = fetch;
        fetch = function(url, options) {{
            const mappedUrl = FILES[url] || url;
            return originalFetch.call(this, mappedUrl, options);
        }};
        
        {}
        
        {}('{}');
        "#,
        js_url, wasm_url, proxy_url, worker_url,
        proxy_handler,
        load_method, worker_url
    )
}

// ==================== MAIN FUNCTION ====================

/// Creates a worker wrapper with all embedded files
pub async fn create_embedded_worker() -> Result<String, JsValue> {
    let is_ff = is_firefox();
    
    // 1. Create URLs for each asset (Blob or Data URL based on browser)
    let js_url = create_asset_url(assets::SQLITE_JS, "text/javascript")?;
    let wasm_url = create_asset_url(assets::SQLITE_WASM, "application/wasm")?;
    let proxy_url = create_asset_url(assets::SQLITE_PROXY, "text/javascript")?;
    let worker_url = create_asset_url(assets::SQLITE_WORKER, "text/javascript")?;
    
    // 2. Generate wrapper code
    let wrapper_code = create_wrapper_code(&js_url, &wasm_url, &proxy_url, &worker_url, is_ff);
    
    // 3. Create final worker wrapper URL
    // if is_ff {
    //     // Firefox: Data URL (avoids origin issues)
    //     Ok(create_data_url(wrapper_code.as_bytes(), "text/javascript"))
    // } else {
    //     // Chrome: Blob URL (more efficient)
    //     create_blob_url(wrapper_code.as_bytes(), "text/javascript")
    // }
    create_blob_url(wrapper_code.as_bytes(), "text/javascript")
}
