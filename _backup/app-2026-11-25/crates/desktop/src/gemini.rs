//! Minimal Gemini REST client (generateContent) with retries and a model fallback chain,
//! mirroring `call()` in code/workflow.py. TLS goes through the OS (SChannel on Windows).

use crate::prompts::Prompt;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
const WAITS: [Option<u64>; 4] = [Some(10), Some(30), Some(60), None];

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
        let config = ureq::Agent::config_builder()
            // The OS TLS stack with the OS certificate store (ureq's default root set is WebPKI).
            .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .build();
        let mut chain = vec![model.to_string()];
        chain.extend(fallbacks.iter().filter(|m| m.as_str() != model).cloned());
        // Models that ran out of daily quota stay skipped for the whole session, like workflow.py's _dead.
        let dead = dead_models().clone();
        Gemini { agent: ureq::Agent::new_with_config(config), key, chain, dead, used: Vec::new() }
    }

    /// Models that actually answered, e.g. "gemini-3.5-flash-lite (4)".
    pub fn models_used(&self) -> String {
        if self.used.is_empty() {
            return self.chain[0].clone();
        }
        self.used.iter().map(|(m, n)| format!("{m} ({n})")).collect::<Vec<_>>().join(", ")
    }

    /// One call. `notice` receives human-readable retry/fallback messages for the UI.
    pub fn call(&mut self, prompt: &Prompt, user: &str, notice: &mut dyn FnMut(String)) -> Result<(String, f64), String> {
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

        let mut last = String::from("no model available");
        for model in self.chain.clone() {
            if self.dead.contains(&model) {
                continue;
            }
            for wait in WAITS {
                let t0 = Instant::now();
                let (status, text) = match self
                    .agent
                    .post(format!("{BASE}/{model}:generateContent"))
                    .header("x-goog-api-key", &self.key)
                    .send_json(&body)
                {
                    // A body that breaks off mid-read is a network error like any other (retried below).
                    Ok(mut resp) => match resp.body_mut().read_to_string() {
                        Ok(t) => (resp.status().as_u16(), t),
                        Err(e) => (0, format!("disconnected while reading the answer: {e}")),
                    },
                    Err(e) => (0, e.to_string()),
                };
                if status == 200 {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(v) => {
                            match self.used.iter_mut().find(|(m, _)| *m == model) {
                                Some((_, n)) => *n += 1,
                                None => self.used.push((model.clone(), 1)),
                            }
                            // A safety block arrives as HTTP 200 with no text: stop with a clear reason
                            // instead of feeding an empty answer to the next prompts.
                            return match extract_text(&v) {
                                Some(t) => Ok((t, (t0.elapsed().as_secs_f64() * 100.0).round() / 100.0)),
                                None => Err(format!("Gemini returned no answer ({})", block_reason(&v))),
                            };
                        }
                        Err(_) => {
                            last = "network: the answer arrived incomplete".into();
                            if let Some(w) = wait {
                                notice(format!("{model}: incomplete answer; retrying in {w}s"));
                                std::thread::sleep(Duration::from_secs(w));
                                continue;
                            }
                            break;
                        }
                    }
                }
                let msg = api_message(&text);
                last = if status == 0 { format!("network: {msg}") } else { format!("HTTP {status}: {msg}") };
                let low = text.to_lowercase();
                if (status == 429 && (low.contains("perday") || low.contains("per day"))) || status == 404 || text.contains("NOT_FOUND") {
                    self.dead.insert(model.clone());
                    dead_models().insert(model.clone());
                    notice(format!("{model} unavailable ({last}); switching model"));
                    break;
                }
                // Network errors worth waiting for are timeouts and dropped connections (as in workflow.py).
                // No route, DNS or TLS failures will not fix themselves in a minute: fail fast.
                let network_transient = status == 0
                    && ["timed out", "timeout", "disconnected", "reset", "closed", "broken pipe"].iter().any(|s| low.contains(s));
                if status == 0 && !network_transient {
                    return Err(format!("Could not reach Gemini ({last}). Check the internet connection."));
                }
                let transient = network_transient
                    || matches!(status, 429 | 500 | 502 | 503 | 504)
                    || text.contains("RESOURCE_EXHAUSTED")
                    || text.contains("UNAVAILABLE");
                if !transient {
                    return Err(format!("Gemini rejected the request ({last})"));
                }
                match wait {
                    Some(w) => {
                        notice(format!("{model} busy ({last}); retrying in {w}s"));
                        std::thread::sleep(Duration::from_secs(w));
                    }
                    None => {
                        notice(format!("{model} still failing; switching model"));
                        break;
                    }
                }
            }
        }
        Err(format!("All models failed ({}). Last error: {last}", self.chain.join(", ")))
    }
}

/// Check a key before saving it: listing one model costs no generation quota.
pub fn check_key(key: &str) -> Result<(), String> {
    let config = ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    match agent.get(format!("{BASE}?pageSize=1")).header("x-goog-api-key", key).call() {
        Ok(mut resp) => {
            let status = resp.status().as_u16();
            let text = resp.body_mut().read_to_string().unwrap_or_default();
            match status {
                200 => Ok(()),
                400 | 401 | 403 => Err(format!("Google rejected this key: {}", api_message(&text))),
                429 => Ok(()), // valid key, just out of quota right now
                s => Err(format!("Could not check the key (HTTP {s}: {}).", api_message(&text))),
            }
        }
        Err(e) => Err(format!("Could not reach Google to check the key ({e}).")),
    }
}

#[cfg(test)]
mod live {
    /// One real router call (uses 1 request of the Gemini quota): `cargo test -p desktop -- --ignored live`
    #[test]
    #[ignore]
    fn live_router_call() {
        let s = crate::config::load();
        let key = s.key.expect("GEMINI_API_KEY not found");
        let mut g = super::Gemini::new(key, &s.model, &s.fallbacks);
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

/// Short, single-line error message (Google errors are JSON with error.message).
fn api_message(text: &str) -> String {
    let m = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| text.to_string());
    let one_line = m.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(140).collect()
}
