//! Visuals drawn from the simulator: states in circle notation, the circuit, and claimed-vs-simulated.

use leptos::prelude::*;
use qcore::{Column, Probs, Trace};

fn fmt_amp(a: f64) -> String {
    if a.abs() < 5e-7 {
        "0".into()
    } else {
        let s = format!("{:.3}", a.abs());
        let s = s.trim_end_matches('0').trim_end_matches('.');
        format!("{}{}", if a < 0.0 { "−" } else { "" }, s)
    }
}

/// Phase angle drawn for a real amplitude: + points up (0°), − points down. Negative angles, because
/// phase turns counterclockwise in circle notation and a positive CSS rotate is clockwise.
pub fn phase_deg(a: f64) -> f64 {
    if a < -1e-9 { -180.0 } else { 0.0 }
}

/// Label brightness for a basis state: empty states recede, occupied ones come forward.
pub fn label_opacity(mag: f64) -> f64 {
    0.32 + 0.63 * mag.clamp(0.0, 1.0).sqrt()
}

/// Circle-notation grid cell for basis label `k` when there are two qubits: one axis per qubit
/// (columns = first qubit, rows = second), so a Bell state lies on the diagonal.
pub fn cell(k: &str) -> String {
    let b: Vec<char> = k.chars().collect();
    match b.as_slice() {
        [c, r] => format!("grid-column:{};grid-row:{}", if *c == '1' { 2 } else { 1 }, if *r == '1' { 2 } else { 1 }),
        _ => String::new(),
    }
}

/// One basis state in circle notation (Johnston, Harrigan & Gimeno-Segovia, *Programming Quantum
/// Computers*, O'Reilly 2019): the outer ring is probability 1; the filled disc's area is the
/// probability |a|² (radius |a|); the line from the centre shows the phase (up = +, down = −).
/// Fill hue follows the sign (lime +, coral −). Only `scale`, `rotate` and `opacity` change, so
/// every transition runs on the compositor.
#[component]
pub fn QubitDisc(
    #[prop(into)] key_label: Signal<String>,
    /// magnitude |a| in 0..=1
    #[prop(into)]
    mag: Signal<f64>,
    /// phase line angle in degrees
    #[prop(into)]
    phase: Signal<f64>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let neg = move || (1.0 - phase.get().to_radians().cos()) / 2.0; // 0 at 0°, 1 at ±180°
    let place = move || key_label.with(|k| cell(k));
    view! {
        <div class="q" style=place>
            <span class="q-disc">
                <span class="q-fill" style=move || format!("scale:{:.4}", mag.get())></span>
                <span class="q-fill neg" style=move || format!("scale:{:.4};opacity:{:.4}", mag.get(), neg())></span>
                <span
                    class="q-phase"
                    style=move || format!("rotate:{:.2}deg;opacity:{}", phase.get(), if mag.get() > 1e-6 { 1 } else { 0 })
                ></span>
            </span>
            <span class="q-key" style=move || format!("opacity:{:.3}", label_opacity(mag.get()))>
                "|" {move || key_label.get()} "⟩"
            </span>
            {children.map(|c| c())}
        </div>
    }
}

/// A multi-qubit state as one circle per basis state. Discs persist between steps, so every gate
/// animates: probabilities grow or shrink, and a sign flip turns the phase line through 180°.
#[component]
pub fn Amps(#[prop(into)] amps: Signal<Vec<f64>>, #[prop(into)] basis: Signal<Vec<String>>) -> impl IntoView {
    let len = move || amps.with(Vec::len);
    let label = move || {
        let parts: Vec<String> = basis.with(|b| {
            amps.with(|a| b.iter().zip(a).map(|(k, x)| format!("|{k}⟩ {}", fmt_amp(*x))).collect())
        });
        format!("Amplitudes: {}", parts.join(", "))
    };
    let amp = move |i: usize| amps.with(|v| v.get(i).copied().unwrap_or(0.0));
    view! {
        <figure class="amps" role="img" aria-label=label data-grid=move || (len() == 4).to_string() style=move || format!("--n:{}", len())>
            <For each=move || 0..len() key=|i| *i let:i>
                <QubitDisc
                    key_label=Signal::derive(move || basis.with(|b| b.get(i).cloned().unwrap_or_default()))
                    mag=Signal::derive(move || amp(i).abs())
                    phase=Signal::derive(move || phase_deg(amp(i)))
                >
                    <span class="q-nums" data-sign=move || if amp(i) < -1e-9 { "neg" } else { "pos" }>
                        <span class="q-val">{move || fmt_amp(amp(i))}</span>
                        <span class="q-prob">{move || format!("{:.0}%", amp(i) * amp(i) * 100.0)}</span>
                    </span>
                </QubitDisc>
            </For>
        </figure>
    }
}

const COL_W: f64 = 112.0;
const ROW_H: f64 = 72.0;
const LEFT: f64 = 92.0;
const TOP: f64 = 44.0;

/// The circuit as SVG; `active` highlights the column the current step applies.
#[component]
pub fn Circuit(#[prop(into)] trace: Signal<Trace>, #[prop(into)] active: Signal<Option<usize>>) -> impl IntoView {
    move || {
        let t = trace.get();
        let q = t.qubits;
        let w = LEFT + COL_W * t.columns.len() as f64 + 40.0;
        let h = TOP * 2.0 + ROW_H * (q.saturating_sub(1)) as f64;
        let y = move |i: usize| TOP + ROW_H * i as f64;
        let cx = |c: usize| LEFT + COL_W * c as f64 + COL_W / 2.0;
        let wires = (0..q)
            .map(|i| {
                view! {
                    <line class="wire" x1=LEFT - 14.0 y1=y(i) x2=w - 16.0 y2=y(i)></line>
                    <text class="wire-label" x=8 y=y(i) dy="0.35em">{t.wires[i].clone()} " |0⟩"</text>
                }
            })
            .collect_view();
        let gates = t
            .columns
            .iter()
            .enumerate()
            .map(|(c, col)| {
                let x = cx(c);
                match col.clone() {
                    Column::H { wires } => wires
                        .into_iter()
                        .map(|i| {
                            view! {
                                <g class="gate">
                                    <rect x=x - 22.0 y=y(i) - 22.0 width=44 height=44 rx=4></rect>
                                    <text x=x y=y(i) dy="0.36em">"H"</text>
                                </g>
                            }
                        })
                        .collect_view()
                        .into_any(),
                    Column::Cnot { control, target } => view! {
                        <g class="cnot">
                            <line x1=x y1=y(control) x2=x y2=y(target) + 18.0></line>
                            <circle class="dot" cx=x cy=y(control) r=7></circle>
                            <circle class="plus" cx=x cy=y(target) r=18></circle>
                            <line x1=x - 18.0 y1=y(target) x2=x + 18.0 y2=y(target)></line>
                        </g>
                    }
                    .into_any(),
                    Column::Block { label } => view! {
                        <g class="gate block">
                            <rect x=x - 40.0 y=y(0) - 26.0 width=80 height=ROW_H * (q - 1) as f64 + 52.0 rx=4></rect>
                            <text x=x y=(y(0) + y(q - 1)) / 2.0 dy="0.36em">{label}</text>
                        </g>
                    }
                    .into_any(),
                    Column::Measure => (0..q)
                        .map(|i| {
                            view! {
                                <g class="meter">
                                    <rect x=x - 22.0 y=y(i) - 18.0 width=44 height=36 rx=4></rect>
                                    <path d=format!(
                                        "M{} {} A14 14 0 0 1 {} {} M{} {} L{} {}",
                                        x - 13.0, y(i) + 9.0, x + 13.0, y(i) + 9.0, x, y(i) + 9.0, x + 9.0, y(i) - 9.0,
                                    )></path>
                                </g>
                            }
                        })
                        .collect_view()
                        .into_any(),
                }
            })
            .collect_view();
        view! {
            <svg class="circuit" viewBox=format!("0 0 {w} {h}") role="img" aria-label=format!("Circuit for {}", t.wires.join(", "))>
                <rect
                    class="col-hi"
                    x=0
                    y=4
                    width=COL_W - 8.0
                    height=h - 8.0
                    rx=8
                    style=move || match active.get() {
                        Some(c) => format!("transform:translateX({}px);opacity:1", LEFT + COL_W * c as f64 + 4.0),
                        None => format!("transform:translateX({}px);opacity:0", LEFT - COL_W),
                    }
                ></rect>
                {wires}
                {gates}
            </svg>
        }
    }
}

/// Claimed (from the explanation) against simulated probabilities, one row per outcome.
#[component]
pub fn Compare(claimed: Probs, truth: Probs) -> impl IntoView {
    let mut keys: Vec<String> = truth.keys().chain(claimed.keys()).cloned().collect();
    keys.sort();
    keys.dedup();
    let rows = keys
        .into_iter()
        .map(|k| {
            let c = claimed.get(&k).copied();
            let t = truth.get(&k).copied().unwrap_or(0.0);
            let ok = c.is_some_and(|c| (c - t).abs() <= 0.02);
            view! {
                <div class="cmp-row" data-ok=if ok { "yes" } else { "no" }>
                    <span class="cmp-key">"|" {k.clone()} "⟩"</span>
                    <span class="cmp-bars">
                        <span class="cmp-bar claimed" style=format!("--v:{}", c.unwrap_or(0.0))></span>
                        <span class="cmp-bar truth" style=format!("--v:{t}")></span>
                    </span>
                    <span class="cmp-num">{c.map_or("—".to_string(), |c| format!("{c:.2}"))}</span>
                    <span class="cmp-num truth">{format!("{t:.2}")}</span>
                </div>
            }
        })
        .collect_view();
    view! {
        <div class="cmp">
            <div class="cmp-head">
                <span></span>
                <span></span>
                <span class="cmp-num">"Explained"</span>
                <span class="cmp-num truth">"Simulated"</span>
            </div>
            {rows}
        </div>
    }
}
