//! Quantum Explainer — native Windows app.
//!
//! A frameless tao window with a Mica backdrop hosts a WebView2 view. The interface (Leptos,
//! compiled to WebAssembly) is embedded in this executable and served over a private `app://`
//! protocol: no browser, no local web server, no open port. API keys stay in this process.

#![windows_subsystem = "windows"] // a GUI app: never open a console window

mod config;
mod gemini;
mod llm;
mod pipeline;
mod prompts;

use rust_embed::Embed;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::Arc;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Icon, WindowBuilder};
use wry::WebViewBuilder;
use wry::http::{Request, Response, header::CONTENT_TYPE};

#[derive(Embed)]
#[folder = "../ui/dist/"]
struct Assets;

enum UserEvent {
    /// JSON event for the UI (delivered to `window.__qe`).
    Emit(Value),
    Minimize,
    ToggleMaximize,
    Drag,
    Close,
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "png" => "image/png",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn serve(req: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(file) => Response::builder()
            .header(CONTENT_TYPE, mime(path))
            .header("Cache-Control", "no-cache")
            // Strict CSP: only our own embedded files; wasm needs 'wasm-unsafe-eval'.
            .header(
                "Content-Security-Policy",
                "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:; connect-src 'self'",
            )
            .body(file.data)
            .unwrap(),
        None => Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).unwrap(),
    }
}

/// Open one fixed, known page in the user's default browser (the app's own view never leaves app://).
fn open_in_browser(url: &'static str) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (verb, target) = (wide("open"), wide(url));
        ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), target.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
    }
}

/// The `status` event: the active provider (fields kept from the single-provider version) plus
/// every provider. Only the last four characters of a key ever leave this process.
fn status() -> Value {
    let c = config::Config::read();
    let active = c.active();
    let providers: Vec<Value> = llm::PRESETS
        .iter()
        .map(|p| {
            let (key, source) = c.key(p);
            json!({
                "id": p.id,
                "label": p.label,
                "has_key": key.is_some(),
                "key_hint": key.as_deref().map(llm::key_hint),
                "key_source": source,
                "model": c.model(p),
                "key_page": p.key_page,
                "lists_models": p.lists_models,
            })
        })
        .collect();
    let (key, source) = c.key(active);
    json!({
        "type": "status",
        "has_key": key.is_some(),
        "key_hint": key.as_deref().map(llm::key_hint),
        "key_source": source,
        "model": c.model(active),
        "provider": active.id,
        "provider_label": active.label,
        "providers": providers,
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// Which provider a pasted key is for: the UI's explicit choice, else an unambiguous key prefix.
fn key_provider(requested: Option<&str>, key: &str) -> Result<&'static llm::Preset, (String, bool)> {
    if let Some(id) = requested.filter(|s| !s.is_empty()) {
        return llm::preset(id).ok_or_else(|| ("Unknown provider. Choose one from the list.".to_string(), true));
    }
    match llm::detect_provider(key) {
        llm::Detected::Provider(id) => Ok(llm::preset(id).expect("detected providers exist")),
        llm::Detected::Ambiguous => Err((
            "Keys that start with \"sk-\" are used by several providers. Choose the provider for this key, then save it again.".into(),
            true,
        )),
        llm::Detected::Unknown => {
            Err(("The app can't tell which provider this key belongs to. Choose the provider, then save it again.".into(), true))
        }
    }
}

/// The provider a message names, or the active one when it names none. An unknown id gives
/// `None`, so a typo can never act on a different provider's key.
fn named_or_active(msg: &Value) -> Option<&'static llm::Preset> {
    match msg.get("provider").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        Some(id) => llm::preset(id),
        None => Some(config::Config::read().active()),
    }
}

fn handle_ipc(body: &str, proxy: &EventLoopProxy<UserEvent>, prompts: &Arc<prompts::Prompts>) {
    let Ok(msg) = serde_json::from_str::<Value>(body) else { return };
    let send = |v: Value| {
        let _ = proxy.send_event(UserEvent::Emit(v));
    };
    match msg.get("cmd").and_then(Value::as_str).unwrap_or("") {
        "hello" => send(status()),
        "win" => {
            let ev = match msg.get("action").and_then(Value::as_str) {
                Some("min") => UserEvent::Minimize,
                Some("max") => UserEvent::ToggleMaximize,
                Some("drag") => UserEvent::Drag,
                Some("close") => UserEvent::Close,
                _ => return,
            };
            let _ = proxy.send_event(ev);
        }
        "save_key" => {
            // Check the key with its provider first (off the UI thread), then store it for this
            // Windows user and make that provider active.
            let key = msg.get("key").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let requested = msg.get("provider").and_then(Value::as_str).map(String::from);
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let send = |v: Value| {
                    let _ = proxy.send_event(UserEvent::Emit(v));
                };
                let preset = match key_provider(requested.as_deref(), &key) {
                    Ok(p) => p,
                    Err((message, needs_provider)) => {
                        send(json!({"type": "key_error", "message": message, "needs_provider": needs_provider}));
                        return;
                    }
                };
                let result = config::check_format(preset, &key)
                    .and_then(|()| llm::check_key(preset, &key))
                    .and_then(|()| config::save_key(preset, &key))
                    .and_then(|()| config::set_active(preset, None));
                match result {
                    Ok(()) => {
                        let mut s = status();
                        s["key_saved"] = json!(true);
                        send(s);
                    }
                    Err(e) => send(json!({"type": "key_error", "message": llm::scrub(&e, &key), "provider": preset.id})),
                }
            });
        }
        "open_key_page" => {
            // Only fixed URLs from the preset table; Gemini's page when no (or an unknown) provider is named.
            let preset = msg.get("provider").and_then(Value::as_str).and_then(llm::preset).unwrap_or_else(llm::default_preset);
            open_in_browser(preset.key_page);
        }
        "clear_key" => {
            // No provider named: the active provider's key is removed. Unknown id: nothing is removed.
            let mut s;
            match named_or_active(&msg) {
                Some(preset) => {
                    let result = config::clear_key(preset);
                    s = status();
                    if let Err(e) = result {
                        s["settings_error"] = json!(e);
                    }
                }
                None => {
                    s = status();
                    s["settings_error"] = json!("Unknown provider.");
                }
            }
            send(s);
        }
        "set_provider" => {
            let mut s;
            match msg.get("provider").and_then(Value::as_str).and_then(llm::preset) {
                Some(preset) => {
                    let model = msg.get("model").and_then(Value::as_str).map(str::trim).filter(|m| !m.is_empty());
                    let result = config::set_active(preset, model);
                    s = status();
                    if let Err(e) = result {
                        s["settings_error"] = json!(e);
                    }
                }
                None => {
                    s = status();
                    s["settings_error"] = json!("Unknown provider.");
                }
            }
            send(s);
        }
        "list_models" => {
            let Some(preset) = msg.get("provider").and_then(Value::as_str).and_then(llm::preset) else {
                let id = msg.get("provider").cloned().unwrap_or(Value::Null);
                send(json!({"type": "models_error", "provider": id, "message": "Unknown provider."}));
                return;
            };
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let reply = match config::key_for(preset).0 {
                    None => Err(format!("Add your {} API key first.", llm::short_name(preset.label))),
                    Some(key) => llm::list_models(preset, &key),
                };
                let v = match reply {
                    Ok(models) => json!({"type": "models", "provider": preset.id, "models": models}),
                    Err(e) => json!({"type": "models_error", "provider": preset.id, "message": e}),
                };
                let _ = proxy.send_event(UserEvent::Emit(v));
            });
        }
        "ask" => {
            let id = msg.get("id").and_then(Value::as_u64).unwrap_or(0);
            let question = msg.get("question").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let level = match msg.get("level").and_then(Value::as_str) {
                Some("intermediate") => "intermediate",
                _ => "beginner",
            };
            let settings = config::load();
            let Some(key) = settings.key.clone() else {
                let name = llm::short_name(settings.preset.label);
                send(json!({"type": "error", "id": id, "message": format!("Add your {name} API key in Settings first.")}));
                return;
            };
            if question.is_empty() {
                return;
            }
            let proxy = proxy.clone();
            let prompts = Arc::clone(prompts);
            std::thread::spawn(move || {
                let mut client = llm::Client::new(settings.preset, key, &settings.model, &settings.fallbacks);
                pipeline::run(&mut client, &prompts, id, &question, level, &mut |v| {
                    let _ = proxy.send_event(UserEvent::Emit(v));
                });
            });
        }
        _ => {}
    }
}

fn icon() -> Option<Icon> {
    // A small generated icon: lime ring on graphite (no image files needed).
    let n = 64u32;
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 - 31.5, y as f32 - 31.5);
            let r = (dx * dx + dy * dy).sqrt();
            let px = if r < 30.0 && r > 21.0 {
                [212, 255, 64, 255]
            } else if r <= 21.0 && r > 8.0 && (dx - dy).abs() < 4.0 {
                [212, 255, 64, 255]
            } else if r < 31.0 {
                [18, 19, 24, 255]
            } else {
                [0, 0, 0, 0]
            };
            rgba.extend_from_slice(&px);
        }
    }
    Icon::from_rgba(rgba, n, n).ok()
}

/// There is no console, so failures must be shown: a native message box.
fn fatal(msg: &str) -> ! {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (text, title) = (wide(msg), wide("Quantum Explainer"));
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
    std::process::exit(1)
}

/// Where WebView2 keeps its profile: never next to the exe (Program Files is read-only).
fn data_dir() -> Option<std::path::PathBuf> {
    config::data_dir()
}

fn main() {
    std::panic::set_hook(Box::new(|info| fatal(&format!("Something went wrong and the app has to close.\n\n{info}"))));
    if let Err(e) = run() {
        fatal(&format!(
            "The window could not start.\n\n{e}\n\nQuantum Explainer needs the Microsoft Edge WebView2 Runtime (included with Windows 11)."
        ));
    }
}

fn run() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let prompts = Arc::new(prompts::Prompts::load());

    // Fit the first window on small screens: at most 90% of the monitor's logical size.
    let (mut w, mut h) = (1360.0f64, 860.0f64);
    if let Some(m) = event_loop.primary_monitor() {
        let s = m.size().to_logical::<f64>(m.scale_factor());
        w = w.min(s.width * 0.9);
        h = h.min(s.height * 0.9);
    }
    let mut wb = WindowBuilder::new()
        .with_title("Quantum Explainer")
        .with_inner_size(LogicalSize::new(w, h))
        .with_min_inner_size(LogicalSize::new(w.min(960.0), h.min(600.0)))
        .with_decorations(false)
        .with_transparent(true)
        .with_window_icon(icon());
    #[cfg(windows)]
    {
        use tao::platform::windows::WindowBuilderExtWindows;
        wb = wb.with_undecorated_shadow(true);
    }
    let window = wb.build(&event_loop).map_err(|e| wry::Error::Io(std::io::Error::other(e.to_string())))?;
    // Windows 11 Mica backdrop behind the transparent title bar; on Windows 10 the page paints it solid.
    #[cfg(windows)]
    let mica = window_vibrancy::apply_mica(&window, Some(true)).is_ok();
    #[cfg(not(windows))]
    let mica = false;

    let ipc_proxy = proxy.clone();
    let mut context = wry::WebContext::new(data_dir());
    let mut builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_custom_protocol("app".into(), |_id, req| serve(req))
        .with_initialization_script(format!("document.documentElement.dataset.mica = '{mica}';"))
        // Only our own pages: no navigation away, no pop-ups, no file drops turning into navigations.
        .with_navigation_handler(|url| url.starts_with("https://app.localhost/") || url.starts_with("app://localhost/"))
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_drag_drop_handler(|_| true);
    #[cfg(windows)]
    {
        use wry::{ScrollBarStyle, Theme, WebViewBuilderExtWindows};
        builder = builder
            .with_https_scheme(true) // https://app.localhost → secure context (WebGPU etc.)
            .with_theme(Theme::Dark)
            .with_scroll_bar_style(ScrollBarStyle::FluentOverlay) // Windows 11 overlay scrollbars
            .with_default_context_menus(cfg!(debug_assertions));
    }
    let webview = builder
        .with_url("app://localhost/")
        .with_transparent(true)
        .with_devtools(cfg!(debug_assertions))
        .with_ipc_handler(move |req: Request<String>| {
            // Defence in depth: only our own page may drive the app.
            if req.uri().host() == Some("app.localhost") || req.uri().scheme_str() == Some("app") {
                handle_ipc(req.body(), &ipc_proxy, &prompts);
            }
        })
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::Emit(v)) => {
                // Pass the JSON as a string literal: no interpolation of untrusted text into code.
                let arg = serde_json::to_string(&v.to_string()).unwrap_or_default();
                let _ = webview.evaluate_script(&format!("window.__qe && window.__qe({arg})"));
            }
            Event::UserEvent(UserEvent::Minimize) => window.set_minimized(true),
            Event::UserEvent(UserEvent::ToggleMaximize) => window.set_maximized(!window.is_maximized()),
            Event::UserEvent(UserEvent::Drag) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::Close) | Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::key_provider;

    #[test]
    fn save_key_provider_resolution() {
        // An explicit provider always wins over the prefix.
        assert_eq!(key_provider(Some("deepseek"), "sk-abcdefghijklmnopqrstuvwxyz").unwrap().id, "deepseek");
        assert_eq!(key_provider(Some("openai"), "AIza-placeholder-not-a-key").unwrap().id, "openai");
        // Otherwise only unambiguous prefixes are routed.
        assert_eq!(key_provider(None, "sk-ant-api03-abcdefghijklmnop").unwrap().id, "anthropic");
        assert_eq!(key_provider(Some(""), "sk-or-v1-abcdefghijklmnop").unwrap().id, "openrouter");
        let (msg, needs) = key_provider(None, "sk-proj-abcdefghijklmnopqrstuvwxyz").unwrap_err();
        assert!(needs && msg.contains("Choose the provider"), "{msg}");
        assert!(key_provider(None, "unknown-key-format-0123456789").unwrap_err().1);
        assert!(key_provider(Some("not-a-provider"), "sk-ant-x").is_err());
    }
}
