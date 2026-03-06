//! Prepared statements (1.5x faster for repeated queries)

use js_sys::{Array, Object, Reflect};
use std::collections::HashMap;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

use crate::modules::core::database::{is_db_open, next_message_id};
use crate::modules::core::worker::w_msg;
use crate::modules::extensions::queries::convert_arrays_to_objects;

lazy_static::lazy_static! {
    /// Cache for prepared statements
    static ref STATEMENT_CACHE: Mutex<HashMap<String, i32>> = Mutex::new(HashMap::new());
}

/// A prepared statement that can be executed multiple times.
///
/// This struct is used internally by `JsPreparedStatement`.
pub struct PreparedStatement {
    id: i32,
    #[allow(dead_code)]
    sql: String,
}

impl PreparedStatement {
    /// Creates a new prepared statement from SQL.
    pub async fn prepare(sql: &str) -> Result<Self, JsValue> {
        if !is_db_open() {
            return Err(JsValue::from_str("Database not opened. Call open() first."));
        }

        // Check cache first
        if let Some(&id) = STATEMENT_CACHE.lock().unwrap().get(sql) {
            return Ok(Self {
                id,
                sql: sql.to_string(),
            });
        }

        // Prepare new statement
        let id = next_message_id() as i32;

        let args_obj = Object::new();
        Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
        Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql))?;
        Reflect::set(&args_obj, &"type".into(), &"prepare".into())?;

        let result = w_msg("prepare".to_string(), args_obj.into()).await?;

        // Extract statement ID
        let stmt_id = extract_stmt_id(&result, id)?;

        STATEMENT_CACHE
            .lock()
            .unwrap()
            .insert(sql.to_string(), stmt_id);

        Ok(Self {
            id: stmt_id,
            sql: sql.to_string(),
        })
    }

    /// Executes the prepared statement with parameters.
    pub async fn execute(&self, params: Vec<JsValue>) -> Result<JsValue, JsValue> {
        if !is_db_open() {
            return Err(JsValue::from_str("Database not opened. Call open() first."));
        }

        let id = next_message_id();

        let args_obj = Object::new();
        Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
        Reflect::set(&args_obj, &"stmtId".into(), &JsValue::from(self.id))?;
        Reflect::set(&args_obj, &"type".into(), &"execute".into())?;

        if !params.is_empty() {
            let args_array = Array::new();
            for arg in params {
                args_array.push(&arg);
            }
            Reflect::set(&args_obj, &"bind".into(), &args_array)?;
        }

        w_msg("execute".to_string(), args_obj.into()).await
    }

    /// Executes a SELECT and returns rows as objects.
    pub async fn query(&self, params: Vec<JsValue>) -> Result<JsValue, JsValue> {
        let result = self.execute(params).await?;

        if let Some(cols) = extract_column_names(&result) {
            convert_arrays_to_objects(result, cols)
        } else {
            Ok(result)
        }
    }

    /// Clears the statement cache.
    pub fn clear_cache() {
        STATEMENT_CACHE.lock().unwrap().clear();
    }
}

// ==================== HELPER FUNCTIONS ====================

fn extract_stmt_id(result: &JsValue, default: i32) -> Result<i32, JsValue> {
    if let Ok(result_obj) = js_sys::Reflect::get(result, &"result".into()) {
        if let Ok(result_obj) = result_obj.dyn_into::<js_sys::Object>() {
            if let Ok(id_val) = js_sys::Reflect::get(&result_obj, &"stmtId".into()) {
                return Ok(id_val.as_f64().unwrap_or(default as f64) as i32);
            }
        }
    }
    Ok(default)
}

fn extract_column_names(result: &JsValue) -> Option<Vec<String>> {
    if let Ok(result_obj) = js_sys::Reflect::get(result, &"result".into()) {
        if let Ok(result_obj) = result_obj.dyn_into::<js_sys::Object>() {
            if let Ok(cols_val) = js_sys::Reflect::get(&result_obj, &"columnNames".into()) {
                if let Ok(cols_array) = cols_val.dyn_into::<js_sys::Array>() {
                    let mut cols = Vec::new();
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

// ==================== JS EXPORTS ====================

/// JS-friendly version of prepared statement.
#[wasm_bindgen(js_name = "PreparedStatement")]
pub struct JsPreparedStatement {
    inner: PreparedStatement,
}

#[wasm_bindgen(js_class = "PreparedStatement")]
impl JsPreparedStatement {
    /// Prepares a SQL statement for execution.
    #[wasm_bindgen(js_name = "prepare")]
    pub async fn prepare(sql: &str) -> Result<JsPreparedStatement, JsValue> {
        Ok(JsPreparedStatement {
            inner: PreparedStatement::prepare(sql).await?,
        })
    }

    /// Executes the prepared statement with parameters.
    #[wasm_bindgen(js_name = "execute")]
    pub async fn execute(&self, params: js_sys::Array) -> Result<JsValue, JsValue> {
        let mut rust_params = Vec::new();
        for i in 0..params.length() {
            rust_params.push(params.get(i));
        }
        self.inner.execute(rust_params).await
    }

    /// Executes a SELECT query and returns rows as objects.
    #[wasm_bindgen(js_name = "query")]
    pub async fn query(&self, params: js_sys::Array) -> Result<JsValue, JsValue> {
        let mut rust_params = Vec::new();
        for i in 0..params.length() {
            rust_params.push(params.get(i));
        }
        self.inner.query(rust_params).await
    }
}
