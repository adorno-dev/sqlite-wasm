//! Batch inserts (10x faster for large datasets)

use wasm_bindgen::prelude::*;

use crate::modules::core::database::{exec, is_db_open};

/// Default chunk size for batch inserts
const DEFAULT_CHUNK_SIZE: usize = 500;

/// Inserts multiple rows in a single transaction (10x faster).
pub async fn insert_batch(sql: &str, batch_params: Vec<Vec<JsValue>>) -> Result<(), JsValue> {
    if !is_db_open() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    if batch_params.is_empty() {
        return Ok(());
    }

    exec("BEGIN IMMEDIATE TRANSACTION", vec![]).await?;

    for params in batch_params {
        if let Err(e) = exec(sql, params).await {
            exec("ROLLBACK", vec![]).await?;
            return Err(e);
        }
    }

    exec("COMMIT", vec![]).await?;
    Ok(())
}

/// Rust-native chunked batch insert.
pub async fn insert_batch_chunked(
    sql: &str,
    batch_params: Vec<Vec<JsValue>>,
    chunk_size: usize,
) -> Result<(), JsValue> {
    if !is_db_open() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    if batch_params.is_empty() {
        return Ok(());
    }

    for chunk in batch_params.chunks(chunk_size) {
        exec("BEGIN IMMEDIATE TRANSACTION", vec![]).await?;

        for params in chunk {
            if let Err(e) = exec(sql, params.to_vec()).await {
                exec("ROLLBACK", vec![]).await?;
                return Err(e);
            }
        }

        exec("COMMIT", vec![]).await?;
    }

    Ok(())
}

/// JS-friendly version of batch insert.
#[wasm_bindgen(js_name = "insert_batch")]
pub async fn insert_batch_js(sql: &str, batch_params: js_sys::Array) -> Result<(), JsValue> {
    let rust_params = js_array_to_vec(batch_params)?;
    insert_batch(sql, rust_params).await
}

/// Batch insert with chunking for very large datasets.
#[wasm_bindgen(js_name = "insert_batch_chunked")]
pub async fn insert_batch_chunked_js(
    sql: &str,
    batch_params: js_sys::Array,
    chunk_size: Option<usize>,
) -> Result<(), JsValue> {
    let chunk_size = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);
    let rust_params = js_array_to_vec(batch_params)?;
    insert_batch_chunked(sql, rust_params, chunk_size).await
}

fn js_array_to_vec(array: js_sys::Array) -> Result<Vec<Vec<JsValue>>, JsValue> {
    let mut result = Vec::new();

    for i in 0..array.length() {
        let row = array.get(i);
        if let Some(row_array) = row.dyn_ref::<js_sys::Array>() {
            let mut row_vec = Vec::new();
            for j in 0..row_array.length() {
                row_vec.push(row_array.get(j));
            }
            result.push(row_vec);
        } else {
            return Err(JsValue::from_str("Each row must be an array"));
        }
    }

    Ok(result)
}
