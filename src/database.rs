use std::sync::{Mutex, atomic::{AtomicU32, Ordering}};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use js_sys::{Object, Array, Reflect};

use crate::worker::w_msg;

static DB_UID: Mutex<Option<JsValue>> = Mutex::new(None);
static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Opens the database with OPFS (Origin Private File System)
#[wasm_bindgen]
pub async fn open(database_name: &str) -> Result<(), JsValue> {
    // If already opened, return immediately
    if DB_UID.lock().unwrap().is_some() {
        return Ok(());
    }

    let args = Object::new();
    Reflect::set(&args, &"filename".into(), &JsValue::from_str(database_name))?;
    Reflect::set(&args, &"vfs".into(), &JsValue::from_str("opfs"))?;

    let open_result = w_msg("open".to_string(), args.into()).await?;
    
    let result_field = Reflect::get(&open_result, &"result".into()).ok();
    
    let uid_value = if let Some(result_obj) = result_field {
        let nested = Reflect::get(&result_obj, &"dbId".into()).ok();
        nested
            .unwrap_or_else(|| Reflect::get(&open_result, &"dbId".into()).unwrap_or(JsValue::NULL))
    } else {
        Reflect::get(&open_result, &"dbId".into()).unwrap_or(JsValue::NULL)
    };

    *DB_UID.lock().unwrap() = Some(uid_value);
    
    Ok(())
}

/// Closes the database
#[wasm_bindgen]
pub async fn close() -> Result<(), JsValue> {
    let uid = DB_UID.lock().unwrap().take();

    if let Some(uid) = uid {
        let args = Object::new();
        Reflect::set(&args, &"dbId".into(), &uid)?;
        // ✅ Directly await w_msg
        w_msg("close".to_string(), args.into()).await?;
    }

    Ok(())
}

/// Executes a SQL statement without returning rows
#[wasm_bindgen]
pub async fn exec(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if DB_UID.lock().unwrap().is_none() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    
    let args_obj = Object::new();
    Reflect::set(&args_obj, &"id".into(), &JsValue::from(id)).ok();
    Reflect::set(&args_obj, &"type".into(), &"exec".into()).ok();
    Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql)).ok();
    
    if !args.is_empty() {
        let args_array = Array::new();
        for arg in args {
            args_array.push(&arg);
        }
        Reflect::set(&args_obj, &"bind".into(), &args_array).ok();
    }
    
    // ✅ Directly await w_msg
    w_msg("exec".to_string(), args_obj.into()).await
}

/// Executes a SQL query and returns rows
#[wasm_bindgen]
pub async fn query(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
    if DB_UID.lock().unwrap().is_none() {
        return Err(JsValue::from_str("Database not opened. Call open() first."));
    }

    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    
    let args_obj = Object::new();
    Reflect::set(&args_obj, &"id".into(), &JsValue::from(id)).ok();
    Reflect::set(&args_obj, &"type".into(), &"exec".into()).ok();
    Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql)).ok();
    Reflect::set(&args_obj, &"rowMode".into(), &"object".into()).ok();
    
    if !args.is_empty() {
        let args_array = Array::new();
        for arg in args {
            args_array.push(&arg);
        }
        Reflect::set(&args_obj, &"bind".into(), &args_array).ok();
    }
    
    // ✅ Directly await w_msg
    w_msg("exec".to_string(), args_obj.into()).await
}

#[wasm_bindgen]
pub fn is_open() -> bool {
    DB_UID.lock().unwrap().is_some()
}

#[wasm_bindgen]
pub fn db_id() -> JsValue {
    DB_UID.lock().unwrap().clone().unwrap_or(JsValue::NULL)
}
