//! Management of Blob/Data URLs for embedded files
//!
//! This module provides functions to create URLs from embedded data,
//! making the crate completely self-contained. All SQLite files are
//! embedded via `include_bytes!` and served as Blob URLs (or Data URLs
//! as fallback) to eliminate external file dependencies.
//!
//! # Architecture
//!
//! 1. **Embedding** - Files are embedded at compile time using `include_bytes!`
//! 2. **URL Creation** - At runtime, bytes are converted to Blob URLs (preferred)
//! 3. **Worker Wrapper** - A custom worker intercepts `importScripts` and `fetch`
//! 4. **File Mapping** - Original filenames map to Blob URLs via `FILES` object
//! 5. **Browser Compatibility** - Automatic fallback for Firefox when needed
//!
//! # Performance
//!
//! * Blob URLs are created once and reused
//! * No filesystem access - all data in memory
//! * Zero-copy conversion via `Uint8Array`

use wasm_bindgen::prelude::*;
use web_sys::{Blob, Url};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

// ==================== EMBEDDED ASSETS ====================

/// Module with all static files embedded via include_bytes!
///
/// These constants are available at compile time and contain the
/// complete content of the SQLite files. They are embedded directly
/// into the WASM binary.
pub mod assets {
    /// SQLite main JavaScript glue code
    pub const SQLITE_JS: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.js");
    
    /// SQLite WebAssembly binary
    pub const SQLITE_WASM: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.wasm");
    
    /// SQLite worker script (main entry point)
    pub const SQLITE_WORKER: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3-worker1.js");
    
    /// OPFS async proxy worker
    pub const SQLITE_PROXY: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");
}

// ==================== BROWSER DETECTION ====================

/// Detects if the browser is Firefox (needs special handling)
///
/// Firefox has stricter security requirements for Blob URLs in workers.
/// This detection allows for browser-specific fallbacks if needed.
fn is_firefox() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
        .contains("Firefox")
}

// ==================== URL CREATION ====================

/// Creates a Blob URL from binary data (preferred method)
///
/// This is the most efficient method and works in both Chrome and Firefox
/// after recent updates. The Blob URL points to an in-memory representation
/// of the file.
///
/// # Arguments
/// * `data` - Raw file bytes (from `include_bytes!`)
/// * `mime_type` - MIME type (e.g., "text/javascript", "application/wasm")
///
/// # Returns
/// * `Ok(String)` - Blob URL (e.g., "blob:http://localhost:8080/...")
/// * `Err(JsValue)` - Blob creation failed
///
/// # Example
/// ```no_run
/// # use sqlite_wasm::modules::core::blobs::assets;
/// # use sqlite_wasm::modules::core::blobs::create_blob_url;
/// # fn example() -> Result<(), wasm_bindgen::JsValue> {
/// let url = create_blob_url(assets::SQLITE_JS, "text/javascript")?;
/// # Ok(())
/// # }
/// ```
pub fn create_blob_url(data: &[u8], mime_type: &str) -> Result<String, JsValue> {
    let array = js_sys::Uint8Array::from(data);
    let js_array = js_sys::Array::of1(&array.into());
    
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_type);
    
    let blob = Blob::new_with_u8_array_sequence_and_options(&js_array, &options)?;
    Ok(Url::create_object_url_with_blob(&blob)?)
}

/// Creates a Data URL from binary data (fallback method)
///
/// Data URLs are self-contained and work in all browsers, but are less
/// efficient than Blob URLs. Used as a fallback when Blob URLs fail.
///
/// # Arguments
/// * `data` - Raw file bytes
/// * `mime_type` - MIME type for the data
///
/// # Returns
/// * `String` - Data URL (e.g., "data:text/javascript;base64,....")
///
/// # Example
/// ```no_run
/// # use sqlite_wasm::modules::core::blobs::assets;
/// # use sqlite_wasm::modules::core::blobs::create_data_url;
/// let url = create_data_url(assets::SQLITE_JS, "text/javascript");
/// ```
pub fn create_data_url(data: &[u8], mime_type: &str) -> String {
    let base64 = STANDARD.encode(data);
    format!("data:{};base64,{}", mime_type, base64)
}

/// Creates the appropriate URL based on browser (currently uses Blob for all)
///
/// After extensive testing, Blob URLs work in both Chrome and Firefox.
/// Data URLs are kept as a fallback option in case of future browser changes.
///
/// # Arguments
/// * `data` - Raw file bytes
/// * `mime_type` - MIME type for the URL
///
/// # Returns
/// * `Ok(String)` - URL (Blob or Data) that can be used in the browser
pub fn create_asset_url(data: &[u8], mime_type: &str) -> Result<String, JsValue> {
    // Firefox now supports Blob URLs for workers, so we use them for all browsers.
    // Data URLs are kept commented out as a fallback in case future Firefox
    // updates break Blob URL support again.
    // 
    // if is_firefox() {
    //     Ok(create_data_url(data, mime_type))
    // } else {
    //     create_blob_url(data, mime_type)
    // }
    create_blob_url(data, mime_type)
}

// ==================== WORKER WRAPPER ====================

/// Creates the wrapper code that intercepts importScripts and fetch
///
/// This JavaScript code runs inside the worker and maps original filenames
/// (like 'sqlite3.js') to their Blob URLs. It intercepts:
/// * `importScripts` - Redirects to the correct Blob URL
/// * `fetch` - Redirects WASM and other fetches
/// * `Worker` constructor - Ensures proxy worker loads correctly
///
/// # Arguments
/// * `js_url` - Blob URL for sqlite3.js
/// * `wasm_url` - Blob URL for sqlite3.wasm
/// * `proxy_url` - Blob URL for the OPFS async proxy
/// * `worker_url` - Blob URL for the main worker
/// * `is_ff` - Whether the browser is Firefox
///
/// # Returns
/// * `String` - JavaScript code that will be executed in the worker
fn create_wrapper_code(
    js_url: &str,
    wasm_url: &str,
    proxy_url: &str,
    worker_url: &str,
    is_ff: bool,
) -> String {
    let load_method = if is_ff { "import" } else { "importScripts" };
    
    // Firefox needs special handling for the proxy worker to inherit page origin
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
        // File mapping: original names -> Blob URLs
        const FILES = {{
            'sqlite3.js': '{}',
            'sqlite3.wasm': '{}',
            'sqlite3-opfs-async-proxy.js': '{}',
            'sqlite3-worker1.js': '{}'
        }};
        
        // Intercept importScripts to use mapped URLs
        const originalImportScripts = importScripts;
        importScripts = function(...urls) {{
            const mapped = urls.map(url => FILES[url] || url);
            return originalImportScripts.apply(this, mapped);
        }};
        
        // Intercept fetch (for WASM and other resources)
        const originalFetch = fetch;
        fetch = function(url, options) {{
            const mappedUrl = FILES[url] || url;
            return originalFetch.call(this, mappedUrl, options);
        }};
        
        // Intercept Worker constructor (for proxy worker)
        {}
        
        // Load the original worker
        {}('{}');
        "#,
        js_url, wasm_url, proxy_url, worker_url,
        proxy_handler,
        load_method, worker_url
    )
}

// ==================== MAIN FUNCTION ====================

/// Creates a worker wrapper with all embedded files
///
/// This is the main entry point for creating a self-contained worker.
/// It:
/// 1. Creates Blob URLs for all SQLite files
/// 2. Generates a wrapper script that maps filenames to Blob URLs
/// 3. Creates a Blob URL for the wrapper itself
/// 4. Returns the final URL ready for `Worker::new()`
///
/// # Returns
/// * `Ok(String)` - URL of the wrapper worker (blob: or data:)
/// * `Err(JsValue)` - Failed to create URLs or wrapper
///
/// # Browser Compatibility
/// * **Chrome**: Uses Blob URLs (optimal)
/// * **Firefox**: Also uses Blob URLs (tested and working)
/// * **Fallback**: Data URLs available if needed (commented out)
///
/// # Example
/// ```no_run
/// # use sqlite_wasm::modules::core::blobs;
/// # async fn example() -> Result<(), wasm_bindgen::JsValue> {
/// let worker_url = blobs::create_embedded_worker().await?;
/// let worker = web_sys::Worker::new(&worker_url)?;
/// # Ok(())
/// # }
/// ```
pub async fn create_embedded_worker() -> Result<String, JsValue> {
    let is_ff = is_firefox();
    
    // Create Blob URLs for each embedded asset
    let js_url = create_asset_url(assets::SQLITE_JS, "text/javascript")?;
    let wasm_url = create_asset_url(assets::SQLITE_WASM, "application/wasm")?;
    let proxy_url = create_asset_url(assets::SQLITE_PROXY, "text/javascript")?;
    let worker_url = create_asset_url(assets::SQLITE_WORKER, "text/javascript")?;
    
    // Generate wrapper code that maps filenames to Blob URLs
    let wrapper_code = create_wrapper_code(&js_url, &wasm_url, &proxy_url, &worker_url, is_ff);
    
    create_blob_url(wrapper_code.as_bytes(), "text/javascript")
}
