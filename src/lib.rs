use futures_channel::oneshot;
use js_sys::{Array, Object, Reflect};
use js_sys::Promise;
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, future_to_promise};
use web_sys::{MessageEvent, Worker, window};

thread_local! {
    static WORKER: RefCell<Option<Worker>> = RefCell::new(None);
    static DB_ID: RefCell<Option<JsValue>> = RefCell::new(None);
}

pub fn w_msg(worker: Worker, msg_type: String, args: JsValue) -> js_sys::Promise {
    future_to_promise(async move {
        let message_id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel::<Result<JsValue, JsValue>>();

        // tx em Rc<RefCell> para poder usar dentro da closure FnMut
        let tx = Rc::new(RefCell::new(Some(tx)));
        let message_id_clone = message_id.clone();
        let worker_clone = worker.clone();

        // Rc + RefCell para a própria closure poder se remover
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

                    // envia resposta via channel
                    if let Some(sender) = tx.borrow_mut().take() {
                        if response_type == JsValue::from_str("error") {
                            let _ = sender.send(Err(data));
                        } else {
                            let _ = sender.send(Ok(data));
                        }
                    }

                    // remove listener
                    if let Some(h) = handler_clone.borrow_mut().take() {
                        worker_clone
                            .remove_event_listener_with_callback(
                                "message",
                                h.as_ref().unchecked_ref(),
                            )
                            .unwrap();
                        h.forget(); // mantém closure viva até aqui
                    }
                }
            }) as Box<dyn FnMut(_)>)
        };

        // salva closure no Rc e adiciona no Worker
        *handler_rc.borrow_mut() = Some(closure);
        if let Some(h) = handler_rc.borrow_mut().take() {
            worker.add_event_listener_with_callback("message", h.as_ref().unchecked_ref())?;
            h.forget(); // mantém closure viva
        }

        // monta objeto JS { type, messageId, args }
        let obj = Object::new();
        Reflect::set(
            &obj,
            &JsValue::from_str("type"),
            &JsValue::from_str(&msg_type),
        )?;
        Reflect::set(
            &obj,
            &JsValue::from_str("messageId"),
            &JsValue::from_str(&message_id),
        )?;
        Reflect::set(&obj, &JsValue::from_str("args"), &args)?;

        worker.post_message(&obj)?;

        // espera resposta
        let result = rx
            .await
            .map_err(|_| JsValue::from_str("Channel closed"))??;

        Ok(result)
    })
}

pub async fn init_db() -> Result<JsValue, JsValue> {
    let worker = get_worker();

    // =========================
    // OPEN DATABASE
    // =========================
    let open_args = Object::new();
    Reflect::set(&open_args, &"filename".into(), &"users.sqlite3".into())?;
    Reflect::set(&open_args, &"vfs".into(), &"opfs".into())?;

    let open_result =
        JsFuture::from(w_msg(worker.clone(), "open".to_string(), open_args.into())).await?;

    // =========================
    // EXTRAI dbId (open.result?.dbId ?? open.dbId)
    // =========================
    let result_field = Reflect::get(&open_result, &"result".into()).ok();

    let db_id = if let Some(result_obj) = result_field {
        let nested = Reflect::get(&result_obj, &"dbId".into()).ok();
        nested
            .unwrap_or_else(|| Reflect::get(&open_result, &"dbId".into()).unwrap_or(JsValue::NULL))
    } else {
        Reflect::get(&open_result, &"dbId".into()).unwrap_or(JsValue::NULL)
    };

    DB_ID.with(|d| *d.borrow_mut() = Some(db_id.clone()));
    web_sys::console::log_2(&"DB opened with dbId:".into(), &db_id);

    // =========================
    // CREATE TABLE
    // =========================
    let exec_args = Object::new();
    Reflect::set(&exec_args, &"dbId".into(), &db_id)?;
    Reflect::set(
        &exec_args,
        &"sql".into(),
        &JsValue::from_str(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL
            )
            "#,
        ),
    )?;

    JsFuture::from(w_msg(worker, "exec".to_string(), exec_args.into())).await?;
    web_sys::console::log_1(&"Table 'users' ready".into());

    Ok(JsValue::from(db_id))
}

// Função assíncrona de sleep no Rust/WASM
pub async fn sleep(ms: i32) {
    let promise = Promise::new(&mut |resolve, _reject| {
        let closure = Closure::once_into_js(move || {
            resolve.call0(&JsValue::NULL).unwrap();
        });

        window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                ms,
            )
            .unwrap();
    });

    JsFuture::from(promise).await.unwrap();
}

pub async fn start_interval() -> Result<(), JsValue> {
    let window = window().unwrap();

    let interval_id = Rc::new(RefCell::new(None));
    let interval_id_clone = interval_id.clone();
    let window_clone = window.clone();

    let closure = Closure::wrap(Box::new(move || {
        let _worker = get_worker();
        let interval_id_clone = interval_id_clone.clone();
        let window_clone = window_clone.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match init_db().await {
                Ok(_) => {
                    if let Some(id) = *interval_id_clone.borrow() {
                        window_clone.clear_interval_with_handle(id);
                    }
                }
                Err(_) => {}
            }
        });
    }) as Box<dyn FnMut()>);

    let id = window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        25,
    )?;

    *interval_id.borrow_mut() = Some(id);

    closure.forget();

    Ok(())
}

#[wasm_bindgen(start)]
pub fn start() {
    // wasm_bindgen_futures::spawn_local(async {
    //     start_interval().await.unwrap();
    // });
}

/// Versão 2: Com caminho físico (backup)
pub fn get_worker() -> Worker {
    WORKER.with(|w| {
        if let Some(worker) = &*w.borrow() {
            worker.clone()
        } else {
            let worker = Worker::new("jswasm/sqlite3-worker1.js")
                .expect("failed to create worker");
            
            *w.borrow_mut() = Some(worker.clone());
            worker
        }
    })
}

/// Executa comando sem retorno
#[wasm_bindgen]
pub async fn exec(sql: String, bind: Array) -> Result<(), JsValue> {
    let (worker, db_id) = get_worker_and_db_id()?;

    let args = Object::new();
    Reflect::set(&args, &"dbId".into(), &db_id)?;
    Reflect::set(&args, &"sql".into(), &sql.into())?;
    Reflect::set(&args, &"bind".into(), &bind.into())?;

    JsFuture::from(w_msg(worker, "exec".to_string(), args.into())).await?;
    Ok(())
}

/// Executa comando que retorna linhas
#[wasm_bindgen]
pub async fn query(sql: String, bind: Option<Array>) -> Result<JsValue, JsValue> {
    let (worker, db_id) = get_worker_and_db_id()?;

    let args = Object::new();
    Reflect::set(&args, &"dbId".into(), &db_id)?;
    Reflect::set(&args, &"sql".into(), &sql.into())?;

    if let Some(b) = bind {
        Reflect::set(&args, &"bind".into(), &b.into())?;
    }

    Reflect::set(&args, &"rowMode".into(), &"object".into())?;

    let result_js = JsFuture::from(w_msg(worker, "exec".to_string(), args.into())).await?;

    let result_rows = Reflect::get(&result_js, &"result".into())
        .ok()
        .and_then(|r| Reflect::get(&r, &"resultRows".into()).ok())
        .or_else(|| Reflect::get(&result_js, &"resultRows".into()).ok())
        .unwrap_or_else(|| Array::new().into());

    Ok(result_rows)
}

/// Recupera worker e dbId global
fn get_worker_and_db_id() -> Result<(Worker, JsValue), JsValue> {
    let worker = WORKER
        .with(|w| w.borrow().clone())
        .ok_or_else(|| JsValue::from_str("Worker not initialized"))?;
    let db_id = DB_ID
        .with(|d| d.borrow().clone())
        .ok_or_else(|| JsValue::from_str("DB not initialized"))?;
    Ok((worker, db_id))
}
