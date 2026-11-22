"""Measure explainer versions v1, v2, v3 on the same verifiable questions.

For each version and question, runs the explainer REPEATS times, extracts the claimed
probabilities with P3, and checks them with the simulator. Gives the real numbers for the
iteration log (Section 3). The final version is measured by run_tests.py (full pipeline).
Results are written after every version to test-data/outputs/versions_<timestamp>.json.
"""
from __future__ import annotations

import csv
import json
import time
from datetime import datetime

from quantum_sim import verify_claim
from workflow import FINAL, MODEL, PROMPTS, ROOT, call, load_prompt, models_used, parse_json

REPEATS = 3
OUT = ROOT / "test-data" / "outputs"
VERSIONS = {
    "v1": PROMPTS / "v1" / "explainer_v1.txt",
    "v2": PROMPTS / "v2" / "explainer_v2.txt",
    "v3": PROMPTS / "v3" / "explainer_v3.txt",
}
# Router output is fixed here so only the explainer differs between versions.
CASES = [
    ("1", "grover2", {"marked": "11"}),
    ("3", "deutsch_jozsa", {"n": 2, "f_kind": "balanced"}),
    ("5", "bell", {}),
    ("7", "grover2", {"marked": "11"}),
]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    out = OUT / f"versions_{datetime.now():%Y%m%d_%H%M}.json"
    rows = {r["id"]: r for r in csv.DictReader((ROOT / "test-data" / "test_cases.csv").open(encoding="utf-8"))}
    results = {"model": MODEL, "repeats": REPEATS, "cases": [c[0] for c in CASES]}
    for ver, path in VERSIONS.items():
        prompt = load_prompt(path)
        runs = []
        for tid, algo, params in CASES:
            row = rows[tid]
            for rep in range(REPEATS):
                try:
                    raw, t = call(prompt, question=row["question"], level=row["level"], algorithm=algo,
                                  params=params, false_premise="none")
                    _, json_clean = parse_json(raw)
                    raw3, _ = call(FINAL["P3_claim_extractor"], algorithm=algo, params=params, explanation=raw)
                    claim, _ = parse_json(raw3)
                    claim = {**(claim if isinstance(claim, dict) else {}), "algorithm": algo, "params": params}
                    v = verify_claim(claim)
                    runs.append({"test_id": tid, "rep": rep, "json_clean": json_clean, "verified": v["ok"],
                                 "found_numbers": bool(claim.get("probabilities")), "claim": claim.get("probabilities"),
                                 "mismatches": v["mismatches"], "latency_s": t, "words": len(raw.split()),
                                 "output": raw})
                except Exception as e:  # API error: recorded separately, not counted as a prompt failure
                    runs.append({"test_id": tid, "rep": rep, "error": repr(e)})
                    print(ver, tid, rep, "ERROR", repr(e)[:100])
                time.sleep(1)
        ok_runs = [r for r in runs if "error" not in r]
        n = len(ok_runs)
        results[ver] = {
            "runs": runs,
            "summary": {
                "completed_runs": n,
                "api_errors": len(runs) - n,
                "verified_correct": sum(r["verified"] for r in ok_runs),
                "stated_final_probabilities": sum(r["found_numbers"] for r in ok_runs),
                "clean_json": sum(r["json_clean"] for r in ok_runs),
                "avg_words": round(sum(r["words"] for r in ok_runs) / n) if n else None,
                "avg_latency_s": round(sum(r["latency_s"] for r in ok_runs) / n, 2) if n else None,
            },
        }
        print(ver, results[ver]["summary"])
        results["model"] = models_used()  # models that actually answered so far
        out.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding="utf-8")  # save as we go
    print("saved", out)


if __name__ == "__main__":
    main()
