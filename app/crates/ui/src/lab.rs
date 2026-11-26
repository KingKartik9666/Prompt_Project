//! Lab: pick a circuit, step through it gate by gate, watch the amplitudes move.
//! Runs entirely on the WebAssembly simulator, so it works without an AI key.

use crate::state::use_app;
use crate::viz::{Amps, Circuit};
use leptos::prelude::*;
use qcore::{Algo, FKind};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bell,
    Dj,
    Grover,
}

#[component]
pub fn Lab() -> impl IntoView {
    let app = use_app();
    let kind = RwSignal::new(Kind::Grover);
    let marked = RwSignal::new(1usize);
    let dj_n = RwSignal::new(2u32);
    let dj_f = RwSignal::new(FKind::Balanced);
    let step = RwSignal::new(0usize);

    let algo = Memo::new(move |_| match kind.get() {
        Kind::Bell => Algo::Bell,
        Kind::Dj => Algo::DeutschJozsa { n: dj_n.get(), f: dj_f.get() },
        Kind::Grover => Algo::Grover2 { marked: marked.get() },
    });
    let trace = Memo::new(move |_| qcore::trace(&algo.get()));
    // A new circuit starts from its first step.
    Effect::new(move |_| {
        algo.track();
        step.set(0);
    });
    let last = move || trace.with(|t| t.steps.len() - 1);
    let cur = Memo::new(move |_| trace.with(|t| t.steps[step.get().min(t.steps.len() - 1)].clone()));
    let amps = Signal::derive(move || cur.with(|s| s.amps.clone()));
    let basis = Signal::derive(move || trace.with(|t| t.basis.clone()));
    let active = Signal::derive(move || cur.with(|s| s.column));

    // ← / → step through the circuit.
    Effect::new(move |prev: Option<u32>| {
        let (k, n) = app.key.get();
        if prev.is_some_and(|p| p != n) {
            match k.as_str() {
                "ArrowRight" => step.update(|s| *s = (*s + 1).min(last())),
                "ArrowLeft" => step.update(|s| *s = s.saturating_sub(1)),
                _ => {}
            }
        }
        n
    });

    let finding = move || {
        let t = trace.get();
        let probs = qcore::probabilities(&t.steps.last().unwrap().amps, t.qubits);
        let parts: Vec<String> = probs.iter().map(|(k, p)| format!("P({k}) = {}", fmt_p(*p))).collect();
        parts.join(" · ")
    };

    let seg = move |k: Kind, label: &'static str| {
        view! {
            <button class="seg" aria-pressed=move || (kind.get() == k).to_string() on:click=move |_| kind.set(k)>
                {label}
            </button>
        }
    };

    view! {
        <section class="screen lab" aria-labelledby="lab-title">
            <header class="title-row">
                <div>
                    <h1 id="lab-title">"Lab"</h1>
                    <p class="sub">
                        {move || algo.with(|a| a.title())} ": " {finding}
                    </p>
                </div>
                <div class="segs" role="group" aria-label="Circuit">
                    {seg(Kind::Bell, "Bell state")}
                    {seg(Kind::Dj, "Deutsch–Jozsa")}
                    {seg(Kind::Grover, "Grover search")}
                </div>
            </header>

            <div class="params">
                <Show when=move || kind.get() == Kind::Grover>
                    <span class="param-label">"Marked item"</span>
                    <div class="segs small" role="group" aria-label="Marked item">
                        {(0..4usize)
                            .map(|m| {
                                view! {
                                    <button class="seg" aria-pressed=move || (marked.get() == m).to_string() on:click=move |_| marked.set(m)>
                                        {format!("|{m:02b}⟩")}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                </Show>
                <Show when=move || kind.get() == Kind::Dj>
                    <span class="param-label">"Input qubits"</span>
                    <div class="segs small" role="group" aria-label="Input qubits">
                        {(1..=3u32)
                            .map(|n| {
                                view! {
                                    <button class="seg" aria-pressed=move || (dj_n.get() == n).to_string() on:click=move |_| dj_n.set(n)>
                                        {n.to_string()}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <span class="param-label">"Oracle"</span>
                    <div class="segs small" role="group" aria-label="Oracle function">
                        {FKind::ALL
                            .into_iter()
                            .map(|f| {
                                view! {
                                    <button class="seg" aria-pressed=move || (dj_f.get() == f).to_string() on:click=move |_| dj_f.set(f)>
                                        {f.label()}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                </Show>
                <Show when=move || kind.get() == Kind::Bell>
                    <span class="param-label">"H on q0, then CNOT from q0 to q1. No parameters."</span>
                </Show>
            </div>

            <div class="lab-grid">
                <div class="panel circuit-panel">
                    <Circuit trace=trace active=active />
                    <p class="note">{move || trace.with(|t| t.note.clone().unwrap_or_default())}</p>
                </div>

                <div class="panel state-panel">
                    <div class="state-head">
                        <h2>{move || cur.with(|s| s.title.clone())}</h2>
                        <p class="ket" aria-live="polite">{move || cur.with(|s| s.ket.clone())}</p>
                    </div>
                    <Amps amps=amps basis=basis />
                    <p class="detail">{move || cur.with(|s| s.detail.clone())}</p>
                </div>
            </div>

            <nav class="stepper" aria-label="Steps">
                <button class="btn ghost" on:click=move |_| step.update(|s| *s = s.saturating_sub(1)) disabled=move || step.get() == 0>
                    "Back"
                </button>
                <ol class="ticks">
                    {move || {
                        trace
                            .get()
                            .steps
                            .iter()
                            .enumerate()
                            .map(|(i, s)| {
                                let title = s.title.clone();
                                view! {
                                    <li>
                                        <button
                                            class="tick"
                                            aria-current=move || if step.get() == i { "step" } else { "false" }
                                            data-done=move || (i < step.get()).to_string()
                                            on:click=move |_| step.set(i)
                                        >
                                            {title}
                                        </button>
                                    </li>
                                }
                            })
                            .collect_view()
                    }}
                </ol>
                <button class="btn primary" on:click=move |_| step.update(|s| *s = (*s + 1).min(last())) disabled=move || { step.get() >= last() }>
                    "Next gate"
                </button>
            </nav>
        </section>
    }
}

pub fn fmt_p(p: f64) -> String {
    if (p - 0.5).abs() < 1e-9 {
        "½".into()
    } else if (p - 0.25).abs() < 1e-9 {
        "¼".into()
    } else {
        let s = format!("{p:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
