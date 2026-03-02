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

/// Initializes the web worker and sets up ready listener
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

/// Sets up listener for worker ready message
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

/// Waits for worker to be ready
#[wasm_bindgen]
pub async fn wait_for_worker() -> Result<(), JsValue> {
    let (tx, rx) = oneshot::channel::<()>();
    *WORKER_READY.lock().unwrap() = Some(tx);
    rx.await
        .map_err(|_| JsValue::from_str("Worker never ready"))?;
    Ok(())
}

/// Gets the global worker instance (zero-cost)
#[inline(always)]
fn get_worker() -> &'static Worker {
    WORKER.get().expect("Worker not initialized")
}

// ==================== REQUEST-RESPONSE MESSAGING ====================

/// Sends a message to worker and waits for response
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

/// Sets up a self-removing listener for a specific message
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
