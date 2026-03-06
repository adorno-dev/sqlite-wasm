use std::sync::atomic::{AtomicUsize, Ordering};
use std::rc::Rc;
use std::cell::RefCell;
use tokio::sync::Mutex as TokioMutex;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};
use wasm_bindgen::prelude::*;
use js_sys::{Object, Array, Reflect};
use futures_channel::oneshot;

use crate::modules::core::database::next_message_id;

/// A connection in the pool with its own worker and database ID
struct PoolConnection {
    worker: Worker,
    db_id: JsValue,
    in_use: Rc<TokioMutex<bool>>,
}

/// A pool of database connections for parallel queries.
///
/// This is useful for applications with many concurrent queries,
/// multi-threaded WebAssembly environments, or intranet applications with high load.
///
/// # Example
/// ```no_run
/// # async fn example() -> Result<(), wasm_bindgen::JsValue> {
/// use sqlite_wasm::modules::core::optimizations::ConnectionPool;
/// 
/// let pool = ConnectionPool::new("myapp.sqlite3", 4).await?;
/// 
/// // Run queries in parallel
/// let (users, products) = tokio::join!(
///     pool.query("SELECT * FROM users", vec![]),
///     pool.query("SELECT * FROM products", vec![])
/// );
/// 
/// // Check pool health
/// println!("{}", pool.stats());
/// # Ok(())
/// # }
/// ```
pub struct ConnectionPool {
    connections: Vec<PoolConnection>,
    current: AtomicUsize,
    db_name: String,
    pool_size: usize,
    timeout_ms: u32,
}

impl ConnectionPool {
    /// Creates a new connection pool with the specified size.
    ///
    /// # Arguments
    /// * `db_name` - Name of the database file (e.g., "app.sqlite3")
    /// * `size` - Number of connections in the pool (usually 2-4)
    ///
    /// # Returns
    /// * `Ok(ConnectionPool)` - Pool ready to use
    /// * `Err(JsValue)` - Failed to create pool
    pub async fn new(db_name: &str, size: usize) -> Result<Self, JsValue> {
        let mut connections = Vec::with_capacity(size);
        
        // Create arguments for opening database
        let args_obj = Object::new();
        Reflect::set(&args_obj, &"filename".into(), &JsValue::from_str(db_name))?;
        Reflect::set(&args_obj, &"vfs".into(), &JsValue::from_str("opfs"))?;
        
        for i in 0..size {
            web_sys::console::log_1(&format!("🔌 Creating pool connection {}...", i + 1).into());
            
            // Create worker with module type (better for isolation)
            let options = WorkerOptions::new();
            options.set_type(WorkerType::Module);
            
            let worker = Worker::new_with_options(
                "/static/sqlite.org/sqlite3-worker1.js",
                &options
            )?;
            
            // Wait for worker to be ready
            Self::wait_for_worker_ready(&worker).await?;
            
            // Open database on this worker
            let open_result = Self::send_message(&worker, "open".to_string(), args_obj.clone().into()).await?;
            let db_id = Self::extract_db_id(&open_result)?;
            
            connections.push(PoolConnection {
                worker,
                db_id,
                in_use: Rc::new(TokioMutex::new(false)),
            });
            
            web_sys::console::log_1(&format!("✅ Pool connection {} ready", i + 1).into());
        }
        
        Ok(Self {
            connections,
            current: AtomicUsize::new(0),
            db_name: db_name.to_string(),
            pool_size: size,
            timeout_ms: 30000, // 30 seconds default
        })
    }
    
    /// Sets the timeout for queries (in milliseconds)
    pub fn set_timeout(&mut self, timeout_ms: u32) {
        self.timeout_ms = timeout_ms;
    }
    
    /// Gets an available connection (round-robin with waiting)
    async fn get_connection(&self) -> Option<&PoolConnection> {
        let start_idx = self.current.fetch_add(1, Ordering::Relaxed) % self.connections.len();
        
        // Try all connections starting from start_idx
        for i in 0..self.connections.len() {
            let idx = (start_idx + i) % self.connections.len();
            let conn = &self.connections[idx];
            
            let mut lock = conn.in_use.lock().await;
            if !*lock {
                *lock = true;
                return Some(conn);
            }
        }
        
        None // All connections busy
    }
    
    /// Executes a query and returns rows as objects
    pub async fn query(&self, sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
        let conn = self.acquire_connection().await?;
        
        let id = next_message_id();
        let args_obj = Object::new();
        Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
        Reflect::set(&args_obj, &"type".into(), &"exec".into())?;
        Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql))?;
        Reflect::set(&args_obj, &"rowMode".into(), &"object".into())?;
        Reflect::set(&args_obj, &"dbId".into(), &conn.db_id)?;
        
        if !args.is_empty() {
            let args_array = Array::new();
            for arg in args {
                args_array.push(&arg);
            }
            Reflect::set(&args_obj, &"bind".into(), &args_array)?;
        }
        
        let result = Self::send_message(&conn.worker, "exec".to_string(), args_obj.into()).await?;
        
        // Release connection
        *conn.in_use.lock().await = false;
        
        Ok(result)
    }
    
    /// Executes a query and returns rows as arrays (2x faster)
    pub async fn query_array(&self, sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
        let conn = self.acquire_connection().await?;
        
        let id = next_message_id();
        let args_obj = Object::new();
        Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
        Reflect::set(&args_obj, &"type".into(), &"exec".into())?;
        Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql))?;
        Reflect::set(&args_obj, &"rowMode".into(), &"array".into())?;
        Reflect::set(&args_obj, &"dbId".into(), &conn.db_id)?;
        
        if !args.is_empty() {
            let args_array = Array::new();
            for arg in args {
                args_array.push(&arg);
            }
            Reflect::set(&args_obj, &"bind".into(), &args_array)?;
        }
        
        let result = Self::send_message(&conn.worker, "exec".to_string(), args_obj.into()).await?;
        
        // Release connection
        *conn.in_use.lock().await = false;
        
        Ok(result)
    }
    
    /// Executes a statement (INSERT, UPDATE, DELETE)
    pub async fn exec(&self, sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
        let conn = self.acquire_connection().await?;
        
        let id = next_message_id();
        let args_obj = Object::new();
        Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
        Reflect::set(&args_obj, &"type".into(), &"exec".into())?;
        Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(sql))?;
        Reflect::set(&args_obj, &"dbId".into(), &conn.db_id)?;
        
        if !args.is_empty() {
            let args_array = Array::new();
            for arg in args {
                args_array.push(&arg);
            }
            Reflect::set(&args_obj, &"bind".into(), &args_array)?;
        }
        
        let result = Self::send_message(&conn.worker, "exec".to_string(), args_obj.into()).await?;
        
        // Release connection
        *conn.in_use.lock().await = false;
        
        Ok(result)
    }
    
    /// Acquires a connection with timeout
    async fn acquire_connection(&self) -> Result<&PoolConnection, JsValue> {
        let start = web_sys::window().unwrap().performance().unwrap().now();
        
        loop {
            if let Some(conn) = self.get_connection().await {
                return Ok(conn);
            }
            
            // Check timeout
            let elapsed = web_sys::window().unwrap().performance().unwrap().now() - start;
            if elapsed > self.timeout_ms as f64 {
                return Err(JsValue::from_str(&format!(
                    "Connection timeout after {}ms", self.timeout_ms
                )));
            }
            
            // Wait 10ms before retrying
            wasm_bindgen_futures::JsFuture::from(
                js_sys::Promise::new(&mut |resolve, _| {
                    web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                        &resolve, 10
                    ).unwrap();
                })
            ).await?;
        }
    }
    
    /// Executes a query with retry logic
    pub async fn query_with_retry(&self, sql: &str, args: Vec<JsValue>, max_retries: usize) -> Result<JsValue, JsValue> {
        let mut last_error = None;
        
        for attempt in 1..=max_retries {
            match self.query(sql, args.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    web_sys::console::log_1(&format!("⚠️ Query failed (attempt {}/{}), retrying...", attempt, max_retries).into());
                    last_error = Some(e);
                    
                    // Exponential backoff: 100ms, 200ms, 400ms, 800ms...
                    let delay_ms = 100 * (2_u32.pow(attempt as u32 - 1));
                    
                    // 🔥 Converte u32 para i32 (delay máximo de ~2.1s)
                    let delay_i32 = delay_ms.min(2000) as i32;
                    
                    wasm_bindgen_futures::JsFuture::from(
                        js_sys::Promise::new(&mut |resolve, _| {
                            web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                                &resolve,
                                delay_i32  // ← agora é i32
                            ).unwrap();
                        })
                    ).await?;
                }
            }
        }
        
        Err(last_error.unwrap())
    }
    
    /// Gets detailed statistics about the pool
    pub fn stats(&self) -> String {
        let available = self.connections
            .iter()
            .filter(|c| {
                match c.in_use.try_lock() {
                    Ok(lock) => !*lock,
                    Err(_) => false,
                }
            })
            .count();
            
        format!(
            "Pool '{}': {}/{} connections available ({} busy) - timeout: {}ms",
            self.db_name,
            available,
            self.pool_size,
            self.pool_size - available,
            self.timeout_ms
        )
    }
    
    /// Returns current pool status as structured data
    pub fn stats_json(&self) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"dbName".into(), &JsValue::from_str(&self.db_name))?;
        js_sys::Reflect::set(&obj, &"poolSize".into(), &JsValue::from_f64(self.pool_size as f64))?;
        js_sys::Reflect::set(&obj, &"timeoutMs".into(), &JsValue::from_f64(self.timeout_ms as f64))?;
        
        let available = self.connections
            .iter()
            .filter(|c| {
                match c.in_use.try_lock() {
                    Ok(lock) => !*lock,
                    Err(_) => false,
                }
            })
            .count();
            
        js_sys::Reflect::set(&obj, &"available".into(), &JsValue::from_f64(available as f64))?;
        js_sys::Reflect::set(&obj, &"busy".into(), &JsValue::from_f64((self.pool_size - available) as f64))?;
        
        Ok(obj.into())
    }
    
    /// Checks health of all connections
    pub async fn health_check(&self) -> Result<Vec<bool>, JsValue> {
        let mut results = Vec::new();
        
        for conn in &self.connections {
            let test_sql = "SELECT 1";
            let id = next_message_id();
            
            let args_obj = Object::new();
            Reflect::set(&args_obj, &"id".into(), &JsValue::from(id))?;
            Reflect::set(&args_obj, &"type".into(), &"exec".into())?;
            Reflect::set(&args_obj, &"sql".into(), &JsValue::from_str(test_sql))?;
            Reflect::set(&args_obj, &"rowMode".into(), &"object".into())?;
            Reflect::set(&args_obj, &"dbId".into(), &conn.db_id)?;
            
            let result = Self::send_message(&conn.worker, "exec".to_string(), args_obj.into()).await;
            results.push(result.is_ok());
        }
        
        Ok(results)
    }
    
    /// Closes all connections in the pool
    pub async fn close_all(&self) -> Result<(), JsValue> {
        for conn in &self.connections {
            let args_obj = Object::new();
            Reflect::set(&args_obj, &"dbId".into(), &conn.db_id)?;
            let _ = Self::send_message(&conn.worker, "close".to_string(), args_obj.into()).await;
            
            // Ensure connection is marked as not in use
            *conn.in_use.lock().await = false;
        }
        Ok(())
    }
    
    // ==================== HELPER FUNCTIONS ====================
    
    async fn wait_for_worker_ready(worker: &Worker) -> Result<(), JsValue> {
        let (tx, rx) = oneshot::channel::<()>();
        let tx_rc = Rc::new(RefCell::new(Some(tx)));
        
        let closure = {
            let tx_rc = tx_rc.clone();
            Closure::wrap(Box::new(move |event: MessageEvent| {
                let data = event.data();
                if let Ok(type_val) = js_sys::Reflect::get(&data, &"type".into()) {
                    if type_val == JsValue::from_str("sqlite3-api") {
                        if let Ok(result_val) = js_sys::Reflect::get(&data, &"result".into()) {
                            if result_val == JsValue::from_str("worker1-ready") {
                                if let Some(tx) = tx_rc.borrow_mut().take() {
                                    let _ = tx.send(());
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>)
        };
        
        worker.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())?;
        closure.forget();
        
        // Timeout de 5 segundos
        let timeout_promise = js_sys::Promise::new(&mut |resolve, _| {
            web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve, 5000
            ).unwrap();
        });
        
        let timeout_future = wasm_bindgen_futures::JsFuture::from(timeout_promise);
        futures::pin_mut!(rx);
        futures::pin_mut!(timeout_future);
        
        match futures::future::select(rx, timeout_future).await {
            futures::future::Either::Left((Ok(()), _)) => Ok(()),
            _ => Err(JsValue::from_str("Worker ready timeout")),
        }
    }
    
    async fn send_message(worker: &Worker, msg_type: String, args: JsValue) -> Result<JsValue, JsValue> {
        let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();
        let tx_rc = Rc::new(RefCell::new(Some(tx)));
        
        let message_id = uuid::Uuid::new_v4().to_string();
        let message_id_clone = message_id.clone();
        
        let closure = {
            let tx_rc = tx_rc.clone();
            Closure::wrap(Box::new(move |event: MessageEvent| {
                let data = event.data();
                if let Ok(id_val) = js_sys::Reflect::get(&data, &"messageId".into()) {
                    if id_val == JsValue::from_str(&message_id_clone) {
                        if let Some(tx) = tx_rc.borrow_mut().take() {
                            let _ = tx.send(Ok(data));
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>)
        };
        
        worker.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())?;
        closure.forget();
        
        let obj = Object::new();
        Reflect::set(&obj, &"type".into(), &JsValue::from_str(&msg_type))?;
        Reflect::set(&obj, &"messageId".into(), &JsValue::from_str(&message_id))?;
        Reflect::set(&obj, &"args".into(), &args)?;
        
        worker.post_message(&obj)?;
        
        match rx.await {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(JsValue::from_str("Channel error")),
        }
    }
    
    fn extract_db_id(result: &JsValue) -> Result<JsValue, JsValue> {
        if let Ok(result_obj) = js_sys::Reflect::get(result, &"result".into()) {
            if let Ok(result_obj) = result_obj.dyn_into::<js_sys::Object>() {
                if let Ok(db_id) = js_sys::Reflect::get(&result_obj, &"dbId".into()) {
                    return Ok(db_id);
                }
            }
        }
        Err(JsValue::from_str("Could not extract database ID"))
    }
}
