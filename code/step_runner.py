"""Run the pipeline with an external LLM (a fresh Claude subagent per call) instead of the Gemini API.

The code here does everything that is not an LLM call: filling prompt templates, parameter
normalisation, the simulator check, the repair decision and logging. Each LLM call is
answered by a separate model instance that sees only that one prompt.

Jobs:
  T<id>              full pipeline on test case <id> from test_cases.csv (e.g. T3)
  V<ver>_<id>_<rep>  explainer version check: v1/v2/v3 explainer + P3 + simulator (e.g. Vv2_5_0)

Commands:
  py step_runner.py next <job>                 print the next prompt, or DONE
  py step_runner.py answer <job> --file <path> record the model's raw answer and advance
  py step_runner.py finalize <label>           build run_claude_*.jsonl, test table and versions json
"""
from __future__ import annotations

import csv
import json
import sys
from datetime import datetime
from pathlib import Path

from quantum_sim import norm_params, verify_claim
from workflow import FINAL, PROMPTS, ROOT, VERIFIABLE, fill, load_prompt, parse_json

STATE = ROOT / "test-data" / "outputs" / "claude_state"
OUT = ROOT / "test-data" / "outputs"
ROWS = {r["id"]: r for r in csv.DictReader((ROOT / "test-data" / "test_cases.csv").open(encoding="utf-8"))}
VERSION_FILES = {v: PROMPTS / v / f"explainer_{v}.txt" for v in ("v1", "v2", "v3")}
VERSION_CASES = {"1": ("grover2", {"marked": "11"}), "3": ("deutsch_jozsa", {"n": 2, "f_kind": "balanced"}),
                 "5": ("bell", {}), "7": ("grover2", {"marked": "11"})}


def path(job: str) -> Path:
    return STATE / f"{job}.json"


def load(job: str) -> dict:
    if path(job).exists():
        return json.loads(path(job).read_text(encoding="utf-8"))
    if job.startswith("T"):
        row = ROWS[job[1:]]
        return {"job": job, "kind": "test", "test_id": row["id"], "step": "P1", "attempt": 0, "feedback": "",
                "log": {"question": row["question"], "level": row["level"]}, "calls": []}
    ver, tid, rep = job[1:].split("_")
    algo, params = VERSION_CASES[tid]
    return {"job": job, "kind": "version", "version": ver, "test_id": tid, "rep": int(rep), "step": "EXPL",
            "algo": algo, "params": params, "log": {}, "calls": []}


def save(st: dict) -> None:
    STATE.mkdir(parents=True, exist_ok=True)
    path(st["job"]).write_text(json.dumps(st, indent=2, ensure_ascii=False), encoding="utf-8")


def current_prompt(st: dict) -> tuple[dict, dict] | None:
    """Return (prompt, variables) for the step the job is on, or None when done."""
    lg, step = st["log"], st["step"]
    if step == "done":
        return None
    if st["kind"] == "version":
        row = ROWS[st["test_id"]]
        if step == "EXPL":
            return load_prompt(VERSION_FILES[st["version"]]), dict(
                question=row["question"], level=row["level"], algorithm=st["algo"], params=st["params"],
                false_premise="none", intent="explain", verifier_feedback="")
        return FINAL["P3_claim_extractor"], dict(algorithm=st["algo"], params=st["params"],
                                                 explanation=lg["explanation_raw"])
    route = lg.get("route") or {}
    if step == "P1":
        return FINAL["P1_router"], dict(question=lg["question"], level=lg["level"])
    if step == "P2":
        return FINAL["P2_explainer"], dict(
            algorithm=route.get("algorithm"), params=lg["params_used"], level=lg["level"],
            false_premise=route.get("false_premise") or "none", intent=route.get("intent") or "explain",
            question=lg["question"], verifier_feedback=st["feedback"])
    if step == "P3":
        expl = lg.get("explanation")
        return FINAL["P3_claim_extractor"], dict(algorithm=route.get("algorithm"), params=lg["params_used"],
                                                 explanation=json.dumps(expl) if isinstance(expl, dict) else str(expl))
    if step == "P4":
        truth = (lg.get("verification") or {}).get("truth")
        return FINAL["P4_practice"], dict(algorithm=route.get("algorithm"), params=lg["params_used"],
                                          level=lg["level"], truth=truth if truth else "not available",
                                          explanation=json.dumps(lg.get("explanation")))
    raise ValueError(step)


def cmd_next(job: str) -> None:
    st = load(job)
    cp = current_prompt(st)
    if cp is None:
        print("DONE")
        return
    prompt, vars = cp
    s = prompt["settings"]
    print(f"STEP: {st['step']}")
    print("=== SYSTEM PROMPT ===")
    print(prompt["system"] or "(none)")
    print("=== USER PROMPT ===")
    print(fill(prompt["user"], **vars))
    print("=== OUTPUT FORMAT ===")
    print("JSON only (no markdown fences, no extra text)" if s.get("response_mime_type") == "application/json"
          else "plain text")
    save(st)


def cmd_answer(job: str, raw: str) -> None:
    st = load(job)
    step, lg = st["step"], st["log"]
    st["calls"].append({"step": step, "attempt": st.get("attempt", 0), "raw": raw})
    if st["kind"] == "version":
        if step == "EXPL":
            lg["explanation_raw"] = raw
            lg["json_clean"] = parse_json(raw)[1]
            lg["words"] = len(raw.split())
            st["step"] = "P3"
        else:
            claim, _ = parse_json(raw)
            claim = {**(claim if isinstance(claim, dict) else {}), "algorithm": st["algo"], "params": st["params"]}
            lg["claim"] = claim
            lg["verification"] = verify_claim(claim)
            st["step"] = "done"
        save(st)
        print("NEXT:", st["step"])
        return

    if step == "P1":
        route, lg["p1_json_clean"] = parse_json(raw)
        lg["route"] = route
        if not route:
            lg["final"] = "Router output was not valid JSON."
            st["step"] = "done"
        elif not route.get("in_scope"):
            lg["final"] = ("I can help with quantum algorithms (Bell states, Deutsch-Jozsa, Grover). "
                           "Please ask a question about one of them.")
            st["step"] = "done"
        else:
            lg["params_used"] = norm_params(route.get("algorithm"), route.get("params"))
            st["step"] = "P2"
    elif step == "P2":
        expl, clean = parse_json(raw)
        lg[f"p2_json_clean_{st['attempt']}"] = clean
        lg["explanation"] = expl or raw
        if lg["route"].get("algorithm") not in VERIFIABLE:
            lg["verification"] = {"ok": None, "note": "not verifiable by simulator; shown as unverified"}
            st["step"] = "P4"
        else:
            st["step"] = "P3"
    elif step == "P3":
        claim, lg[f"p3_json_clean_{st['attempt']}"] = parse_json(raw)
        claim = {**(claim if isinstance(claim, dict) else {}),
                 "algorithm": lg["route"].get("algorithm"), "params": lg["params_used"]}
        lg["claim"] = claim
        ver = verify_claim(claim)
        lg["verification"] = ver
        lg[f"verified_attempt_{st['attempt']}"] = ver["ok"]
        if ver["ok"] or st["attempt"] >= 1:
            st["step"] = "P4"
        else:
            st["attempt"] = 1
            st["feedback"] = ("Verifier feedback: a simulator found these errors in your previous answer: "
                              + "; ".join(ver["mismatches"]) + f". Correct result: {ver['truth']}. Fix the explanation.")
            st["step"] = "P2"
    elif step == "P4":
        lg["practice"], lg["p4_json_clean"] = parse_json(raw)
        st["step"] = "done"
    save(st)
    print("NEXT:", st["step"])


def cmd_finalize(label: str) -> None:
    from run_tests import auto_check, esc, short

    stamp = datetime.now().strftime("%Y%m%d_%H%M")
    states = {p.stem: json.loads(p.read_text(encoding="utf-8")) for p in STATE.glob("*.json")}

    tests = sorted((s for s in states.values() if s["kind"] == "test"), key=lambda s: int(s["test_id"]))
    table = [f"Model: {label} · run {stamp} · automatic checks only (confirm by hand)", "",
             "| Test no. | Input (short) | Type | Expected output | Actual output (short) | Pass / Fail |",
             "|---|---|---|---|---|---|"]
    with (OUT / f"run_claude_{stamp}.jsonl").open("w", encoding="utf-8") as f:
        for s in tests:
            row, log = ROWS[s["test_id"]], dict(s["log"], model=label)
            if s["step"] != "done":
                ok, why, verdict, actual = None, f"incomplete (stopped at {s['step']})", "Error", "not completed"
            else:
                ok, why = auto_check(row, log)
                verdict, actual = ("Pass" if ok else "Fail"), short(log)
            log.update({"test_id": s["test_id"], "type": row["type"], "auto_pass": ok, "auto_reason": why,
                        "llm_calls": s["calls"]})
            f.write(json.dumps(log, ensure_ascii=False) + "\n")
            table.append("| " + " | ".join(esc(c) for c in (row["id"], row["question"][:60], row["type"],
                                                         row["expected"], actual, verdict)) + " |")
            print(s["test_id"], verdict, why)
    (OUT / f"test_table_claude_{stamp}.md").write_text("\n".join(table) + "\n", encoding="utf-8")

    versions = {"model": label}
    for ver in ("v1", "v2", "v3"):
        runs = [s for s in states.values() if s["kind"] == "version" and s["version"] == ver]
        done = [s for s in runs if s["step"] == "done"]
        versions[ver] = {
            "runs": [{"test_id": s["test_id"], "rep": s["rep"], "json_clean": s["log"].get("json_clean"),
                      "verified": s["log"]["verification"]["ok"],
                      "claim": (s["log"].get("claim") or {}).get("probabilities"),
                      "mismatches": s["log"]["verification"]["mismatches"], "words": s["log"].get("words"),
                      "output": s["log"].get("explanation_raw")} for s in done],
            "summary": {"completed_runs": len(done), "incomplete": len(runs) - len(done),
                        "verified_correct": sum(bool(s["log"]["verification"]["ok"]) for s in done),
                        "stated_final_probabilities": sum(bool((s["log"].get("claim") or {}).get("probabilities"))
                                                          for s in done),
                        "clean_json": sum(bool(s["log"].get("json_clean")) for s in done),
                        "avg_words": round(sum(s["log"].get("words", 0) for s in done) / len(done)) if done else None},
        }
        print(ver, versions[ver]["summary"])
    (OUT / f"versions_claude_{stamp}.json").write_text(json.dumps(versions, indent=2, ensure_ascii=False),
                                                       encoding="utf-8")
    print("saved", OUT)


if __name__ == "__main__":
    c = sys.argv[1]
    if c == "next":
        cmd_next(sys.argv[2])
    elif c == "answer":
        job = sys.argv[2]
        raw = Path(sys.argv[4]).read_text(encoding="utf-8") if "--file" in sys.argv else sys.stdin.read()
        cmd_answer(job, raw)
    elif c == "finalize":
        cmd_finalize(" ".join(sys.argv[2:]) or "claude")
    else:
        raise SystemExit(__doc__)
