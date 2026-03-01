//src/utils.rs
use wasm_bindgen_futures::JsFuture;

pub async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                &resolve,
                ms
            )
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}
