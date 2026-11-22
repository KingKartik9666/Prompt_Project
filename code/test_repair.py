"""Control-flow test for the repair retry (no API key, no LLM calls).

In both real test runs every verifiable answer was verified on the first attempt, so the repair
retry never fired. This test replaces the model with scripted answers to check the code path:
P2 attempt 0 claims a wrong result -> the simulator flags it -> P2 is called again with the
verifier feedback -> attempt 1 is verified. It does NOT show that a real model can fix its answer.

Run:  py test_repair.py
"""
from __future__ import annotations

import json

import workflow

WRONG = {"goal": "g", "steps": [], "final_result": "Measuring gives 01 with probability 0.5 and 11 with probability 0.5.",
         "analogy": "", "misconception": "", "uncertain": []}
RIGHT = {**WRONG, "final_result": "Measuring gives 11 with probability 1.0 after one iteration."}
SCRIPT = {
    "P1_router": [json.dumps({"in_scope": True, "algorithm": "grover2", "params": {"marked": "11"}, "intent": "explain",
                              "false_premise": None, "language": "english", "injection_attempt": False, "reason": "test"})],
    "P2_explainer": [json.dumps(WRONG), json.dumps(RIGHT)],
    "P3_claim_extractor": [json.dumps({"probabilities": {"01": 0.5, "11": 0.5}, "found_in_text": True}),
                           json.dumps({"probabilities": {"11": 1.0}, "found_in_text": True})],
    "P4_practice": [json.dumps({"question": "q", "type": "conceptual", "hint": "h", "answer_key": "a", "review": {}})],
}
seen: list[tuple[str, dict]] = []


def fake_call(prompt: dict, **vars):
    name = next(k for k, v in workflow.FINAL.items() if v is prompt)
    seen.append((name, vars))
    return SCRIPT[name].pop(0), 0.0


def main() -> None:
    workflow.call = fake_call
    log = workflow.run("How does Grover's algorithm find item 11 among 4 items?", "beginner")
    p2_calls = [v for n, v in seen if n == "P2_explainer"]
    checks = {
        "attempt 0 flagged by simulator": log.get("verified_attempt_0") is False,
        "P2 called twice": len(p2_calls) == 2,
        "attempt 0 had no feedback": p2_calls[0]["verifier_feedback"] == "",
        "attempt 1 got verifier feedback": p2_calls[1]["verifier_feedback"].startswith("Verifier feedback:"),
        "attempt 1 verified": log.get("verified_attempt_1") is True,
        "P4 ran after repair": "practice" in log,
    }
    print("feedback sent on attempt 1:", p2_calls[1]["verifier_feedback"])
    for k, ok in checks.items():
        print(("PASS " if ok else "FAIL ") + k)
    print("ALL PASS" if all(checks.values()) else "SOME CHECKS FAILED")


if __name__ == "__main__":
    main()
