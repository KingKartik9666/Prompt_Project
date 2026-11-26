//! Anthropic Messages API over raw HTTP (Rust has no official Anthropic SDK).
//!
//! Request shape follows https://platform.claude.com/docs/en/api/overview.md and the Messages
//! reference: on API version 2026-10-21, `temperature` / `top_p` / `top_k` return 400, and on
//! Claude Opus 5.5 thinking is always on (adaptive), so neither sampling nor `thinking` is sent;
//! depth is set with `output_config.effort`. Refusals arrive as HTTP 200 with
//! `stop_reason: "refusal"`; on models that support it, server-side fallbacks (`fallbacks:
//! "default"` with the `server-side-fallback-2026-07-01` beta) let Anthropic re-run a declined
//! request on a fallback model inside the same call. Effort and fallbacks are sent only to models
//! documented to accept them, so any model picked from `/v1/models` gets a valid request.

use super::{Outcome, Preset, format_used, key_check_result, list_error, read, record_use, send_with_retries, short_name};
use crate::prompts::Prompt;
use serde_json::{Value, json};

pub const API_VERSION: &str = "2026-10-21";
pub const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
/// Non-streaming default that stays under HTTP timeouts.
pub const MAX_TOKENS: u32 = 16000;
pub const EFFORT: &str = "medium";
/// Adaptive thinking at 16k output tokens can take minutes: allow 10 minutes per attempt.
const TIMEOUT_SECS: u64 = 600;

pub struct Anthropic {
    agent: ureq::Agent,
    preset: &'static Preset,
    key: String,
    model: String,
    used: Vec<(String, u32)>,
}

/// Models that take server-side `fallbacks` (Anthropic's claude-api reference: include it for
/// claude-fable-5-2, claude-fable-5-1, claude-opus-5-5, claude-opus-5, claude-sonnet-5-5; Claude
/// Mythos 5.2 shares Fable 5.2's API surface). Other models get neither the field nor the beta
/// header: e.g. Claude Haiku 5.5 has no server-side fallback (a fallback list there is a 400).
const FALLBACK_MODELS: [&str; 6] =
    ["claude-opus-5-5", "claude-opus-5", "claude-fable-5-2", "claude-fable-5-1", "claude-mythos-5-2", "claude-sonnet-5-5"];

/// Model families that accept `output_config.effort` (same reference: Opus 4.5 and every model from
/// the 4.6 generation on; it errors on Sonnet 4.5 and Haiku 4.5). Others use their default depth.
const EFFORT_PREFIXES: [&str; 10] = [
    "claude-opus-4-5",
    "claude-opus-4-6",
    "claude-opus-4-7",
    "claude-opus-4-8",
    "claude-opus-5",
    "claude-sonnet-4-6",
    "claude-sonnet-5",
    "claude-haiku-5",
    "claude-fable-",
    "claude-mythos-",
];

pub fn uses_fallbacks(model: &str) -> bool {
    FALLBACK_MODELS.contains(&model)
}

pub fn supports_effort(model: &str) -> bool {
    EFFORT_PREFIXES.iter().any(|p| model.starts_with(p))
}

/// Headers for every request. The docs call `x-api-key` a legacy fallback for
/// `Authorization: Bearer`, still supported; it is the form this app was specified to use.
pub fn headers(key: &str, beta: bool) -> Vec<(&'static str, String)> {
    let mut h = vec![("x-api-key", key.to_string()), ("anthropic-version", API_VERSION.to_string())];
    if beta {
        h.push(("anthropic-beta", FALLBACK_BETA.to_string()));
    }
    h
}

/// The Messages request body for one prompt.
pub fn body(prompt: &Prompt, user: &str, model: &str) -> Value {
    let mut b = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "messages": [{"role": "user", "content": user}],
    });
    if supports_effort(model) {
        b["output_config"] = json!({"effort": EFFORT});
    }
    if uses_fallbacks(model) {
        b["fallbacks"] = json!("default");
    }
    if let Some(s) = &prompt.system {
        b["system"] = json!(s);
    }
    b
}

/// The answer text of a Messages response: `text` blocks only (`thinking` and `fallback` blocks
/// are skipped). A refusal is an error and its content is never read.
pub fn parse(v: &Value) -> Result<String, String> {
    let stop = v.get("stop_reason").and_then(Value::as_str).unwrap_or("");
    if stop == "refusal" {
        // stop_details is only populated for refusals; still guard every step.
        let category = v.pointer("/stop_details/category").and_then(Value::as_str);
        let detail = category.map(|c| format!(" (category: {c})")).unwrap_or_default();
        return Err(format!("Claude declined to answer this request{detail}. Try rephrasing the question."));
    }
    let Some(blocks) = v.get("content").and_then(Value::as_array) else {
        return Err("Anthropic returned no answer (empty response).".into());
    };
    let text: String = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    if text.trim().is_empty() {
        return Err(format!("Anthropic returned no answer text (stop reason: {}).", if stop.is_empty() { "none" } else { stop }));
    }
    Ok(text)
}

impl Anthropic {
    pub fn new(preset: &'static Preset, key: String, model: &str) -> Self {
        Anthropic { agent: super::agent(TIMEOUT_SECS), preset, key, model: model.to_string(), used: Vec::new() }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    /// Models that answered; with server-side fallbacks this can differ from the requested model.
    pub fn models_used(&self) -> String {
        format_used(&self.used, &self.model)
    }

    pub fn call(&mut self, prompt: &Prompt, user: &str, notice: &mut dyn FnMut(String)) -> Result<(String, f64), String> {
        let body = body(prompt, user, &self.model);
        let url = format!("{}/messages", self.preset.base);
        let label = short_name(self.preset.label);
        let (agent, key, beta) = (&self.agent, &self.key, uses_fallbacks(&self.model));
        let mut send = || {
            let mut req = agent.post(&url);
            for (k, v) in headers(key, beta) {
                req = req.header(k, v);
            }
            read(req.send_json(&body))
        };
        match send_with_retries(label, &self.model, false, notice, &mut send, &|_, _| false) {
            Outcome::Answer(v, secs) => {
                let served = v.get("model").and_then(Value::as_str).unwrap_or(&self.model).to_string();
                let text = parse(&v)?;
                record_use(&mut self.used, &served);
                Ok((text, secs))
            }
            Outcome::Fatal(e) => Err(e),
            Outcome::SwitchModel(last) | Outcome::Exhausted(last) => Err(format!("{label} kept failing ({last}). Please try again later.")),
        }
    }
}

/// One GET to the models endpoint (no generation cost).
fn get_models(agent: &ureq::Agent, preset: &Preset, key: &str, query: &[(&str, &str)]) -> (u16, String) {
    let mut req = agent.get(format!("{}/models", preset.base));
    for (k, v) in headers(key, false) {
        req = req.header(k, v);
    }
    for (k, v) in query {
        req = req.query(*k, *v);
    }
    read(req.call())
}

/// Key check: listing one model is free. 200 means the key is valid.
pub fn check_key(preset: &Preset, key: &str) -> Result<(), String> {
    let (status, text) = get_models(&super::agent(20), preset, key, &[("limit", "1")]);
    key_check_result(preset.label, status, &text)
}

/// Every model id, following `has_more` / `after_id` pagination (limit 1..=1000 per page).
pub fn list_models(preset: &Preset, key: &str) -> Result<Vec<String>, String> {
    let agent = super::agent(30);
    let label = short_name(preset.label);
    let mut ids = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..10 {
        let mut query = vec![("limit", "1000")];
        if let Some(a) = &after {
            query.push(("after_id", a.as_str()));
        }
        let (status, text) = get_models(&agent, preset, key, &query);
        if status != 200 {
            return Err(list_error(preset.label, status, &text));
        }
        let v: Value = serde_json::from_str(&text).map_err(|_| format!("{label} sent an unreadable model list."))?;
        ids.extend(super::openai::parse_model_ids(&v));
        let more = v.get("has_more").and_then(Value::as_bool) == Some(true);
        after = v.get("last_id").and_then(Value::as_str).map(String::from);
        if !more || after.is_none() {
            break;
        }
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(system: Option<&str>) -> Prompt {
        Prompt { temperature: Some(0.2), mime: Some("application/json".into()), system: system.map(String::from), user: String::new() }
    }

    #[test]
    fn body_has_no_sampling_or_thinking_and_enables_fallbacks() {
        let b = body(&prompt(Some("You are a tutor.")), "Explain a Bell state.", DEFAULT_MODEL);
        for banned in ["temperature", "top_p", "top_k", "thinking"] {
            assert!(b.get(banned).is_none(), "{banned} must not be sent");
        }
        assert_eq!(b["model"], "claude-opus-5-5");
        assert_eq!(b["max_tokens"], 16000);
        assert_eq!(b["system"], "You are a tutor.");
        assert_eq!(b["messages"], json!([{"role": "user", "content": "Explain a Bell state."}]));
        assert_eq!(b["output_config"], json!({"effort": "medium"}));
        assert_eq!(b["fallbacks"], "default");
        assert!(body(&prompt(None), "q", DEFAULT_MODEL).get("system").is_none());
    }

    #[test]
    fn fallbacks_and_effort_only_where_documented() {
        assert!(uses_fallbacks(DEFAULT_MODEL) && supports_effort(DEFAULT_MODEL));
        // Haiku 5.5: effort yes, server-side fallback no.
        let haiku = body(&prompt(None), "q", "claude-haiku-5-5");
        assert!(haiku.get("fallbacks").is_none());
        assert_eq!(haiku["output_config"], json!({"effort": "medium"}));
        assert!(!uses_fallbacks("claude-haiku-5-5"));
        // Haiku 4.5 / Sonnet 4.5: neither (effort errors there).
        for old in ["claude-haiku-4-5", "claude-haiku-4-5-20251001", "claude-sonnet-4-5"] {
            let b = body(&prompt(None), "q", old);
            assert!(b.get("fallbacks").is_none() && b.get("output_config").is_none(), "{old}");
            assert!(b.get("temperature").is_none());
        }
        assert!(supports_effort("claude-opus-4-8") && !uses_fallbacks("claude-opus-4-8"));
    }

    #[test]
    fn headers_carry_version_and_beta() {
        assert_eq!(API_VERSION, "2026-10-21");
        assert_eq!(FALLBACK_BETA, "server-side-fallback-2026-07-01");
        let h = headers("k", true);
        assert!(h.contains(&("x-api-key", "k".into())));
        assert!(h.contains(&("anthropic-version", "2026-10-21".into())));
        assert!(h.contains(&("anthropic-beta", "server-side-fallback-2026-07-01".into())));
        assert!(!headers("k", false).iter().any(|(n, _)| *n == "anthropic-beta"));
    }

    #[test]
    fn parse_joins_text_blocks_and_skips_others() {
        let v = json!({
            "model": "claude-opus-5-5",
            "stop_reason": "end_turn",
            "content": [
                {"type": "thinking", "thinking": "", "signature": "x"},
                {"type": "fallback", "from": {"model": "a"}, "to": {"model": "b"}},
                {"type": "text", "text": "{\"in_scope\": "},
                {"type": "text", "text": "true}"}
            ]
        });
        assert_eq!(parse(&v).unwrap(), "{\"in_scope\": true}");
    }

    #[test]
    fn refusal_is_an_error_and_content_is_not_read() {
        let v = json!({
            "stop_reason": "refusal",
            "stop_details": {"type": "refusal", "category": "cyber", "explanation": null},
            "content": [{"type": "text", "text": "partial"}]
        });
        let e = parse(&v).unwrap_err();
        assert!(e.contains("declined") && e.contains("cyber") && !e.contains("partial"), "{e}");
        assert!(parse(&json!({"stop_reason": "refusal"})).is_err());
    }

    #[test]
    fn empty_answer_is_an_error() {
        assert!(parse(&json!({"stop_reason": "max_tokens", "content": [{"type": "thinking", "thinking": ""}]})).is_err());
        assert!(parse(&json!({})).is_err());
    }
}
