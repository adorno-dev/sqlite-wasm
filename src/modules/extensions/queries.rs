//! Optimized query modes (array-based, 2x faster)
//! Column name caching for repeated queries

use js_sys::{Array, Object, Reflect};
use std::collections::HashMap;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

use crate::modules::core::database::{is_db_open, next_message_id};
use crate::modules::core::worker::w_msg;

lazy_static::lazy_static! {
    /// Cache for column names of frequent queries
    static ref COLUMN_CACHE: Mutex<HashMap<String, Vec<String>>> = Mutex::new(HashMap::new());
}

/// Executes a SQL query and returns rows as arrays (2x faster).
#[wasm_bindgen(js_name = "query_array")]
pub async fn query_array(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if !is_db_open() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let args_obj = create_query_args(sql, args, "array")?;
    w_msg("exec".to_string(), args_obj.into()).await
}

/// Executes a SQL query and returns rows as objects (optimized with caching).
#[wasm_bindgen(js_name = "query_optimized")]
pub async fn query_optimized(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if !is_db_open() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    // Try cache first
    let cache_key = format!("{}-{:?}", sql, args);
    if let Some(cols) = COLUMN_CACHE.lock().unwrap().get(&cache_key).cloned() {
        let array_result = query_array(sql, args).await?;
        return convert_arrays_to_objects(array_result, cols);
    }

    // Cache miss: get arrays AND column names
    let args_obj = create_query_args_with_columns(sql, args)?;
    let result = w_msg("exec".to_string(), args_obj.into()).await?;

    // Extract and cache column names
    if let Some(cols) = extract_column_names(&result) {
        COLUMN_CACHE.lock().unwrap().insert(cache_key, cols.clone());
        return convert_arrays_to_objects(result, cols);
    }

    Ok(result)
}

// ==================== HELPER FUNCTIONS ====================

fn create_query_args(sql: &str, args: Vec<JsValue>, mode: &str) -> Result<Object, JsValue> {
    let id = next_message_id();
    let args_obj = Object::new();

    Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
    Reflect::set(&args_obj, &"type".into(), &"exec".into())?;
    Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql))?;
    Reflect::set(&args_obj, &"rowMode".into(), &JsValue::from_str(mode))?;

    if !args.is_empty() {
        let args_array = Array::new();
        for arg in args {
            args_array.push(&arg);
        }
        Reflect::set(&args_obj, &"bind".into(), &args_array)?;
    }

    Ok(args_obj)
}

fn create_query_args_with_columns(sql: &str, args: Vec<JsValue>) -> Result<Object, JsValue> {
    let id = next_message_id();
    let args_obj = Object::new();

    Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
    Reflect::set(&args_obj, &"type".into(), &"exec".into())?;
    Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql))?;
    Reflect::set(&args_obj, &"rowMode".into(), &"array".into())?;
    Reflect::set(&args_obj, &"columnNames".into(), &JsValue::TRUE)?;

    if !args.is_empty() {
        let args_array = Array::new();
        for arg in args {
            args_array.push(&arg);
        }
        Reflect::set(&args_obj, &"bind".into(), &args_array)?;
    }

    Ok(args_obj)
}

fn extract_column_names(result: &JsValue) -> Option<Vec<String>> {
    let mut cols = Vec::new();

    if let Ok(result_obj) = js_sys::Reflect::get(result, &"result".into()) {
        if let Ok(result_obj) = result_obj.dyn_into::<js_sys::Object>() {
            if let Ok(cols_val) = js_sys::Reflect::get(&result_obj, &"columnNames".into()) {
                if let Ok(cols_array) = cols_val.dyn_into::<js_sys::Array>() {
                    for i in 0..cols_array.length() {
                        if let Some(col) = cols_array.get(i).as_string() {
                            cols.push(col);
                        }
                    }
                    return Some(cols);
                }
            }
        }
    }

    None
}

pub fn convert_arrays_to_objects(
    result: JsValue,
    column_names: Vec<String>,
) -> Result<JsValue, JsValue> {
    let rows_array = js_sys::Array::new();

    if let Ok(result_obj) = js_sys::Reflect::get(&result, &"result".into()) {
        if let Ok(result_obj) = result_obj.dyn_into::<js_sys::Object>() {
            if let Ok(rows_val) = js_sys::Reflect::get(&result_obj, &"resultRows".into()) {
                if let Ok(arrays) = rows_val.dyn_into::<js_sys::Array>() {
                    for i in 0..arrays.length() {
                        if let Ok(array) = arrays.get(i).dyn_into::<js_sys::Array>() {
                            let obj = js_sys::Object::new();
                            for (j, col) in column_names.iter().enumerate() {
                                let j_u32 = j as u32;
                                if j_u32 < array.length() {
                                    let val = array.get(j_u32);
                                    Reflect::set(&obj, &col.clone().into(), &val)?;
                                }
                            }
                            rows_array.push(&obj);
                        }
                    }
                }
            }
        }
    }

    let final_result = js_sys::Object::new();
    Reflect::set(&final_result, &"resultRows".into(), &rows_array)?;

    Ok(final_result.into())
}
