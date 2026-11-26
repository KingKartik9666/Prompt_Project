//! Atlas content model: sections and topic pages (JSON files under crates/ui/content).
//!
//! The schema is documented for authors in app/docs/atlas-spec.md. `check` is the gate every
//! content file must pass (run by `cargo test -p qcore`): circuits must run, every stated outcome
//! must match the simulator, and every link must point at a topic that exists.

use crate::circuit::{Circuit, marginal, probs, run};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub blurb: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// e.g. "Nielsen & Chuang, Quantum Computation and Quantum Information, ch. 1" — chapter-level
    /// only unless the exact section was checked. No invented page or section numbers.
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Outcome claimed in the text, checked against the simulator by `check`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Expect {
    /// Which qubits the probabilities are over (default: all, in order).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qubits: Option<Vec<usize>>,
    /// Basis label → probability; labels missing here must have probability 0.
    pub probs: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// Prose. `body` paragraphs are separated by blank lines; `*word*` is emphasis, `|0⟩` kets are fine.
    Text {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        heading: Option<String>,
        body: String,
    },
    /// A displayed formula in plain Unicode (no TeX), with an optional caption.
    Math {
        expr: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    /// An interactive circuit: built gate by gate on the page and stepped through by the reader.
    Circuit {
        title: String,
        circuit: Circuit,
        expect: Expect,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    Misconception {
        myth: String,
        truth: String,
    },
    Practice {
        question: String,
        answer: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },
    /// A short aside: history, hardware note, "in practice".
    Note {
        title: String,
        body: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    pub id: String,
    pub section: String,
    pub title: String,
    /// One-line hook shown on the Atlas map and in search.
    pub kicker: String,
    /// Two or three plain sentences: what this is and why it matters.
    pub summary: String,
    pub level: Level,
    #[serde(default)]
    pub prereqs: Vec<String>,
    #[serde(default)]
    pub next: Vec<String>,
    #[serde(default)]
    pub related: Vec<String>,
    /// Extra words people might search for.
    #[serde(default)]
    pub keywords: Vec<String>,
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub sources: Vec<Source>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Beginner,
    Intermediate,
    Advanced,
}

/// Probability tolerance for stated outcomes (authors may round to 3–4 decimals).
pub const TOL: f64 = 5e-4;

/// Check one circuit block's stated outcome against the simulator.
pub fn check_circuit(c: &Circuit, e: &Expect) -> Result<(), String> {
    let r = run(c)?;
    let last = r.states.last().expect("initial state");
    let got: BTreeMap<String, f64> = match &e.qubits {
        Some(keep) => {
            if keep.iter().any(|&q| q >= c.qubits) {
                return Err("expect.qubits out of range".into());
            }
            marginal(last, c.qubits, keep)
        }
        None => crate::basis(c.qubits).into_iter().zip(probs(last)).collect(),
    };
    let width = e.qubits.as_ref().map_or(c.qubits, Vec::len);
    let mut errs = vec![];
    for k in e.probs.keys() {
        if k.len() != width || !k.chars().all(|ch| ch == '0' || ch == '1') {
            errs.push(format!("label \"{k}\" is not {width} bits"));
        }
    }
    let total: f64 = e.probs.values().sum();
    if (total - 1.0).abs() > TOL * 4.0 {
        errs.push(format!("stated probabilities sum to {total:.4}, not 1"));
    }
    for (k, &p) in &got {
        let claimed = e.probs.get(k).copied().unwrap_or(0.0);
        if (claimed - p).abs() > TOL {
            errs.push(format!("{k}: text says {claimed}, simulator gives {p:.6}"));
        }
    }
    if errs.is_empty() { Ok(()) } else { Err(errs.join("; ")) }
}

/// The content gate: everything a page claims that the simulator can check, plus link integrity.
pub fn check(sections: &[Section], topics: &[Topic]) -> Vec<String> {
    let mut errs = vec![];
    let ids: BTreeSet<&str> = topics.iter().map(|t| t.id.as_str()).collect();
    let secs: BTreeSet<&str> = sections.iter().map(|s| s.id.as_str()).collect();
    if ids.len() != topics.len() {
        errs.push("duplicate topic ids".into());
    }
    for t in topics {
        let at = |m: String| format!("{}: {m}", t.id);
        if !secs.contains(t.section.as_str()) {
            errs.push(at(format!("unknown section {}", t.section)));
        }
        if t.id.is_empty() || !t.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            errs.push(at("id must be lowercase-kebab".into()));
        }
        for (kind, list) in [("prereq", &t.prereqs), ("next", &t.next), ("related", &t.related)] {
            for l in list {
                if !ids.contains(l.as_str()) {
                    errs.push(at(format!("{kind} link to missing topic {l}")));
                }
                if l == &t.id {
                    errs.push(at(format!("{kind} links to itself")));
                }
            }
        }
        if !t.blocks.iter().any(|b| matches!(b, Block::Text { .. })) {
            errs.push(at("no text block".into()));
        }
        for (i, b) in t.blocks.iter().enumerate() {
            if let Block::Circuit { circuit, expect, title, .. } = b {
                if let Err(e) = check_circuit(circuit, expect) {
                    errs.push(at(format!("block {i} \"{title}\": {e}")));
                }
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// The real content files: every one must parse and pass `check`.
    #[test]
    fn atlas_content_passes_the_gate() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/content");
        let sections: Vec<Section> =
            serde_json::from_str(&std::fs::read_to_string(dir.join("sections.json")).expect("sections.json")).expect("sections.json parses");
        let mut topics = vec![];
        let mut parse_errs = vec![];
        let mut paths: Vec<_> = std::fs::read_dir(dir.join("topics")).expect("topics dir").flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths.into_iter().filter(|p| p.extension().is_some_and(|e| e == "json")) {
            let text = std::fs::read_to_string(&p).unwrap();
            match serde_json::from_str::<Topic>(&text) {
                Ok(t) => {
                    if p.file_stem().and_then(|s| s.to_str()) != Some(t.id.as_str()) {
                        parse_errs.push(format!("{}: file name must equal id {}", p.display(), t.id));
                    }
                    topics.push(t)
                }
                Err(e) => parse_errs.push(format!("{}: {e}", p.display())),
            }
        }
        let mut errs = parse_errs;
        errs.extend(check(&sections, &topics));
        assert!(errs.is_empty(), "{} content problem(s):\n{}", errs.len(), errs.join("\n"));
        assert!(!topics.is_empty());
    }

    #[test]
    fn gate_catches_a_wrong_claim_and_a_dead_link() {
        let sec = vec![Section { id: "s".into(), title: "S".into(), blurb: "".into() }];
        let bad: Topic = serde_json::from_value(serde_json::json!({
            "id": "t", "section": "s", "title": "T", "kicker": "k", "summary": "s", "level": "beginner",
            "next": ["nowhere"],
            "blocks": [
                {"type": "text", "body": "hi"},
                {"type": "circuit", "title": "H", "circuit": {"qubits": 1, "ops": [{"g": "H", "t": [0]}]},
                 "expect": {"probs": {"0": 0.9, "1": 0.1}}}
            ]
        }))
        .unwrap();
        let errs = check(&sec, &[bad]);
        assert!(errs.iter().any(|e| e.contains("missing topic nowhere")));
        assert!(errs.iter().any(|e| e.contains("simulator gives 0.5")));
    }
}
