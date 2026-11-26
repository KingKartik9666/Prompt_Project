//! One client for every OpenAI-compatible provider: `POST {base}/chat/completions` with
//! `Authorization: Bearer <key>`, and `GET {base}/models` for the key check and model list.
//!
//! Only `model` and `messages` are sent. Temperature and `response_format` are left out because
//! their support differs between providers and models (some reasoning models reject a custom
//! temperature); the pipeline's JSON parser already copes with fenced or chatty JSON.

use super::{Outcome, Preset, format_used, key_check_result, list_error, read, record_use, send_with_retries, short_name};
use crate::prompts::Prompt;
use serde_json::{Value, json};

/// Reasoning models can think for minutes before answering: allow 10 minutes per attempt.
const TIMEOUT_SECS: u64 = 600;

pub struct OpenAiCompat {
    agent: ureq::Agent,
    preset: &'static Preset,
    key: String,
    model: String,
    used: Vec<(String, u32)>,
}

/// The chat-completions request body: optional system message, then the user message.
pub fn body(prompt: &Prompt, user: &str, model: &str) -> Value {
    let mut messages = Vec::new();
    if let Some(s) = &prompt.system {
        messages.push(json!({"role": "system", "content": s}));
    }
    messages.push(json!({"role": "user", "content": user}));
    json!({"model": model, "messages": messages})
}

/// The answer text of `choices[0].message`. `content` may be a string or (on some providers) an
/// array of parts; refusals and content-filter stops are errors.
pub fn parse(v: &Value, label: &str) -> Result<String, String> {
    let Some(choice) = v.pointer("/choices/0") else {
        return Err(format!("{label} returned no answer (empty response)."));
    };
    if let Some(r) = choice.pointer("/message/refusal").and_then(Value::as_str).filter(|r| !r.trim().is_empty()) {
        let r: String = r.chars().take(200).collect();
        return Err(format!("{label} declined to answer: {r}"));
    }
    let finish = choice.get("finish_reason").and_then(Value::as_str);
    if finish == Some("content_filter") {
        return Err(format!("{label} blocked the answer (content filter). Try rephrasing the question."));
    }
    let text = match choice.pointer("/message/content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|p| p.get("text").and_then(Value::as_str))
            .collect(),
        _ => String::new(),
    };
    if text.trim().is_empty() {
        return Err(format!("{label} returned no answer text (finish reason: {}).", finish.unwrap_or("none")));
    }
    Ok(text)
}

/// Model ids from a list response: `{"data": [{"id": …}]}` (OpenAI and most providers, also
/// Anthropic) or a bare `[{"id": …}]` array (shown in some providers' examples). Sorted, unique.
pub fn parse_model_ids(v: &Value) -> Vec<String> {
    let items = v.get("data").and_then(Value::as_array).or_else(|| v.as_array());
    let mut ids: Vec<String> = items
        .into_iter()
        .flatten()
        .filter_map(|m| m.get("id").and_then(Value::as_str))
        .filter(|id| !id.trim().is_empty())
        .map(String::from)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

impl OpenAiCompat {
    pub fn new(preset: &'static Preset, key: String, model: &str) -> Self {
        OpenAiCompat { agent: super::agent(TIMEOUT_SECS), preset, key, model: model.to_string(), used: Vec::new() }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn models_used(&self) -> String {
        format_used(&self.used, &self.model)
    }

    pub fn call(&mut self, prompt: &Prompt, user: &str, notice: &mut dyn FnMut(String)) -> Result<(String, f64), String> {
        let body = body(prompt, user, &self.model);
        let url = format!("{}/chat/completions", self.preset.base);
        let label = short_name(self.preset.label);
        let (agent, auth) = (&self.agent, format!("Bearer {}", self.key));
        let mut send = || read(agent.post(&url).header("Authorization", &auth).send_json(&body));
        match send_with_retries(label, &self.model, false, notice, &mut send, &|_, _| false) {
            Outcome::Answer(v, secs) => {
                let text = parse(&v, label)?;
                // Routers (OpenRouter) report the model that actually served the request.
                let served = v.get("model").and_then(Value::as_str).filter(|m| !m.is_empty()).unwrap_or(&self.model).to_string();
                record_use(&mut self.used, &served);
                Ok((text, secs))
            }
            Outcome::Fatal(e) => Err(e),
            Outcome::SwitchModel(last) | Outcome::Exhausted(last) => Err(format!("{label} kept failing ({last}). Please try again later.")),
        }
    }
}

fn get_models(agent: &ureq::Agent, preset: &Preset, key: &str) -> (u16, String) {
    read(agent.get(format!("{}/models", preset.base)).header("Authorization", format!("Bearer {key}")).call())
}

/// Key check. Listing models costs nothing; for a provider without a documented model list
/// (Qwen) the check is one minimal chat request to its documented default model.
pub fn check_key(preset: &'static Preset, key: &str) -> Result<(), String> {
    let (status, text) = if preset.lists_models {
        get_models(&super::agent(30), preset, key)
    } else {
        // A real generation: give a thinking model time to answer.
        let agent = super::agent(120);
        let probe = Prompt { temperature: None, mime: None, system: None, user: String::new() };
        let body = body(&probe, "Reply with OK.", preset.default_model);
        read(agent.post(format!("{}/chat/completions", preset.base)).header("Authorization", format!("Bearer {key}")).send_json(&body))
    };
    key_check_result(preset.label, status, &text)
}

/// Live model list from `GET {base}/models`.
pub fn list_models(preset: &'static Preset, key: &str) -> Result<Vec<String>, String> {
    let label = short_name(preset.label);
    if !preset.lists_models {
        return Err(format!(
            "{label} does not publish a model list for its OpenAI-compatible mode, so the app uses {}.",
            preset.default_model
        ));
    }
    let (status, text) = get_models(&super::agent(30), preset, key);
    if status != 200 {
        return Err(list_error(preset.label, status, &text));
    }
    let v: Value = serde_json::from_str(&text).map_err(|_| format!("{label} sent an unreadable model list."))?;
    let ids = parse_model_ids(&v);
    if ids.is_empty() {
        return Err(format!("{label} returned an empty model list."));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_is_system_then_user_with_nothing_else() {
        let p = Prompt { temperature: Some(0.2), mime: Some("application/json".into()), system: Some("sys".into()), user: String::new() };
        let b = body(&p, "hi", "m");
        assert_eq!(
            b,
            json!({"model": "m", "messages": [{"role": "system", "content": "sys"}, {"role": "user", "content": "hi"}]})
        );
        let p = Prompt { temperature: None, mime: None, system: None, user: String::new() };
        assert_eq!(body(&p, "hi", "m")["messages"], json!([{"role": "user", "content": "hi"}]));
    }

    #[test]
    fn parse_reads_first_choice() {
        let v = json!({"model": "x", "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": "{\"a\": 1}", "refusal": null}}]});
        assert_eq!(parse(&v, "OpenAI").unwrap(), "{\"a\": 1}");
        let parts = json!({"choices": [{"message": {"content": [{"type": "text", "text": "a"}, {"type": "image_url"}, {"type": "text", "text": "b"}]}}]});
        assert_eq!(parse(&parts, "X").unwrap(), "ab");
    }

    #[test]
    fn parse_errors_on_refusal_filter_and_empty() {
        let refusal = json!({"choices": [{"finish_reason": "stop", "message": {"content": null, "refusal": "I can't help with that."}}]});
        assert!(parse(&refusal, "OpenAI").unwrap_err().contains("declined"));
        let filtered = json!({"choices": [{"finish_reason": "content_filter", "message": {"content": ""}}]});
        assert!(parse(&filtered, "X").unwrap_err().contains("content filter"));
        assert!(parse(&json!({"choices": []}), "X").is_err());
        assert!(parse(&json!({"choices": [{"message": {"content": null}}]}), "X").is_err());
    }

    #[test]
    fn model_ids_from_both_list_shapes() {
        let wrapped = json!({"object": "list", "data": [{"id": "b"}, {"id": "a"}, {"id": "a"}, {"object": "model"}]});
        assert_eq!(parse_model_ids(&wrapped), vec!["a", "b"]);
        let bare = json!([{"id": "mistral-small-latest"}, {"id": "codestral-latest"}]);
        assert_eq!(parse_model_ids(&bare), vec!["codestral-latest", "mistral-small-latest"]);
        assert!(parse_model_ids(&json!({"error": {"message": "x"}})).is_empty());
    }
}
