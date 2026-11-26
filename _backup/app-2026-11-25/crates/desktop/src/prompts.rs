//! The four evaluated prompts (P1–P4), embedded at compile time from `prompts/final/`,
//! plus the helpers the pipeline needs: template filling and Python-compatible JSON.
//!
//! The prompt text sent to Gemini is byte-for-byte what the Python prototype sends, so the
//! Milestone 2 test results still describe this app's behaviour.

use serde_json::Value;

pub struct Prompt {
    pub temperature: Option<f64>,
    pub mime: Option<String>,
    pub system: Option<String>,
    pub user: String,
}

pub struct Prompts {
    pub router: Prompt,
    pub explainer: Prompt,
    pub claims: Prompt,
    pub practice: Prompt,
}

impl Prompts {
    pub fn load() -> Self {
        Prompts {
            router: parse(include_str!("../../../../prompts/final/P1_router.txt")),
            explainer: parse(include_str!("../../../../prompts/final/P2_explainer.txt")),
            claims: parse(include_str!("../../../../prompts/final/P3_claim_extractor.txt")),
            practice: parse(include_str!("../../../../prompts/final/P4_practice.txt")),
        }
    }
}

/// Parse `### SETTINGS` / `### SYSTEM` / `### USER` sections (same rules as workflow.py).
pub fn parse(text: &str) -> Prompt {
    let (mut settings, mut system, mut user) = (String::new(), String::new(), String::new());
    let mut cur: Option<&mut String> = None;
    let mut seen = false;
    for line in text.lines() {
        match line.trim_end() {
            "### SETTINGS" => {
                cur = Some(&mut settings);
                seen = true;
                continue;
            }
            "### SYSTEM" => {
                cur = Some(&mut system);
                seen = true;
                continue;
            }
            "### USER" => {
                cur = Some(&mut user);
                seen = true;
                continue;
            }
            _ => {}
        }
        if let Some(buf) = cur.as_deref_mut() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    if !seen {
        user = text.to_string();
    }
    let mut p = Prompt { temperature: None, mime: None, system: None, user: user.trim().to_string() };
    for line in settings.lines() {
        if let Some((k, v)) = line.split_once(':') {
            match k.trim() {
                "temperature" => p.temperature = v.trim().parse().ok(),
                "response_mime_type" => p.mime = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    let system = system.trim();
    if !system.is_empty() {
        p.system = Some(system.to_string());
    }
    p
}

/// Replace known `{name}` placeholders in one pass, so JSON braces in the prompt survive and
/// text inside a value (e.g. a learner typing "{params}") is never substituted again.
pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + 256);
    let mut rest = template;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match vars.iter().find(|(k, _)| after.starts_with(k) && after[k.len()..].starts_with('}')) {
            Some((k, v)) => {
                out.push_str(v);
                rest = &after[k.len() + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `json.dumps(value)` with Python's defaults: ", " / ": " separators and ASCII-only output.
pub fn py_dumps(v: &Value) -> String {
    let mut s = String::new();
    dump(v, &mut s);
    s
}

fn dump(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => dump_str(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                dump(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                dump_str(k, out);
                out.push_str(": ");
                dump(x, out);
            }
            out.push('}');
        }
    }
}

fn dump_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Parse model output as JSON. The flag is false when the raw text was not clean JSON
/// (the object had to be cut out of surrounding text) — same as workflow.parse_json.
pub fn parse_json(text: &str) -> (Option<Value>, bool) {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return (Some(v), true);
    }
    if let (Some(a), Some(b)) = (text.find('{'), text.rfind('}')) {
        if a < b {
            if let Ok(v) = serde_json::from_str::<Value>(&text[a..=b]) {
                return (Some(v), false);
            }
        }
    }
    (None, false)
}

/// Python truthiness of a JSON value.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
    }
}

/// How workflow.py inserts a value into a template: strings raw, everything else as JSON.
pub fn as_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => py_dumps(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn embedded_prompts_parse() {
        let p = Prompts::load();
        for (pr, t) in [(&p.router, 0.0), (&p.explainer, 0.2), (&p.claims, 0.0), (&p.practice, 0.4)] {
            assert_eq!(pr.temperature, Some(t));
            assert_eq!(pr.mime.as_deref(), Some("application/json"));
            assert!(pr.system.is_some());
            assert!(!pr.user.is_empty());
        }
        assert!(p.router.user.contains("{question}"));
        assert!(p.explainer.user.contains("{verifier_feedback}"));
    }

    #[test]
    fn fill_is_single_pass_and_keeps_json_braces() {
        let t = r#"Q: {question} P: {params} {"keep": 1} {unknown}"#;
        let out = fill(t, &[("question", "say {params}"), ("params", "{}")]);
        assert_eq!(out, r#"Q: say {params} P: {} {"keep": 1} {unknown}"#);
    }

    #[test]
    fn py_dumps_matches_python() {
        // Expected strings are copied from CPython's json.dumps output.
        let v = json!({"marked": "01", "n": 2, "p": 1.0, "s": "\u{e9}\"x\n", "a": [1, null, true]});
        // `b` is one backslash, written as a char escape so no tool can turn "é" back into "é".
        let b = '\u{5c}';
        let expected = format!(r#"{{"marked": "01", "n": 2, "p": 1.0, "s": "{b}u00e9{b}"x{b}n", "a": [1, null, true]}}"#);
        assert_eq!(py_dumps(&v), expected);
        assert_eq!(py_dumps(&json!("\u{1d713}")), format!(r#""{b}ud835{b}udf13""#));
    }

    #[test]
    fn parse_json_recovers_wrapped_objects() {
        assert_eq!(parse_json(r#"{"a":1}"#), (Some(json!({"a": 1})), true));
        assert_eq!(parse_json("```json\n{\"a\":1}\n```"), (Some(json!({"a": 1})), false));
        assert_eq!(parse_json("nope"), (None, false));
    }
}
