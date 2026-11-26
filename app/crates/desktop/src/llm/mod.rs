//! Provider abstraction: one `Client` the pipeline talks to, whichever LLM the learner chose.
//!
//! Three wire formats cover every preset: Google's native Gemini API, Anthropic's Messages API,
//! and the OpenAI-compatible `chat/completions` API that the other providers expose.
//! Every base URL, key page and default model below was checked against the provider's own
//! documentation (fetched 2026-11-25); the page used is cited next to each preset.

pub mod anthropic;
pub mod openai;

use crate::gemini::{self, Gemini};
use crate::prompts::Prompt;
use serde_json::Value;
use std::time::{Duration, Instant};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

/// Which wire protocol a provider speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Gemini,
    Anthropic,
    OpenAi,
}

/// One provider the app can use. Everything here is fixed at compile time: the web view can only
/// name a provider by `id`, never supply a URL.
#[derive(Debug)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    /// API root. Gemini: the models collection; Anthropic: `/v1`; OpenAI-compatible: the base
    /// such that `{base}/chat/completions` is the chat endpoint.
    pub base: &'static str,
    /// The provider's official page for creating an API key.
    pub key_page: &'static str,
    /// Environment variable read as a fallback when no key is saved in Credential Manager.
    pub env: &'static str,
    /// Model used until the learner picks one (documented id; see the citation per preset).
    pub default_model: &'static str,
    /// Whether the provider documents a model-list endpoint we can call with the key.
    pub lists_models: bool,
}

pub const GEMINI: &str = "gemini";

pub static PRESETS: &[Preset] = &[
    // https://ai.google.dev/gemini-api/docs/api-key (key page button: aistudio.google.com/apikey)
    // https://ai.google.dev/api/models/list-models (GET /v1beta/models)
    Preset {
        id: GEMINI,
        label: "Google Gemini",
        kind: Kind::Gemini,
        base: gemini::BASE,
        key_page: "https://aistudio.google.com/apikey",
        env: "GEMINI_API_KEY",
        default_model: crate::config::DEFAULT_GEMINI_MODEL,
        lists_models: true,
    },
    // https://platform.claude.com/docs/en/api/overview.md (base, key page "platform.claude.com/settings/keys")
    // https://platform.claude.com/docs/en/api/models/list.md (GET /v1/models, data[].id, has_more/after_id)
    Preset {
        id: "anthropic",
        label: "Anthropic (Claude)",
        kind: Kind::Anthropic,
        base: "https://api.anthropic.com/v1",
        key_page: "https://platform.claude.com/settings/keys",
        env: "ANTHROPIC_API_KEY",
        default_model: anthropic::DEFAULT_MODEL,
        lists_models: true,
    },
    // https://developers.openai.com/api/reference/overview (base https://api.openai.com/v1, Bearer auth)
    // https://developers.openai.com/api/reference/resources/models/methods/list (GET /models, data[].id)
    // https://developers.openai.com/api/docs/quickstart (key page, model "gpt-6.2-astra")
    // https://developers.openai.com/api/docs/models ("If you're not sure where to start, use GPT-6.2 Astra")
    Preset {
        id: "openai",
        label: "OpenAI",
        kind: Kind::OpenAi,
        base: "https://api.openai.com/v1",
        key_page: "https://platform.openai.com/api-keys",
        env: "OPENAI_API_KEY",
        default_model: "gpt-6.2-astra",
        lists_models: true,
    },
    // https://api-docs.deepseek.com/ (base_url "https://api.deepseek.com", Bearer auth, key page,
    //   "Use deepseek-flash as the model name")
    // https://api-docs.deepseek.com/api/list-models (GET /models, data[].id)
    Preset {
        id: "deepseek",
        label: "DeepSeek",
        kind: Kind::OpenAi,
        base: "https://api.deepseek.com",
        key_page: "https://platform.deepseek.com/api_keys",
        env: "DEEPSEEK_API_KEY",
        default_model: "deepseek-flash",
        lists_models: true,
    },
    // https://platform.kimi.ai/docs/api/chat (base_url "https://api.moonshot.ai/v1", Bearer auth,
    //   key page, model "kimi-k3.1-pro"); platform.moonshot.ai redirects to platform.kimi.ai.
    // https://platform.kimi.ai/docs/api/list-models (GET /v1/models, data[].id). Keys from the
    //   China platform (platform.kimi.com) are not interchangeable with this endpoint.
    Preset {
        id: "moonshot",
        label: "Moonshot AI (Kimi)",
        kind: Kind::OpenAi,
        base: "https://api.moonshot.ai/v1",
        key_page: "https://platform.kimi.ai/console/api-keys",
        env: "MOONSHOT_API_KEY",
        default_model: "kimi-k3.1-pro",
        lists_models: true,
    },
    // https://www.alibabacloud.com/help/en/model-studio/base-url ("Singapore
    //   https://dashscope-intl.aliyuncs.com/compatible-mode/v1"; a legacy shared domain that is
    //   "still available" and "accepts an API Key from any workspace in the same region")
    // https://www.alibabacloud.com/help/en/model-studio/compatibility-of-openai-with-dashscope
    //   (Bearer auth, Singapore example uses "qwen4-max")
    // https://www.alibabacloud.com/help/en/model-studio/get-api-key (key page, per region)
    // No OpenAI-style `{base}/models` is documented, so this preset has no live model list.
    Preset {
        id: "qwen",
        label: "Alibaba Qwen (Model Studio, international)",
        kind: Kind::OpenAi,
        base: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1",
        key_page: "https://modelstudio.console.alibabacloud.com/ap-southeast-1/settings/api-key",
        env: "DASHSCOPE_API_KEY",
        default_model: "qwen4-max",
        lists_models: false,
    },
    // https://docs.x.ai/developers/model-capabilities/legacy/chat-completions (base_url
    //   "https://api.x.ai/v1", model "grok-4.7"; Chat Completions is labelled legacy but supported)
    // https://docs.x.ai/developers/rest-api-reference/management/auth (Bearer token)
    // https://docs.x.ai/developers/rest-api-reference/inference/models (GET /v1/models, data[].id)
    // https://docs.x.ai/developers/quickstart (key page)
    Preset {
        id: "xai",
        label: "xAI (Grok)",
        kind: Kind::OpenAi,
        base: "https://api.x.ai/v1",
        key_page: "https://console.x.ai/team/default/api-keys",
        env: "XAI_API_KEY",
        default_model: "grok-4.7",
        lists_models: true,
    },
    // https://docs.mistral.ai/getting-started/quickstarts/studio/activate-and-generate-api-key
    //   (curl https://api.mistral.ai/v1/chat/completions, Bearer auth, "mistral-small-latest",
    //   key page https://console.mistral.ai/api-keys)
    // https://docs.mistral.ai/api/endpoint/models (GET /v1/models; schema and example disagree on
    //   the wrapper, so the parser accepts both shapes)
    Preset {
        id: "mistral",
        label: "Mistral AI",
        kind: Kind::OpenAi,
        base: "https://api.mistral.ai/v1",
        key_page: "https://console.mistral.ai/api-keys",
        env: "MISTRAL_API_KEY",
        default_model: "mistral-small-latest",
        lists_models: true,
    },
    // https://console.groq.com/docs/openai (base_url "https://api.groq.com/openai/v1", key page /keys)
    // https://console.groq.com/docs/models (Bearer auth, GET /models, production model
    //   "llama-3.3-70b-versatile")
    Preset {
        id: "groq",
        label: "Groq",
        kind: Kind::OpenAi,
        base: "https://api.groq.com/openai/v1",
        key_page: "https://console.groq.com/keys",
        env: "GROQ_API_KEY",
        default_model: "llama-3.3-70b-versatile",
        lists_models: true,
    },
    // https://docs.together.ai/docs/openai-api-compatibility (base_url "https://api.together.ai/v1",
    //   key page, model "MiniMaxAI/MiniMax-M3.1-Flash", "GET /v1/models | Supported")
    // https://docs.together.ai/docs/api-keys-authentication.md (Authorization: Bearer)
    Preset {
        id: "together",
        label: "Together AI",
        kind: Kind::OpenAi,
        base: "https://api.together.ai/v1",
        key_page: "https://api.together.ai/settings/projects/~current/api-keys",
        env: "TOGETHER_API_KEY",
        default_model: "MiniMaxAI/MiniMax-M3.1-Flash",
        lists_models: true,
    },
    // https://openrouter.ai/docs/api/reference/authentication (base "https://openrouter.ai/api/v1",
    //   "Authorization: Bearer <OPENROUTER_API_KEY>", keys begin with "sk-or-v1-", key page,
    //   example model "openai/gpt-5.2")
    // https://openrouter.ai/docs/api/api-reference/models/get-models (GET /models, data[].id)
    Preset {
        id: "openrouter",
        label: "OpenRouter",
        kind: Kind::OpenAi,
        base: "https://openrouter.ai/api/v1",
        key_page: "https://openrouter.ai/keys",
        env: "OPENROUTER_API_KEY",
        default_model: "openai/gpt-5.2",
        lists_models: true,
    },
];

/// Look a provider up by id (ids come from the web view, so unknown ids simply return `None`).
pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// The provider used when nothing else is configured (keeps old installs working unchanged).
pub fn default_preset() -> &'static Preset {
    preset(GEMINI).expect("gemini preset")
}

/// Result of guessing a provider from a pasted key.
#[derive(Debug, PartialEq, Eq)]
pub enum Detected {
    Provider(&'static str),
    /// A prefix several providers share (plain `sk-`): the learner must choose.
    Ambiguous,
    Unknown,
}

/// Guess the provider from a key prefix, only where the prefix is unambiguous. Order matters:
/// the specific `sk-ant-` / `sk-or-` prefixes are tested before the shared `sk-`.
/// Sources: `sk-ant-` platform.claude.com/docs/en/get-api-key ("starts with `sk-ant-`");
/// `sk-or-` openrouter.ai/docs/api/reference/authentication; `AIza` the example key on
/// docs.cloud.google.com/docs/authentication/api-keys; `gsk_` console.groq.com/docs/
/// production-readiness/security-onboarding (example key); `xai-` docs.x.ai/developers/
/// rest-api-reference/management/auth (example key). Detection only routes a key; it never
/// rejects one, and an explicit provider from the UI always wins.
pub fn detect_provider(key: &str) -> Detected {
    let key = key.trim();
    let table: [(&str, &str); 5] = [
        ("sk-ant-", "anthropic"),
        ("sk-or-", "openrouter"),
        ("AIza", GEMINI),
        ("gsk_", "groq"),
        ("xai-", "xai"),
    ];
    if let Some((_, id)) = table.iter().find(|(prefix, _)| key.starts_with(prefix)) {
        return Detected::Provider(id);
    }
    if key.starts_with("sk-") { Detected::Ambiguous } else { Detected::Unknown }
}

/// The last four characters of a key: the only part of a key that ever reaches the web view.
pub fn key_hint(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    chars[chars.len().saturating_sub(4)..].iter().collect()
}

/// Remove the key from any text that might be shown in the UI (some APIs echo part of it back).
pub fn scrub(text: &str, key: &str) -> String {
    let key = key.trim();
    if key.len() < 8 {
        return text.to_string();
    }
    text.replace(key, &format!("…{}", key_hint(key)))
}

/// HTTP agent shared by every provider: the OS TLS stack (SChannel) with the OS certificate store,
/// and HTTP error statuses returned as values so the caller can read the error body.
pub fn agent(timeout_secs: u64) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(timeout_secs)))
        .build();
    ureq::Agent::new_with_config(config)
}

/// Responses up to this size are read (OpenRouter's full model list is large).
const MAX_BODY: u64 = 32 * 1024 * 1024;

/// Status and body of one request; status 0 means the request never got an HTTP answer.
pub fn read(result: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> (u16, String) {
    match result {
        // A body that breaks off mid-read is a network error like any other (retried by the caller).
        Ok(mut resp) => match resp.body_mut().with_config().limit(MAX_BODY).read_to_string() {
            Ok(t) => (resp.status().as_u16(), t),
            Err(e) => (0, format!("disconnected while reading the answer: {e}")),
        },
        Err(e) => (0, e.to_string()),
    }
}

/// Short, single-line error message. Google, Anthropic and OpenAI-style APIs all put it at
/// `error.message`; anything else is shown as-is, truncated.
pub fn api_message(text: &str) -> String {
    let m = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v.pointer("/error/message").and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| text.to_string());
    let one_line = m.split_whitespace().collect::<Vec<_>>().join(" ");
    one_line.chars().take(140).collect()
}

/// Waits between attempts on one model, as in code/workflow.py (`None` = last attempt).
pub const WAITS: [Option<u64>; 4] = [Some(10), Some(30), Some(60), None];

/// How a run of attempts against one model ended.
pub enum Outcome {
    /// HTTP 200 with a JSON body, and how long the successful attempt took (seconds, 2 d.p.).
    Answer(Value, f64),
    /// The model is gone for this session (e.g. daily quota, unknown model): try the next one.
    SwitchModel(String),
    /// Every retry failed with a transient error; carries the last error.
    Exhausted(String),
    /// A permanent error: stop the whole call with this message.
    Fatal(String),
}

/// Send one request with retries and the UI notices the Gemini client always had, for any provider.
/// `send` performs one attempt; `unavailable` says whether an error means "this model is gone".
/// `more_models` only changes the wording of the final notice.
pub fn send_with_retries(
    label: &str,
    model: &str,
    more_models: bool,
    notice: &mut dyn FnMut(String),
    send: &mut dyn FnMut() -> (u16, String),
    unavailable: &dyn Fn(u16, &str) -> bool,
) -> Outcome {
    let mut last = String::from("no attempt made");
    for wait in WAITS {
        let t0 = Instant::now();
        let (status, text) = send();
        if status == 200 {
            match serde_json::from_str::<Value>(&text) {
                Ok(v) => return Outcome::Answer(v, (t0.elapsed().as_secs_f64() * 100.0).round() / 100.0),
                Err(_) => {
                    last = "network: the answer arrived incomplete".into();
                    if let Some(w) = wait {
                        notice(format!("{model}: incomplete answer; retrying in {w}s"));
                        std::thread::sleep(Duration::from_secs(w));
                        continue;
                    }
                    return Outcome::Exhausted(last);
                }
            }
        }
        let msg = api_message(&text);
        last = if status == 0 { format!("network: {msg}") } else { format!("HTTP {status}: {msg}") };
        if unavailable(status, &text) {
            notice(format!("{model} unavailable ({last}); switching model"));
            return Outcome::SwitchModel(last);
        }
        let low = text.to_lowercase();
        // Network errors worth waiting for are timeouts and dropped connections (as in workflow.py).
        // No route, DNS or TLS failures will not fix themselves in a minute: fail fast.
        let network_transient =
            status == 0 && ["timed out", "timeout", "disconnected", "reset", "closed", "broken pipe"].iter().any(|s| low.contains(s));
        if status == 0 && !network_transient {
            return Outcome::Fatal(format!("Could not reach {label} ({last}). Check the internet connection."));
        }
        // 529 is Anthropic's "overloaded"; the strings are Google's busy/quota statuses.
        let transient = network_transient
            || matches!(status, 429 | 500 | 502 | 503 | 504 | 529)
            || text.contains("RESOURCE_EXHAUSTED")
            || text.contains("UNAVAILABLE")
            || text.contains("overloaded_error");
        if !transient {
            return Outcome::Fatal(format!("{label} rejected the request ({last})"));
        }
        match wait {
            Some(w) => {
                notice(format!("{model} busy ({last}); retrying in {w}s"));
                std::thread::sleep(Duration::from_secs(w));
            }
            None => {
                notice(format!("{model} still failing; {}", if more_models { "switching model" } else { "giving up" }));
                return Outcome::Exhausted(last);
            }
        }
    }
    Outcome::Exhausted(last)
}

/// Count one answer from `model` (for the "models used" line shown after a run).
pub fn record_use(used: &mut Vec<(String, u32)>, model: &str) {
    match used.iter_mut().find(|(m, _)| m == model) {
        Some((_, n)) => *n += 1,
        None => used.push((model.to_string(), 1)),
    }
}

/// "model (n), model (n)", or the configured model when nothing answered yet.
pub fn format_used(used: &[(String, u32)], configured: &str) -> String {
    if used.is_empty() {
        return configured.to_string();
    }
    used.iter().map(|(m, n)| format!("{m} ({n})")).collect::<Vec<_>>().join(", ")
}

/// The client the pipeline uses: one per question, for the active provider.
pub enum Client {
    Gemini(Gemini),
    Anthropic(anthropic::Anthropic),
    OpenAi(openai::OpenAiCompat),
}

impl Client {
    /// `fallbacks` only applies to Gemini (its client-side model chain); Anthropic falls back
    /// server-side, and the OpenAI-compatible providers use the single chosen model.
    pub fn new(preset: &'static Preset, key: String, model: &str, fallbacks: &[String]) -> Self {
        match preset.kind {
            Kind::Gemini => Client::Gemini(Gemini::new(key, model, fallbacks)),
            Kind::Anthropic => Client::Anthropic(anthropic::Anthropic::new(preset, key, model)),
            Kind::OpenAi => Client::OpenAi(openai::OpenAiCompat::new(preset, key, model)),
        }
    }

    /// One call. `notice` receives human-readable retry/fallback messages for the UI.
    /// Errors and notices are scrubbed of the key before they can reach the UI.
    pub fn call(&mut self, prompt: &Prompt, user: &str, notice: &mut dyn FnMut(String)) -> Result<(String, f64), String> {
        let key = match self {
            Client::Gemini(c) => c.key().to_string(),
            Client::Anthropic(c) => c.key().to_string(),
            Client::OpenAi(c) => c.key().to_string(),
        };
        let mut clean_notice = |m: String| notice(scrub(&m, &key));
        let r = match self {
            Client::Gemini(c) => c.call(prompt, user, &mut clean_notice),
            Client::Anthropic(c) => c.call(prompt, user, &mut clean_notice),
            Client::OpenAi(c) => c.call(prompt, user, &mut clean_notice),
        };
        r.map_err(|e| scrub(&e, &key))
    }

    /// Models that actually answered, e.g. "claude-opus-5-5 (4)".
    pub fn models_used(&self) -> String {
        match self {
            Client::Gemini(c) => c.models_used(),
            Client::Anthropic(c) => c.models_used(),
            Client::OpenAi(c) => c.models_used(),
        }
    }
}

/// Check a key with its provider before saving it. Errors are human-readable and key-free.
pub fn check_key(preset: &'static Preset, key: &str) -> Result<(), String> {
    let r = match preset.kind {
        Kind::Gemini => gemini::check_key(key),
        Kind::Anthropic => anthropic::check_key(preset, key),
        Kind::OpenAi => openai::check_key(preset, key),
    };
    r.map_err(|e| scrub(&e, key))
}

/// Live model list for a provider, sorted. Errors are human-readable and key-free.
pub fn list_models(preset: &'static Preset, key: &str) -> Result<Vec<String>, String> {
    let r = match preset.kind {
        Kind::Gemini => gemini::list_models(key),
        Kind::Anthropic => anthropic::list_models(preset, key),
        Kind::OpenAi => openai::list_models(preset, key),
    };
    r.map_err(|e| scrub(&e, key))
}

/// Provider name for messages: the label without its parenthesised detail ("Anthropic (Claude)" → "Anthropic").
pub fn short_name(label: &str) -> &str {
    label.split(" (").next().unwrap_or(label)
}

/// Shared wording for a key check that got an HTTP answer.
pub fn key_check_result(label: &str, status: u16, text: &str) -> Result<(), String> {
    let label = short_name(label);
    match status {
        200 => Ok(()),
        429 => Ok(()), // a valid key that is rate-limited right now
        401 | 403 => Err(format!("That key was rejected by {label} ({status}).")),
        0 => Err(format!("Could not reach {label} to check the key ({}).", api_message(text))),
        s => Err(format!("Could not check the key with {label} (HTTP {s}: {}).", api_message(text))),
    }
}

/// Shared wording for a model-list request that did not return 200.
pub fn list_error(label: &str, status: u16, text: &str) -> String {
    let label = short_name(label);
    match status {
        401 | 403 => format!("{label} rejected the saved key ({status}). Save a new key and try again."),
        429 => format!("{label} is rate-limiting this key right now (429). Try again in a minute."),
        0 => format!("Could not reach {label} to load the model list ({}).", api_message(text)),
        s => format!("Could not load the model list from {label} (HTTP {s}: {}).", api_message(text)),
    }
}

#[cfg(test)]
mod live {
    /// One real router call with the active provider and model (costs one request on that
    /// provider): `cargo test -p desktop -- --ignored live`
    #[test]
    #[ignore]
    fn live_active_provider_router_call() {
        let s = crate::config::load();
        let key = s.key.expect("no key for the active provider");
        let mut c = super::Client::new(s.preset, key, &s.model, &s.fallbacks);
        let p = crate::prompts::Prompts::load();
        let user = crate::prompts::fill(&p.router.user, &[("question", "How do you create a Bell state?"), ("level", "beginner")]);
        let r = c.call(&p.router, &user, &mut |m| eprintln!("notice: {m}"));
        eprintln!("{} / {}: {r:?}", s.preset.id, c.models_used());
        assert!(r.is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_route_only_when_unambiguous() {
        assert_eq!(detect_provider("sk-ant-api03-abcdefghijklmnopqrstuvwxyz"), Detected::Provider("anthropic"));
        assert_eq!(detect_provider("sk-or-v1-0123456789abcdef"), Detected::Provider("openrouter"));
        assert_eq!(detect_provider("  AIza-placeholder-not-a-key "), Detected::Provider("gemini"));
        assert_eq!(detect_provider("gsk_abcdefghijklmnopqrstuvwxyz"), Detected::Provider("groq"));
        assert_eq!(detect_provider("xai-abcdefghijklmnopqrstuvwxyz"), Detected::Provider("xai"));
        // OpenAI, DeepSeek, Moonshot and Qwen keys can all start with plain "sk-".
        assert_eq!(detect_provider("sk-proj-abcdefghijklmnopqrstuvwxyz"), Detected::Ambiguous);
        assert_eq!(detect_provider("sk-abcdefghijklmnopqrstuvwxyz"), Detected::Ambiguous);
        assert_eq!(detect_provider("LLM_123_abcdefghijklmnopqrstuvwxyz"), Detected::Unknown);
    }

    #[test]
    fn every_detected_provider_exists_and_ids_are_unique() {
        for k in ["sk-ant-x", "sk-or-x", "AIzax", "gsk_x", "xai-x"] {
            let Detected::Provider(id) = detect_provider(k) else { panic!("{k}") };
            assert!(preset(id).is_some(), "{id}");
        }
        let mut ids: Vec<_> = PRESETS.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), PRESETS.len());
        assert!(PRESETS.iter().all(|p| p.base.starts_with("https://") && p.key_page.starts_with("https://")));
    }

    #[test]
    fn keys_never_leak_into_messages() {
        let key = "sk-ant-api03-SECRETSECRETSECRET-wxyz";
        assert_eq!(key_hint(key), "wxyz");
        let msg = scrub(&format!("invalid x-api-key {key}"), key);
        assert!(!msg.contains("SECRET") && msg.ends_with("…wxyz"), "{msg}");
    }

    #[test]
    fn key_check_wording() {
        assert_eq!(key_check_result("Anthropic (Claude)", 401, ""), Err("That key was rejected by Anthropic (401).".into()));
        assert_eq!(short_name("Moonshot AI (Kimi)"), "Moonshot AI");
        assert_eq!(short_name("OpenAI"), "OpenAI");
        assert!(key_check_result("X", 200, "").is_ok());
        assert!(key_check_result("X", 429, "").is_ok());
    }
}
