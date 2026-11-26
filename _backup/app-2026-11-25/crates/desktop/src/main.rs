//! Quantum Explainer — native Windows app.
//!
//! A frameless tao window with a Mica backdrop hosts a WebView2 view. The interface (Leptos,
//! compiled to WebAssembly) is embedded in this executable and served over a private `app://`
//! protocol: no browser, no local web server, no open port. The Gemini key stays in this process.

#![windows_subsystem = "windows"] // a GUI app: never open a console window

mod config;
mod gemini;
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

fn status(settings: &config::Settings) -> Value {
    // Only the last four characters ever leave this process, so people can tell which key is in use.
    let hint = settings.key.as_deref().map(|k| k.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<String>());
    json!({
        "type": "status",
        "has_key": settings.key.is_some(),
        "key_hint": hint,
        "key_source": settings.key_source,
        "model": settings.model,
        "version": env!("CARGO_PKG_VERSION"),
    })
}

fn handle_ipc(body: &str, proxy: &EventLoopProxy<UserEvent>, prompts: &Arc<prompts::Prompts>) {
    let Ok(msg) = serde_json::from_str::<Value>(body) else { return };
    let send = |v: Value| {
        let _ = proxy.send_event(UserEvent::Emit(v));
    };
    match msg.get("cmd").and_then(Value::as_str).unwrap_or("") {
        "hello" => send(status(&config::load())),
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
            // Check the key with Google first (off the UI thread), then store it for this Windows user.
            let key = msg.get("key").and_then(Value::as_str).unwrap_or("").trim().to_string();
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let send = |v: Value| {
                    let _ = proxy.send_event(UserEvent::Emit(v));
                };
                let result = config::check_format(&key)
                    .and_then(|()| gemini::check_key(&key))
                    .and_then(|()| config::save_key(&key));
                match result {
                    Ok(()) => {
                        let mut s = status(&config::load());
                        s["key_saved"] = json!(true);
                        send(s);
                    }
                    Err(e) => send(json!({"type": "key_error", "message": e})),
                }
            });
        }
        "open_key_page" => open_in_browser("https://aistudio.google.com/apikey"),
        "clear_key" => {
            let _ = config::clear_key();
            send(status(&config::load()));
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
                send(json!({"type": "error", "id": id, "message": "Add your Gemini API key in Settings first."}));
                return;
            };
            if question.is_empty() {
                return;
            }
            let proxy = proxy.clone();
            let prompts = Arc::clone(prompts);
            std::thread::spawn(move || {
                let mut gemini = gemini::Gemini::new(key, &settings.model, &settings.fallbacks);
                pipeline::run(&mut gemini, &prompts, id, &question, level, &mut |v| {
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
    std::env::var_os("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("QuantumExplainer"))
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
