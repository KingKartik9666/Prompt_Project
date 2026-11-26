//! App-wide state and how host events update it.

use leptos::prelude::*;
use qcore::Algo;
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Atlas,
    Topic,
    Tutor,
    Lab,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Stage {
    #[default]
    Waiting,
    Active,
    Done,
    Skipped,
    Failed,
}

pub const STAGES: [(&str, &str, &str); 4] = [
    ("route", "Read the question", "The AI model works out the algorithm, your level and any mistaken belief."),
    ("explain", "Explain it", "Step by step, with the state after every gate."),
    ("verify", "Check the numbers", "The built-in simulator recomputes every final probability."),
    ("practice", "Practise", "One new question, reviewed against the verified result."),
];

/// One AI provider the desktop host can use (from the host's status event).
#[derive(Clone, Default, Debug, PartialEq)]
pub struct Provider {
    pub id: String,
    pub label: String,
    pub has_key: bool,
    pub key_hint: String,
    pub model: String,
    pub lists_models: bool,
}

#[derive(Clone, Default, Debug)]
pub struct Status {
    pub known: bool,
    /// Active provider id and label, e.g. "anthropic" / "Anthropic (Claude)".
    pub provider: String,
    pub provider_label: String,
    pub providers: Vec<Provider>,
    /// Live model list for a provider: (provider id, ids), or an error message.
    pub models: Option<(String, Result<Vec<String>, String>)>,
    pub needs_provider: bool,
    pub has_key: bool,
    pub key_source: String,
    pub key_hint: String,
    pub key_error: Option<String>,
    pub checking: bool,
    pub just_saved: bool,
}

#[derive(Clone, Default, Debug)]
pub struct Run {
    pub id: u64,
    pub question: String,
    pub level: String,
    pub running: bool,
    pub stages: [Stage; 4],
    pub route: Option<Value>,
    pub verifiable: bool,
    pub explanation: Option<Value>,
    pub repaired: Option<Vec<String>>,
    pub verification: Option<Value>,
    pub algo: Option<Algo>,
    pub practice: Option<Value>,
    pub final_msg: Option<String>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub models: String,
    pub seconds: f64,
}

#[derive(Clone, Copy)]
pub struct AppState {
    pub screen: RwSignal<Screen>,
    pub status: RwSignal<Status>,
    pub run: RwSignal<Run>,
    /// Last arrow key pressed (key, counter) so screens can react without their own listeners.
    pub key: RwSignal<(String, u32)>,
    /// The Atlas topic shown on Screen::Topic.
    pub topic: RwSignal<String>,
    pub host: bool,
}

pub fn use_app() -> AppState {
    expect_context::<AppState>()
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().to_string()
}

impl AppState {
    pub fn handle(&self, ev: Value) {
        let kind = s(&ev, "type");
        if kind == "status" {
            let providers = ev
                .get("providers")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|p| Provider {
                            id: s(p, "id"),
                            label: s(p, "label"),
                            has_key: p.get("has_key").and_then(Value::as_bool).unwrap_or(false),
                            key_hint: s(p, "key_hint"),
                            model: s(p, "model"),
                            lists_models: p.get("lists_models").and_then(Value::as_bool).unwrap_or(true),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let models = self.status.with_untracked(|st| st.models.clone());
            self.status.set(Status {
                known: true,
                provider: s(&ev, "provider"),
                provider_label: s(&ev, "provider_label"),
                providers,
                models,
                needs_provider: false,
                has_key: ev.get("has_key").and_then(Value::as_bool).unwrap_or(false),
                key_source: s(&ev, "key_source"),
                key_hint: s(&ev, "key_hint"),
                key_error: None,
                checking: false,
                just_saved: ev.get("key_saved").and_then(Value::as_bool).unwrap_or(false),
            });
            return;
        }
        if kind == "models" || kind == "models_error" {
            let p = s(&ev, "provider");
            let list = if kind == "models" {
                Ok(ev.get("models").and_then(Value::as_array).map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).collect()).unwrap_or_default())
            } else {
                Err(s(&ev, "message"))
            };
            self.status.update(|st| st.models = Some((p, list)));
            return;
        }
        if kind == "key_error" {
            self.status.update(|st| {
                st.needs_provider = ev.get("needs_provider").and_then(Value::as_bool).unwrap_or(false);
                st.key_error = Some(s(&ev, "message"));
                st.checking = false;
            });
            return;
        }
        let id = ev.get("id").and_then(Value::as_u64).unwrap_or(0);
        self.run.update(|r| {
            if id != r.id {
                return; // an older question still finishing
            }
            match kind.as_str() {
                "stage" => {
                    let i = STAGES.iter().position(|(k, ..)| *k == s(&ev, "stage"));
                    let st = match s(&ev, "state").as_str() {
                        "active" => Stage::Active,
                        "done" => Stage::Done,
                        "skipped" => Stage::Skipped,
                        "error" => Stage::Failed,
                        _ => Stage::Waiting,
                    };
                    if let Some(i) = i {
                        r.stages[i] = st;
                    }
                }
                "route" => {
                    r.route = ev.get("data").cloned();
                    r.verifiable = ev.get("verifiable").and_then(Value::as_bool).unwrap_or(false);
                }
                "explanation" => r.explanation = ev.get("data").cloned(),
                "repair" => {
                    r.repaired = Some(
                        ev.get("mismatches")
                            .and_then(Value::as_array)
                            .map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                    )
                }
                "verification" => {
                    r.verification = ev.get("data").cloned();
                    let key = s(&ev, "trace_algo");
                    r.algo = qcore::norm_params(&key, ev.get("params").unwrap_or(&Value::Null));
                }
                "practice" => r.practice = ev.get("data").cloned(),
                "notice" => r.notice = Some(s(&ev, "message")),
                "done" => {
                    r.running = false;
                    r.final_msg = ev.get("final").and_then(Value::as_str).map(String::from);
                    r.models = s(&ev, "models");
                    r.seconds = ev.get("seconds").and_then(Value::as_f64).unwrap_or(0.0);
                    r.notice = None;
                }
                "error" => {
                    r.running = false;
                    r.error = Some(s(&ev, "message"));
                    r.notice = None;
                    for st in r.stages.iter_mut() {
                        if *st == Stage::Active {
                            *st = Stage::Failed;
                        }
                    }
                }
                _ => {}
            }
        });
    }

    pub fn ask(&self, question: String, level: &str) {
        let id = self.run.with_untracked(|r| r.id) + 1;
        self.run.set(Run { id, question: question.clone(), level: level.into(), running: true, ..Default::default() });
        crate::bridge::send(serde_json::json!({"cmd": "ask", "id": id, "question": question, "level": level}));
    }

    pub fn go(&self, to: Screen) {
        if self.screen.get_untracked() == to {
            return;
        }
        let screen = self.screen;
        crate::bridge::view_transition(move || {
            screen.set(to);
            crate::bridge::scroll_stage_top();
        });
    }

    /// Open an Atlas topic page (also from one topic page to another).
    pub fn open_topic(&self, id: &str) {
        let (screen, topic, id) = (self.screen, self.topic, id.to_string());
        crate::bridge::view_transition(move || {
            topic.set(id);
            screen.set(Screen::Topic);
            crate::bridge::scroll_stage_top();
        });
    }
}
