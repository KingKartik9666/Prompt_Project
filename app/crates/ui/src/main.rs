//! Quantum Explainer interface (Leptos, client-side, compiled to WebAssembly).

mod atlas;
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
        topic: RwSignal::new(String::new()),
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
                    Screen::Atlas => view! { <atlas::Atlas /> }.into_any(),
                    Screen::Topic => view! { <atlas::TopicPage /> }.into_any(),
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
                aria-current=move || {
                    let s = app.screen.get();
                    if s == to || (to == Screen::Atlas && s == Screen::Topic) { "page" } else { "false" }
                }
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
                {link(Screen::Atlas, "Atlas")}
                {link(Screen::Tutor, "Tutor")}
                {link(Screen::Lab, "Lab")}
            </nav>
            <div class="title-fill"></div>
            <button class="nav-link key-btn" popovertarget="settings" data-has-key=move || app.status.with(|s| s.has_key).to_string()>
                {move || app.status.with(|s| if s.has_key { format!("{} connected", short_label(&s.provider_label)) } else { "Add AI key".to_string() })}
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
            <h2 id="settings-title">"AI provider"</h2>
            <KeyForm id="key-settings" />
        </div>
    }
}

/// "Anthropic (Claude)" → "Claude", "Google Gemini" → "Gemini", "OpenAI" → "OpenAI".
fn short_label(label: &str) -> String {
    if let (Some(a), Some(b)) = (label.find('('), label.rfind(')')) {
        return label[a + 1..b].to_string();
    }
    label.strip_prefix("Google ").unwrap_or(label).to_string()
}

/// Anyone can connect their own key for any supported provider (Gemini, Claude, OpenAI, Kimi, Qwen,
/// DeepSeek, Grok, Mistral, Groq, Together, OpenRouter). The host checks it with the provider, then
/// stores it in Windows Credential Manager for this Windows account only. Used in Settings and on the
/// Tutor's first screen.
#[component]
pub fn KeyForm(id: &'static str) -> impl IntoView {
    let app = use_app();
    let key = RwSignal::new(String::new());
    // "" = detect the provider from the key's prefix
    let choice = RwSignal::new(String::new());
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
        let p = choice.get_untracked();
        let msg = if p.is_empty() { json!({"cmd": "save_key", "key": k}) } else { json!({"cmd": "save_key", "key": k, "provider": p}) };
        bridge::send(msg);
        key.set(String::new());
    };
    let providers = move || app.status.with(|s| s.providers.clone());
    let active = move || app.status.with(|s| s.provider.clone());
    let options = move || {
        providers()
            .into_iter()
            .map(|p| {
                let label = if p.has_key { format!("{} ✓", p.label) } else { p.label.clone() };
                view! { <option value=p.id.clone()>{label}</option> }
            })
            .collect_view()
    };
    let connected = move || {
        providers()
            .into_iter()
            .filter(|p| p.has_key)
            .map(|p| {
                let pid = p.id.clone();
                let on = {
                    let pid = pid.clone();
                    move || active() == pid
                };
                view! {
                    <button type="button" class="seg" aria-pressed=move || on().to_string()
                        on:click=move |_| bridge::send(json!({"cmd": "set_provider", "provider": pid.clone()}))>
                        {short_label(&p.label)}
                    </button>
                }
            })
            .collect_view()
    };
    let models = move || {
        app.status.with(|s| match &s.models {
            Some((p, Ok(list))) if *p == s.provider => Some(list.clone()),
            _ => None,
        })
    };
    let models_err = move || {
        app.status.with(|s| match &s.models {
            Some((p, Err(e))) if *p == s.provider => e.clone(),
            _ => String::new(),
        })
    };
    let current_model = move || app.status.with(|s| s.providers.iter().find(|p| p.id == s.provider).map(|p| p.model.clone()).unwrap_or_default());
    view! {
        <p class="sheet-text">
            {move || {
                let s = app.status.get();
                if !app.host {
                    "Preview mode: open the desktop app to connect an AI provider.".to_string()
                } else if s.just_saved {
                    format!("Key ending {} works and is saved for your Windows account. {} is now answering your questions.", s.key_hint, s.provider_label)
                } else if s.has_key {
                    format!("{} is answering, with the key ending {}. Paste another provider's key to add it.", s.provider_label, s.key_hint)
                } else {
                    "The tutor works with your own API key from any major provider: Gemini, Claude, OpenAI, Kimi, Qwen, DeepSeek, Grok, Mistral and more. The key is checked with the provider, stored in Windows Credential Manager for your account only, and never shown again.".into()
                }
            }}
        </p>
        <Show when=move || app.status.with(|s| s.providers.iter().filter(|p| p.has_key).count() > 1)>
            <div class="sheet-row">
                <span class="field-label">"Use"</span>
                <div class="segs small">{connected}</div>
            </div>
        </Show>
        <Show when=move || app.status.with(|s| s.has_key)>
            <div class="sheet-row">
                <label class="field-label" for=format!("{id}-model")>"Model"</label>
                {move || match models() {
                    Some(list) => {
                        let cur = current_model();
                        let opts = list
                            .into_iter()
                            .map(|m| {
                                let sel = m == cur;
                                view! { <option value=m.clone() selected=sel>{m.clone()}</option> }
                            })
                            .collect_view();
                        view! {
                            <select id=format!("{id}-model") class="sheet-select"
                                on:change=move |e| bridge::send(json!({"cmd": "set_provider", "provider": active(), "model": event_target_value(&e)}))>
                                {opts}
                            </select>
                        }
                        .into_any()
                    }
                    None => view! {
                        <span class="sheet-model">{current_model}</span>
                        <button type="button" class="link" on:click=move |_| bridge::send(json!({"cmd": "list_models", "provider": active()}))>"Choose another"</button>
                    }
                    .into_any(),
                }}
            </div>
            <p class="error-line">{models_err}</p>
        </Show>
        <form
            class="sheet-form"
            on:submit=move |e| {
                e.prevent_default();
                save();
            }
        >
            <label class="field-label" for=format!("{id}-provider")>"Provider"</label>
            <select id=format!("{id}-provider") class="sheet-select" on:change=move |e| choice.set(event_target_value(&e))>
                <option value="" selected=true>"Detect from the key"</option>
                {options}
            </select>
            <label class="field-label" for=id>"API key"</label>
            <input
                id=id
                type="password"
                autocomplete="off"
                spellcheck="false"
                placeholder="Paste your API key"
                prop:value=move || key.get()
                on:input=move |e| key.set(event_target_value(&e))
            />
            <p class="error-line" role="alert">
                {move || app.status.with(|s| {
                    let e = s.key_error.clone().unwrap_or_default();
                    if s.needs_provider && !e.is_empty() { format!("{e} Choose the provider above.") } else { e }
                })}
            </p>
            <div class="sheet-actions">
                <button type="button" class="link get-key"
                    on:click=move |_| {
                        let p = choice.get_untracked();
                        let p = if p.is_empty() { active() } else { p };
                        bridge::send(json!({"cmd": "open_key_page", "provider": p}))
                    }>
                    "Get a key"
                </button>
                <Show when=move || app.status.with(|s| s.has_key && s.key_source == "Windows Credential Manager")>
                    <button type="button" class="btn ghost" on:click=move |_| bridge::send(json!({"cmd": "clear_key", "provider": active()}))>"Remove"</button>
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
