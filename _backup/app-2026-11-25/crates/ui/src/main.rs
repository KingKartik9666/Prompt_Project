//! Quantum Explainer interface (Leptos, client-side, compiled to WebAssembly).

mod bridge;
mod glow;
mod home;
mod lab;
mod state;
mod tutor;
mod viz;

use leptos::prelude::*;
use serde_json::json;
use state::{AppState, Screen, use_app};
use wasm_bindgen::JsCast;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let host = bridge::has_host();
    let app = AppState {
        screen: RwSignal::new(Screen::Home),
        status: RwSignal::new(Default::default()),
        run: RwSignal::new(Default::default()),
        key: RwSignal::new((String::new(), 0)),
        host,
    };
    provide_context(app);
    bridge::listen(move |ev| app.handle(ev));
    if host {
        bridge::send(json!({"cmd": "hello"}));
    }

    let _keys = window_event_listener(leptos::ev::keydown, move |e| {
        let typing = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|el| el.closest("input, textarea").ok().flatten().is_some());
        if !typing && matches!(e.key().as_str(), "ArrowLeft" | "ArrowRight") {
            app.key.update(|(k, n)| {
                *k = e.key();
                *n += 1;
            });
        }
    });

    glow::install();

    // Home's scroll-scrubbed motion: new targets on scroll and when the window is resized.
    let sync_home = move || {
        if app.screen.get_untracked() == Screen::Home {
            home::on_scroll();
        }
    };
    let _resize = window_event_listener(leptos::ev::resize, move |_| {
        if app.screen.get_untracked() == Screen::Home {
            home::on_resize();
        }
    });

    view! {
        <div class="app" data-host=host.to_string()>
            <TitleBar />
            <main
                class="stage"
                data-screen=move || format!("{:?}", app.screen.get()).to_lowercase()
                on:scroll=move |_| sync_home()
            >
                <div class="glow" aria-hidden="true"></div>
                {move || match app.screen.get() {
                    Screen::Home => view! { <home::Home /> }.into_any(),
                    Screen::Tutor => view! { <tutor::Tutor /> }.into_any(),
                    Screen::Lab => view! { <lab::Lab /> }.into_any(),
                }}
            </main>
            <Settings />
        </div>
    }
}

#[component]
fn TitleBar() -> impl IntoView {
    let app = use_app();
    let win = |action: &'static str| move |_| bridge::send(json!({"cmd": "win", "action": action}));
    let on_down = move |e: web_sys::MouseEvent| {
        if e.button() != 0 || !app.host {
            return;
        }
        let interactive = e
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|el| el.closest("button, a, input, [data-nodrag]").ok().flatten().is_some());
        if !interactive {
            if e.detail() == 2 {
                bridge::send(json!({"cmd": "win", "action": "max"}));
            } else {
                bridge::send(json!({"cmd": "win", "action": "drag"}));
            }
        }
    };
    let link = move |to: Screen, label: &'static str| {
        view! {
            <button
                class="nav-link"
                aria-current=move || if app.screen.get() == to { "page" } else { "false" }
                on:click=move |_| app.go(to)
            >
                {label}
            </button>
        }
    };
    view! {
        <header class="titlebar" on:mousedown=on_down>
            <button class="wordmark" on:click=move |_| app.go(Screen::Home) aria-label="Quantum Explainer, home">
                <svg class="mark" viewBox="0 0 32 32" aria-hidden="true">
                    <path d="M6 4v24" />
                    <path d="M16 4l10 12-10 12" />
                    <circle cx="16" cy="16" r="3.2" />
                </svg>
                <span>"Quantum Explainer"</span>
            </button>
            <nav class="nav" aria-label="Screens">
                {link(Screen::Home, "Home")}
                {link(Screen::Tutor, "Tutor")}
                {link(Screen::Lab, "Lab")}
            </nav>
            <div class="title-fill"></div>
            <button class="nav-link key-btn" popovertarget="settings" data-has-key=move || app.status.with(|s| s.has_key).to_string()>
                {move || if app.status.with(|s| s.has_key) { "Gemini connected" } else { "Add Gemini key" }}
            </button>
            <Show when=move || app.host>
                <div class="win-buttons">
                    <button class="win" aria-label="Minimise" on:click=win("min")>
                        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 6h8" /></svg>
                    </button>
                    <button class="win" aria-label="Maximise" on:click=win("max")>
                        <svg viewBox="0 0 12 12" aria-hidden="true"><rect x="2" y="2" width="8" height="8" /></svg>
                    </button>
                    <button class="win win-close" aria-label="Close" on:click=win("close")>
                        <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" /></svg>
                    </button>
                </div>
            </Show>
        </header>
    }
}

#[component]
fn Settings() -> impl IntoView {
    view! {
        <div id="settings" popover class="sheet" role="dialog" aria-labelledby="settings-title">
            <h2 id="settings-title">"Gemini key"</h2>
            <KeyForm id="key-settings" />
        </div>
    }
}

/// Anyone can connect their own Gemini key: it is checked with Google, then stored for their
/// Windows account only. Used in Settings and on the Tutor's first screen.
#[component]
pub fn KeyForm(id: &'static str) -> impl IntoView {
    let app = use_app();
    let key = RwSignal::new(String::new());
    let save = move || {
        let k = key.get_untracked();
        if k.trim().is_empty() || !app.host {
            return;
        }
        app.status.update(|s| {
            s.checking = true;
            s.key_error = None;
            s.just_saved = false;
        });
        bridge::send(json!({"cmd": "save_key", "key": k}));
        key.set(String::new());
    };
    view! {
        <p class="sheet-text">
            {move || {
                let s = app.status.get();
                if !app.host {
                    "Preview mode: open the desktop app to connect Gemini.".to_string()
                } else if s.just_saved {
                    format!("Key ending {} works and is saved for your Windows account. You can ask questions now.", s.key_hint)
                } else if s.has_key {
                    format!("Using the key ending {} from the {}. Model: {}. Paste another key to switch.", s.key_hint, s.key_source, s.model)
                } else {
                    "The tutor uses Google Gemini with your own free API key. Paste it here: it is checked with Google, stored in Windows Credential Manager for your account only, and never shown again.".into()
                }
            }}
        </p>
        <form
            class="sheet-form"
            on:submit=move |e| {
                e.prevent_default();
                save();
            }
        >
            <label class="field-label" for=id>"API key"</label>
            <input
                id=id
                type="password"
                autocomplete="off"
                spellcheck="false"
                placeholder="AIza…"
                prop:value=move || key.get()
                on:input=move |e| key.set(event_target_value(&e))
            />
            <p class="error-line" role="alert">{move || app.status.with(|s| s.key_error.clone().unwrap_or_default())}</p>
            <div class="sheet-actions">
                <button type="button" class="link get-key" on:click=move |_| bridge::send(json!({"cmd": "open_key_page"}))>
                    "Get a free key"
                </button>
                <Show when=move || app.status.with(|s| s.has_key && s.key_source == "Windows Credential Manager")>
                    <button type="button" class="btn ghost" on:click=move |_| bridge::send(json!({"cmd": "clear_key"}))>"Remove"</button>
                </Show>
                <button
                    type="submit"
                    class="btn primary"
                    disabled=move || { key.with(|k| k.trim().is_empty()) || !app.host || app.status.with(|s| s.checking) }
                >
                    {move || if app.status.with(|s| s.checking) { "Checking…" } else { "Save key" }}
                </button>
            </div>
        </form>
    }
}
