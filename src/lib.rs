//src/lib.rs
mod utils;
mod bindings;

use futures_channel::oneshot;
use js_sys::{Array, Object, Reflect};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Mutex, Once,
        atomic::{AtomicU32, Ordering},
    },
};
use uuid::Uuid;
use wasm_bindgen::{
    JsCast, JsValue,
    prelude::{Closure, wasm_bindgen},
};
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{MessageEvent, Worker};

use crate::utils::sleep;

static DB_UID: Mutex<Option<JsValue>> = Mutex::new(None);
static COUNTER: AtomicU32 = AtomicU32::new(0);
static WORKER_INIT: Once = Once::new();
static mut WORKER: Option<&'static Worker> = None;

#[wasm_bindgen(start)]
pub fn initialize_bindings() {
    bindings::initialize_bindings();
}

// Função separada com #[wasm_bindgen] para inicialização
#[wasm_bindgen]
pub async fn initialize_worker(script_path: &str) -> Result<(), JsValue> {
    let worker = Worker::new(script_path)?;

    WORKER_INIT.call_once(|| {
        let leaked: &'static Worker = Box::leak(Box::new(worker));
        unsafe {
            WORKER = Some(leaked);
        }
    });

    bindings::initialize_bindings();

    Ok(())
}

// Função auxiliar interna (sem #[wasm_bindgen])
#[inline(always)]
fn get_worker() -> Result<&'static Worker, JsValue> {
    if !WORKER_INIT.is_completed() {
        return Err(JsValue::from_str(
            "[Worker] Worker not initialized. Call initialize_worker() first.",
        ));
    }
    unsafe { Ok(WORKER.unwrap_unchecked()) }
}

fn w_msg(msg_type: String, args: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let message_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();

        let worker = get_worker()?;

        let tx = Rc::new(RefCell::new(Some(tx)));
        let message_id_clone = message_id.clone();
        let worker_clone = worker.clone();

        let handler_rc: Rc<RefCell<Option<Closure<dyn FnMut(MessageEvent)>>>> =
            Rc::new(RefCell::new(None));
        let handler_clone = handler_rc.clone();

        let closure = {
            let tx = tx.clone();
            Closure::wrap(Box::new(move |event: MessageEvent| {
                let data = event.data();
                let incoming_id =
                    Reflect::get(&data, &JsValue::from_str("messageId")).unwrap_or(JsValue::NULL);

                if incoming_id == JsValue::from_str(&message_id_clone) {
                    let response_type =
                        Reflect::get(&data, &JsValue::from_str("type")).unwrap_or(JsValue::NULL);

                    if let Some(sender) = tx.borrow_mut().take() {
                        match response_type == JsValue::from_str("error") {
                            true => sender.send(Err(data)).ok(),
                            false => sender.send(Ok(data)).ok(),
                        };
                    }

                    if let Some(h) = handler_clone.borrow_mut().take() {
                        worker_clone
                            .remove_event_listener_with_callback(
                                "message",
                                h.as_ref().unchecked_ref(),
                            )
                            .ok();
                        h.forget();
                    }
                }
            }) as Box<dyn FnMut(_)>)
        };

        *handler_rc.borrow_mut() = Some(closure);
        if let Some(h) = handler_rc.borrow_mut().take() {
            worker
                .add_event_listener_with_callback("message", h.as_ref().unchecked_ref())
                .ok();
            h.forget();
        }

        let obj = Object::new();
        Reflect::set(
            &obj,
            &JsValue::from_str("type"),
            &JsValue::from_str(&msg_type),
        )
        .ok();
        Reflect::set(
            &obj,
            &JsValue::from_str("messageId"),
            &JsValue::from_str(&message_id),
        )
        .ok();
        Reflect::set(&obj, &JsValue::from_str("args"), &args).ok();

        worker.post_message(&obj).ok();

        let result = rx
            .await
            .map_err(|_| JsValue::from_str("Channel closed"))??;

        Ok(result)
    })
}

#[allow(unused)]
#[wasm_bindgen]
pub async fn open() -> Result<(), JsValue> {

    // Dá tempo pro worker carregar
    sleep(100).await;

    // Se já tem uid, retorna
    if DB_UID.lock().unwrap().is_some() {
        return Ok(());
    }

    let args = Object::new();
    Reflect::set(&args, &"filename".into(), &"users.sqlite3".into())?;
    Reflect::set(&args, &"vfs".into(), &"opfs".into())?;

    let worker = get_worker();
    let open_result = JsFuture::from(w_msg("open".to_string(), args.into())).await?;
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

#[allow(unused)]
#[wasm_bindgen]
pub async fn close() -> Result<(), JsValue> {
    let worker = get_worker()?;

    let uid = DB_UID.lock().unwrap().take(); // Remove o uid

    if let Some(uid) = uid {
        let args = Object::new();
        Reflect::set(&args, &"dbId".into(), &uid)?;
        JsFuture::from(w_msg("close".to_string(), args.into())).await?;
    }

    Ok(())
}

#[wasm_bindgen]
pub async fn exec(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
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
    
    JsFuture::from(w_msg("exec".to_string(), args_obj.into())).await
}

#[wasm_bindgen]
pub async fn query(sql: &str, args: Vec<JsValue>) -> Result<JsValue, JsValue> {
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
    
    JsFuture::from(w_msg("exec".to_string(), args_obj.into())).await
}
