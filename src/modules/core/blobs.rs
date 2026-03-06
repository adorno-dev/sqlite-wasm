//! Gerenciamento de Blob URLs para arquivos embutidos
//!
//! Este módulo fornece uma interface limpa para criar Blob URLs
//! a partir de dados embutidos com include_bytes!.

use wasm_bindgen::prelude::*;
use web_sys::{Blob, Url};

/// Estrutura que gerencia um conjunto de Blob URLs
pub struct EmbeddedAssets {
    worker_url: String,
    js_url: String,
    proxy_url: String,
    wasm_url: String,
}

impl EmbeddedAssets {
    /// Cria todas as Blob URLs a partir dos dados embutidos
    pub fn new() -> Result<Self, JsValue> {
        // Dados embutidos (poderiam vir de um módulo separado)
        const SQLITE_JS: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.js");
        const SQLITE_WASM: &[u8] = include_bytes!("../../../static/sqlite.org/sqlite3.wasm");
        const SQLITE_WORKER: &[u8] =
            include_bytes!("../../../static/sqlite.org/sqlite3-worker1.js");
        const SQLITE_OPFS_ASYNC_PROXY: &[u8] =
            include_bytes!("../../../static/sqlite.org/sqlite3-opfs-async-proxy.js");

        Ok(Self {
            worker_url: Self::create_worker_url(SQLITE_WORKER)?,
            js_url: Self::create_js_url(SQLITE_JS)?,
            proxy_url: Self::create_proxy_url(SQLITE_OPFS_ASYNC_PROXY)?,
            wasm_url: Self::create_wasm_url(SQLITE_WASM)?,
        })
    }

    /// URL do worker principal
    pub fn worker_url(&self) -> &str {
        &self.worker_url
    }

    /// URL do arquivo sqlite3.js
    pub fn js_url(&self) -> &str {
        &self.js_url
    }

    /// URL do proxy OPFS
    pub fn proxy_url(&self) -> &str {
        &self.proxy_url
    }

    /// URL do binário WASM
    pub fn wasm_url(&self) -> &str {
        &self.wasm_url
    }

    /// Cria Blob URL para o worker (texto)
    fn create_worker_url(data: &[u8]) -> Result<String, JsValue> {
        let code = String::from_utf8_lossy(data).to_string(); // ← .to_string() AQUI!
        let array = js_sys::Array::of1(&code.into());
        let blob = Blob::new_with_str_sequence(&array)?;
        Url::create_object_url_with_blob(&blob)
    }

    /// Cria Blob URL para o proxy OPFS
    fn create_proxy_url(data: &[u8]) -> Result<String, JsValue> {
        let code = String::from_utf8_lossy(data).to_string(); // ← .to_string() AQUI!
        let array = js_sys::Array::of1(&code.into());
        let blob = Blob::new_with_str_sequence(&array)?;
        Url::create_object_url_with_blob(&blob)
    }

    /// Cria Blob URL para arquivos WASM (binário)
    fn create_wasm_url(data: &[u8]) -> Result<String, JsValue> {
        let array = js_sys::Uint8Array::from(data);
        let js_array = js_sys::Array::of1(&array.into());

        // Cria o blob com as opções
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("application/wasm");

        let blob = Blob::new_with_u8_array_sequence_and_options(&js_array, &options)?;

        Url::create_object_url_with_blob(&blob)
    }

    /// Cria Blob URL para arquivos JavaScript
    fn create_js_url(data: &[u8]) -> Result<String, JsValue> {
        let code = String::from_utf8_lossy(data).to_string();
        let array = js_sys::Array::of1(&code.into());

        let options = web_sys::BlobPropertyBag::new();
        options.set_type("application/javascript");

        let blob = Blob::new_with_str_sequence_and_options(&array, &options)?;

        Url::create_object_url_with_blob(&blob)
    }
}

// pub fn create_embedded_worker(assets: &EmbeddedAssets) -> Result<String, JsValue> {
//     // 1. Cria blob com MIME type explícito
//     let options = web_sys::BlobPropertyBag::new();
//     options.set_type("application/javascript");
//
//     // 2. Wrapper COMPLETO com TODOS os arquivos mapeados
//     let worker_code = format!(
//         r#"
//         // MAPEAMENTO DE TODOS OS ARQUIVOS
//         const FILES = {{
//             'sqlite3.js': '{}',
//             'sqlite3.wasm': '{}',
//             'sqlite3-opfs-async-proxy.js': '{}',
//             'sqlite3-worker1.js': '{}'
//         }};
//
//         // Intercepta importScripts
//         const originalImportScripts = importScripts;
//         importScripts = function(...urls) {{
//             const mapped = urls.map(url => FILES[url] || url);
//             return originalImportScripts.apply(this, mapped);
//         }};
//
//         // Intercepta fetch
//         const originalFetch = fetch;
//         fetch = function(url, options) {{
//             const mappedUrl = FILES[url] || url;
//             return originalFetch.call(this, mappedUrl, options);
//         }};
//
//         // Intercepta Worker
//         const originalWorker = Worker;
//         Worker = function(url, options) {{
//             const mappedUrl = FILES[url] || url;
//             return new originalWorker(mappedUrl, options);
//         }};
//
//         // Carrega o worker original (que também está mapeado)
//         importScripts('{}');
//         "#,
//         assets.js_url(),
//         assets.wasm_url(),
//         assets.proxy_url(),
//         assets.worker_url(),  // ← 4 ARGUMENTOS pro FILES
//         assets.worker_url()    // ← 5º ARGUMENTO pro importScripts
//     );
//
//     let worker_blob = web_sys::Blob::new_with_str_sequence_and_options(
//         &js_sys::Array::of1(&worker_code.into()),
//         &options
//     )?;
//
//     Ok(web_sys::Url::create_object_url_with_blob(&worker_blob)?)
// }

/// Detecta se é Firefox
fn is_firefox() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
        .contains("Firefox")
}

// pub fn create_embedded_worker(assets: &EmbeddedAssets) -> Result<String, JsValue> {
//     let is_ff = is_firefox();
//
//     let options = web_sys::BlobPropertyBag::new();
//     options.set_type("application/javascript");
//
//     let worker_code = format!(
//         r#"
//         // MAPEAMENTO DE TODOS OS ARQUIVOS
//         const FILES = {{
//             'sqlite3.js': '{}',
//             'sqlite3.wasm': '{}',
//             'sqlite3-opfs-async-proxy.js': '{}',
//             'sqlite3-worker1.js': '{}'
//         }};
//
//         // LOG PRA VER SE O MAPEAMENTO EXISTE
//         console.log('📦 FILES mapeados:', FILES);
//
//         // Intercepta importScripts COM DEBUG
//         const originalImportScripts = importScripts;
//         importScripts = function(...urls) {{
//             console.log('🔄 importScripts chamado com:', urls);
//             const mapped = urls.map(url => {{
//                 console.log('  ↪ mapeando:', url, '→', FILES[url] || url);
//                 return FILES[url] || url;
//             }});
//             console.log('  ✅ URLs mapeadas:', mapped);
//             return originalImportScripts.apply(this, mapped);
//         }};
//
//         // Intercepta fetch
//         const originalFetch = fetch;
//         fetch = function(url, options) {{
//             const mappedUrl = FILES[url] || url;
//             return originalFetch.call(this, mappedUrl, options);
//         }};
//
//         // Intercepta Worker
//         const originalWorker = Worker;
//         Worker = function(url, options) {{
//             const mappedUrl = FILES[url] || url;
//             return new originalWorker(mappedUrl, options);
//         }};
//
//         {}('{}');
//         "#,
//         assets.js_url(),
//         assets.wasm_url(),
//         assets.proxy_url(),
//         assets.worker_url(),
//         if is_ff { "import" } else { "importScripts" },
//         assets.worker_url()
//     );
//
//     let worker_blob = web_sys::Blob::new_with_str_sequence_and_options(
//         &js_sys::Array::of1(&worker_code.into()),
//         &options
//     )?;
//
//     Ok(web_sys::Url::create_object_url_with_blob(&worker_blob)?)
// }

pub fn create_embedded_worker(assets: &EmbeddedAssets) -> Result<String, JsValue> {
    let is_ff = is_firefox();

    // 🔥 FIREFOX PRECISA DE text/javascript, CHROME ACEITA application/javascript
    let options = web_sys::BlobPropertyBag::new();
    if is_ff {
        options.set_type("text/javascript");
    } else {
        options.set_type("application/javascript");
    }

    let worker_code = format!(
        r#"   
    // MAPEAMENTO DE TODOS OS ARQUIVOS
    const FILES = {{
        'sqlite3.js': '{}',
        'sqlite3.wasm': '{}',
        'sqlite3-opfs-async-proxy.js': '{}',   
        'sqlite3-worker1.js': '{}'
    }};   
    
    console.log('📦 FILES mapeados:', FILES);
    
    const originalImportScripts = importScripts;
    importScripts = function(...urls) {{
        console.log('🔄 importScripts chamado com:', urls);
        const mapped = urls.map(url => {{
            console.log('  ↪ mapeando:', url, '→', FILES[url] || url);
            return FILES[url] || url;
        }});
        console.log('  ✅ URLs mapeadas:', mapped);
        return originalImportScripts.apply(this, mapped);
    }};
    
    const originalFetch = fetch;
    fetch = function(url, options) {{
        const mappedUrl = FILES[url] || url;
        return originalFetch.call(this, mappedUrl, options);
    }};
    
    const originalWorker = Worker;
    Worker = function(url, options) {{
        const mappedUrl = FILES[url] || url;
        return new originalWorker(mappedUrl, options);
    }};
    
    {}('{}');
    "#,
        assets.js_url(),
        assets.wasm_url(),
        assets.proxy_url(),
        assets.worker_url(),
        if is_ff { "import" } else { "importScripts" },
        assets.worker_url()
    );

    let worker_blob = web_sys::Blob::new_with_str_sequence_and_options(
        &js_sys::Array::of1(&worker_code.into()),
        &options,
    )?;

    Ok(web_sys::Url::create_object_url_with_blob(&worker_blob)?)
}

macro_rules! revoke_urls {
    ($($url:expr),*) => {
        $(let _ = Url::revoke_object_url($url);)*
    };
}

impl Drop for EmbeddedAssets {
    fn drop(&mut self) {
        revoke_urls!(
            &self.worker_url,
            &self.js_url,
            &self.proxy_url,
            &self.wasm_url
        );
    }
}
