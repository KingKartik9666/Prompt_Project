//! Home: the scroll story, in the manner of a product launch page.
//!
//! Scrolling stays native (nothing is hijacked). Every moving part is a paused animation, CSS
//! (`[data-scrub]` in app.css) or Web Animations (the hero's state morph, built below), and a small
//! driver sets each one's `currentTime` from scroll progress. The progress follows the scroll
//! position through a critically damped spring, so a mouse-wheel notch glides instead of jumping,
//! and the motion is continuous: there are no discrete states to snap between.

use crate::state::{Screen, use_app};
use crate::viz::{Circuit, QubitDisc, label_opacity, phase_deg};
use js_sys::{Array, Function, Object, Reflect};
use leptos::prelude::*;
use qcore::Algo;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};

/// Each scrubbed animation spans this many ms; currentTime = progress × DURATION_MS.
const DURATION_MS: f64 = 1000.0;
/// Spring response in seconds (critically damped: no overshoot). Apple's heaviest scroll smoothing
/// reaches 95% in about 0.47 s; 0.6 s here (95% in ~0.45 s) because the user asked for calmer motion.
const RESPONSE_S: f64 = 0.6;
const EASE_SOFT: &str = "cubic-bezier(0.45, 0, 0.55, 1)";

struct Track {
    el: HtmlElement,
    /// The pinned section whose progress drives this element, or None for its own pass through the view.
    section: Option<HtmlElement>,
    anims: Vec<JsValue>,
    x: f64,
    v: f64,
    target: f64,
}

#[derive(Default)]
struct Motion {
    tracks: Vec<Track>,
    running: bool,
    last: Option<f64>,
}

thread_local! {
    static MOTION: RefCell<Motion> = RefCell::default();
}

fn stage() -> Option<HtmlElement> {
    web_sys::window()?.document()?.query_selector(".stage").ok().flatten()?.dyn_into().ok()
}

fn call(target: &JsValue, method: &str, args: &[&JsValue]) -> Option<JsValue> {
    let f: Function = Reflect::get(target, &method.into()).ok()?.dyn_into().ok()?;
    let arr: Array = args.iter().copied().collect();
    f.apply(target, &arr).ok()
}

fn animations(el: &Element) -> Vec<JsValue> {
    call(el, "getAnimations", &[]).map(|a| Array::from(&a).to_vec()).unwrap_or_default()
}

/// A paused Web Animation of `frames` (offset, [(property, value)]) that the driver scrubs.
fn scrub_anim(el: &Element, frames: &[(f64, Vec<(&str, String)>)]) {
    let kf = Array::new();
    for (off, props) in frames {
        let o = Object::new();
        let _ = Reflect::set(&o, &"offset".into(), &(*off).into());
        let _ = Reflect::set(&o, &"easing".into(), &EASE_SOFT.into());
        for (k, v) in props {
            let _ = Reflect::set(&o, &(*k).into(), &v.as_str().into());
        }
        kf.push(&o);
    }
    let opts = Object::new();
    let _ = Reflect::set(&opts, &"duration".into(), &DURATION_MS.into());
    let _ = Reflect::set(&opts, &"fill".into(), &"both".into());
    if let Some(anim) = call(el, "animate", &[&kf, &opts]) {
        call(&anim, "pause", &[]);
        // tag it so mount_motion picks it up and drives it with the hero's progress
        let _ = el.set_attribute("data-scrub", "hero");
    }
}

/// Hero timeline, as fractions of the hero's pinned scroll (7.5 screens): the window opens to fill the
/// screen by 0.22 (CSS `engulf`), the discs settle by 0.30, then Grover's four states morph in place,
/// each over about one screen of scrolling. (from, to) of each morph; the gaps between are holds.
/// The in-between frames of the H and diffusion morphs are illustrative (only the oracle step is a
/// pure phase rotation); the held states are exact simulator output.
const MORPHS: [(f64, f64); 3] = [(0.36, 0.5), (0.56, 0.7), (0.76, 0.9)];

/// Build the hero's state-morph animations from the simulator trace.
fn build_hero(states: &[Vec<f64>]) {
    let Some(stage) = stage() else { return };
    let Ok(discs) = stage.query_selector_all(".hero-media .q") else { return };
    // Keyframe offsets: hold each state, then morph to the next.
    let mut offsets = vec![0.0];
    for (a, b) in MORPHS {
        offsets.push(a);
        offsets.push(b);
    }
    offsets.push(1.0);
    // State index shown at each offset: 0,0,1,1,2,2,3,3
    let state_at = |k: usize| k / 2;
    for i in 0..discs.length() {
        let Some(q) = discs.item(i).and_then(|n| n.dyn_into::<Element>().ok()) else { continue };
        let i = i as usize;
        // Phase angles accumulate in one direction, so + → − → + turns the line all the way round.
        let mut angle = Vec::with_capacity(states.len());
        let mut acc = phase_deg(states[0][i]);
        for (s, st) in states.iter().enumerate() {
            if s > 0 && phase_deg(st[i]) != phase_deg(states[s - 1][i]) {
                acc -= 180.0;
            }
            angle.push(acc);
        }
        let frames = |f: &dyn Fn(usize) -> Vec<(&'static str, String)>| -> Vec<(f64, Vec<(&'static str, String)>)> {
            offsets.iter().enumerate().map(|(k, &o)| (o, f(state_at(k)))).collect()
        };
        let mag = |s: usize| states[s][i].abs();
        let neg = |s: usize| (1.0 - angle[s].to_radians().cos()) / 2.0;
        if let Ok(Some(el)) = q.query_selector(".q-fill:not(.neg)") {
            scrub_anim(&el, &frames(&|s| vec![("scale", format!("{:.4}", mag(s)))]));
        }
        if let Ok(Some(el)) = q.query_selector(".q-fill.neg") {
            scrub_anim(&el, &frames(&|s| vec![("scale", format!("{:.4}", mag(s))), ("opacity", format!("{:.4}", neg(s)))]));
        }
        if let Ok(Some(el)) = q.query_selector(".q-phase") {
            let op = |s: usize| if mag(s) > 1e-6 { "1" } else { "0" };
            scrub_anim(&el, &frames(&|s| vec![("rotate", format!("{:.1}deg", angle[s])), ("opacity", op(s).into())]));
        }
        if let Ok(Some(el)) = q.query_selector(".q-key") {
            scrub_anim(&el, &frames(&|s| vec![("opacity", format!("{:.3}", label_opacity(mag(s))))]));
        }
    }
    // Captions: each one is visible during its state's hold and cross-fades at the morph midpoints.
    let Ok(caps) = stage.query_selector_all(".hero-cap") else { return };
    let mids: Vec<f64> = MORPHS.iter().map(|(a, b)| (a + b) / 2.0).collect();
    // Sequential, not overlapping: the old caption is gone before the new one rises (no double text).
    let (near, far) = (0.004, 0.04);
    for k in 0..caps.length() {
        let Some(el) = caps.item(k).and_then(|n| n.dyn_into::<Element>().ok()) else { continue };
        let k = k as usize;
        let show = |on: bool, y: &str| vec![("opacity", if on { "1" } else { "0" }.to_string()), ("translate", format!("0 {y}"))];
        // The first caption is already there in the small window, so it never looks empty.
        let mut f = if k == 0 {
            vec![(0.0, show(true, "0px"))]
        } else {
            let start = mids[k - 1];
            vec![(0.0, show(false, "16px")), (start + near, show(false, "16px")), (start + far, show(true, "0px"))]
        };
        if let Some(&end) = mids.get(k) {
            f.push((end - far, show(true, "0px")));
            f.push((end - near, show(false, "-16px")));
            f.push((1.0, show(false, "-16px")));
        } else {
            f.push((1.0, show(true, "0px")));
        }
        let f: Vec<_> = f.into_iter().map(|(o, p)| (o.clamp(0.0, 1.0), p)).collect();
        scrub_anim(&el, &f);
    }
}

/// Collect every scrubbed element and its animations, and jump to the current scroll position.
fn mount_motion() {
    let Some(stage) = stage() else { return };
    let mut tracks = vec![];
    if let Ok(list) = stage.query_selector_all("[data-scrub]") {
        for i in 0..list.length() {
            let Some(el) = list.item(i).and_then(|n| n.dyn_into::<HtmlElement>().ok()) else { continue };
            let section = match el.get_attribute("data-scrub").as_deref() {
                Some("hero") => stage.query_selector(".hero").ok().flatten(),
                Some("show") => stage.query_selector(".showcase").ok().flatten(),
                _ => None,
            }
            .and_then(|s| s.dyn_into::<HtmlElement>().ok());
            let anims = animations(&el);
            if !anims.is_empty() {
                tracks.push(Track { el, section, anims, x: 0.0, v: 0.0, target: 0.0 });
            }
        }
    }
    MOTION.with_borrow_mut(|m| *m = Motion { tracks, ..Default::default() });
    measure_hero();
    retarget(&stage, true);
}

/// The small window's rectangle, as insets of the full-screen media layer (px), written on
/// `.hero-stage` so the window (`engulf`) and the headline halves (`part-*`) share it. Measured on
/// mount and resize only; the window is ~34% of the screen tall, centred on the headline line.
fn measure_hero() {
    let Some(stage) = stage() else { return };
    let (Some(hs), Some(gap)) = (
        stage.query_selector(".hero-stage").ok().flatten().and_then(|e| e.dyn_into::<HtmlElement>().ok()),
        stage.query_selector(".hero-gap").ok().flatten(),
    ) else {
        return;
    };
    let s = hs.get_bounding_client_rect();
    let g = gap.get_bounding_client_rect();
    let (w, h) = (s.width(), s.height());
    let win_h = (h * 0.34).min(g.width() * 0.75);
    let cy = g.top() + g.height() / 2.0 - s.top();
    let inset = 0.012 * w; // breathing room between the words and the window
    let (l, r) = (g.left() - s.left() + inset, s.right() - g.right() + inset);
    let (t, b) = (cy - win_h / 2.0, h - (cy + win_h / 2.0));
    let css = hs.style();
    for (k, v) in [("--t", t), ("--r", r), ("--b", b), ("--l", l)] {
        let _ = css.set_property(k, &format!("{:.1}px", v.max(0.0)));
    }
}

/// Window resized: re-measure the hero window, then retarget.
pub fn on_resize() {
    measure_hero();
    on_scroll();
}

fn clear_motion() {
    MOTION.with_borrow_mut(|m| *m = Motion::default());
}

/// Scroll or resize while Home is shown: new targets; the frame loop springs toward them.
pub fn on_scroll() {
    if let Some(stage) = stage() {
        retarget(&stage, false);
    }
}

/// Layout positions (offsetTop/offsetHeight ignore transforms, so animated elements don't feed
/// back into their own progress). `.stage` is the offsetParent (position: relative).
fn progress(t: &Track, scroll: f64, h: f64) -> f64 {
    let p = match &t.section {
        Some(sec) => (scroll - sec.offset_top() as f64) / (sec.offset_height() as f64 - h).max(1.0),
        // 0 when the element's top enters at the bottom, 1 when its bottom leaves at the top.
        None => (scroll + h - t.el.offset_top() as f64) / (h + t.el.offset_height() as f64),
    };
    p.clamp(0.0, 1.0)
}

fn apply(t: &Track) {
    let ms = JsValue::from_f64(t.x.clamp(0.0, 1.0) * DURATION_MS);
    for a in &t.anims {
        let _ = Reflect::set(a, &"currentTime".into(), &ms);
    }
}

fn retarget(stage: &HtmlElement, snap: bool) {
    let (scroll, h) = (stage.scroll_top() as f64, stage.client_height() as f64);
    let start = MOTION.with_borrow_mut(|m| {
        for t in &mut m.tracks {
            t.target = progress(t, scroll, h);
            if snap {
                t.x = t.target;
                t.v = 0.0;
                apply(t);
            }
        }
        let go = !snap && !m.running && m.tracks.iter().any(|t| (t.target - t.x).abs() > 1e-4);
        if go {
            m.running = true;
            m.last = None;
        }
        go
    });
    if start {
        next_frame();
    }
}

fn next_frame() {
    let cb = Closure::once_into_js(tick);
    if let Some(w) = web_sys::window() {
        let _ = w.request_animation_frame(cb.unchecked_ref());
    }
}

/// One frame of the critically damped spring x'' = ω²(target − x) − 2ωx', ω = 2π / response,
/// integrated in ≤4 ms sub-steps (stable at any frame rate).
fn tick(now: f64) {
    let again = MOTION.with_borrow_mut(|m| {
        let dt = m.last.map_or(1.0 / 60.0, |l| ((now - l) / 1000.0).clamp(0.0, 0.1));
        m.last = Some(now);
        let w = std::f64::consts::TAU / RESPONSE_S;
        let steps = (dt / 0.004).ceil().max(1.0);
        let h = dt / steps;
        let mut moving = false;
        for t in &mut m.tracks {
            if (t.target - t.x).abs() < 1e-4 && t.v.abs() < 1e-3 {
                if t.x != t.target {
                    t.x = t.target;
                    t.v = 0.0;
                    apply(t);
                }
                continue;
            }
            for _ in 0..steps as usize {
                t.v += (w * w * (t.target - t.x) - 2.0 * w * t.v) * h;
                t.x += t.v * h;
            }
            apply(t);
            moving = true;
        }
        m.running = moving;
        moving
    });
    if again {
        next_frame();
    }
}

#[component]
pub fn Home() -> impl IntoView {
    let app = use_app();
    let grover = qcore::trace(&Algo::Grover2 { marked: 1 });
    // Start, Hadamard on both, Oracle, Diffusion: the states the hero morphs through.
    let states: Vec<Vec<f64>> = grover.steps.iter().take(4).map(|s| s.amps.clone()).collect();
    let first = states[0].clone();
    let discs = grover
        .basis
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let a = first[i];
            view! { <QubitDisc key_label=k.clone() mag=a.abs() phase=phase_deg(a) /> }
        })
        .collect_view();
    let caps = grover
        .steps
        .iter()
        .take(4)
        .enumerate()
        .map(|(k, s)| {
            view! {
                <div class="hero-cap" data-scrub="hero">
                    <p class="hero-cap-step">{format!("Step {} of 4", k + 1)}</p>
                    <p class="hero-cap-title">{s.title.clone()}</p>
                    <p class="hero-cap-ket ket">{s.ket.clone()}</p>
                    <p class="hero-cap-detail">{s.detail.clone()}</p>
                </div>
            }
        })
        .collect_view();
    let showcase_trace = Signal::stored(qcore::trace(&Algo::Grover2 { marked: 1 }));

    // After mount: build the hero morph, collect every scrubbed animation, sync to the scroll position.
    Effect::new(move |_| {
        let states = states.clone();
        request_animation_frame(move || {
            if !crate::bridge::reduced_motion() {
                build_hero(&states);
            }
            mount_motion();
        });
    });
    on_cleanup(clear_motion);

    view! {
        <div class="home">
            <section class="hero" aria-labelledby="hero-title">
                <div class="hero-stage">
                    <h1 id="hero-title" class="hero-line">
                        <span class="hero-half left" data-scrub="hero"><span class="hero-in">"Every" <br /> "qubit"</span></span>
                        <span class="hero-gap" aria-hidden="true"></span>
                        <span class="hero-half right" data-scrub="hero"><span class="hero-in">"is" <br /> "proven."</span></span>
                    </h1>
                    // The window between the two halves; it grows until it fills the whole screen.
                    <div class="hero-media" data-scrub="hero" aria-label="Grover search on two qubits, in circle notation" role="img">
                        <div class="hero-media-in" data-scrub="hero">
                            <p class="hero-over">"Grover search · two qubits · looking for |01⟩"</p>
                            <div class="hero-discs" data-scrub="hero">{discs}</div>
                            <div class="hero-caps">{caps}</div>
                        </div>
                    </div>
                    <p class="hero-sub" data-scrub="hero">
                        "Quantum Explainer teaches the Bell state, Deutsch–Jozsa and Grover search step by step. A simulator built into the app recomputes each answer's final probabilities before you read them."
                    </p>
                    <button class="btn primary hero-cta" data-scrub="hero" on:click=move |_| app.go(Screen::Tutor)>"Ask a question"</button>
                </div>
            </section>

            <section class="stairs" aria-labelledby="stairs-title">
                <div class="stair" data-reveal>
                    <h2 id="stairs-title" class="stair-line">"First it reads your question."</h2>
                    <p>"Which algorithm, which inputs, your level, and any belief that needs correcting, such as exponential speedup for Grover."</p>
                </div>
                <div class="stair" data-reveal>
                    <p class="stair-line">"Then the AI explains."</p>
                    <p>"Column vectors for beginners, bra-ket for intermediate learners, with the full state written after every gate."</p>
                </div>
                <div class="stair" data-reveal>
                    <p class="stair-line accent">"The simulator checks."</p>
                    <p>"Final probabilities are pulled out of the text and recomputed exactly. A mismatch sends the explanation back once with the right answer."</p>
                </div>
                <div class="stair" data-reveal>
                    <p class="stair-line">"Then you practise."</p>
                    <p>"One new question at your level, reviewed against the verified result, with a hint you open only if you need it."</p>
                </div>
            </section>

            <section class="showcase" aria-labelledby="show-title">
                <div class="show-stage">
                    <h2 id="show-title" class="show-head">
                        <span class="show-top">"See the state"</span>
                        <span class="show-bottom" data-scrub="show">"after every gate."</span>
                    </h2>
                    <div class="show-frame">
                        <div class="show-zoom" data-scrub="show">
                            <Circuit trace=showcase_trace active=Signal::stored(None) />
                        </div>
                    </div>
                    <p class="show-cap" data-scrub="show">"The oracle flips one sign; diffusion turns that sign into certainty. Step through it yourself in the Lab."</p>
                </div>
            </section>

            <section class="fields" aria-label="What makes it different">
                <div class="field f1">
                    <p class="field-text" data-reveal>"Language models are fluent and sometimes wrong about the numbers. Here a wrong final probability never reaches you silently: it is corrected or flagged."</p>
                    <h2 class="field-phrase" data-reveal>"Checked, not trusted."</h2>
                </div>
                <div class="field f2">
                    <p class="field-text" data-reveal>"Choose beginner or intermediate. Ask in English or Hinglish; the answer comes back in simple English."</p>
                    <h2 class="field-phrase" data-reveal>"Your level, your words."</h2>
                </div>
                <div class="field f3">
                    <p class="field-text" data-reveal>"Shor or teleportation still get an explanation, clearly marked unverified. Off-topic questions and instructions hidden in a question are turned away."</p>
                    <h2 class="field-phrase" data-reveal>"Honest about limits."</h2>
                </div>
            </section>

            <section class="close" aria-labelledby="close-title">
                <h2 id="close-title" class="close-head" data-reveal>"Ask it anything quantum."</h2>
                <div class="close-row">
                    <p>"Or open the Lab and push the gates yourself. It needs no key and no connection."</p>
                    <div class="close-actions">
                        <button class="btn on-light ghost" on:click=move |_| app.go(Screen::Lab)>"Open the Lab"</button>
                        <button class="btn on-light" on:click=move |_| app.go(Screen::Tutor)>"Open the tutor"</button>
                    </div>
                </div>
            </section>
        </div>
    }
}
