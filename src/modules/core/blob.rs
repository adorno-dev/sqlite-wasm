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

    /// Cria Blob URL para arquivos JavaScript
    fn create_js_url(data: &[u8]) -> Result<String, JsValue> {
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
        let blob = Blob::new_with_u8_array_sequence(&js_array)?;
        Url::create_object_url_with_blob(&blob)
    }
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
