//! Talking to the native host (window.ipc ↔ window.__qe) and small browser helpers.

use serde_json::Value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "ipc"], js_name = postMessage, catch)]
    fn ipc_post(msg: &str) -> Result<(), JsValue>;
}

/// True when running inside the desktop app (not in a plain browser preview).
pub fn has_host() -> bool {
    let w = web_sys::window().expect("window");
    js_sys::Reflect::get(&w, &"ipc".into()).map(|v| !v.is_undefined()).unwrap_or(false)
}

pub fn send(msg: Value) {
    let _ = ipc_post(&msg.to_string());
}

/// Install `window.__qe(json)`, which the host calls for every event.
pub fn listen(mut on_event: impl FnMut(Value) + 'static) {
    let cb = Closure::<dyn FnMut(String)>::new(move |s: String| {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            on_event(v);
        }
    });
    let w = web_sys::window().expect("window");
    let _ = js_sys::Reflect::set(&w, &"__qe".into(), cb.as_ref());
    cb.forget();
}

/// New screen or page: start at the top.
pub fn scroll_stage_top() {
    if let Some(st) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.query_selector(".stage").ok().flatten()) {
        st.set_scroll_top(0);
    }
}

pub fn reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok().flatten())
        .is_some_and(|m| m.matches())
}

/// Run `update` inside a same-document View Transition (Chromium), so the DOM change animates.
/// The promise resolves on the next macrotask, after Leptos has applied the update.
pub fn view_transition(update: impl FnOnce() + 'static) {
    let doc = web_sys::window().and_then(|w| w.document()).expect("document");
    let start = js_sys::Reflect::get(&doc, &"startViewTransition".into()).ok().filter(|f| f.is_function());
    let Some(start) = start.filter(|_| !reduced_motion()) else {
        update();
        return;
    };
    let mut update = Some(update);
    let cb = Closure::once_into_js(move || {
        if let Some(f) = update.take() {
            f();
        }
        js_sys::Promise::new(&mut |resolve, _| {
            let _ = web_sys::window().unwrap().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
        })
    });
    let _ = js_sys::Function::from(start).call1(&doc, &cb);
}
