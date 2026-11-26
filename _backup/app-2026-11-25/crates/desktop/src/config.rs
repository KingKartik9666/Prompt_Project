//! Where the Gemini key and model come from. The key never reaches the web view.
//!
//! Order: Windows Credential Manager (set from the app) → environment variable → a `.env`
//! file in the executable's folder or any parent (developer convenience, same as the prototype).

use std::path::{Path, PathBuf};

const SERVICE: &str = "QuantumExplainer";
const ACCOUNT: &str = "gemini_api_key";
pub const DEFAULT_MODEL: &str = "gemini-3.5-flash-lite";
const DEFAULT_FALLBACKS: &str = "gemini-3.1-flash-lite";

pub struct Settings {
    pub key: Option<String>,
    pub key_source: &'static str,
    pub model: String,
    pub fallbacks: Vec<String>,
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

fn entry() -> Option<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).ok()
}

pub fn load() -> Settings {
    let env = dotenv();
    let lookup = |k: &str| -> Option<String> {
        std::env::var(k)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| env.iter().find(|(ek, v)| ek == k && !v.is_empty()).map(|(_, v)| v.clone()))
    };
    // A key the user entered in the app always wins, so anyone can switch to their own key.
    let (key, key_source) = if let Some(k) = entry().and_then(|e| e.get_password().ok()).filter(|s| !s.trim().is_empty()) {
        (Some(k), "Windows Credential Manager")
    } else if let Some(k) = std::env::var("GEMINI_API_KEY").ok().filter(|s| !s.trim().is_empty()) {
        (Some(k), "environment variable")
    } else if let Some(k) = lookup("GEMINI_API_KEY") {
        (Some(k), ".env file")
    } else {
        (None, "not set")
    };
    let model = lookup("GEMINI_MODEL").unwrap_or_else(|| DEFAULT_MODEL.into());
    let fallbacks = lookup("GEMINI_FALLBACKS")
        .unwrap_or_else(|| DEFAULT_FALLBACKS.into())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Settings { key, key_source, model, fallbacks }
}

pub fn check_format(key: &str) -> Result<(), String> {
    if key.len() < 20 || key.len() > 200 || key.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("That doesn't look like a Gemini API key. Copy it again from Google AI Studio.".into());
    }
    Ok(())
}

pub fn save_key(key: &str) -> Result<(), String> {
    let key = key.trim();
    check_format(key)?;
    entry()
        .ok_or("Credential Manager is unavailable.")?
        .set_password(key)
        .map_err(|e| format!("Could not save the key: {e}"))
}

pub fn clear_key() -> Result<(), String> {
    match entry() {
        Some(e) => match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Could not remove the key: {e}")),
        },
        None => Ok(()),
    }
}
