//! P1 router → P2 explainer → P3 claim extractor + simulator check (one repair retry) → P4 practice.
//! Same control flow as `run()` in code/workflow.py; every stage is streamed to the UI as an event.

use crate::gemini::Gemini;
use crate::prompts::{Prompts, as_text, fill, parse_json, py_dumps, truthy};
use serde_json::{Map, Value, json};
use std::time::Instant;

const OUT_OF_SCOPE: &str =
    "I can help with quantum algorithms (Bell states, Deutsch-Jozsa, Grover). Please ask a question about one of them.";

pub fn run(
    gemini: &mut Gemini,
    prompts: &Prompts,
    id: u64,
    question: &str,
    level: &str,
    emit: &mut dyn FnMut(Value),
) {
    let t0 = Instant::now();
    let mut ev = |kind: &str, mut body: Value| {
        body["type"] = json!(kind);
        body["id"] = json!(id);
        emit(body);
    };
    // Retry / fallback notices go to the UI as small toasts.
    let result = (|| -> Result<Option<String>, String> {
        // ── P1 router ──
        ev("stage", json!({"stage": "route", "state": "active"}));
        let (raw, t) = gemini.call(
            &prompts.router,
            &fill(&prompts.router.user, &[("question", question), ("level", level)]),
            &mut |m| ev_notice(&mut ev, m),
        )?;
        let (route, clean) = parse_json(&raw);
        // Same test as workflow.py's `if not route`: an empty object counts as no answer.
        let Some(Value::Object(route)) = route.filter(truthy) else {
            ev("stage", json!({"stage": "route", "state": "error"}));
            return Ok(Some("The router's answer was not valid JSON. Please try again.".into()));
        };
        let in_scope = route.get("in_scope").is_some_and(truthy);
        // workflow.py passes a missing algorithm through json.dumps → "null".
        let algo_name = route.get("algorithm").map_or_else(|| "null".to_string(), as_text);
        let algo = if in_scope { qcore::norm_params(&algo_name, route.get("params").unwrap_or(&Value::Null)) } else { None };
        let params = algo.as_ref().map_or_else(|| json!({}), qcore::Algo::params_json);
        ev(
            "route",
            json!({"data": Value::Object(route.clone()), "params": params, "verifiable": algo.is_some(), "seconds": t, "clean": clean}),
        );
        ev("stage", json!({"stage": "route", "state": "done"}));
        if !in_scope {
            for s in ["explain", "verify", "practice"] {
                ev("stage", json!({"stage": s, "state": "skipped"}));
            }
            return Ok(Some(OUT_OF_SCOPE.into()));
        }

        let text_of = |m: &Map<String, Value>, k: &str, default: &str| -> String {
            m.get(k).filter(|v| truthy(v)).map(as_text).unwrap_or_else(|| default.into())
        };
        let false_premise = text_of(&route, "false_premise", "none");
        let intent = text_of(&route, "intent", "explain");
        let params_s = py_dumps(&params);

        // ── P2 explainer + P3 claims + simulator, with one repair retry ──
        let mut feedback = String::new();
        let mut explanation = Value::Null;
        let mut truth: Option<Value> = None;
        for attempt in 0..2 {
            ev("stage", json!({"stage": "explain", "state": "active", "attempt": attempt}));
            let user = fill(
                &prompts.explainer.user,
                &[
                    ("algorithm", &algo_name),
                    ("params", &params_s),
                    ("level", level),
                    ("false_premise", &false_premise),
                    ("intent", &intent),
                    ("question", question),
                    ("verifier_feedback", &feedback),
                ],
            );
            let (raw, t) = gemini.call(&prompts.explainer, &user, &mut |m| ev_notice(&mut ev, m))?;
            let (expl, clean) = parse_json(&raw);
            let expl = expl.filter(truthy);
            explanation = expl.clone().unwrap_or_else(|| Value::String(raw.clone()));
            ev("explanation", json!({"attempt": attempt, "data": explanation, "clean": clean, "seconds": t}));
            ev("stage", json!({"stage": "explain", "state": "done", "attempt": attempt}));

            let Some(algo) = &algo else {
                ev("verification", json!({"attempt": attempt, "data": {"ok": null, "note": "Not checkable by the simulator: shown as unverified."}}));
                ev("stage", json!({"stage": "verify", "state": "skipped"}));
                break;
            };
            ev("stage", json!({"stage": "verify", "state": "active", "attempt": attempt}));
            let expl_text = expl.as_ref().map_or_else(|| raw.clone(), py_dumps);
            let user3 = fill(&prompts.claims.user, &[("algorithm", &algo_name), ("params", &params_s), ("explanation", &expl_text)]);
            let (raw3, _) = gemini.call(&prompts.claims, &user3, &mut |m| ev_notice(&mut ev, m))?;
            let (claim, _) = parse_json(&raw3);
            // Ground truth always comes from the router's normalised params, never from P3's echo.
            let probs = claim.as_ref().and_then(|c| c.get("probabilities")).cloned().unwrap_or(Value::Null);
            let ver = qcore::verify(algo, &probs, 0.02);
            truth = Some(qcore::probs_json(&ver.truth));
            ev("verification", json!({"attempt": attempt, "data": ver, "trace_algo": algo.key(), "params": params}));
            ev("stage", json!({"stage": "verify", "state": if ver.ok { "done" } else { "error" }, "attempt": attempt}));
            if ver.ok {
                break;
            }
            feedback = format!(
                "Verifier feedback: a simulator found these errors in your previous answer: {}. Correct result: {}. Fix the explanation.",
                ver.mismatches.join("; "),
                qcore::py_repr(&ver.truth)
            );
            if attempt == 0 {
                ev("repair", json!({"mismatches": ver.mismatches}));
            }
        }

        // ── P4 practice ──
        ev("stage", json!({"stage": "practice", "state": "active"}));
        let truth_s = truth.as_ref().map_or_else(|| "not available".to_string(), py_dumps);
        let user4 = fill(
            &prompts.practice.user,
            &[
                ("algorithm", &algo_name),
                ("params", &params_s),
                ("level", level),
                ("truth", &truth_s),
                ("explanation", &py_dumps(&explanation)),
            ],
        );
        let (raw4, _) = gemini.call(&prompts.practice, &user4, &mut |m| ev_notice(&mut ev, m))?;
        let (practice, _) = parse_json(&raw4);
        ev("practice", json!({"data": practice.unwrap_or(Value::Null)}));
        ev("stage", json!({"stage": "practice", "state": "done"}));
        Ok(None)
    })();

    let seconds = (t0.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    match result {
        Ok(final_msg) => ev("done", json!({"final": final_msg, "models": gemini.models_used(), "seconds": seconds})),
        Err(e) => ev("error", json!({"message": e, "models": gemini.models_used()})),
    }
}

fn ev_notice(ev: &mut impl FnMut(&str, Value), message: String) {
    ev("notice", json!({"message": message}));
}
