//! Tutor: ask a question, watch the four stages run, read the checked explanation, practise.

use crate::state::{STAGES, Stage, use_app};
use crate::KeyForm;
use crate::viz::{Amps, Compare};
use leptos::prelude::*;
use qcore::Probs;
use serde_json::Value;

const EXAMPLES: [&str; 5] = [
    "How do you create a Bell state?",
    "Walk me through Deutsch-Jozsa for a balanced function on 2 input bits.",
    "Grover gives exponential speedup over classical search, right? Explain how it works with 4 items.",
    "Explain Grover search on 2 qubits when the marked item is 01.",
    "Grover algorithm kaise kaam karta hai? simple mein samjhao",
];

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn probs(v: Option<&Value>) -> Probs {
    v.and_then(Value::as_object)
        .map(|m| m.iter().filter_map(|(k, x)| x.as_f64().map(|p| (k.clone(), p))).collect())
        .unwrap_or_default()
}

#[component]
pub fn Tutor() -> impl IntoView {
    let app = use_app();
    let question = RwSignal::new(String::new());
    let level = RwSignal::new("beginner");
    let run = app.run;
    let run_id = Memo::new(move |_| run.with(|r| r.id));

    let can_ask = move || {
        app.status.with(|s| s.has_key) && !question.with(|q| q.trim().is_empty()) && !run.with(|r| r.running)
    };
    let submit = move || {
        if can_ask() {
            app.ask(question.get_untracked().trim().to_string(), level.get_untracked());
        }
    };

    view! {
        <section class="screen tutor" aria-labelledby="tutor-title">
            <aside class="composer">
                <h1 id="tutor-title">"Tutor"</h1>
                <p class="sub">"Ask about the Bell state, Deutsch–Jozsa or Grover search. The final probabilities in every answer are checked before you read them."</p>

                <form
                    class="ask"
                    on:submit=move |e| {
                        e.prevent_default();
                        submit();
                    }
                >
                    <label class="field-label" for="q">"Your question"</label>
                    <textarea
                        id="q"
                        rows="3"
                        placeholder="Why does Grover need only one query for four items?"
                        prop:value=move || question.get()
                        on:input=move |e| question.set(event_target_value(&e))
                        on:keydown=move |e: web_sys::KeyboardEvent| {
                            if e.key() == "Enter" && (e.ctrl_key() || e.meta_key()) {
                                e.prevent_default();
                                submit();
                            }
                        }
                    ></textarea>
                    <div class="ask-row">
                        <div class="segs small" role="group" aria-label="Level">
                            {["beginner", "intermediate"]
                                .into_iter()
                                .map(|l| {
                                    view! {
                                        <button
                                            type="button"
                                            class="seg"
                                            aria-pressed=move || (level.get() == l).to_string()
                                            on:click=move |_| level.set(l)
                                        >
                                            {if l == "beginner" { "Beginner" } else { "Intermediate" }}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <button class="btn primary" type="submit" disabled=move || !can_ask()>
                            {move || if run.with(|r| r.running) { "Working…" } else { "Explain it" }}
                        </button>
                    </div>
                </form>
                // First run on a new PC: connect a key right here, no settings hunt.
                <Show when=move || app.status.with(|s| (s.known && !s.has_key) || s.just_saved)>
                    <section class="connect" aria-labelledby="connect-title">
                        <h2 id="connect-title">"Connect an AI provider"</h2>
                        <KeyForm id="key-inline" />
                        <p class="hint-line">"The Lab works without a key."</p>
                    </section>
                </Show>

                <div class="examples">
                    <h2 class="small-head">"Try one of these"</h2>
                    <ul>
                        {EXAMPLES
                            .into_iter()
                            .map(|q| {
                                view! {
                                    <li>
                                        <button class="example" on:click=move |_| question.set(q.to_string())>{q}</button>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                </div>

                <Pipeline />
            </aside>

            <div class="result">
                // Re-mount only when a new question starts (the id changes), never on each host event,
                // so the chosen step and the open hint survive while later stages arrive.
                {move || {
                    let id = run_id.get();
                    if id == 0 { view! { <EmptyResult /> }.into_any() } else { view! { <ResultView /> }.into_any() }
                }}
            </div>
        </section>
    }
}

#[component]
fn Pipeline() -> impl IntoView {
    let run = use_app().run;
    view! {
        <ol class="pipeline" aria-label="Progress">
            {STAGES
                .iter()
                .enumerate()
                .map(|(i, (_, title, desc))| {
                    let state = move || match run.with(|r| r.stages[i]) {
                        Stage::Waiting => "waiting",
                        Stage::Active => "active",
                        Stage::Done => "done",
                        Stage::Skipped => "skipped",
                        Stage::Failed => "failed",
                    };
                    let spoken = move || match state() {
                        "active" => ": in progress",
                        "done" => ": done",
                        "skipped" => ": skipped",
                        "failed" => ": failed",
                        _ => "",
                    };
                    view! {
                        <li class="pl-stage" data-state=state>
                            <span class="pl-dot" aria-hidden="true"></span>
                            <span class="pl-title">{*title} <span class="vh">{spoken}</span></span>
                            <span class="pl-desc">{*desc}</span>
                        </li>
                    }
                })
                .collect_view()}
            <li class="pl-notice" role="status">{move || run.with(|r| r.notice.clone().unwrap_or_default())}</li>
        </ol>
    }
}

#[component]
fn EmptyResult() -> impl IntoView {
    let bell = qcore::trace(&qcore::Algo::Bell);
    let last = bell.steps.last().unwrap().amps.clone();
    view! {
        <div class="empty">
            <h2 class="empty-head">"No wrong answer reaches you unflagged."</h2>
            <p class="lead">
                "After the AI explains, the simulator inside this app recomputes the final probabilities. A wrong number sends the explanation back once with the correct result; if it is still wrong, you are told."
            </p>
            <div class="empty-demo">
                <Amps amps=Signal::stored(last) basis=Signal::stored(bell.basis) />
                <p class="detail">"The Bell state the simulator expects: |00⟩ and |11⟩ at ½ each."</p>
            </div>
        </div>
    }
}

#[component]
fn ResultView() -> impl IntoView {
    let run = use_app().run;
    let step = RwSignal::new(0usize);
    let app = use_app();
    Effect::new(move |prev: Option<u32>| {
        let (k, n) = app.key.get();
        let len = run.with_untracked(|r| {
            r.explanation.as_ref().and_then(|e| e.get("steps")).and_then(Value::as_array).map_or(0, Vec::len)
        });
        if prev.is_some_and(|p| p != n) && len > 0 {
            match k.as_str() {
                "ArrowRight" => step.update(|s| *s = (*s + 1).min(len - 1)),
                "ArrowLeft" => step.update(|s| *s = s.saturating_sub(1)),
                _ => {}
            }
        }
        n
    });

    // Each part reads only its own slice of the run, so a new event re-renders just what changed.
    let head_m = Memo::new(move |_| run.with(|r| (r.question.clone(), r.level.clone(), r.route.clone())));
    let verdict_m = Memo::new(move |_| {
        run.with(|r| (r.error.clone(), r.final_msg.clone(), r.verification.clone(), r.repaired.clone(), r.running, r.notice.clone()))
    });
    let expl_m = Memo::new(move |_| run.with(|r| r.explanation.clone()));
    let algo_m = Memo::new(move |_| run.with(|r| r.algo.clone()));
    let practice_m = Memo::new(move |_| run.with(|r| r.practice.clone()));
    let footer_m = Memo::new(move |_| run.with(|r| (r.running, r.models.clone(), r.seconds)));

    let header = move || {
        let (question, level, route) = head_m.get();
        let route = route.unwrap_or(Value::Null);
        let algo = text(&route, "algorithm").replace('_', " ");
        let mut tags = vec![];
        if !algo.is_empty() {
            tags.push(algo);
        }
        if let Some(i) = route.get("intent").and_then(Value::as_str).filter(|i| *i != "explain") {
            tags.push(i.to_string());
        }
        if let Some(l) = route.get("language").and_then(Value::as_str).filter(|l| *l != "english") {
            tags.push(format!("asked in {l}"));
        }
        if route.get("injection_attempt").and_then(Value::as_bool) == Some(true) {
            tags.push("instructions in the question were ignored".into());
        }
        view! {
            <header class="q-head">
                <p class="q-text">{question}</p>
                <p class="q-meta">{tags.join(" · ")} {(!tags.is_empty()).then_some(" · ")} {level}</p>
            </header>
        }
    };

    let verdict = move || {
        let (error, final_msg, verification, repaired, running, notice) = verdict_m.get();
        if let Some(e) = error {
            return view! { <div class="verdict" data-kind="error"><h2>"Something went wrong"</h2><p>{e}</p></div> }.into_any();
        }
        if let Some(f) = final_msg {
            return view! { <div class="verdict" data-kind="info"><h2>"Not a quantum-algorithm question"</h2><p>{f}</p></div> }.into_any();
        }
        let Some(v) = verification else {
            // Retry and fallback notices (e.g. "model busy; retrying in 10s") show here, where they are seen.
            let msg = match (running, notice) {
                (true, Some(n)) => n,
                (true, None) => "Checking will start once the explanation arrives.".to_string(),
                _ => String::new(),
            };
            return view! { <div class="verdict" data-kind="pending"><h2>"Waiting for the explanation"</h2><p>{msg}</p></div> }.into_any();
        };
        match v.get("ok").and_then(Value::as_bool) {
            None => view! {
                <div class="verdict" data-kind="unverified">
                    <h2>"Explained, not verified"</h2>
                    <p>"The simulator covers the Bell state, Deutsch–Jozsa and 2-qubit Grover. Treat the numbers below with care."</p>
                </div>
            }
            .into_any(),
            Some(ok) => {
                let truth = probs(v.get("truth"));
                let claimed = probs(v.get("claimed"));
                let (title, body) = match (ok, repaired) {
                    (true, None) => ("Verified by the simulator".to_string(), "Every final probability in the explanation matches the simulation within 0.02.".to_string()),
                    (true, Some(m)) => ("Corrected, then verified".to_string(), format!("The first explanation was wrong ({}). It was sent back once with the simulator's result and now matches.", m.join("; "))),
                    (false, _) => ("The numbers do not match".to_string(), format!("Even after one correction: {}. Trust the simulated column.", v.get("mismatches").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; ")).unwrap_or_default())),
                };
                view! {
                    <div class="verdict" data-kind=if ok { "ok" } else { "bad" }>
                        <h2 role="status">{title}</h2>
                        <p>{body}</p>
                        <Compare claimed=claimed truth=truth />
                    </div>
                }
                .into_any()
            }
        }
    };

    let explanation = move || {
        let Some(e) = expl_m.get() else {
            return view! { <div class="steps loading" aria-busy="true"><div class="skel"></div><div class="skel"></div><div class="skel short"></div></div> }.into_any();
        };
        if let Value::String(raw) = &e {
            return view! { <div class="steps"><p class="detail">{raw.clone()}</p></div> }.into_any();
        }
        let steps: Vec<Value> = e.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
        let total = steps.len();
        let goal = text(&e, "goal");
        let final_result = text(&e, "final_result");
        let analogy = text(&e, "analogy");
        let misconception = text(&e, "misconception");
        let uncertain: Vec<String> = e.get("uncertain").and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        let steps_view = steps
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let action = text(&s, "action");
                let state = s.get("state").map(|x| x.as_str().map(String::from).unwrap_or_else(|| x.to_string())).unwrap_or_default();
                view! {
                    <li class="step" aria-current=move || if step.get() == i { "step" } else { "false" } on:click=move |_| step.set(i)>
                        <p class="step-action">{action}</p>
                        <p class="ket">{state}</p>
                    </li>
                }
            })
            .collect_view();
        view! {
            <div class="steps">
                <h2 class="goal">{goal}</h2>
                <ol class="step-list">{steps_view}</ol>
                <div class="step-nav">
                    <button class="btn ghost" on:click=move |_| step.update(|s| *s = s.saturating_sub(1)) disabled=move || step.get() == 0>"Back"</button>
                    <span class="step-count">{move || format!("{} of {total}", (step.get() + 1).min(total.max(1)))}</span>
                    <button class="btn ghost" on:click=move |_| step.update(|s| *s = (*s + 1).min(total.saturating_sub(1))) disabled=move || { step.get() + 1 >= total }>"Next"</button>
                </div>
                <p class="final">{final_result}</p>
                {(!analogy.is_empty()).then(|| view! { <p class="aside-line"><strong>"Picture it: "</strong>{analogy.clone()}</p> })}
                {(!misconception.is_empty()).then(|| view! { <p class="aside-line"><strong>"Common mistake: "</strong>{misconception.clone()}</p> })}
                {(!uncertain.is_empty()).then(|| view! { <p class="aside-line"><strong>"The tutor was unsure about: "</strong>{uncertain.join("; ")}</p> })}
            </div>
        }
        .into_any()
    };

    let state_view = move || {
        algo_m.get().map(|a| {
            let t = qcore::trace(&a);
            let last = t.steps.last().unwrap().amps.clone();
            view! {
                <div class="panel sim-state">
                    <h3 class="small-head">"Simulated final state"</h3>
                    <p class="ket">{t.steps.last().unwrap().ket.clone()}</p>
                    <Amps amps=Signal::stored(last) basis=Signal::stored(t.basis) />
                </div>
            }
        })
    };

    let practice = move || {
        practice_m.get().filter(|p| p.is_object()).map(|p| {
            view! {
                <section class="practice" aria-labelledby="practice-title">
                    <h2 id="practice-title">"Practise"</h2>
                    <p class="practice-q">{text(&p, "question")}</p>
                    <details class="reveal">
                        <summary>"Show a hint"</summary>
                        <p>{text(&p, "hint")}</p>
                    </details>
                    <details class="reveal">
                        <summary>"Show the answer"</summary>
                        <p>{text(&p, "answer_key")}</p>
                    </details>
                </section>
            }
        })
    };

    let footer = move || {
        let (running, models, seconds) = footer_m.get();
        (!running && !models.is_empty()).then(|| view! { <p class="meta-line">{format!("Answered by {models} in {seconds:.1} s")}</p> })
    };

    view! {
        <article class="result-body">
            {header}
            {verdict}
            <div class="result-grid">
                {explanation}
                <div class="result-side">
                    {state_view}
                    {practice}
                </div>
            </div>
            {footer}
        </article>
    }
}
