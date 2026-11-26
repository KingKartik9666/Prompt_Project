//! General n-qubit statevector simulator with complex amplitudes (used by the Atlas topic pages).
//!
//! Conventions, shared with the rest of qcore: qubit 0 is the most significant bit of a basis
//! label ("10" means qubit 0 is 1, qubit 1 is 0), and every circuit starts in |0…0⟩ unless `init`
//! says otherwise. Gates follow the standard textbook definitions (Nielsen & Chuang, ch. 4):
//!   H, X, Y, Z, S = diag(1, i), SDG = S†, T = diag(1, e^{iπ/4}), TDG = T†,
//!   RX(θ) = e^{−iθX/2}, RY(θ) = e^{−iθY/2}, RZ(θ) = e^{−iθZ/2}, P(θ) = diag(1, e^{iθ}),
//! and their controlled forms (any number of controls) via `c`. SWAP exchanges its two targets.
//! Simulation is exact up to f64 rounding; sizes are capped at MAX_QUBITS to keep the UI responsive.

use serde::{Deserialize, Serialize};
use std::f64::consts::{FRAC_1_SQRT_2, PI};

pub const MAX_QUBITS: usize = 8;

/// A complex number (only what the simulator needs).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct C {
    pub re: f64,
    pub im: f64,
}

impl C {
    pub const ZERO: C = C { re: 0.0, im: 0.0 };
    pub const ONE: C = C { re: 1.0, im: 0.0 };
    pub const I: C = C { re: 0.0, im: 1.0 };
    pub fn new(re: f64, im: f64) -> C {
        C { re, im }
    }
    pub fn polar(r: f64, phi: f64) -> C {
        C { re: r * phi.cos(), im: r * phi.sin() }
    }
    pub fn norm2(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    pub fn abs(self) -> f64 {
        self.norm2().sqrt()
    }
    /// Phase in (−π, π].
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    pub fn add(self, o: C) -> C {
        C { re: self.re + o.re, im: self.im + o.im }
    }
    pub fn mul(self, o: C) -> C {
        C { re: self.re * o.re - self.im * o.im, im: self.re * o.im + self.im * o.re }
    }
    pub fn scale(self, k: f64) -> C {
        C { re: self.re * k, im: self.im * k }
    }
}

/// An angle written in content files: a number of radians, or text like "pi/4", "-pi/2", "3pi/4",
/// "2*pi/3", "pi", "0.25".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Angle {
    Rad(f64),
    Text(String),
}

impl Angle {
    pub fn radians(&self) -> Result<f64, String> {
        match self {
            Angle::Rad(x) => Ok(*x),
            Angle::Text(s) => parse_angle(s),
        }
    }
}

pub fn parse_angle(s: &str) -> Result<f64, String> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase().replace('π', "pi");
    let bad = || format!("cannot read angle \"{s}\" (use radians or forms like pi/4, -3pi/4, 2*pi/3)");
    // f64 parsing accepts "nan", "inf" and "1e999"; a non-finite angle turns every amplitude into NaN.
    let finite = |x: f64| if x.is_finite() { Ok(x) } else { Err(bad()) };
    if let Ok(x) = t.parse::<f64>() {
        return finite(x);
    }
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1.0, r),
        None => (1.0, t.strip_prefix('+').unwrap_or(&t)),
    };
    let (num, den) = match rest.split_once('/') {
        Some((a, b)) => (a, b.parse::<f64>().map_err(|_| bad())?),
        None => (rest, 1.0),
    };
    let coef = num.strip_suffix("pi").ok_or_else(bad)?.trim_end_matches('*');
    let k = if coef.is_empty() { 1.0 } else { coef.parse::<f64>().map_err(|_| bad())? };
    if den == 0.0 {
        return Err(bad());
    }
    finite(sign * k * PI / den)
}

/// One step of a circuit: a gate on one or more targets, optionally controlled.
/// Single-qubit gates with several targets apply to each target (e.g. H on every wire, one step).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Op {
    /// Gate name: I X Y Z H S SDG T TDG RX RY RZ P SWAP, or CX CY CZ CP CCX CSWAP (shorthands that
    /// expect the control in `c`). Case-insensitive.
    pub g: String,
    #[serde(default)]
    pub t: Vec<usize>,
    #[serde(default)]
    pub c: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theta: Option<Angle>,
    /// What this step does, in words (shown under the circuit while stepping).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Circuit {
    pub qubits: usize,
    /// Starting basis state, e.g. "01" (default all zeros).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init: Option<String>,
    pub ops: Vec<Op>,
}

type M2 = [[C; 2]; 2];

fn m(a: C, b: C, c: C, d: C) -> M2 {
    [[a, b], [c, d]]
}

/// The 2×2 matrix of a single-qubit gate (controls stripped by the caller).
fn matrix(g: &str, theta: Option<f64>) -> Result<M2, String> {
    let need = |name: &str| theta.ok_or(format!("{name} needs theta"));
    let r = FRAC_1_SQRT_2;
    let (z, o) = (C::ZERO, C::ONE);
    Ok(match g {
        "I" => m(o, z, z, o),
        "X" => m(z, o, o, z),
        "Y" => m(z, C::new(0.0, -1.0), C::I, z),
        "Z" => m(o, z, z, C::new(-1.0, 0.0)),
        "H" => m(C::new(r, 0.0), C::new(r, 0.0), C::new(r, 0.0), C::new(-r, 0.0)),
        "S" => m(o, z, z, C::I),
        "SDG" => m(o, z, z, C::new(0.0, -1.0)),
        "T" => m(o, z, z, C::polar(1.0, PI / 4.0)),
        "TDG" => m(o, z, z, C::polar(1.0, -PI / 4.0)),
        "P" => m(o, z, z, C::polar(1.0, need("P")?)),
        "RX" => {
            let h = need("RX")? / 2.0;
            m(C::new(h.cos(), 0.0), C::new(0.0, -h.sin()), C::new(0.0, -h.sin()), C::new(h.cos(), 0.0))
        }
        "RY" => {
            let h = need("RY")? / 2.0;
            m(C::new(h.cos(), 0.0), C::new(-h.sin(), 0.0), C::new(h.sin(), 0.0), C::new(h.cos(), 0.0))
        }
        "RZ" => {
            let h = need("RZ")? / 2.0;
            m(C::polar(1.0, -h), z, z, C::polar(1.0, h))
        }
        other => return Err(format!("unknown gate {other}")),
    })
}

/// Expand shorthands: CX → X with control, CCX → X with two controls, CP → P with control, …
fn canonical(op: &Op) -> (String, usize) {
    let g = op.g.to_uppercase();
    let extra = match g.as_str() {
        "CNOT" => return ("X".into(), 1),
        "TOFFOLI" => return ("X".into(), 2),
        "FREDKIN" => return ("SWAP".into(), 1),
        _ => g.chars().take_while(|c| *c == 'C').count(),
    };
    // A leading C on a known base gate means "controlled"; but "CX" etc. only — never strip a C that is
    // part of a base name (there is none in this gate set).
    if extra > 0 && g.len() > extra {
        (g[extra..].to_string(), extra)
    } else {
        (g, 0)
    }
}

/// Full statevector of a circuit after every step: states[0] is the initial state.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub qubits: usize,
    pub states: Vec<Vec<C>>,
}

pub fn initial(qubits: usize, init: Option<&str>) -> Result<Vec<C>, String> {
    if qubits == 0 || qubits > MAX_QUBITS {
        return Err(format!("qubits must be 1..={MAX_QUBITS}"));
    }
    let mut s = vec![C::ZERO; 1 << qubits];
    let idx = match init {
        None => 0,
        Some(b) => {
            if b.len() != qubits || !b.chars().all(|c| c == '0' || c == '1') {
                return Err(format!("init \"{b}\" must be {qubits} bits"));
            }
            usize::from_str_radix(b, 2).map_err(|e| e.to_string())?
        }
    };
    s[idx] = C::ONE;
    Ok(s)
}

pub fn apply(state: &mut [C], n: usize, op: &Op) -> Result<(), String> {
    let (base, implied) = canonical(op);
    let theta = op.theta.as_ref().map(Angle::radians).transpose()?;
    // Before the SWAP early return: the diagram draws any theta, so a fixed gate must not carry one.
    if theta.is_some() && !matches!(base.as_str(), "P" | "RX" | "RY" | "RZ") {
        return Err(format!("{} does not take theta", op.g));
    }
    let controls = &op.c;
    if implied > 0 && controls.len() != implied {
        return Err(format!("{} needs {implied} control(s) in c, got {}", op.g, controls.len()));
    }
    let check = |q: usize| if q < n { Ok(()) } else { Err(format!("{}: qubit {q} out of range (n = {n})", op.g)) };
    for &q in controls.iter().chain(&op.t) {
        check(q)?;
    }
    if op.t.is_empty() {
        return Err(format!("{} has no target", op.g));
    }
    let bit = |q: usize| 1usize << (n - 1 - q);
    let cmask: usize = controls.iter().map(|&q| bit(q)).fold(0, |a, b| a | b);
    if op.t.iter().any(|q| cmask & bit(*q) != 0) {
        return Err(format!("{}: a qubit cannot be both control and target", op.g));
    }
    if base == "SWAP" {
        let [a, b] = op.t[..] else { return Err("SWAP needs exactly two targets".into()) };
        let (ba, bb) = (bit(a), bit(b));
        for i in 0..state.len() {
            if i & cmask == cmask && i & ba != 0 && i & bb == 0 {
                state.swap(i, (i & !ba) | bb);
            }
        }
        return Ok(());
    }
    let u = matrix(&base, theta)?;
    for &q in &op.t {
        let tb = bit(q);
        for i in 0..state.len() {
            if i & tb == 0 && i & cmask == cmask {
                let j = i | tb;
                let (a0, a1) = (state[i], state[j]);
                state[i] = u[0][0].mul(a0).add(u[0][1].mul(a1));
                state[j] = u[1][0].mul(a0).add(u[1][1].mul(a1));
            }
        }
    }
    Ok(())
}

pub fn run(c: &Circuit) -> Result<Run, String> {
    let mut s = initial(c.qubits, c.init.as_deref())?;
    let mut states = vec![s.clone()];
    for (k, op) in c.ops.iter().enumerate() {
        apply(&mut s, c.qubits, op).map_err(|e| format!("step {}: {e}", k + 1))?;
        states.push(s.clone());
    }
    Ok(Run { qubits: c.qubits, states })
}

pub fn labels(n: usize) -> Vec<String> {
    crate::basis(n)
}

/// Probabilities of every basis state (no rounding, zeros kept), in label order.
pub fn probs(state: &[C]) -> Vec<f64> {
    state.iter().map(|a| a.norm2()).collect()
}

/// Marginal probabilities over `keep` (in that order), keyed by their bits, e.g. keep [2] → {"0","1"}.
pub fn marginal(state: &[C], n: usize, keep: &[usize]) -> std::collections::BTreeMap<String, f64> {
    let mut out = std::collections::BTreeMap::new();
    for (i, a) in state.iter().enumerate() {
        let key: String = keep.iter().map(|&q| if i & (1 << (n - 1 - q)) != 0 { '1' } else { '0' }).collect();
        *out.entry(key).or_insert(0.0) += a.norm2();
    }
    out
}

/// A phase as a readable multiple of π when it is one (k·π/4 or k·π/3 or k·π/6), else in radians.
pub fn phase_text(phi: f64) -> String {
    for den in [1i64, 2, 3, 4, 6, 8] {
        let k = phi * den as f64 / PI;
        if (k - k.round()).abs() < 1e-9 {
            let k = k.round() as i64;
            return match (k, den) {
                (0, _) => "0".into(),
                (1, 1) => "π".into(),
                (-1, 1) => "−π".into(),
                (k, 1) => format!("{k}π"),
                (1, d) => format!("π/{d}"),
                (-1, d) => format!("−π/{d}"),
                (k, d) if k < 0 => format!("−{}π/{d}", -k),
                (k, d) => format!("{k}π/{d}"),
            };
        }
    }
    format!("{phi:.3}")
}

/// Ket notation for a complex state, e.g. "(|00⟩ + |11⟩)/√2", "(|0⟩ + i|1⟩)/√2", "0.866|0⟩ + 0.5|1⟩".
pub fn ket(state: &[C], n: usize) -> String {
    let labels = crate::basis(n);
    let nz: Vec<(usize, C)> = state.iter().copied().enumerate().filter(|(_, a)| a.norm2() > 1e-12).collect();
    if nz.is_empty() {
        return "0".into();
    }
    // Unit phase factors read as signs or i.
    let unit = |a: C| -> Option<&'static str> {
        let a = a.scale(1.0 / a.abs());
        let close = |x: f64, y: f64| (a.re - x).abs() < 1e-9 && (a.im - y).abs() < 1e-9;
        if close(1.0, 0.0) {
            Some("+")
        } else if close(-1.0, 0.0) {
            Some("−")
        } else if close(0.0, 1.0) {
            Some("+i")
        } else if close(0.0, -1.0) {
            Some("−i")
        } else {
            None
        }
    };
    let mag = nz[0].1.abs();
    let same = nz.iter().all(|(_, a)| (a.abs() - mag).abs() < 1e-9);
    let all_unit = nz.iter().all(|(_, a)| unit(*a).is_some());
    if same && all_unit {
        let mut s = String::new();
        for (k, (i, a)) in nz.iter().enumerate() {
            let u = unit(*a).unwrap_or("+");
            let (sign, ii) = (if u.starts_with('−') { "−" } else { "+" }, if u.ends_with('i') { "i" } else { "" });
            if k == 0 {
                s.push_str(&format!("{}{ii}|{}⟩", if sign == "−" { "−" } else { "" }, labels[*i]));
            } else {
                s.push_str(&format!(" {sign} {ii}|{}⟩", labels[*i]));
            }
        }
        if nz.len() == 1 {
            return s;
        }
        let denom = match nz.len() {
            2 => "√2".to_string(),
            4 => "2".to_string(),
            8 => "2√2".to_string(),
            16 => "4".to_string(),
            32 => "4√2".to_string(),
            64 => "8".to_string(),
            k => format!("√{k}"),
        };
        return format!("({s})/{denom}");
    }
    let coef = |a: C| -> String {
        if a.im.abs() < 1e-9 {
            format!("{:.3}", a.re)
        } else if a.re.abs() < 1e-9 {
            format!("{:.3}i", a.im)
        } else {
            format!("{:.3}·e^(i{})", a.abs(), phase_text(a.arg()))
        }
    };
    nz.iter()
        .enumerate()
        .map(|(k, (i, a))| {
            let c = coef(*a);
            let c = if k > 0 && !c.starts_with('-') { format!("+ {c}") } else if k > 0 { format!("− {}", &c[1..]) } else { c.replacen('-', "−", 1) };
            format!("{c}|{}⟩", labels[*i])
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(g: &str, t: &[usize], c: &[usize]) -> Op {
        Op { g: g.into(), t: t.to_vec(), c: c.to_vec(), theta: None, note: None }
    }
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn angles_parse() {
        assert!(close(parse_angle("pi/4").unwrap(), PI / 4.0));
        assert!(close(parse_angle("-3pi/4").unwrap(), -3.0 * PI / 4.0));
        assert!(close(parse_angle("2*pi/3").unwrap(), 2.0 * PI / 3.0));
        assert!(close(parse_angle("π").unwrap(), PI));
        assert!(close(parse_angle("0.5").unwrap(), 0.5));
        assert!(parse_angle("pi/0").is_err() && parse_angle("x").is_err());
    }

    #[test]
    fn non_finite_angles_are_rejected() {
        for s in ["nan", "NaN", "inf", "-inf", "infinity", "1e999", "pi/nan", "nanpi", "1e308pi", "pi/1e-320"] {
            assert!(parse_angle(s).is_err(), "{s} accepted");
        }
    }

    #[test]
    fn theta_on_a_fixed_gate_is_an_error() {
        // The diagram would draw "Z π/4" while the simulator ran plain Z.
        for g in ["Z", "H", "CZ", "SWAP"] {
            let t: &[usize] = if g == "SWAP" { &[0, 1] } else if g == "CZ" { &[1] } else { &[0] };
            let c: &[usize] = if g == "CZ" { &[0] } else { &[] };
            let mut o = op(g, t, c);
            o.theta = Some(Angle::Text("pi/4".into()));
            let circ = Circuit { qubits: 2, init: None, ops: vec![o] };
            assert!(run(&circ).is_err(), "{g} with theta accepted");
        }
        for g in ["RX", "RY", "RZ", "P", "CP"] {
            let mut o = if g == "CP" { op(g, &[1], &[0]) } else { op(g, &[0], &[]) };
            o.theta = Some(Angle::Text("pi/4".into()));
            assert!(run(&Circuit { qubits: 2, init: None, ops: vec![o] }).is_ok(), "{g} with theta rejected");
        }
    }

    #[test]
    fn bell_matches_real_simulator() {
        let c = Circuit { qubits: 2, init: None, ops: vec![op("H", &[0], &[]), op("CX", &[1], &[0])] };
        let r = run(&c).unwrap();
        let p = probs(r.states.last().unwrap());
        assert!(close(p[0], 0.5) && close(p[3], 0.5) && close(p[1], 0.0));
        assert_eq!(ket(r.states.last().unwrap(), 2), "(|00⟩ + |11⟩)/√2");
    }

    #[test]
    fn phases_and_identities() {
        // S·S = Z, T·T = S, H·Z·H = X, RZ(π) = −i·Z (global phase), Y = iXZ.
        let mut s = initial(1, Some("1")).unwrap();
        apply(&mut s, 1, &op("S", &[0], &[])).unwrap();
        assert_eq!(ket(&s, 1), "i|1⟩");
        apply(&mut s, 1, &op("S", &[0], &[])).unwrap();
        assert_eq!(ket(&s, 1), "−|1⟩");
        let mut h = initial(1, None).unwrap();
        for g in ["H", "Z", "H"] {
            apply(&mut h, 1, &op(g, &[0], &[])).unwrap();
        }
        assert_eq!(ket(&h, 1), "|1⟩");
        let mut plus_i = initial(1, None).unwrap();
        apply(&mut plus_i, 1, &op("H", &[0], &[])).unwrap();
        apply(&mut plus_i, 1, &op("S", &[0], &[])).unwrap();
        assert_eq!(ket(&plus_i, 1), "(|0⟩ + i|1⟩)/√2");
        let mut ry = initial(1, None).unwrap();
        apply(&mut ry, 1, &Op { theta: Some(Angle::Text("pi/3".into())), ..op("RY", &[0], &[]) }).unwrap();
        assert!(close(ry[0].norm2(), 0.75) && close(ry[1].norm2(), 0.25));
    }

    #[test]
    fn controls_swap_and_toffoli() {
        let mut s = initial(3, Some("110")).unwrap();
        apply(&mut s, 3, &op("CCX", &[2], &[0, 1])).unwrap();
        assert_eq!(ket(&s, 3), "|111⟩");
        apply(&mut s, 3, &op("X", &[1], &[])).unwrap();
        apply(&mut s, 3, &op("SWAP", &[0, 1], &[])).unwrap();
        assert_eq!(ket(&s, 3), "|011⟩");
        assert!(apply(&mut s, 3, &op("CX", &[1], &[])).is_err()); // missing control
        assert!(apply(&mut s, 3, &op("CX", &[1], &[1])).is_err()); // control == target
        assert!(apply(&mut s, 3, &op("H", &[3], &[])).is_err()); // out of range
    }

    #[test]
    fn teleportation_by_deferred_measurement() {
        // q0 = RY(π/3)|0⟩ teleported to q2; classical corrections replaced by CX(q1→q2), CZ(q0→q2).
        let th = Some(Angle::Text("pi/3".into()));
        let c = Circuit {
            qubits: 3,
            init: None,
            ops: vec![
                Op { theta: th, ..op("RY", &[0], &[]) },
                op("H", &[1], &[]),
                op("CX", &[2], &[1]),
                op("CX", &[1], &[0]),
                op("H", &[0], &[]),
                op("CX", &[2], &[1]),
                op("CZ", &[2], &[0]),
            ],
        };
        let r = run(&c).unwrap();
        let mq2 = marginal(r.states.last().unwrap(), 3, &[2]);
        assert!(close(mq2["0"], 0.75) && close(mq2["1"], 0.25));
        // and the norm is preserved at every step
        for s in &r.states {
            assert!(close(probs(s).iter().sum::<f64>(), 1.0));
        }
    }
}
