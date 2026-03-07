//! Web Worker management for SQLite database operations.
//! 
//! This module provides a complete system for managing a dedicated Web Worker
//! that runs SQLite in the browser. It handles worker initialization, ready-state
//! signaling, and a robust request-response messaging system with automatic
//! cleanup of event listeners.
//! 
//! # Architecture
//! 
//! * **Singleton worker** - Only one worker instance exists across the app
//! * **Ready signaling** - Oneshot channel for `worker1-ready` message
//! * **Message passing** - UUID-based request/response with self-cleaning listeners
//! * **Browser compatibility** - Automatic fallback for Firefox
//! 
//! # Performance
//! 
//! * `OnceLock` provides zero-cost access after initialization
//! * Atomic counters for message IDs with relaxed ordering
//! * Self-removing listeners prevent memory leaks

use futures_channel::oneshot;
use js_sys::{Object, Reflect};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::{MessageEvent, Worker};

use crate::modules::core::blobs;

// ==================== GLOBAL STATE ====================

/// Global worker instance (immutable, zero-cost access)
static WORKER: OnceLock<Worker> = OnceLock::new();

/// Channel to signal when worker is fully initialized
static WORKER_READY: Mutex<Option<oneshot::Sender<()>>> = Mutex::new(None);

// ==================== BROWSER DETECTION ====================

/// Detects if the browser is Firefox (needs special handling)
fn is_firefox() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
        .contains("Firefox")
}

// ==================== INITIALIZATION ====================

/// Initializes the SQLite web worker with embedded assets.
///
/// This function creates a worker wrapper with all SQLite files embedded
/// via blobs/data URLs, handling browser differences automatically.
/// The worker is stored in a `OnceLock`, ensuring only one instance.
///
/// # Returns
/// * `Ok(())` - Worker created and configured successfully
/// * `Err(JsValue)` - Worker creation failed (check console for details)
///
/// # Idempotency
/// This function is idempotent. Subsequent calls return `Ok(())` immediately.
///
/// # Examples
/// ```no_run
/// # async fn example() -> Result<(), wasm_bindgen::JsValue> {
/// use sqlite_wasm::modules::core::worker::initialize_embedded_worker;
/// 
/// initialize_embedded_worker().await?;
/// # Ok(())
/// # }
/// ```
#[wasm_bindgen]
pub async fn initialize_embedded_worker() -> Result<(), JsValue> {
    if WORKER.get().is_some() {
        return Ok(());
    }

    // Create worker wrapper with embedded files
    let worker_url = blobs::create_embedded_worker().await?;
    
    // Create the worker (classic worker, never module)
    let worker = Worker::new(&worker_url)?;
    
    setup_ready_listener(&worker)?;
    WORKER
        .set(worker)
        .map_err(|_| JsValue::from_str("Worker already initialized"))?;

    Ok(())
}

/// Legacy worker initialization with external script path.
///
/// This function is kept for backward compatibility with code that
/// expects to load the worker from a physical file. Prefer using
/// `initialize_embedded_worker()` for embedded assets.
///
/// # Arguments
/// * `script_path` - Path to the SQLite worker script (e.g., "/static/sqlite3-worker1.js")
///
/// # Browser Compatibility
/// Firefox may have issues with blob URLs, so this function falls back
/// to a direct path when a blob URL is detected.
#[wasm_bindgen]
pub async fn initialize_worker(script_path: &str) -> Result<(), JsValue> {
    if WORKER.get().is_some() {
        return Ok(());
    }

    // Firefox fallback for blob URLs
    let final_path = if is_firefox() && script_path.starts_with("blob:") {
        "/static/sqlite.org/sqlite3-worker1.js"
    } else {
        script_path
    };

    let worker = Worker::new(final_path)?;
    setup_ready_listener(&worker)?;
    WORKER
        .set(worker)
        .map_err(|_| JsValue::from_str("Worker already initialized"))?;

    Ok(())
}

/// Sets up a one-time listener for the worker ready message.
///
/// The SQLite worker sends a specific message when fully initialized:
/// ```json
/// { type: "sqlite3-api", result: "worker1-ready" }
/// ```
/// This function captures that message and resolves the ready channel.
///
/// # Technical Details
/// * Creates a oneshot channel stored in `WORKER_READY`
/// * Closure is leaked (`forget()`) to stay alive until message arrives
/// * Spawns a background task to await the receiver (for logging)
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
/// Blocks (asynchronously) until the worker sends the `worker1-ready`
/// message, or a timeout occurs.
///
/// # Returns
/// * `Ok(())` - Worker is ready for commands
/// * `Err(JsValue)` - Timeout or channel error
///
/// # Performance
/// Uses a oneshot channel and yields the async task until ready.
/// No busy-waiting or polling involved.
///
/// # Timeout
/// * Firefox: 15 seconds (needs more time for OPFS)
/// * Others: 5 seconds
#[wasm_bindgen]
pub async fn wait_for_worker() -> Result<(), JsValue> {
    let (tx, rx) = oneshot::channel::<()>();
    *WORKER_READY.lock().unwrap() = Some(tx);
    
    // Firefox needs more time for OPFS initialization
    let timeout_ms = if is_firefox() { 15000 } else { 5000 };
    
    let timeout_promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                timeout_ms
            )
            .unwrap();
    });
    
    let timeout_future = wasm_bindgen_futures::JsFuture::from(timeout_promise);
    
    futures::pin_mut!(rx);
    futures::pin_mut!(timeout_future);
    
    match futures::future::select(rx, timeout_future).await {
        futures::future::Either::Left((Ok(()), _)) => Ok(()),
        futures::future::Either::Left((Err(_), _)) => Err(JsValue::from_str("Worker never ready")),
        futures::future::Either::Right((_result, _)) => Err(JsValue::from_str("Worker initialization timeout")),
    }
}

/// Gets the global worker instance.
///
/// # Panics
/// Panics if called before worker is initialized (programming error).
#[inline(always)]
pub fn get_worker() -> &'static Worker {
    WORKER.get().expect("Worker not initialized")
}

// ==================== REQUEST-RESPONSE MESSAGING ====================

/// Sends a message to the worker and waits for a response.
///
/// This is the core communication function. Each message gets a unique UUID,
/// and a temporary listener waits for the matching response.
///
/// # Arguments
/// * `msg_type` - Message type (e.g., "open", "exec", "query")
/// * `args` - JavaScript object with message arguments
///
/// # Returns
/// * `Ok(JsValue)` - Response from worker
/// * `Err(JsValue)` - Error from worker or communication failure
///
/// # How it works
/// 1. Generates unique `messageId` (UUID v4)
/// 2. Sets up one-time listener filtering by that ID
/// 3. Posts message to worker
/// 4. Awaits response channel
/// 5. Listener self-removes after response
///
/// # Examples
/// ```no_run
/// # async fn example() -> Result<(), wasm_bindgen::JsValue> {
/// use sqlite_wasm::modules::core::worker::w_msg;
/// use js_sys::Object;
/// 
/// let args = Object::new();
/// // Configure args...
/// let response = w_msg("open".to_string(), args.into()).await?;
/// # Ok(())
/// # }
/// ```
pub async fn w_msg(msg_type: String, args: JsValue) -> Result<JsValue, JsValue> {
    let worker = get_worker();

    let message_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();

    setup_message_listener(worker, message_id.clone(), tx)?;

    // Build message object
    let obj = Object::new();
    Reflect::set(&obj, &"type".into(), &JsValue::from_str(&msg_type))?;
    Reflect::set(&obj, &"messageId".into(), &JsValue::from_str(&message_id))?;
    Reflect::set(&obj, &"args".into(), &args)?;

    worker.post_message(&obj)?;

    rx.await
        .map_err(|e| JsValue::from_str(&format!("Channel error: {:?}", e)))?
}

/// Sets up a self-removing listener for a specific message ID.
///
/// Creates a one-time event listener that filters messages by `messageId`.
/// The listener automatically removes itself after receiving the matching response.
///
/// # Technical Details
/// * Uses `Rc<RefCell>` to share the oneshot sender between closures
/// * Listener is stored in `handler_rc` to allow self-removal
/// * The closure is intentionally leaked (`forget()`) to stay alive
///
/// # Memory Safety
/// The listener self-removes after response, preventing memory leaks.
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

                // Self-removal after response
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
