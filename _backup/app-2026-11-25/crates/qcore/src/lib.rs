//! Quantum core shared by the desktop host and the WebAssembly UI.
//!
//! * A tiny statevector simulator for the three verifiable circuits (Bell, Deutsch-Jozsa, Grover-2).
//! * Step-by-step traces (state after every gate) and a circuit layout for the Lab view.
//! * The claim verifier used by the tutor pipeline (port of `code/quantum_sim.py`, same behaviour).
//!
//! All three circuits keep real amplitudes, so `f64` is enough. Qubit 0 is the most significant
//! bit of a basis label, matching the Python prototype ("10" means qubit 0 is 1).

use serde::Serialize;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::FRAC_1_SQRT_2;

pub type Probs = BTreeMap<String, f64>;

/// Kind of Deutsch-Jozsa oracle function.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FKind {
    Constant0,
    Constant1,
    Balanced,
}

impl FKind {
    pub const ALL: [FKind; 3] = [FKind::Constant0, FKind::Constant1, FKind::Balanced];
    pub fn as_str(self) -> &'static str {
        match self {
            FKind::Constant0 => "constant0",
            FKind::Constant1 => "constant1",
            FKind::Balanced => "balanced",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            FKind::Constant0 => "f(x) = 0",
            FKind::Constant1 => "f(x) = 1",
            FKind::Balanced => "f(x) = parity(x)",
        }
    }
}

/// A verifiable algorithm with normalised parameters.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Algo {
    Bell,
    DeutschJozsa { n: u32, f: FKind },
    Grover2 { marked: usize },
}

impl Algo {
    pub fn key(&self) -> &'static str {
        match self {
            Algo::Bell => "bell",
            Algo::DeutschJozsa { .. } => "deutsch_jozsa",
            Algo::Grover2 { .. } => "grover2",
        }
    }
    pub fn title(&self) -> &'static str {
        match self {
            Algo::Bell => "Bell state",
            Algo::DeutschJozsa { .. } => "Deutsch–Jozsa",
            Algo::Grover2 { .. } => "Grover search",
        }
    }
    pub fn qubits(&self) -> usize {
        match *self {
            Algo::DeutschJozsa { n, .. } => n as usize,
            _ => 2,
        }
    }
    pub fn params_json(&self) -> Value {
        match self {
            Algo::Bell => json!({}),
            Algo::DeutschJozsa { n, f } => json!({"n": n, "f_kind": f.as_str()}),
            Algo::Grover2 { marked } => json!({"marked": format!("{marked:02b}")}),
        }
    }
}

// ───────────────────────── gates on an n-qubit statevector ─────────────────────────

fn h(state: &mut [f64], q: usize, n: usize) {
    let bit = 1 << (n - 1 - q);
    for i in 0..state.len() {
        if i & bit == 0 {
            let j = i | bit;
            let (a, b) = (state[i], state[j]);
            state[i] = FRAC_1_SQRT_2 * (a + b);
            state[j] = FRAC_1_SQRT_2 * (a - b);
        }
    }
}

fn cnot(state: &mut [f64], c: usize, t: usize, n: usize) {
    let (cb, tb) = (1 << (n - 1 - c), 1 << (n - 1 - t));
    for i in 0..state.len() {
        if i & cb != 0 && i & tb == 0 {
            state.swap(i, i | tb);
        }
    }
}

fn dj_sign(f: FKind, x: usize) -> f64 {
    match f {
        FKind::Constant0 => 1.0,
        FKind::Constant1 => -1.0,
        FKind::Balanced => {
            if x.count_ones() % 2 == 0 {
                1.0
            } else {
                -1.0
            }
        }
    }
}

fn round6(x: f64) -> f64 {
    let r = (x * 1e6).round() / 1e6;
    if r == 0.0 { 0.0 } else { r } // no "-0.0"
}

pub fn basis(n: usize) -> Vec<String> {
    (0..1usize << n).map(|i| format!("{i:0n$b}")).collect()
}

/// Measurement probabilities (|amplitude|², rounded to 6 decimals, zeros dropped).
pub fn probabilities(state: &[f64], n: usize) -> Probs {
    state
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            let p = a * a;
            (p > 1e-9).then(|| (format!("{i:0n$b}"), round6(p)))
        })
        .collect()
}

// ───────────────────────── step traces + circuit layout ─────────────────────────

/// What a circuit column draws.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Column {
    /// One "H" box on each listed wire.
    H { wires: Vec<usize> },
    /// Control dot and target ⊕.
    Cnot { control: usize, target: usize },
    /// A labelled box spanning all wires (oracle, diffusion).
    Block { label: String },
    /// Meter on every wire.
    Measure,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Step {
    pub title: String,
    pub detail: String,
    /// Ket notation of the state after this step, e.g. "(|00⟩ + |11⟩)/√2".
    pub ket: String,
    /// Circuit column this step applies (None = initial state).
    pub column: Option<usize>,
    pub amps: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Trace {
    pub qubits: usize,
    pub wires: Vec<String>,
    pub basis: Vec<String>,
    pub columns: Vec<Column>,
    pub steps: Vec<Step>,
    pub note: Option<String>,
}

/// Format a real statevector as a ket sum, factoring a common magnitude when all are equal.
pub fn ket(amps: &[f64], n: usize) -> String {
    let labels = basis(n);
    let nz: Vec<(usize, f64)> = amps.iter().copied().enumerate().filter(|(_, a)| a.abs() > 1e-9).collect();
    if nz.is_empty() {
        return "0".into();
    }
    let mag = nz[0].1.abs();
    let same = nz.iter().all(|(_, a)| (a.abs() - mag).abs() < 1e-9);
    let term = |sign: f64, label: &str| format!("{}|{label}⟩", if sign < 0.0 { "− " } else { "+ " });
    if nz.len() == 1 {
        let (i, a) = nz[0];
        return format!("{}|{}⟩", if a < 0.0 { "−" } else { "" }, labels[i]);
    }
    if same {
        let mut s = String::new();
        for (k, (i, a)) in nz.iter().enumerate() {
            if k == 0 {
                s.push_str(&format!("{}|{}⟩", if *a < 0.0 { "−" } else { "" }, labels[*i]));
            } else {
                s.push_str(&term(*a, &labels[*i]));
            }
            if k + 1 < nz.len() {
                s.push(' ');
            }
        }
        let k = nz.len();
        let denom = match k {
            2 => "√2".to_string(),
            4 => "2".to_string(),
            8 => "2√2".to_string(),
            16 => "4".to_string(),
            _ => format!("√{k}"),
        };
        return format!("({s})/{denom}");
    }
    nz.iter()
        .map(|(i, a)| format!("{a:+.3}|{}⟩", labels[*i]))
        .collect::<Vec<_>>()
        .join(" ")
}

fn step(title: &str, detail: impl Into<String>, column: Option<usize>, amps: &[f64], n: usize) -> Step {
    Step { title: title.into(), detail: detail.into(), ket: ket(amps, n), column, amps: amps.to_vec() }
}

pub fn trace(algo: &Algo) -> Trace {
    match *algo {
        Algo::Bell => {
            let n = 2;
            let mut s = vec![1.0, 0.0, 0.0, 0.0];
            let mut steps = vec![step("Start", "Both qubits begin in |0⟩. Measuring now would give 00 every time.", None, &s, n)];
            h(&mut s, 0, n);
            steps.push(step(
                "Hadamard on q0",
                "H puts the first qubit into an equal superposition: half the amplitude on |0⟩, half on |1⟩. The second qubit is untouched.",
                Some(0),
                &s,
                n,
            ));
            cnot(&mut s, 0, 1, n);
            steps.push(step(
                "CNOT q0 → q1",
                "CNOT flips q1 only where q0 is 1, so |10⟩ becomes |11⟩. The two qubits are now entangled: neither has a state of its own.",
                Some(1),
                &s,
                n,
            ));
            steps.push(step(
                "Measure",
                "Outcomes 00 and 11 each appear with probability ½; 01 and 10 never do. The results always agree.",
                Some(2),
                &s,
                n,
            ));
            Trace {
                qubits: n,
                wires: vec!["q0".into(), "q1".into()],
                basis: basis(n),
                columns: vec![Column::H { wires: vec![0] }, Column::Cnot { control: 0, target: 1 }, Column::Measure],
                steps,
                note: None,
            }
        }
        Algo::DeutschJozsa { n, f } => {
            let n = n as usize;
            let size = 1usize << n;
            let mut s = vec![0.0; size];
            s[0] = 1.0;
            let mut steps = vec![step(
                "Start",
                format!("{n} input qubits start in |{}⟩. A helper qubit (not drawn) is prepared in |−⟩ so the oracle can act as a phase.", "0".repeat(n)),
                None,
                &s,
                n,
            )];
            for q in 0..n {
                h(&mut s, q, n);
            }
            steps.push(step(
                "Hadamard on every input",
                format!("Now all {size} inputs are present at once, each with amplitude 1/√{size}. One oracle call will touch all of them."),
                Some(0),
                &s,
                n,
            ));
            for (x, a) in s.iter_mut().enumerate() {
                *a *= dj_sign(f, x);
            }
            let effect = match f {
                FKind::Constant0 => "f is 0 everywhere, so no sign changes.",
                FKind::Constant1 => "f is 1 everywhere, so every sign flips: only a global phase, which cannot be observed.",
                FKind::Balanced => "f is 1 on exactly half the inputs, so half the amplitudes turn negative.",
            };
            steps.push(step(
                "Oracle U_f (one query)",
                format!("Phase kickback: the oracle multiplies |x⟩ by (−1)^f(x). {effect}"),
                Some(1),
                &s,
                n,
            ));
            for q in 0..n {
                h(&mut s, q, n);
            }
            let verdict = match f {
                FKind::Balanced => format!("The amplitudes cancel on |{}⟩, so P(all zeros) = 0: the function is balanced.", "0".repeat(n)),
                _ => format!("Everything interferes back onto |{}⟩, so P(all zeros) = 1: the function is constant.", "0".repeat(n)),
            };
            steps.push(step("Hadamard again", format!("Interference. {verdict}"), Some(2), &s, n));
            steps.push(step(
                "Measure",
                "One query settles constant vs balanced with certainty. A classical algorithm can need 2ⁿ⁻¹ + 1 queries.",
                Some(3),
                &s,
                n,
            ));
            Trace {
                qubits: n,
                wires: (0..n).map(|i| format!("x{i}")).collect(),
                basis: basis(n),
                columns: vec![
                    Column::H { wires: (0..n).collect() },
                    Column::Block { label: "U_f".into() },
                    Column::H { wires: (0..n).collect() },
                    Column::Measure,
                ],
                steps,
                note: Some("Helper qubit |−⟩ omitted: it only turns f(x) into a sign.".into()),
            }
        }
        Algo::Grover2 { marked } => {
            let n = 2;
            let m = format!("{marked:02b}");
            let mut s = vec![1.0, 0.0, 0.0, 0.0];
            let mut steps = vec![step("Start", "Two qubits, four possible items: 00, 01, 10, 11.", None, &s, n)];
            h(&mut s, 0, n);
            h(&mut s, 1, n);
            steps.push(step(
                "Hadamard on both",
                "Every item gets amplitude ½, so each would be measured with probability ¼: a uniform guess.",
                Some(0),
                &s,
                n,
            ));
            s[marked] = -s[marked];
            steps.push(step(
                "Oracle marks the item",
                format!("The oracle flips the sign of |{m}⟩. Probabilities are unchanged (still ¼ each): the mark is hidden in the phase."),
                Some(1),
                &s,
                n,
            ));
            let mean = s.iter().sum::<f64>() / 4.0;
            for a in &mut s {
                *a = 2.0 * mean - *a;
            }
            steps.push(step(
                "Diffusion 2|s⟩⟨s| − I",
                format!("Reflect every amplitude about the mean (¼). The three unmarked items fall to 0 and |{m}⟩ rises to 1."),
                Some(2),
                &s,
                n,
            ));
            steps.push(step(
                "Measure",
                format!("For 4 items one iteration is exact: P({m}) = 1. In general Grover needs about √N queries, a quadratic speedup."),
                Some(3),
                &s,
                n,
            ));
            Trace {
                qubits: n,
                wires: vec!["q0".into(), "q1".into()],
                basis: basis(n),
                columns: vec![
                    Column::H { wires: vec![0, 1] },
                    Column::Block { label: "Oracle".into() },
                    Column::Block { label: "Diffuse".into() },
                    Column::Measure,
                ],
                steps,
                note: None,
            }
        }
    }
}

/// Ground-truth measurement probabilities for an algorithm.
pub fn truth(algo: &Algo) -> Probs {
    let t = trace(algo);
    probabilities(&t.steps.last().expect("trace has steps").amps, t.qubits)
}

// ───────────────────────── normalisation (router output → typed params) ─────────────────────────

/// '|01>', ' 01 ', '|0>|1>' → "01"
pub fn norm_key(k: &str) -> String {
    k.chars().filter(|c| matches!(c, '0' | '1')).collect()
}

fn value_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Apply the defaults the router prompt promises. `None` = not simulator-verifiable.
pub fn norm_params(algo: &str, p: &Value) -> Option<Algo> {
    let get = |k: &str| p.as_object().and_then(|m| m.get(k));
    match algo {
        "bell" => Some(Algo::Bell),
        "grover2" => {
            let m = get("marked").map(value_str).map(|s| norm_key(&s)).unwrap_or_else(|| "11".into());
            let marked = match m.as_str() {
                "00" | "01" | "10" | "11" => usize::from_str_radix(&m, 2).unwrap_or(3),
                _ => 3,
            };
            Some(Algo::Grover2 { marked })
        }
        "deutsch_jozsa" => {
            let n = match get("n") {
                Some(Value::Number(x)) => x.as_f64().map(|f| f.trunc() as i64).unwrap_or(2),
                Some(Value::String(s)) => s.trim().parse::<i64>().unwrap_or(2),
                Some(Value::Bool(b)) => *b as i64,
                _ => 2,
            }
            .clamp(1, 3) as u32;
            let fk = get("f_kind").map(value_str).unwrap_or_else(|| "balanced".into());
            let f = match fk.trim().to_lowercase().as_str() {
                "constant0" | "constant" => FKind::Constant0,
                "constant1" => FKind::Constant1,
                _ => FKind::Balanced,
            };
            Some(Algo::DeutschJozsa { n, f })
        }
        _ => None,
    }
}

/// 0.5, "0.5", "1/2", "50%" → 0.5; anything else → None.
pub fn to_prob(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::String(s) => {
            let s = s.trim();
            if let Some(pct) = s.strip_suffix('%') {
                pct.trim().parse::<f64>().ok().map(|x| x / 100.0)
            } else if let Some((a, b)) = s.split_once('/') {
                let (a, b) = (a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?);
                (b != 0.0).then(|| a / b)
            } else {
                s.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

// ───────────────────────── verification ─────────────────────────

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Verification {
    pub ok: bool,
    pub truth: Probs,
    pub claimed: Probs,
    pub mismatches: Vec<String>,
}

pub fn probs_json(p: &Probs) -> Value {
    Value::Object(p.iter().map(|(k, v)| (k.clone(), json!(v))).collect::<Map<_, _>>())
}

/// Python-style dict repr, e.g. {'01': 1.0} (used in the repair feedback text, as in the prototype).
pub fn py_repr(p: &Probs) -> String {
    let items: Vec<String> = p.iter().map(|(k, v)| format!("'{k}': {v:?}")).collect();
    format!("{{{}}}", items.join(", "))
}

/// Python `repr()` of a str: single quotes unless the text has ' and no ".
fn py_str_repr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || (0x7f..0xa0).contains(&(c as u32)) => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

/// Python `repr()` of a json.loads result (None/True/False, 'str', [..], {..}), as the prototype's `{v!r}`.
pub fn py_value_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(b) => (if *b { "True" } else { "False" }).into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => py_str_repr(s),
        Value::Array(a) => format!("[{}]", a.iter().map(py_value_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(m) => format!(
            "{{{}}}",
            m.iter().map(|(k, v)| format!("{}: {}", py_str_repr(k), py_value_repr(v))).collect::<Vec<_>>().join(", ")
        ),
    }
}

/// Compare claimed final probabilities with the simulator (absolute tolerance `tol`).
pub fn verify(algo: &Algo, claimed_raw: &Value, tol: f64) -> Verification {
    let mut truth = truth(algo);
    let mut mism = Vec::new();
    let mut claimed: Probs = BTreeMap::new();
    if let Value::Object(m) = claimed_raw {
        for (k, v) in m {
            match to_prob(v) {
                Some(p) => *claimed.entry(norm_key(k)).or_insert(0.0) += p,
                None => mism.push(format!("{k}: unparseable value {}", py_value_repr(v))),
            }
        }
    }
    let mut keys: BTreeSet<String> = truth.keys().chain(claimed.keys()).cloned().collect();
    if claimed.is_empty() && mism.is_empty() {
        mism.push("no final probability stated".into());
    }
    if let Algo::DeutschJozsa { n, .. } = *algo {
        // Many balanced functions exist; only P(all zeros) is fixed (1 constant, 0 balanced).
        let zeros = "0".repeat(n as usize);
        let tz = truth.get(&zeros).copied().unwrap_or(0.0);
        truth = BTreeMap::from([(zeros.clone(), tz)]);
        keys = if claimed.is_empty() { BTreeSet::new() } else { BTreeSet::from([zeros.clone()]) };
        let bad: Vec<String> = claimed.keys().filter(|k| k.len() != n as usize).map(|k| py_str_repr(k)).collect();
        if !bad.is_empty() {
            mism.push(format!("keys must be {n}-bit input-register strings, got [{}]", bad.join(", ")));
        }
        let s: f64 = claimed.values().sum();
        let others = claimed.keys().any(|k| *k != zeros);
        if s > 1.0 + tol || (others && (s - 1.0).abs() > tol) {
            mism.push(format!("claimed probabilities sum to {s:.3}, expected 1"));
        }
    }
    for k in &keys {
        let (t, c) = (truth.get(k).copied().unwrap_or(0.0), claimed.get(k).copied().unwrap_or(0.0));
        if (t - c).abs() > tol {
            mism.push(format!("{k}: claimed {c:?}, simulated {t:?}"));
        }
    }
    Verification { ok: mism.is_empty(), truth, claimed, mismatches: mism }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pairs: &[(&str, f64)]) -> Probs {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn ground_truth_matches_python_prototype() {
        assert_eq!(truth(&Algo::Bell), p(&[("00", 0.5), ("11", 0.5)]));
        assert_eq!(truth(&Algo::DeutschJozsa { n: 2, f: FKind::Balanced }), p(&[("11", 1.0)]));
        assert_eq!(truth(&Algo::DeutschJozsa { n: 2, f: FKind::Constant0 }), p(&[("00", 1.0)]));
        assert_eq!(truth(&Algo::DeutschJozsa { n: 3, f: FKind::Constant1 }), p(&[("000", 1.0)]));
        assert_eq!(truth(&Algo::DeutschJozsa { n: 1, f: FKind::Balanced }), p(&[("1", 1.0)]));
        for m in 0..4 {
            assert_eq!(truth(&Algo::Grover2 { marked: m }), p(&[(format!("{m:02b}").as_str(), 1.0)]));
        }
    }

    #[test]
    fn traces_are_normalised_and_consistent() {
        for a in [
            Algo::Bell,
            Algo::Grover2 { marked: 2 },
            Algo::DeutschJozsa { n: 3, f: FKind::Balanced },
            Algo::DeutschJozsa { n: 1, f: FKind::Constant1 },
        ] {
            let t = trace(&a);
            for s in &t.steps {
                let norm: f64 = s.amps.iter().map(|x| x * x).sum();
                assert!((norm - 1.0).abs() < 1e-9, "{a:?} {} not normalised", s.title);
                assert_eq!(s.amps.len(), 1 << t.qubits);
                if let Some(c) = s.column {
                    assert!(c < t.columns.len());
                }
            }
        }
    }

    #[test]
    fn kets_read_naturally() {
        assert_eq!(ket(&[1.0, 0.0, 0.0, 0.0], 2), "|00⟩");
        let b = trace(&Algo::Bell);
        assert_eq!(b.steps[2].ket, "(|00⟩ + |11⟩)/√2");
        let g = trace(&Algo::Grover2 { marked: 1 });
        assert_eq!(g.steps[2].ket, "(|00⟩ − |01⟩ + |10⟩ + |11⟩)/2");
        assert_eq!(g.steps[3].ket, "|01⟩");
        let c1 = trace(&Algo::DeutschJozsa { n: 2, f: FKind::Constant1 });
        assert_eq!(c1.steps[3].ket, "−|00⟩");
    }

    #[test]
    fn verifier_matches_python_prototype() {
        let g1 = Algo::Grover2 { marked: 1 };
        assert!(verify(&g1, &json!({"01": 1.0}), 0.02).ok);
        assert_eq!(verify(&g1, &json!({"01": 0.5}), 0.02).mismatches, vec!["01: claimed 0.5, simulated 1.0"]);
        let bal = Algo::DeutschJozsa { n: 2, f: FKind::Balanced };
        assert!(verify(&bal, &json!({"10": 1.0}), 0.02).ok);
        assert_eq!(
            verify(&bal, &json!({"001": 1.0}), 0.02).mismatches,
            vec!["keys must be 2-bit input-register strings, got ['001']"]
        );
        assert_eq!(
            verify(&Algo::Bell, &json!({"00": "half", "11": null, "01": [0.5], "10": {"p": true}}), 0.02).mismatches[..4],
            [
                "00: unparseable value 'half'",
                "11: unparseable value None",
                "01: unparseable value [0.5]",
                "10: unparseable value {'p': True}",
            ]
        );
        assert_eq!(py_value_repr(&json!("it's")), r#""it's""#);
        assert_eq!(py_value_repr(&json!("a\nb\\")), r"'a\nb\\'");
        let c0 = Algo::DeutschJozsa { n: 2, f: FKind::Constant0 };
        assert_eq!(verify(&c0, &json!({"00": 0.5, "11": 0.5}), 0.02).mismatches, vec!["00: claimed 0.5, simulated 1.0"]);
        assert!(verify(&Algo::Bell, &json!({"|00>": "1/2", "|11>": "50%"}), 0.02).ok);
        assert_eq!(
            verify(&Algo::Grover2 { marked: 3 }, &json!({}), 0.02).mismatches,
            vec!["no final probability stated", "11: claimed 0.0, simulated 1.0"]
        );
    }

    #[test]
    fn norm_params_matches_python_prototype() {
        assert_eq!(norm_params("grover2", &json!({"marked": "|10>"})), Some(Algo::Grover2 { marked: 2 }));
        assert_eq!(norm_params("grover2", &Value::Null), Some(Algo::Grover2 { marked: 3 }));
        assert_eq!(
            norm_params("deutsch_jozsa", &json!({"n": "2", "f_kind": "constant"})),
            Some(Algo::DeutschJozsa { n: 2, f: FKind::Constant0 })
        );
        assert_eq!(norm_params("deutsch_jozsa", &json!({"n": 9})), Some(Algo::DeutschJozsa { n: 3, f: FKind::Balanced }));
        assert_eq!(norm_params("other_quantum", &json!({})), None);
    }
}
