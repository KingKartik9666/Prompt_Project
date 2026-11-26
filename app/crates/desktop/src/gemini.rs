//! Minimal Gemini REST client (generateContent) with retries and a model fallback chain,
//! mirroring `call()` in code/workflow.py. TLS goes through the OS (SChannel on Windows).
//! The retry loop itself is shared with the other providers (`llm::send_with_retries`).

use crate::llm::{self, Outcome, api_message};
use crate::prompts::Prompt;
use serde_json::{Value, json};
use std::collections::HashSet;

pub const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

fn dead_models() -> std::sync::MutexGuard<'static, HashSet<String>> {
    static DEAD: std::sync::LazyLock<std::sync::Mutex<HashSet<String>>> = std::sync::LazyLock::new(Default::default);
    DEAD.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct Gemini {
    agent: ureq::Agent,
    key: String,
    chain: Vec<String>,
    dead: HashSet<String>,
    used: Vec<(String, u32)>,
}

impl Gemini {
    pub fn new(key: String, model: &str, fallbacks: &[String]) -> Self {
        let mut chain = vec![model.to_string()];
        chain.extend(fallbacks.iter().filter(|m| m.as_str() != model).cloned());
        // Models that ran out of daily quota stay skipped for the whole session, like workflow.py's _dead.
        let dead = dead_models().clone();
        Gemini { agent: llm::agent(120), key, chain, dead, used: Vec::new() }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    /// Models that actually answered, e.g. "gemini-3.5-flash-lite (4)".
    pub fn models_used(&self) -> String {
        llm::format_used(&self.used, &self.chain[0])
    }

    /// One call. `notice` receives human-readable retry/fallback messages for the UI.
    pub fn call(&mut self, prompt: &Prompt, user: &str, notice: &mut dyn FnMut(String)) -> Result<(String, f64), String> {
        let body = body(prompt, user);
        let mut last = String::from("no model available");
        for model in self.chain.clone() {
            if self.dead.contains(&model) {
                continue;
            }
            let url = format!("{BASE}/{model}:generateContent");
            let (agent, key) = (&self.agent, &self.key);
            let mut send = || llm::read(agent.post(&url).header("x-goog-api-key", key).send_json(&body));
            // Out of daily quota, or a model that no longer exists: skip it for the session.
            let unavailable = |status: u16, text: &str| {
                let low = text.to_lowercase();
                (status == 429 && (low.contains("perday") || low.contains("per day"))) || status == 404 || text.contains("NOT_FOUND")
            };
            match llm::send_with_retries("Gemini", &model, true, notice, &mut send, &unavailable) {
                Outcome::Answer(v, secs) => {
                    llm::record_use(&mut self.used, &model);
                    // A safety block arrives as HTTP 200 with no text: stop with a clear reason
                    // instead of feeding an empty answer to the next prompts.
                    return match extract_text(&v) {
                        Some(t) => Ok((t, secs)),
                        None => Err(format!("Gemini returned no answer ({})", block_reason(&v))),
                    };
                }
                Outcome::SwitchModel(l) => {
                    self.dead.insert(model.clone());
                    dead_models().insert(model.clone());
                    last = l;
                }
                Outcome::Exhausted(l) => last = l,
                Outcome::Fatal(e) => return Err(e),
            }
        }
        Err(format!("All models failed ({}). Last error: {last}", self.chain.join(", ")))
    }
}

/// The generateContent body: the prompt's temperature and response MIME type are honoured here.
fn body(prompt: &Prompt, user: &str) -> Value {
    let mut generation = json!({});
    if let Some(t) = prompt.temperature {
        generation["temperature"] = json!(t);
    }
    if let Some(m) = &prompt.mime {
        generation["responseMimeType"] = json!(m);
    }
    let mut body = json!({
        "contents": [{"role": "user", "parts": [{"text": user}]}],
        "generationConfig": generation,
    });
    if let Some(s) = &prompt.system {
        body["systemInstruction"] = json!({"parts": [{"text": s}]});
    }
    body
}

/// Check a key before saving it: listing one model costs no generation quota.
pub fn check_key(key: &str) -> Result<(), String> {
    match llm::read(llm::agent(20).get(format!("{BASE}?pageSize=1")).header("x-goog-api-key", key).call()) {
        (200, _) => Ok(()),
        (400 | 401 | 403, text) => Err(format!("Google rejected this key: {}", api_message(&text))),
        (429, _) => Ok(()), // valid key, just out of quota right now
        (0, text) => Err(format!("Could not reach Google to check the key ({}).", api_message(&text))),
        (s, text) => Err(format!("Could not check the key (HTTP {s}: {}).", api_message(&text))),
    }
}

/// Models that support generateContent, as bare ids (the API names them `models/{id}`).
/// https://ai.google.dev/api/models/list-models: `models[]`, `nextPageToken`, pageSize ≤ 1000.
pub fn list_models(key: &str) -> Result<Vec<String>, String> {
    let agent = llm::agent(30);
    let mut ids = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..10 {
        let mut req = agent.get(BASE).header("x-goog-api-key", key).query("pageSize", "1000");
        if let Some(t) = &token {
            req = req.query("pageToken", t);
        }
        let (status, text) = llm::read(req.call());
        if status != 200 {
            return Err(llm::list_error("Google Gemini", status, &text));
        }
        let v: Value = serde_json::from_str(&text).map_err(|_| "Google sent an unreadable model list.".to_string())?;
        ids.extend(parse_model_ids(&v));
        token = v.get("nextPageToken").and_then(Value::as_str).filter(|t| !t.is_empty()).map(String::from);
        if token.is_none() {
            break;
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// `models[].name` without the `models/` prefix, keeping only models that can generateContent.
fn parse_model_ids(v: &Value) -> Vec<String> {
    v.get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|m| {
            m.get("supportedGenerationMethods")
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().any(|s| s.as_str() == Some("generateContent")))
        })
        .filter_map(|m| m.get("name").and_then(Value::as_str))
        .map(|n| n.strip_prefix("models/").unwrap_or(n).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn model_list_keeps_generators_and_strips_prefix() {
        let v = json!({"models": [
            {"name": "models/gemini-3.5-flash-lite", "supportedGenerationMethods": ["generateContent", "countTokens"]},
            {"name": "models/text-embedding-004", "supportedGenerationMethods": ["embedContent"]},
            {"name": "models/no-methods"}
        ]});
        assert_eq!(super::parse_model_ids(&v), vec!["gemini-3.5-flash-lite"]);
    }
}

#[cfg(test)]
mod live {
    /// One real router call (uses 1 request of the Gemini quota): `cargo test -p desktop -- --ignored live`
    #[test]
    #[ignore]
    fn live_router_call() {
        let gemini = crate::llm::default_preset();
        let c = crate::config::Config::read();
        let key = c.key(gemini).0.expect("GEMINI_API_KEY not found");
        let mut g = super::Gemini::new(key, &c.model(gemini), &c.fallbacks());
        let p = crate::prompts::Prompts::load();
        let user = crate::prompts::fill(&p.router.user, &[("question", "How do you create a Bell state?"), ("level", "beginner")]);
        let r = g.call(&p.router, &user, &mut |m| eprintln!("notice: {m}"));
        eprintln!("result: {r:?}");
        assert!(r.is_ok());
    }
}

/// Concatenate the answer text, skipping any "thought" parts.
/// `None` when the response carries no answer parts at all (a blocked prompt or candidate).
fn extract_text(v: &Value) -> Option<String> {
    let finish = v.pointer("/candidates/0/finishReason").and_then(Value::as_str).unwrap_or("STOP");
    if !matches!(finish, "STOP" | "MAX_TOKENS" | "FINISH_REASON_UNSPECIFIED") {
        return None;
    }
    v.pointer("/candidates/0/content/parts").and_then(Value::as_array).map(|parts| {
        parts
            .iter()
            .filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true))
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect::<String>()
    })
}

fn block_reason(v: &Value) -> String {
    v.pointer("/promptFeedback/blockReason")
        .or_else(|| v.pointer("/candidates/0/finishReason"))
        .and_then(Value::as_str)
        .map_or_else(|| "empty response".to_string(), |r| format!("blocked: {r}"))
}
