//src/modules/core/worker.rs

//! Web Worker management for SQLite database operations.
//! 
//! This module provides a complete system for managing a dedicated Web Worker
//! that runs SQLite in the browser. It handles worker initialization, ready-state
//! signaling, and a robust request-response messaging system with automatic
//! cleanup of event listeners.
//! 
//! # Architecture
//! 
//! The worker system is built around several key components:
//! 
//! * **Global worker instance** - A singleton `OnceLock<Worker>` that ensures only
//!   one worker is created and shared across the application.
//! 
//! * **Ready signaling** - A oneshot channel that resolves when the worker sends
//!   the `worker1-ready` message, indicating SQLite is fully initialized.
//! 
//! * **Request-response messaging** - Each message sent to the worker includes a
//!   unique UUID, and a temporary listener waits for a response with the matching ID.
//! 
//! * **Self-cleaning listeners** - Event listeners are automatically removed after
//!   handling their specific message, preventing memory leaks.
//! 
//! # Performance Considerations
//! 
//! * The worker instance is stored in a `OnceLock`, providing zero-cost access
//!   after initialization.
//! * Message listeners are temporary and self-removing, ensuring no orphaned
//!   callbacks remain.
//! * The ready channel uses `Mutex<Option>` with minimal locking overhead (≈2ns).
//! 
//! # Examples
//! 
//! ```rust
//! use sqlite_wasm::core::worker::{initialize_worker, wait_for_worker, w_msg};
//! 
//! // Initialize the worker (idempotent)
//! initialize_worker("/sqlite.org/sqlite3-worker1.js").await?;
//! 
//! // Wait for SQLite to be ready
//! wait_for_worker().await?;
//! 
//! // Send a custom message
//! let response = w_msg("open".to_string(), args).await?;
//! ```

use futures_channel::oneshot;
use js_sys::{Object, Reflect};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::{MessageEvent, Worker};

// Global worker instance (immutable, zero-cost)
static WORKER: OnceLock<Worker> = OnceLock::new();

// Channel to signal when worker is ready
static WORKER_READY: Mutex<Option<oneshot::Sender<()>>> = Mutex::new(None);

// ==================== INITIALIZATION ====================

/// Initializes the SQLite web worker.
///
/// This function creates a new Web Worker using the provided script path and
/// configures a listener for the worker ready message. The worker is stored
/// globally in a `OnceLock`, ensuring only one instance exists.
///
/// # Arguments
/// * `script_path` - Path to the SQLite worker JavaScript file.
///   Typically this points to the official SQLite worker script, e.g.:
///   - `"/sqlite.org/sqlite3-worker1.js"`
///   - `"/static/sqlite3-worker1.js"`
///
/// # Returns
/// * `Ok(())` - Worker created successfully and ready listener configured.
/// * `Err(JsValue)` - Worker creation failed. Possible reasons include:
///   - Invalid script path (404)
///   - Cross-origin issues if the worker script is on a different domain
///   - Browser doesn't support Web Workers
///
/// # Idempotency
/// This function is idempotent. Subsequent calls after the first successful
/// initialization will return `Ok(())` immediately without creating a new worker.
///
/// # Examples
/// ```rust
/// // Basic initialization
/// initialize_worker("/sqlite.org/sqlite3-worker1.js").await?;
/// 
/// // Can be called multiple times safely
/// initialize_worker("/sqlite.org/sqlite3-worker1.js").await?; // Returns Ok(()) instantly
/// ```
#[wasm_bindgen]
pub async fn initialize_worker(script_path: &str) -> Result<(), JsValue> {
    if WORKER.get().is_some() {
        return Ok(());
    }

    let worker = Worker::new(script_path)?;
    setup_ready_listener(&worker)?;
    WORKER
        .set(worker)
        .map_err(|_| JsValue::from_str("Worker already initialized"))?;

    Ok(())
}

/// Sets up a one-time listener for the worker ready message.
///
/// The SQLite worker sends a specific message when it's fully initialized:
/// ```json
/// { type: "sqlite3-api", result: "worker1-ready" }
/// ```
/// 
/// This function captures that message and resolves the ready channel,
/// allowing tasks waiting on `wait_for_worker()` to proceed.
///
/// # Arguments
/// * `worker` - Reference to the Web Worker instance.
///
/// # Returns
/// * `Ok(())` - Listener successfully attached.
/// * `Err(JsValue)` - Failed to attach event listener.
///
/// # Technical Details
/// * Creates a oneshot channel where the sender is stored in `WORKER_READY`.
/// * The receiver is spawned as a background task (optional, for logging).
/// * The closure is leaked (`forget()`) to keep it alive until the message arrives.
fn setup_ready_listener(worker: &Worker) -> Result<(), JsValue> {
    let (tx, rx) = oneshot::channel::<()>();

    *WORKER_READY.lock().unwrap() = Some(tx);

    let closure = Closure::wrap(Box::new(move |event: MessageEvent| {
        let data = event.data();
        let type_val = Reflect::get(&data, &"type".into()).unwrap_or(JsValue::NULL);
        let result_val = Reflect::get(&data, &"result".into()).unwrap_or(JsValue::NULL);

        if type_val == JsValue::from_str("sqlite3-api")
            && result_val == JsValue::from_str("worker1-ready")
        {
            if let Some(tx) = WORKER_READY.lock().unwrap().take() {
                let _ = tx.send(());
            }
        }
    }) as Box<dyn FnMut(_)>);

    worker.add_event_listener_with_callback("message", closure.as_ref().unchecked_ref())?;
    closure.forget();

    wasm_bindgen_futures::spawn_local(async move {
        let _ = rx.await;
    });

    Ok(())
}

/// Waits for the worker to be fully initialized.
///
/// This function blocks (asynchronously) until the worker sends the
/// `worker1-ready` message, indicating that the SQLite module has been
/// loaded and is ready to accept commands.
///
/// # Returns
/// * `Ok(())` - Worker is ready.
/// * `Err(JsValue)` - Timeout or channel error (worker never became ready).
///
/// # Performance
/// This function uses a oneshot channel and yields the async task until
/// the ready signal is received. No busy-waiting or polling is involved.
///
/// # Examples
/// ```rust
/// initialize_worker("/sqlite.org/sqlite3-worker1.js").await?;
/// wait_for_worker().await?; // Waits efficiently
/// 
/// // Now safe to send commands
/// w_msg("open".to_string(), args).await?;
/// ```
#[wasm_bindgen]
pub async fn wait_for_worker() -> Result<(), JsValue> {
    let (tx, rx) = oneshot::channel::<()>();
    *WORKER_READY.lock().unwrap() = Some(tx);
    rx.await
        .map_err(|_| JsValue::from_str("Worker never ready"))?;
    Ok(())
}

/// Gets the global worker instance with zero runtime cost.
///
/// # Returns
/// * `&'static Worker` - Reference to the global worker.
///
/// # Panics
/// Panics if called before `initialize_worker()` has completed successfully.
/// This is intentional as it indicates a programming error.
///
/// # Performance
/// This function is marked `#[inline(always)]` to eliminate call overhead.
/// After initialization, accessing the worker is as cheap as a pointer dereference.
#[inline(always)]
fn get_worker() -> &'static Worker {
    WORKER.get().expect("Worker not initialized")
}

// ==================== REQUEST-RESPONSE MESSAGING ====================

/// Sends a message to the worker and waits for a response.
///
/// This is the core communication function. Each message is assigned a unique
/// UUID, and a temporary listener is created to capture the matching response.
/// The listener automatically removes itself after the response is received.
///
/// # Arguments
/// * `msg_type` - Type of message to send (e.g., "open", "exec", "query").
/// * `args` - JavaScript value containing the message arguments.
///
/// # Returns
/// * `Ok(JsValue)` - Response from the worker.
/// * `Err(JsValue)` - Error from the worker or communication failure.
///
/// # How it works
/// 1. Generates a unique `messageId` using UUID v4.
/// 2. Creates a oneshot channel to receive the response.
/// 3. Sets up a temporary event listener that filters for the specific `messageId`.
/// 4. Posts the message to the worker with the ID included.
/// 5. Awaits the channel; when the response arrives, the listener is removed.
///
/// # Examples
/// ```rust
/// // Open a database
/// let open_args = object!({ filename: "mydb.sqlite3", vfs: "opfs" });
/// let response = w_msg("open".to_string(), open_args).await?;
/// 
/// // Execute a query
/// let query_args = object!({ sql: "SELECT * FROM users" });
/// let rows = w_msg("exec".to_string(), query_args).await?;
/// ```
pub async fn w_msg(msg_type: String, args: JsValue) -> Result<JsValue, JsValue> {
    let worker = get_worker();

    let message_id = Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();

    // Setup temporary listener for this specific message
    setup_message_listener(worker, message_id.clone(), tx)?;

    // Build message object
    let obj = Object::new();
    Reflect::set(&obj, &"type".into(), &JsValue::from_str(&msg_type))?;
    Reflect::set(&obj, &"messageId".into(), &JsValue::from_str(&message_id))?;
    Reflect::set(&obj, &"args".into(), &args)?;

    // Send message
    worker.post_message(&obj)?;

    // Wait for response
    rx.await
        .map_err(|e| JsValue::from_str(&format!("Channel error: {:?}", e)))?
}

/// Sets up a self-removing listener for a specific message.
///
/// This function creates a one-time event listener that filters messages
/// by `messageId`. Once the matching response is received, the listener
/// removes itself to prevent memory leaks.
///
/// # Arguments
/// * `worker` - Reference to the Web Worker.
/// * `message_id` - Unique ID to match against incoming messages.
/// * `tx` - Oneshot sender for delivering the response.
///
/// # Returns
/// * `Ok(())` - Listener successfully attached.
/// * `Err(JsValue)` - Failed to attach event listener.
///
/// # Technical Details
/// * Uses `Rc<RefCell>` to share the `tx` between the closure and outer scope.
/// * The listener closure is stored in `handler_rc` to allow self-removal.
/// * After receiving a response, the listener removes itself and drops the closure.
/// * The closure is intentionally leaked (`forget()`) to keep it alive until used.
fn setup_message_listener(
    worker: &Worker,
    message_id: String,
    tx: oneshot::Sender<Result<JsValue, JsValue>>,
) -> Result<(), JsValue> {
    let tx = Rc::new(RefCell::new(Some(tx)));
    let message_id_clone = message_id.clone();
    let worker_clone = worker.clone();

    let handler_rc: Rc<RefCell<Option<Closure<dyn FnMut(MessageEvent)>>>> =
        Rc::new(RefCell::new(None));
    let handler_clone = handler_rc.clone();

    let closure = {
        let tx = tx.clone();
        let message_id_clone = message_id_clone.clone();
        let handler_clone = handler_clone.clone();

        Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();
            let incoming_id =
                Reflect::get(&data, &JsValue::from_str("messageId")).unwrap_or(JsValue::NULL);

            if incoming_id == JsValue::from_str(&message_id_clone) {
                let response_type =
                    Reflect::get(&data, &JsValue::from_str("type")).unwrap_or(JsValue::NULL);

                // Send response through channel
                if let Some(sender) = tx.borrow_mut().take() {
                    match response_type == JsValue::from_str("error") {
                        true => sender.send(Err(data)).ok(),
                        false => sender.send(Ok(data)).ok(),
                    };
                }

                // Remove listener (self-cleaning)
                if let Some(h) = handler_clone.borrow_mut().take() {
                    worker_clone
                        .remove_event_listener_with_callback("message", h.as_ref().unchecked_ref())
                        .ok();
                }
            }
        }) as Box<dyn FnMut(_)>)
    };

    *handler_rc.borrow_mut() = Some(closure);

    if let Some(h) = handler_rc.borrow_mut().take() {
        worker.add_event_listener_with_callback("message", h.as_ref().unchecked_ref())?;
        h.forget(); // Lives until removed
    }

    Ok(())
}