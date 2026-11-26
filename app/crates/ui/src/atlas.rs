//! The Atlas: every topic in quantum computing as a map that grows dot by dot, plus one deep page
//! per topic (prose, interactive simulator-checked circuits, misconceptions, practice, sources).
//! Content lives in crates/ui/content (embedded at build time by build.rs) and every circuit claim
//! in it is checked against the simulator by `cargo test -p qcore`.

use crate::state::{Screen, use_app};
use crate::viz::QubitDisc;
use leptos::prelude::*;
use qcore::circuit::{self, C, Circuit, Op};
use qcore::topic::{Block, Level, Section, Topic};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::OnceLock;
use std::time::Duration;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/content.rs"));
}

pub struct Content {
    pub sections: Vec<Section>,
    pub topics: Vec<Topic>,
}

/// Parsed once. Files that fail to parse are skipped (the qcore test catches them at build time).
pub fn content() -> &'static Content {
    static C: OnceLock<Content> = OnceLock::new();
    C.get_or_init(|| Content {
        sections: serde_json::from_str(embedded::SECTIONS).unwrap_or_default(),
        topics: embedded::TOPICS.iter().filter_map(|t| serde_json::from_str(t).ok()).collect(),
    })
}

pub fn topic(id: &str) -> Option<&'static Topic> {
    content().topics.iter().find(|t| t.id == id)
}

fn section_title(id: &str) -> String {
    content().sections.iter().find(|s| s.id == id).map_or_else(|| id.to_string(), |s| s.title.clone())
}

fn level_text(l: Level) -> &'static str {
    match l {
        Level::Beginner => "Beginner",
        Level::Intermediate => "Intermediate",
        Level::Advanced => "Advanced",
    }
}

// ───────────────────────── the growing map ─────────────────────────

#[derive(Clone, PartialEq)]
enum Kind {
    Root,
    Hub,
    Topic,
    /// A section with no pages yet.
    Empty,
}

#[derive(Clone)]
struct Node {
    label: String,
    sub: String,
    x: f64,
    y: f64,
    kind: Kind,
    /// Topic id to open, if any.
    open: Option<String>,
}

#[derive(Clone)]
struct Map {
    w: f64,
    h: f64,
    nodes: Vec<Node>,
    /// Growth tree: (parent, child). Children appear after their parent, along the edge.
    tree: Vec<(usize, usize)>,
    /// Cross links drawn once everything has grown (prerequisites between topics).
    links: Vec<(usize, usize)>,
}

/// Appearance time of every node: breadth-first from node 0, so the map grows outward from one dot.
fn schedule(m: &Map) -> (Vec<f64>, f64) {
    const POP: f64 = 900.0; // a dot expanding
    const REACH: f64 = 1100.0; // an edge growing to the next dot
    const STAGGER: f64 = 140.0;
    let n = m.nodes.len();
    let mut kids: Vec<Vec<usize>> = vec![vec![]; n];
    for &(p, c) in &m.tree {
        kids[p].push(c);
    }
    let mut t = vec![0.0; n];
    let mut q = VecDeque::from([0usize]);
    let mut end: f64 = POP;
    while let Some(p) = q.pop_front() {
        for (k, &c) in kids[p].iter().enumerate() {
            t[c] = t[p] + POP * 0.6 + REACH + k as f64 * STAGGER;
            end = end.max(t[c] + POP);
            q.push_back(c);
        }
    }
    (t, end)
}

fn curve(a: &Node, b: &Node) -> String {
    // a gentle bow, so links read as grown rather than ruled
    let (mx, my) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (cx, cy) = (mx - dy * 0.12, my + dx * 0.12);
    format!("M{:.1} {:.1} Q{:.1} {:.1} {:.1} {:.1}", a.x, a.y, cx, cy, b.x, b.y)
}

#[component]
fn GrowMap(map: Map, #[prop(optional, into)] dim: Option<Signal<Vec<bool>>>, #[prop(optional)] class: &'static str) -> impl IntoView {
    let app = use_app();
    let (t, end) = schedule(&map);
    let nodes = map.nodes.clone();
    let edges = map
        .tree
        .iter()
        .map(|&(p, c)| {
            let d = curve(&nodes[p], &nodes[c]);
            let delay = t[p] + 540.0;
            view! { <path class="mg-edge" d=d pathLength="1" style=format!("--d:{delay:.0}ms")></path> }
        })
        .collect_view();
    let links = map
        .links
        .iter()
        .enumerate()
        .map(|(k, &(a, b))| {
            let d = curve(&nodes[a], &nodes[b]);
            let delay = end + 200.0 + k as f64 * 60.0;
            view! { <path class="mg-link" d=d pathLength="1" style=format!("--d:{delay:.0}ms")></path> }
        })
        .collect_view();
    let dots = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let kind = match n.kind {
                Kind::Root => "root",
                Kind::Hub => "hub",
                Kind::Topic => "topic",
                Kind::Empty => "empty",
            };
            let r = match n.kind {
                Kind::Root => 15.0,
                Kind::Hub => 10.0,
                Kind::Topic => 6.5,
                Kind::Empty => 6.0,
            };
            let open = n.open.clone();
            let clickable = open.is_some();
            let below = n.y > map.h * 0.5;
            let (ly, anchor) = if n.kind == Kind::Root { (r + 26.0, "middle") } else if below { (r + 20.0, "middle") } else { (-r - 12.0, "middle") };
            let dim_me = dim.map(move |d| Signal::derive(move || d.with(|v| v.get(i).copied().unwrap_or(false))));
            let label = n.label.clone();
            let sub = n.sub.clone();
            view! {
                <g
                    class=format!("mg-node {kind}")
                    data-dim=move || dim_me.map(|s| s.get()).unwrap_or(false).to_string()
                    style=format!("--d:{:.0}ms;translate:{:.1}px {:.1}px", t[i], n.x, n.y)
                    tabindex=if clickable { "0" } else { "-1" }
                    role=if clickable { "link" } else { "presentation" }
                    on:click=move |_| {
                        if let Some(id) = &open {
                            app.open_topic(id);
                        }
                    }
                    on:keydown={
                        let open = n.open.clone();
                        move |e: web_sys::KeyboardEvent| {
                            if e.key() == "Enter" {
                                if let Some(id) = &open {
                                    app.open_topic(id);
                                }
                            }
                        }
                    }
                >
                    <circle class="mg-halo" r=r * 2.6></circle>
                    <circle class="mg-dot" r=r></circle>
                    <text class="mg-label" y=ly text-anchor=anchor>{label}</text>
                    {(!sub.is_empty()).then(|| view! { <text class="mg-sub" y={ly + if below || n.kind == Kind::Root { 16.0 } else { -16.0 }} text-anchor=anchor>{sub}</text> })}
                </g>
            }
        })
        .collect_view();
    view! {
        <svg class=format!("growmap {class}") viewBox=format!("0 0 {} {}", map.w, map.h) role="img" aria-label="Map of related topics">
            <g class="mg-links">{links}</g>
            <g class="mg-edges">{edges}</g>
            <g class="mg-nodes">{dots}</g>
        </svg>
    }
}

/// The whole Atlas: one root, a hub per section on an ellipse, its topics fanned outward.
fn atlas_map() -> (Map, Vec<String>) {
    let c = content();
    let (w, h) = (1200.0, 780.0);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut nodes = vec![Node { label: "Quantum computing".into(), sub: String::new(), x: cx, y: cy, kind: Kind::Root, open: None }];
    let mut ids = vec![String::new()];
    let mut tree = vec![];
    let ns = c.sections.len().max(1) as f64;
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (si, s) in c.sections.iter().enumerate() {
        let a = -std::f64::consts::FRAC_PI_2 + si as f64 * std::f64::consts::TAU / ns;
        let (hx, hy) = (cx + a.cos() * 250.0, cy + a.sin() * 165.0);
        let topics: Vec<&Topic> = c.topics.iter().filter(|t| t.section == s.id).collect();
        let hub = nodes.len();
        nodes.push(Node {
            label: s.title.clone(),
            sub: match topics.len() { 0 => "coming soon".into(), 1 => "1 topic".into(), n => format!("{n} topics") },
            x: hx,
            y: hy,
            kind: if topics.is_empty() { Kind::Empty } else { Kind::Hub },
            open: None,
        });
        ids.push(String::new());
        tree.push((0, hub));
        let k = topics.len() as f64;
        for (ti, tp) in topics.iter().enumerate() {
            let spread = 0.95_f64.min(0.22 * k);
            let off = if k > 1.0 { -spread / 2.0 + spread * ti as f64 / (k - 1.0) } else { 0.0 };
            let ring = 140.0 + 38.0 * (ti % 2) as f64;
            let (x, y) = (hx + (a + off).cos() * ring * 1.25, hy + (a + off).sin() * ring * 0.9);
            let i = nodes.len();
            nodes.push(Node {
                label: tp.title.clone(),
                sub: String::new(),
                x: x.clamp(70.0, w - 70.0),
                y: y.clamp(40.0, h - 40.0),
                kind: Kind::Topic,
                open: Some(tp.id.clone()),
            });
            ids.push(tp.id.clone());
            index.insert(tp.id.as_str(), i);
            tree.push((hub, i));
        }
    }
    let mut links = vec![];
    for tp in &c.topics {
        for p in &tp.prereqs {
            if let (Some(&a), Some(&b)) = (index.get(p.as_str()), index.get(tp.id.as_str())) {
                links.push((a, b));
            }
        }
    }
    (Map { w, h, nodes, tree, links }, ids)
}

/// A topic's neighbourhood: prerequisites to the left, next steps to the right, related below.
fn topic_map(tp: &Topic) -> Map {
    let (w, h) = (1000.0, 360.0);
    let mut nodes = vec![Node { label: tp.title.clone(), sub: section_title(&tp.section), x: w / 2.0, y: 150.0, kind: Kind::Root, open: None }];
    let mut tree = vec![];
    let add = |ids: &[String], x: f64, y0: f64, dy: f64, nodes: &mut Vec<Node>, tree: &mut Vec<(usize, usize)>| {
        let known: Vec<&Topic> = ids.iter().filter_map(|i| topic(i)).collect();
        let n = known.len() as f64;
        for (k, t) in known.iter().enumerate() {
            let y = y0 + (k as f64 - (n - 1.0) / 2.0) * dy;
            let i = nodes.len();
            nodes.push(Node { label: t.title.clone(), sub: String::new(), x, y, kind: Kind::Topic, open: Some(t.id.clone()) });
            tree.push((0, i));
        }
    };
    add(&tp.prereqs, 160.0, 150.0, 74.0, &mut nodes, &mut tree);
    add(&tp.next, 840.0, 150.0, 74.0, &mut nodes, &mut tree);
    let related: Vec<String> = tp.related.clone();
    let n = related.iter().filter(|r| topic(r).is_some()).count() as f64;
    for (k, t) in related.iter().filter_map(|r| topic(r)).enumerate() {
        let x = w / 2.0 + (k as f64 - (n - 1.0) / 2.0) * 190.0;
        let i = nodes.len();
        nodes.push(Node { label: t.title.clone(), sub: String::new(), x, y: 300.0, kind: Kind::Topic, open: Some(t.id.clone()) });
        tree.push((0, i));
    }
    Map { w, h, nodes, tree, links: vec![] }
}

// ───────────────────────── Atlas screen ─────────────────────────

fn matches(t: &Topic, q: &str) -> bool {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let hay = format!("{} {} {} {}", t.title, t.kicker, t.summary, t.keywords.join(" ")).to_lowercase();
    q.split_whitespace().all(|w| hay.contains(w))
}

#[component]
pub fn Atlas() -> impl IntoView {
    let app = use_app();
    let c = content();
    let query = RwSignal::new(String::new());
    let (map, ids) = atlas_map();
    let dim = Signal::derive(move || {
        let q = query.get();
        ids.iter().map(|id| !q.trim().is_empty() && (id.is_empty() || topic(id).is_none_or(|t| !matches(t, &q)))).collect::<Vec<_>>()
    });
    let results = move || {
        let q = query.get();
        c.topics.iter().filter(|t| matches(t, &q)).collect::<Vec<_>>()
    };
    let sections = c
        .sections
        .iter()
        .map(|s| {
            let sid = s.id.clone();
            let cards = move || {
                let q = query.get();
                c.topics
                    .iter()
                    .filter(|t| t.section == sid && matches(t, &q))
                    .map(|t| {
                        let id = t.id.clone();
                        view! {
                            <button class="a-card" on:click=move |_| app.open_topic(&id)>
                                <span class="a-card-level">{level_text(t.level)}</span>
                                <span class="a-card-title">{t.title.clone()}</span>
                                <span class="a-card-kicker">{t.kicker.clone()}</span>
                            </button>
                        }
                    })
                    .collect_view()
            };
            let count = c.topics.iter().filter(|t| t.section == s.id).count();
            view! {
                <section class="a-section" data-reveal>
                    <header class="a-section-head">
                        <h2>{s.title.clone()}</h2>
                        <p>{s.blurb.clone()}</p>
                    </header>
                    <div class="a-cards">
                        {cards}
                        {(count == 0).then(|| view! { <p class="a-soon">"Pages for this section are being written."</p> })}
                    </div>
                </section>
            }
        })
        .collect_view();
    view! {
        <div class="atlas">
            <header class="a-hero">
                <p class="eyebrow">"The Atlas"</p>
                <h1 class="a-title">"Every idea in quantum computing, connected."</h1>
                <p class="a-lead">
                    {format!("{} topics so far, from what a qubit is to how quantum computers search. Every circuit on these pages is run by the simulator inside this app.", c.topics.len())}
                </p>
                <label class="a-search">
                    <span class="vh">"Search topics"</span>
                    <input
                        type="search"
                        placeholder="Search: entanglement, phase, Grover…"
                        prop:value=move || query.get()
                        on:input=move |e| query.set(event_target_value(&e))
                    />
                </label>
                <Show when=move || !query.get().trim().is_empty()>
                    <p class="a-count">
                        {move || match results().len() {
                            0 => "No page yet for that. Try a broader word.".to_string(),
                            1 => "1 page".to_string(),
                            n => format!("{n} pages"),
                        }}
                    </p>
                </Show>
            </header>
            <div class="a-map">
                <GrowMap map=map dim=dim class="atlas-map" />
            </div>
            <div class="a-sections">{sections}</div>
        </div>
    }
}

// ───────────────────────── topic page ─────────────────────────

/// Paragraphs from a text body: blank lines split paragraphs, *word* is emphasis.
fn paragraphs(body: &str) -> impl IntoView + use<> {
    body.split("\n\n")
        .map(|p| {
            let parts = p
                .split('*')
                .enumerate()
                .map(|(i, s)| if i % 2 == 1 { view! { <em>{s.to_string()}</em> }.into_any() } else { s.to_string().into_any() })
                .collect_view();
            view! { <p>{parts}</p> }
        })
        .collect_view()
}

#[component]
pub fn TopicPage() -> impl IntoView {
    let app = use_app();
    move || {
        let id = app.topic.get();
        let Some(tp) = topic(&id) else {
            return view! { <div class="tp"><p class="tp-missing">"That page does not exist yet."</p></div> }.into_any();
        };
        let blocks = tp.blocks.iter().map(render_block).collect_view();
        let next = tp
            .next
            .iter()
            .chain(&tp.related)
            .filter_map(|i| topic(i))
            .map(|t| {
                let id = t.id.clone();
                view! {
                    <button class="a-card" on:click=move |_| app.open_topic(&id)>
                        <span class="a-card-level">{level_text(t.level)}</span>
                        <span class="a-card-title">{t.title.clone()}</span>
                        <span class="a-card-kicker">{t.kicker.clone()}</span>
                    </button>
                }
            })
            .collect_view();
        let sources = tp.sources.iter().map(|s| view! { <li>{s.title.clone()}</li> }).collect_view();
        view! {
            <article class="tp">
                <nav class="tp-crumbs">
                    <button class="link" on:click=move |_| app.go(Screen::Atlas)>"Atlas"</button>
                    <span>" / " {section_title(&tp.section)}</span>
                </nav>
                <header class="tp-head">
                    <p class="eyebrow">{format!("{} · {}", section_title(&tp.section), level_text(tp.level))}</p>
                    <h1 class="tp-title">{tp.title.clone()}</h1>
                    <p class="tp-kicker">{tp.kicker.clone()}</p>
                </header>
                <div class="tp-map">
                    <GrowMap map=topic_map(tp) class="topic-map" />
                </div>
                <p class="tp-summary" data-reveal>{tp.summary.clone()}</p>
                <div class="tp-blocks">{blocks}</div>
                <section class="tp-next" data-reveal>
                    <h2>"Keep going"</h2>
                    <div class="a-cards">{next}</div>
                </section>
                {(!tp.sources.is_empty()).then(|| view! {
                    <section class="tp-sources">
                        <h2>"Sources"</h2>
                        <ul>{sources}</ul>
                    </section>
                })}
            </article>
        }
        .into_any()
    }
}

fn render_block(b: &Block) -> AnyView {
    match b.clone() {
        Block::Text { heading, body } => view! {
            <section class="tb tb-text" data-reveal>
                {heading.map(|h| view! { <h2>{h}</h2> })}
                {paragraphs(&body)}
            </section>
        }
        .into_any(),
        Block::Math { expr, caption } => view! {
            <figure class="tb tb-math" data-reveal>
                <div class="tb-expr ket">{expr}</div>
                {caption.map(|c| view! { <figcaption>{c}</figcaption> })}
            </figure>
        }
        .into_any(),
        Block::Circuit { title, circuit, expect, caption } => view! { <CircuitBlock title=title circuit=circuit expect=expect.probs qubits=expect.qubits caption=caption /> }.into_any(),
        Block::Misconception { myth, truth } => view! {
            <aside class="tb tb-myth" data-reveal>
                <p class="tb-myth-label">"Common belief"</p>
                <p class="tb-myth-text">{myth}</p>
                <p class="tb-myth-label truth">"Actually"</p>
                <p>{truth}</p>
            </aside>
        }
        .into_any(),
        Block::Practice { question, answer, hint } => view! {
            <section class="tb tb-practice" data-reveal>
                <p class="tb-myth-label">"Try it"</p>
                <p class="tb-q">{question}</p>
                {hint.map(|h| view! { <details class="reveal"><summary>"Hint"</summary><p>{h}</p></details> })}
                <details class="reveal"><summary>"Answer"</summary><p>{answer}</p></details>
            </section>
        }
        .into_any(),
        Block::Note { title, body } => view! {
            <aside class="tb tb-note" data-reveal>
                <h3>{title}</h3>
                {paragraphs(&body)}
            </aside>
        }
        .into_any(),
    }
}

// ───────────────────────── interactive circuit ─────────────────────────

const COL: f64 = 96.0;
const ROW: f64 = 70.0;
const LEFT: f64 = 70.0;
const TOP: f64 = 40.0;

fn angle_label(op: &Op) -> String {
    op.theta.as_ref().and_then(|a| a.radians().ok()).map(circuit::phase_text).unwrap_or_default()
}

fn base_name(op: &Op) -> String {
    let g = op.g.to_uppercase();
    let stripped = g.trim_start_matches('C');
    let base = match g.as_str() {
        "CNOT" | "TOFFOLI" => "X",
        "FREDKIN" => "SWAP",
        _ if stripped.is_empty() => g.as_str(),
        _ if op.c.is_empty() => g.as_str(),
        _ => stripped,
    };
    match base {
        "SDG" => "S†".into(),
        "TDG" => "T†".into(),
        b => b.to_string(),
    }
}

/// One drawn gate (a column of the circuit).
fn gate_view(op: &Op, col: usize, n: usize) -> AnyView {
    let x = LEFT + COL * col as f64 + COL / 2.0;
    let y = |q: usize| TOP + ROW * q as f64;
    let base = base_name(op);
    let ang = angle_label(op);
    let mut qs: Vec<usize> = op.c.iter().chain(&op.t).copied().collect();
    qs.sort();
    let wire_line = (qs.len() > 1).then(|| {
        let (a, b) = (y(*qs.first().unwrap()), y(*qs.last().unwrap()));
        view! { <line class="cg-link" x1=x y1=a x2=x y2=b></line> }
    });
    let controls = op.c.iter().map(|&q| view! { <circle class="cg-ctrl" cx=x cy=y(q) r=6></circle> }).collect_view();
    let targets = op
        .t
        .iter()
        .map(|&q| {
            let cy = y(q);
            if base == "X" && !op.c.is_empty() {
                view! {
                    <g class="cg-plus">
                        <circle cx=x cy=cy r=16></circle>
                        <line x1=x - 16.0 y1=cy x2=x + 16.0 y2=cy></line>
                        <line x1=x y1=cy - 16.0 x2=x y2=cy + 16.0></line>
                    </g>
                }
                .into_any()
            } else if base == "Z" && !op.c.is_empty() {
                view! { <circle class="cg-ctrl" cx=x cy=cy r=6></circle> }.into_any()
            } else if base == "SWAP" {
                view! {
                    <g class="cg-swap">
                        <line x1=x - 9.0 y1=cy - 9.0 x2=x + 9.0 y2=cy + 9.0></line>
                        <line x1=x - 9.0 y1=cy + 9.0 x2=x + 9.0 y2=cy - 9.0></line>
                    </g>
                }
                .into_any()
            } else {
                let wide = !ang.is_empty();
                let w = if wide { 64.0 } else { 44.0 };
                view! {
                    <g class="cg-box">
                        <rect x=x - w / 2.0 y=cy - 22.0 width=w height=44 rx=6></rect>
                        <text x=x y=cy + if wide { -3.0 } else { 0.0 } dy="0.36em">{base.clone()}</text>
                        {wide.then(|| view! { <text class="cg-ang" x=x y=cy + 13.0 dy="0.36em">{ang.clone()}</text> })}
                    </g>
                }
                .into_any()
            }
        })
        .collect_view();
    let _ = n;
    view! { <g class="cg">{wire_line}{controls}{targets}</g> }.into_any()
}

/// A circuit that assembles itself gate by gate as it scrolls into view, then can be stepped through.
/// Every stated outcome is checked against the simulator by the content test; the numbers shown
/// here are computed live by the same simulator.
#[component]
fn CircuitBlock(title: String, circuit: Circuit, expect: BTreeMap<String, f64>, qubits: Option<Vec<usize>>, caption: Option<String>) -> impl IntoView {
    let run = circuit::run(&circuit);
    let Ok(run) = run else {
        return view! { <p class="tb tb-error">"This circuit could not be simulated."</p> }.into_any();
    };
    let n = circuit.qubits;
    let steps = circuit.ops.len();
    let k = RwSignal::new(steps); // start on the final state
    let playing = RwSignal::new(false);
    let w = LEFT + COL * steps as f64 + 40.0;
    let h = TOP * 2.0 + ROW * (n.saturating_sub(1)) as f64;
    let init = circuit.init.clone().unwrap_or_else(|| "0".repeat(n));
    let wires = (0..n)
        .map(|q| {
            let yy = TOP + ROW * q as f64;
            let bit = init.chars().nth(q).unwrap_or('0');
            view! {
                <line class="cg-wire" x1=LEFT - 16.0 y1=yy x2=w - 12.0 y2=yy pathLength="1"></line>
                <text class="cg-wlabel" x=6 y=yy dy="0.35em">{format!("q{q} |{bit}⟩")}</text>
            }
        })
        .collect_view();
    // Each gate builds in as the block scrolls up the window: gate i over its own slice of the range.
    let gates = circuit
        .ops
        .iter()
        .enumerate()
        .map(|(i, op)| {
            let span = 30.0 / (steps.max(1) as f64);
            let (a, b) = (8.0 + span * i as f64, 8.0 + span * (i as f64 + 1.6));
            view! {
                <g class="cg-col" style=format!("animation-range: entry {a:.1}% cover {b:.1}%") data-on=move || (k.get() == i + 1).to_string()>
                    {gate_view(op, i, n)}
                </g>
            }
        })
        .collect_view();
    let states = StoredValue::new(run.states.clone());
    let state = Signal::derive(move || states.with_value(|s| s[k.get().min(s.len() - 1)].clone()));
    let notes: Vec<String> = circuit.ops.iter().map(|o| o.note.clone().unwrap_or_default()).collect();
    let note = move || {
        let i = k.get();
        if i == 0 { format!("Start: |{}⟩.", init_label(&circuit_init(n, &states))) } else { notes.get(i - 1).cloned().filter(|s| !s.is_empty()).unwrap_or_else(|| format!("After step {i}.")) }
    };
    let ket = move || circuit::ket(&state.get(), n);
    // The checked outcome, recomputed live.
    let last = run.states.last().cloned().unwrap_or_default();
    let got: BTreeMap<String, f64> = match &qubits {
        Some(keep) => circuit::marginal(&last, n, keep),
        None => qcore::basis(n).into_iter().zip(circuit::probs(&last)).collect(),
    };
    let checks = got
        .iter()
        .filter(|(_, p)| **p > 1e-9)
        .map(|(key, p)| {
            let claimed = expect.get(key).copied().unwrap_or(0.0);
            let ok = (claimed - p).abs() <= qcore::topic::TOL;
            view! { <span class="cb-chip" data-ok=ok.to_string()>{format!("P({key}) = {}", nice(*p))}</span> }
        })
        .collect_view();
    let over = qubits.as_ref().map(|q| format!(" for qubit{} {}", if q.len() > 1 { "s" } else { "" }, q.iter().map(|x| format!("q{x}")).collect::<Vec<_>>().join(", "))).unwrap_or_default();
    let play = move |_| {
        if playing.get_untracked() {
            playing.set(false);
            return;
        }
        playing.set(true);
        k.set(0);
        fn tick(k: RwSignal<usize>, playing: RwSignal<bool>, steps: usize) {
            set_timeout(
                move || {
                    if !playing.get_untracked() {
                        return;
                    }
                    let next = k.get_untracked() + 1;
                    k.set(next.min(steps));
                    if next < steps {
                        tick(k, playing, steps);
                    } else {
                        playing.set(false);
                    }
                },
                Duration::from_millis(1700),
            );
        }
        tick(k, playing, steps);
    };
    let ticks = (0..=steps)
        .map(|i| {
            view! {
                <li>
                    <button class="cb-tick" aria-current=move || if k.get() == i { "step" } else { "false" } on:click=move |_| { playing.set(false); k.set(i); }>
                        <span class="vh">{format!("Step {i}")}</span>
                    </button>
                </li>
            }
        })
        .collect_view();
    view! {
        <section class="tb cb" data-reveal>
            <header class="cb-head">
                <h3>{title}</h3>
                <p class="cb-checked">{format!("Checked by the simulator{over}")}</p>
            </header>
            <div class="cb-body">
                <div class="cb-circuit">
                    <svg class="cg-svg" viewBox=format!("0 0 {w} {h}") role="img" aria-label="Circuit diagram">
                        <g class="cg-wires">{wires}</g>
                        {gates}
                    </svg>
                </div>
                <div class="cb-state">
                    <StateView state=state n=n />
                    <p class="cb-ket ket">{ket}</p>
                </div>
            </div>
            <div class="cb-controls">
                <button class="btn ghost" on:click=play>{move || if playing.get() { "Pause" } else { "Play" }}</button>
                <button class="btn ghost" disabled=move || k.get() == 0 on:click=move |_| { playing.set(false); k.update(|v| *v = v.saturating_sub(1)); }>"Back"</button>
                <ol class="cb-ticks">{ticks}</ol>
                <button class="btn primary" disabled={move || k.get() >= steps} on:click=move |_| { playing.set(false); k.update(|v| *v = (*v + 1).min(steps)); }>"Next gate"</button>
            </div>
            <p class="cb-note">{note}</p>
            <div class="cb-result">{checks}</div>
            {caption.map(|c| view! { <p class="cb-caption">{c}</p> })}
        </section>
    }
    .into_any()
}

fn circuit_init(n: usize, states: &StoredValue<Vec<Vec<C>>>) -> Vec<C> {
    let _ = n;
    states.with_value(|s| s[0].clone())
}

fn init_label(s: &[C]) -> String {
    let n = s.len().trailing_zeros() as usize;
    let i = s.iter().position(|a| a.norm2() > 0.5).unwrap_or(0);
    format!("{i:0n$b}")
}

fn nice(p: f64) -> String {
    for (v, s) in [(1.0, "1"), (0.5, "½"), (0.25, "¼"), (0.75, "¾"), (0.125, "⅛")] {
        if (p - v).abs() < 1e-9 {
            return s.into();
        }
    }
    format!("{p:.3}")
}

/// A complex state in circle notation: radius |a|, phase line at arg(a) (counterclockwise).
#[component]
pub fn StateView(#[prop(into)] state: Signal<Vec<C>>, n: usize) -> impl IntoView {
    let labels = qcore::basis(n);
    let len = labels.len();
    let discs = labels
        .into_iter()
        .enumerate()
        .map(|(i, key)| {
            let amp = move || state.with(|s| s.get(i).copied().unwrap_or(C::ZERO));
            let mag = Signal::derive(move || amp().abs());
            // CSS rotate is clockwise, phase turns counterclockwise.
            let phase = Signal::derive(move || {
                let a = amp();
                if a.norm2() < 1e-12 { 0.0 } else { -a.arg().to_degrees() }
            });
            view! { <QubitDisc key_label=key mag=mag phase=phase /> }
        })
        .collect_view();
    view! {
        <figure class="amps stateview" data-grid=(len == 4).to_string() style=format!("--n:{len}") role="img" aria-label=move || format!("State {}", circuit::ket(&state.get(), n))>
            {discs}
        </figure>
    }
}
