//! Where each provider's key, the active provider and the chosen models come from.
//! Keys never reach the web view and are never written to a file by the app.
//!
//! Keys: Windows Credential Manager (service `QuantumExplainer`, account `<provider>_api_key`;
//! Gemini keeps `gemini_api_key`, so keys saved by earlier versions still work) → the provider's
//! environment variable (e.g. `ANTHROPIC_API_KEY`) → a `.env` file in the executable's folder or
//! any parent (developer convenience, same as the prototype).
//!
//! Choices: `%LOCALAPPDATA%\QuantumExplainer\settings.json` holds the active provider and the
//! model chosen per provider. It never contains a key.

use crate::llm::{self, Kind, PRESETS, Preset, short_name};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const SERVICE: &str = "QuantumExplainer";
pub const DEFAULT_GEMINI_MODEL: &str = "gemini-3.5-flash-lite";
const DEFAULT_FALLBACKS: &str = "gemini-3.1-flash-lite";

/// The persisted choices (settings.json). Deliberately has no field that could hold a key.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct Stored {
    /// Active provider id.
    pub provider: Option<String>,
    /// Chosen model per provider id.
    pub models: BTreeMap<String, String>,
}

/// Everything needed to run the pipeline with the active provider.
pub struct Settings {
    pub preset: &'static Preset,
    pub key: Option<String>,
    pub model: String,
    /// Gemini's client-side fallback chain (unused by the other providers).
    pub fallbacks: Vec<String>,
}

/// The app's per-user folder (also the WebView2 profile): never next to the exe.
pub fn data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("QuantumExplainer"))
}

fn settings_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("settings.json"))
}

/// The saved choices; a missing or unreadable file means "nothing chosen yet".
pub fn read_stored() -> Stored {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Save the choices: write a temporary file next to settings.json, then replace it.
pub fn write_stored(s: &Stored) -> Result<(), String> {
    let path = settings_path().ok_or("LOCALAPPDATA is not set.")?;
    let dir = path.parent().ok_or("Bad settings path.")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Could not create the settings folder: {e}"))?;
    let text = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Could not save settings: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Could not save settings: {e}"))
}

fn dotenv() -> Vec<(String, String)> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            starts.push(d.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    for start in starts {
        for dir in start.ancestors() {
            if let Some(v) = read_env_file(&dir.join(".env")) {
                return v;
            }
        }
    }
    Vec::new()
}

fn read_env_file(p: &Path) -> Option<Vec<(String, String)>> {
    let text = std::fs::read_to_string(p).ok()?;
    Some(
        text.lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().trim_matches('"').to_string()))
            .collect(),
    )
}

/// Credential Manager account for a provider: `gemini_api_key`, `anthropic_api_key`, …
pub fn account(preset: &Preset) -> String {
    format!("{}_api_key", preset.id)
}

fn entry(preset: &Preset) -> Option<keyring::Entry> {
    keyring::Entry::new(SERVICE, &account(preset)).ok()
}

/// One read of everything outside the keyring (settings.json and the .env file).
pub struct Config {
    pub stored: Stored,
    env: Vec<(String, String)>,
}

impl Config {
    pub fn read() -> Self {
        Config { stored: read_stored(), env: dotenv() }
    }

    /// Environment variable first, then the .env file.
    fn lookup(&self, k: &str) -> Option<String> {
        std::env::var(k)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| self.env.iter().find(|(ek, v)| ek == k && !v.is_empty()).map(|(_, v)| v.clone()))
    }

    /// The key for a provider and where it came from.
    pub fn key(&self, preset: &Preset) -> (Option<String>, &'static str) {
        // A key the user entered in the app always wins, so anyone can switch to their own key.
        if let Some(k) = entry(preset).and_then(|e| e.get_password().ok()).filter(|s| !s.trim().is_empty()) {
            (Some(k), "Windows Credential Manager")
        } else if let Some(k) = std::env::var(preset.env).ok().filter(|s| !s.trim().is_empty()) {
            (Some(k), "environment variable")
        } else if let Some(k) = self.lookup(preset.env) {
            (Some(k), ".env file")
        } else {
            (None, "not set")
        }
    }

    /// The model for a provider: the learner's choice, else (Gemini) GEMINI_MODEL, else the default.
    pub fn model(&self, preset: &Preset) -> String {
        if let Some(m) = self.stored.models.get(preset.id).filter(|m| !m.trim().is_empty()) {
            return m.clone();
        }
        if preset.kind == Kind::Gemini {
            if let Some(m) = self.lookup("GEMINI_MODEL") {
                return m;
            }
        }
        preset.default_model.to_string()
    }

    /// Gemini's fallback chain (GEMINI_FALLBACKS, comma-separated).
    pub fn fallbacks(&self) -> Vec<String> {
        self.lookup("GEMINI_FALLBACKS")
            .unwrap_or_else(|| DEFAULT_FALLBACKS.into())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// The active provider: the saved choice; otherwise Gemini (as before) unless only another
    /// provider has a key (e.g. just ANTHROPIC_API_KEY is set).
    pub fn active(&self) -> &'static Preset {
        if let Some(p) = self.stored.provider.as_deref().and_then(llm::preset) {
            return p;
        }
        let gemini = llm::default_preset();
        if self.key(gemini).0.is_some() {
            return gemini;
        }
        PRESETS.iter().find(|p| self.key(p).0.is_some()).unwrap_or(gemini)
    }
}

/// Settings for the active provider.
pub fn load() -> Settings {
    let c = Config::read();
    let preset = c.active();
    let (key, _) = c.key(preset);
    Settings { preset, key, model: c.model(preset), fallbacks: c.fallbacks() }
}

/// The key for one provider and its source (see `Config::key`).
pub fn key_for(preset: &Preset) -> (Option<String>, &'static str) {
    Config::read().key(preset)
}

pub fn check_format(preset: &Preset, key: &str) -> Result<(), String> {
    if key.len() < 20 || key.len() > 400 || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!(
            "That doesn't look like a {} API key. Copy it again from the provider's key page.",
            short_name(preset.label)
        ));
    }
    Ok(())
}

/// A model id chosen in the UI: plain characters only (it ends up in a URL path for Gemini).
pub fn check_model_id(model: &str) -> Result<(), String> {
    let ok = !model.is_empty()
        && model.len() <= 200
        && model.chars().all(|c| c.is_ascii_alphanumeric() || "._-/:~@+".contains(c))
        && !model.contains("..");
    if ok { Ok(()) } else { Err("That model id contains characters the app does not accept.".into()) }
}

pub fn save_key(preset: &Preset, key: &str) -> Result<(), String> {
    let key = key.trim();
    check_format(preset, key)?;
    entry(preset)
        .ok_or("Credential Manager is unavailable.")?
        .set_password(key)
        .map_err(|e| format!("Could not save the key: {e}"))
}

pub fn clear_key(preset: &Preset) -> Result<(), String> {
    match entry(preset) {
        Some(e) => match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Could not remove the key: {e}")),
        },
        None => Ok(()),
    }
}

/// Make `preset` the active provider and, optionally, remember a model for it.
pub fn set_active(preset: &Preset, model: Option<&str>) -> Result<(), String> {
    let mut s = read_stored();
    s.provider = Some(preset.id.to_string());
    if let Some(m) = model {
        check_model_id(m)?;
        s.models.insert(preset.id.to_string(), m.to_string());
    }
    write_stored(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_keep_the_old_gemini_name() {
        assert_eq!(account(llm::default_preset()), "gemini_api_key");
        assert_eq!(account(llm::preset("anthropic").unwrap()), "anthropic_api_key");
    }

    #[test]
    fn stored_settings_hold_no_key() {
        let mut s = Stored { provider: Some("anthropic".into()), models: BTreeMap::new() };
        s.models.insert("anthropic".into(), "claude-opus-5-5".into());
        let v = serde_json::to_value(&s).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, vec!["provider", "models"]);
        // Old, partial or foreign files load as defaults instead of failing.
        assert_eq!(serde_json::from_str::<Stored>("{}").unwrap(), Stored::default());
        let extra: Stored = serde_json::from_str(r#"{"provider":"groq","api_key":"x"}"#).unwrap();
        assert_eq!(extra.provider.as_deref(), Some("groq"));
        assert!(!serde_json::to_string(&extra).unwrap().contains("api_key"));
    }

    #[test]
    fn model_ids_are_plain() {
        for ok in ["claude-opus-5-5", "openai/gpt-5.2", "MiniMaxAI/MiniMax-M3.1-Flash", "~openai/gpt-sol-latest"] {
            assert!(check_model_id(ok).is_ok(), "{ok}");
        }
        for bad in ["", "a b", "../x", "x?y", "x#y"] {
            assert!(check_model_id(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn stored_model_wins_over_default() {
        let preset = llm::preset("openai").unwrap();
        let mut c = Config { stored: Stored::default(), env: Vec::new() };
        assert_eq!(c.model(preset), preset.default_model);
        c.stored.models.insert("openai".into(), "chosen".into());
        assert_eq!(c.model(preset), "chosen");
    }
}
